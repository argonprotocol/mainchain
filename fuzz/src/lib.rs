#![cfg(feature = "fuzzing")]

use argon_primitives::{
	bitcoin::{
		BitcoinNetwork, CompressedBitcoinPubkey, H256Le, OpaqueBitcoinXpub, UtxoRef,
		SATOSHIS_PER_BITCOIN,
	},
	block_seal::MiningSlotConfig,
	tick::{TickDigest, Ticker},
	vault::VaultTerms,
	AccountId, BlockNumber, Digestset, PriceIndex as Prices, PriceProvider,
};
use argon_runtime::{
	AllPalletsWithSystem, BitcoinFissions, BitcoinLocks, BitcoinUtxos, BlockSeal, Executive,
	Header, Notebook, NotebookVerifyError, PriceIndex, Runtime, RuntimeCall, RuntimeGenesisConfig,
	RuntimeOrigin, System, Ticks, Timestamp, Vaults,
};
use bitcoin::{
	bip32::{ChildNumber, Xpriv, Xpub},
	secp256k1::{Secp256k1, SecretKey},
	Network,
};
use pallet_bitcoin_fissions::{FissionByOwnerAndId, NextFissionIdByOwner};
use pallet_bitcoin_locks::LocksById;
use polkadot_sdk::{
	frame_support::{
		storage::with_storage_layer,
		traits::{TryState, TryStateSelect},
	},
	sp_io::TestExternalities,
	sp_runtime::{
		traits::{Dispatchable, Header as HeaderT},
		BuildStorage, FixedU128, StateVersion,
	},
};

pub const CREATE: u8 = 0;
pub const RATCHET: u8 = 1;
pub const CLOSE: u8 = 2;
pub const SET_GUARANTEE: u8 = 3;
pub const REPRICE: u8 = 4;
pub const OPERATIONS: [&str; 5] = ["create", "ratchet", "close", "set-guarantee", "reprice"];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LifecycleStats {
	pub successful: [u32; 5],
	pub rejected: [u32; 5],
}

/// Based on the real-pallet liquid lifecycle and outside-spend examples in Vaults' tests.
/// Each input starts with funded Bitcoin and an active Fission, then mutates three-byte actions:
/// operation, existing-Fission selector (high bit selects a wrong owner), and amount/price choice.
/// Verified Bitcoin observations enter at the UTXO tracker's production callback boundary;
/// external Bitcoin verification and extrinsic signatures, nonces and fees are outside this target.
pub fn run(data: &[u8]) -> LifecycleStats {
	let operator = AccountId::new([1; 32]);
	let owner = AccountId::new([2; 32]);
	let attacker = AccountId::new([3; 32]);
	let mut genesis = RuntimeGenesisConfig::default();
	genesis.balances.balances = [operator.clone(), owner.clone(), attacker.clone()]
		.into_iter()
		.map(|account| (account, 1_000_000_000_000))
		.collect();
	genesis.ownership.balances = genesis.balances.balances.clone();
	genesis.ticks.ticker = Ticker::new(60_000, 60);
	genesis.operational_accounts.is_operational_account_invite_only = false;
	genesis.bitcoin_utxos.network = BitcoinNetwork::Regtest;
	genesis.price_index.operator = Some(operator.clone());
	genesis.mining_slot.mining_config = MiningSlotConfig {
		ticks_before_bid_end_for_vrf_close: 200,
		ticks_between_slots: 1_440,
		slot_bidding_start_after_ticks: 12_960,
	};

	let storage = genesis.build_storage().expect("Bitcoin fuzz genesis must build");
	TestExternalities::new(storage).execute_with(|| {
		initialize_block(1, Default::default(), &operator);
		set_price(100_000, &operator);
		let secp = Secp256k1::new();
		// Deterministic fixture keys, matching the existing Regtest examples; never production
		// keys.
		let master = Xpriv::new_master(Network::Regtest, &[1; 32]).unwrap();
		let key = master
			.derive_priv(&secp, &[ChildNumber::from_hardened_idx(1).unwrap()])
			.unwrap();
		Vaults::create(
			RuntimeOrigin::signed(operator.clone()),
			pallet_vaults::VaultConfig {
				terms: VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::from_rational(1, 100),
					bitcoin_base_fee: 0,
				},
				delegate_account_id: None,
				securitization: 500_000_000_000,
				bitcoin_xpubkey: OpaqueBitcoinXpub(Xpub::from_priv(&secp, &key).encode()),
			},
		)
		.expect("Bitcoin fuzz Vault must be created");
		let header = data.first().copied().unwrap_or_default();
		let guarantee =
			if header & 1 == 0 { SATOSHIS_PER_BITCOIN / 10 } else { SATOSHIS_PER_BITCOIN };
		BitcoinLocks::create_receive_address(
			RuntimeOrigin::signed(owner.clone()),
			1,
			guarantee,
			CompressedBitcoinPubkey(
				bitcoin::secp256k1::PublicKey::from_secret_key(
					&secp,
					&SecretKey::from_slice(&[2; 32]).unwrap(),
				)
				.serialize(),
			),
			None,
		)
		.expect("Bitcoin fuzz Lock must be created");
		let funding = UtxoRef { txid: H256Le([4; 32]), output_index: 0 };
		with_storage_layer(|| {
			BitcoinUtxos::utxo_detected(1, funding.clone(), SATOSHIS_PER_BITCOIN, 0)
		})
		.expect("Bitcoin fuzz funding must be accepted");
		BitcoinFissions::create(
			RuntimeOrigin::signed(owner.clone()),
			0,
			77,
			1,
			SATOSHIS_PER_BITCOIN / 4,
			100_000_000_000,
			None,
		)
		.expect("Bitcoin fuzz initial Fission must be created");
		let mut stats = LifecycleStats::default();
		stats.successful[CREATE as usize] = 1;
		check_state();

		for action in data.get(1..).unwrap_or_default().chunks_exact(3).take(32) {
			let operation = action[0] % (REPRICE + 1);
			if operation == REPRICE {
				set_price(50_000 + u32::from(action[2] % 6) * 25_000, &operator);
				stats.successful[REPRICE as usize] += 1;
				continue;
			}
			advance_block(&operator);
			let signer = if action[1] & 0x80 == 0 { owner.clone() } else { attacker.clone() };
			let ids: Vec<_> =
				FissionByOwnerAndId::<Runtime>::iter_prefix(&owner).map(|(id, _)| id).collect();
			let fission_id =
				ids.get(usize::from(action[1]) % ids.len().max(1)).copied().unwrap_or_default();
			let call =
				match operation {
					CREATE => {
						let lock = LocksById::<Runtime>::get(1)
							.expect("source Lock remains until settlement");
						let available = lock.funded_satoshis - lock.fissioned_satoshis;
						let satoshis = match action[2] % 6 {
							0 => 1,
							1 => available / 4,
							2 => available / 2,
							3 => available,
							4 => available + 1,
							_ => 0,
						};
						RuntimeCall::BitcoinFissions(pallet_bitcoin_fissions::Call::create {
							fission_id: NextFissionIdByOwner::<Runtime>::get(&signer),
							liquid_id: 77,
							lock_id: 1,
							satoshis,
							microgons_at_target_per_btc:
								PriceIndex::get_btc_price_in_target_microgons(SATOSHIS_PER_BITCOIN)
									.unwrap(),
							fee_coupon: None,
						})
					},
					RATCHET => {
						set_price(50_000 + u32::from(action[2] % 6) * 25_000, &operator);
						RuntimeCall::BitcoinFissions(pallet_bitcoin_fissions::Call::ratchet {
							fission_id,
							microgons_at_target_per_btc:
								PriceIndex::get_btc_price_in_target_microgons(SATOSHIS_PER_BITCOIN)
									.unwrap(),
							fee_coupon: None,
						})
					},
					CLOSE => RuntimeCall::BitcoinFissions(pallet_bitcoin_fissions::Call::close {
						fission_id,
					}),
					SET_GUARANTEE =>
						RuntimeCall::BitcoinLocks(pallet_bitcoin_locks::Call::set_guarantee {
							lock_id: 1,
							satoshis: match action[2] % 4 {
								0 => 0,
								1 => SATOSHIS_PER_BITCOIN / 10,
								2 => SATOSHIS_PER_BITCOIN / 2,
								_ => SATOSHIS_PER_BITCOIN,
							},
							options: None,
						}),
					_ => unreachable!(),
				};
			let before = polkadot_sdk::sp_io::storage::root(StateVersion::V1);
			let result = call.dispatch(RuntimeOrigin::signed(signer.clone()));
			if signer == attacker {
				assert!(result.is_err(), "wrong owner changed a Bitcoin position");
			}
			if result.is_ok() {
				stats.successful[operation as usize] += 1;
			} else {
				stats.rejected[operation as usize] += 1;
				assert_eq!(
					polkadot_sdk::sp_io::storage::root(StateVersion::V1),
					before,
					"failed Bitcoin action changed storage"
				);
			}
			check_state();
		}

		set_price(100_000, &operator);
		if header & 2 != 0 {
			for (id, _) in FissionByOwnerAndId::<Runtime>::iter_prefix(&owner).collect::<Vec<_>>() {
				let before = polkadot_sdk::sp_io::storage::root(StateVersion::V1);
				let result = BitcoinFissions::close(RuntimeOrigin::signed(owner.clone()), id);
				if result.is_ok() {
					stats.successful[CLOSE as usize] += 1;
				} else {
					assert_eq!(
						result,
						Err(pallet_bitcoin_fissions::Error::<Runtime>::InsufficientFunds.into()),
						"terminal close failed unexpectedly"
					);
					stats.rejected[CLOSE as usize] += 1;
					assert_eq!(
						polkadot_sdk::sp_io::storage::root(StateVersion::V1),
						before,
						"failed terminal close changed storage"
					);
				}
				check_state();
			}
		}
		with_storage_layer(|| BitcoinUtxos::utxo_spent(1, Some(funding), 0))
			.expect("Bitcoin fuzz settlement must succeed");
		assert!(!LocksById::<Runtime>::contains_key(1));
		assert_eq!(FissionByOwnerAndId::<Runtime>::iter_prefix(&owner).count(), 0);
		check_state();
		Executive::finalize_block();
		check_state();
		stats
	})
}

fn set_price(bitcoin_usd: u32, operator: &AccountId) {
	PriceIndex::submit(
		RuntimeOrigin::signed(operator.clone()),
		Prices {
			btc_usd_price: FixedU128::from_u32(bitcoin_usd),
			argonot_usd_price: FixedU128::from_u32(1),
			argon_usd_price: FixedU128::from_u32(1),
			argon_usd_target_price: FixedU128::from_u32(1),
			argon_time_weighted_average_liquidity: 1_000_000,
			tick: Ticks::current_tick(),
			usdc_usd_price: Some(FixedU128::from_u32(1)),
			stabilizer_pool_sqrt_price_x96: None,
		},
		None,
	)
	.expect("Bitcoin fuzz price must be accepted");
	advance_block(operator);
}

fn advance_block(author: &AccountId) {
	let parent_hash = Executive::finalize_block().hash();
	check_state();
	initialize_block(System::block_number() + 1, parent_hash, author);
	check_state();
}

fn initialize_block(
	number: BlockNumber,
	parent_hash: polkadot_sdk::sp_core::H256,
	author: &AccountId,
) {
	let digests: Digestset<NotebookVerifyError, AccountId> = Digestset {
		author: author.clone(),
		block_vote: Default::default(),
		voting_key: None,
		fork_power: None,
		frame_info: None,
		tick: TickDigest(u64::from(number)),
		notebooks: Default::default(),
	};
	let header = Header::new(
		number,
		Default::default(),
		Default::default(),
		parent_hash,
		digests.create_pre_runtime_digest(),
	);
	Executive::initialize_block(&header);
	Timestamp::set(RuntimeOrigin::none(), u64::from(number) * 60_000)
		.expect("fuzz timestamp must match tick");
	Notebook::submit(RuntimeOrigin::none(), Vec::new()).expect("fuzz notebook inherent must apply");
	BlockSeal::apply(
		RuntimeOrigin::none(),
		argon_primitives::inherents::BlockSealInherent::Compute,
	)
	.expect("fuzz compute seal must apply");
}

fn check_state() {
	AllPalletsWithSystem::try_state(System::block_number(), TryStateSelect::All)
		.expect("Bitcoin runtime invariant failed");
	argon_runtime::try_state::check::<Runtime>().expect("Bitcoin backing invariant failed");
}
