use std::collections::BTreeMap;

use bitcoin::PublicKey;
use env_logger::{Builder, Env};
use frame_support::{
	derive_impl, parameter_types, traits::Currency, weights::constants::RocksDbWeight,
};
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::{ConstU32, ConstU64, H256};
use sp_runtime::{BuildStorage, DispatchError, DispatchResult};

use crate as pallet_bond;
use crate::BitcoinVerifier;
use argon_bitcoin::UtxoUnlocker;
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, NetworkKind, Satoshis, UtxoId, UtxoRef,
	},
	ensure,
	tick::{Tick, Ticker},
	vault::{Bond, BondError, BondType, Vault, VaultArgons, VaultProvider},
	BitcoinUtxoTracker, PriceProvider, TickProvider, UtxoBondedEvents, VaultId, VotingSchedule,
};

pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Bonds: pallet_bond
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
	pub static MinimumBondAmount:u128 = 1_000_000;
	pub const BlocksPerYear:u32 = 1440*365;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

parameter_types! {
	pub static MaxUnlockingUtxos: u32 = 10;
	pub static BitcoinPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(62000.00));
	pub static ArgonPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(1.00));
	pub static ArgonCPI: Option<argon_primitives::ArgonCPI> = Some(FixedI128::from_float(0.1));
	pub static UtxoUnlockCosignDeadlineBlocks: BitcoinHeight = 5;
	pub static BitcoinBondReclamationBlocks: BitcoinHeight = 30;
	pub static BitcoinBondDurationBlocks: BitcoinHeight = 365;
	pub static BitcoinBlockHeight: BitcoinHeight = 0;
	pub static MinimumBondSatoshis: Satoshis = 10_000_000;
	pub static DefaultVault: Vault<u64, Balance> = Vault {
		bonded_argons: VaultArgons {
			allocated: 100_000_000_000,
			reserved: 0,
			annual_percent_rate: FixedU128::from_float(10.0),
			base_fee: 0,
		},
		bitcoin_argons: VaultArgons {
			allocated: 200_000_000_000,
			reserved: 0,
			annual_percent_rate: FixedU128::from_float(10.0),
			base_fee: 0,
		},
		operator_account_id: 1,
		added_securitization_percent: FixedU128::from_float(0.0),
		mining_reward_sharing_percent_take: FixedU128::from_float(0.0),
		added_securitization_argons: 0,
		is_closed: false,
		pending_terms: None,
		pending_bonded_argons: None,
		pending_bitcoins: 0,
	};

	pub static NextUtxoId: UtxoId = 1;
	pub static WatchedUtxosById: BTreeMap<UtxoId, (BitcoinCosignScriptPubkey, Satoshis, BitcoinHeight)> = BTreeMap::new();

	pub static GetUtxoRef: Option<UtxoRef> = None;

	pub static LastBondEvent: Option<(UtxoId, u64, Balance)> = None;
	pub static LastUnlockEvent: Option<(UtxoId, bool, Balance)> = None;

	pub static GetBitcoinNetwork: BitcoinNetwork = BitcoinNetwork::Regtest;

	pub static DefaultVaultBitcoinPubkey: PublicKey = "02e3af28965693b9ce1228f9d468149b831d6a0540b25e8a9900f71372c11fb277".parse::<PublicKey>().unwrap();
	pub static DefaultVaultReclaimBitcoinPubkey: PublicKey = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766".parse::<PublicKey>().unwrap();

	pub static MockFeeResult: (Balance, Balance) = (0, 0);

	pub static CurrentTick: Tick = 2;
	pub static PreviousTick: Tick = 1;
	pub static ElapsedTicks: Tick = 0;
}

pub struct EventHandler;
impl UtxoBondedEvents<u64, Balance> for EventHandler {
	fn utxo_bonded(
		utxo_id: UtxoId,
		account_id: &u64,
		amount: Balance,
	) -> Result<(), DispatchError> {
		LastBondEvent::set(Some((utxo_id, *account_id, amount)));
		Ok(())
	}
	fn utxo_unlocked(
		utxo_id: UtxoId,
		remove_pending_mints: bool,
		amount_burned: Balance,
	) -> DispatchResult {
		LastUnlockEvent::set(Some((utxo_id, remove_pending_mints, amount_burned)));
		Ok(())
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		ArgonCPI::get()
	}
	fn get_latest_argon_price_in_us_cents() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_latest_btc_price_in_us_cents() -> Option<FixedU128> {
		BitcoinPricePerUsd::get()
	}
}

pub struct StaticVaultProvider;

impl VaultProvider for StaticVaultProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn get(vault_id: VaultId) -> Option<Vault<Self::AccountId, Self::Balance>> {
		if vault_id == 1 {
			Some(DefaultVault::get())
		} else {
			None
		}
	}

	fn compensate_lost_bitcoin(
		bond: &mut Bond<Self::AccountId, Self::Balance>,
		market_rate: Self::Balance,
		redemption_rate: Self::Balance,
	) -> Result<(Self::Balance, Self::Balance), BondError> {
		let rate = redemption_rate.min(redemption_rate);
		DefaultVault::mutate(|a| {
			a.bitcoin_argons.destroy_funds(market_rate).expect("should not fail");
		});
		bond.amount -= rate;
		Ok((0, rate))
	}

	fn burn_vault_bitcoin_funds(
		_bond: &Bond<Self::AccountId, Self::Balance>,
		amount_to_burn: Self::Balance,
	) -> Result<(), BondError> {
		DefaultVault::mutate(|a| {
			a.bitcoin_argons.destroy_funds(amount_to_burn).expect("should not fail")
		});

		Ok(())
	}

	fn bond_funds(
		_vault_id: VaultId,
		amount: Self::Balance,
		bond_type: BondType,
		_ticks: Tick,
		_bond_account_id: &Self::AccountId,
	) -> Result<(Self::Balance, Self::Balance), BondError> {
		ensure!(
			DefaultVault::get().mut_argons(&bond_type).allocated >= amount,
			BondError::InsufficientVaultFunds
		);
		DefaultVault::mutate(|a| a.mut_argons(&bond_type).reserved += amount);
		Ok(MockFeeResult::get())
	}

	fn release_bonded_funds(
		bond: &Bond<Self::AccountId, Self::Balance>,
	) -> Result<Self::Balance, BondError> {
		DefaultVault::mutate(|a| a.mut_argons(&bond.bond_type).reduce_reserved(bond.amount));
		Ok(bond.total_fee.saturating_sub(bond.prepaid_fee))
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_utxo_id: UtxoId,
		_owner_pubkey: CompressedBitcoinPubkey,
		_vault_claim_height: BitcoinHeight,
		_open_claim_height: BitcoinHeight,
		_current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), BondError> {
		Ok((
			BitcoinXPub {
				public_key: DefaultVaultBitcoinPubkey::get().into(),
				chain_code: [0; 32],
				depth: 0,
				parent_fingerprint: [0; 4],
				child_number: 0,
				network: NetworkKind::Test,
			},
			BitcoinXPub {
				public_key: DefaultVaultReclaimBitcoinPubkey::get().into(),
				chain_code: [0; 32],
				depth: 0,
				parent_fingerprint: [0; 4],
				child_number: 1,
				network: NetworkKind::Test,
			},
			BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::from([0; 32]) },
		))
	}

	fn modify_pending_bitcoin_funds(
		_vault_id: VaultId,
		_amount: Self::Balance,
		_remove_pending: bool,
	) -> Result<(), BondError> {
		Ok(())
	}
}

pub struct StaticBitcoinVerifier;
impl BitcoinVerifier<Test> for StaticBitcoinVerifier {
	fn verify_signature(
		_utxo_unlocker: UtxoUnlocker,
		_pubkey: CompressedBitcoinPubkey,
		_signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}

pub struct StaticBitcoinUtxoTracker;
impl BitcoinUtxoTracker for StaticBitcoinUtxoTracker {
	fn new_utxo_id() -> UtxoId {
		let id = NextUtxoId::get();
		NextUtxoId::set(id + 1);
		id
	}

	fn get(_utxo_id: UtxoId) -> Option<UtxoRef> {
		GetUtxoRef::get()
	}

	fn watch_for_utxo(
		utxo_id: UtxoId,
		script_pubkey: BitcoinCosignScriptPubkey,
		satoshis: Satoshis,
		watch_for_spent_until: BitcoinHeight,
	) -> Result<(), DispatchError> {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.insert(utxo_id, (script_pubkey, satoshis, watch_for_spent_until));
		});
		Ok(())
	}

	fn unwatch(utxo_id: UtxoId) {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.remove(&utxo_id);
		});
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn previous_tick() -> Tick {
		PreviousTick::get()
	}
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn voting_schedule() -> VotingSchedule {
		todo!()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 2)
	}
	fn elapsed_ticks() -> Tick {
		ElapsedTicks::get()
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
}

impl pallet_bond::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type ArgonTicksPerDay = ConstU64<1440>;
	type MinimumBondAmount = MinimumBondAmount;
	type MaxConcurrentlyExpiringBonds = ConstU32<10>;
	type BondEvents = EventHandler;
	type PriceProvider = StaticPriceProvider;
	type VaultProvider = StaticVaultProvider;
	type MaxUnlockingUtxos = MaxUnlockingUtxos;
	type UtxoUnlockCosignDeadlineBlocks = UtxoUnlockCosignDeadlineBlocks;
	type BitcoinUtxoTracker = StaticBitcoinUtxoTracker;
	type BitcoinBondReclamationBlocks = BitcoinBondReclamationBlocks;
	type BitcoinBondDurationBlocks = BitcoinBondDurationBlocks;
	type BitcoinBlockHeight = BitcoinBlockHeight;
	type BitcoinSignatureVerifier = StaticBitcoinVerifier;
	type GetBitcoinNetwork = GetBitcoinNetwork;
	type TickProvider = StaticTickProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_bond::GenesisConfig::<Test> {
		minimum_bitcoin_bond_satoshis: MinimumBondSatoshis::get(),
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
