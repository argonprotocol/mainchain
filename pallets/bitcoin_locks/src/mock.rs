use std::collections::BTreeMap;

use bitcoin::PublicKey;
use env_logger::{Builder, Env};
use frame_support::{
	derive_impl, parameter_types, traits::Currency, weights::constants::RocksDbWeight,
};
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::{ConstU32, ConstU64, H256};
use sp_runtime::{BuildStorage, DispatchError, DispatchResult};

use crate as pallet_bitcoin_locks;
use crate::BitcoinVerifier;
use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, NetworkKind, Satoshis, UtxoId, UtxoRef,
	},
	ensure,
	tick::{Tick, Ticker},
	vault::{
		BitcoinObligationProvider, FundType, Obligation, ObligationError, ObligationExpiration,
		ReleaseFundsResult, Vault, VaultArgons, VaultTerms,
	},
	BitcoinUtxoTracker, ObligationEvents, ObligationId, PriceProvider, TickProvider,
	UtxoLockEvents, VaultId, VotingSchedule,
};

pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		BitcoinLocks: pallet_bitcoin_locks
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
	pub static MaxConcurrentlyReleasingLocks: u32 = 10;
	pub static BitcoinPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(62000.00));
	pub static ArgonPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(1.00));
	pub static ArgonCPI: Option<argon_primitives::ArgonCPI> = Some(FixedI128::from_float(0.1));
	pub static LockReleaseCosignDeadlineBlocks: BitcoinHeight = 5;
	pub static LockReclamationBlocks: BitcoinHeight = 30;
	pub static LockDurationBlocks: BitcoinHeight = 365;
	pub static BitcoinBlockHeight: BitcoinHeight = 0;
	pub static MinimumLockSatoshis: Satoshis = 10_000_000;
	pub static DefaultVault: Vault<u64, Balance> = Vault {
		bonded_bitcoin_argons : VaultArgons {
			allocated: 100_000_000_000,
			reserved: 0,
		},
		locked_bitcoin_argons: VaultArgons {
			allocated: 200_000_000_000,
			reserved: 0,
		},
		terms: VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_float(10.0),
			bitcoin_base_fee: 0,
		},
		activation_tick: 1,
		operator_account_id: 1,
		added_securitization_percent: FixedU128::from_float(0.0),
		added_securitization_argons: 0,
		is_closed: false,
		pending_terms: None,
		pending_bitcoins: 0,
	};

	pub static NextUtxoId: UtxoId = 1;
	pub static NextObligationId: ObligationId = 1;
	pub static WatchedUtxosById: BTreeMap<UtxoId, (BitcoinCosignScriptPubkey, Satoshis, BitcoinHeight)> = BTreeMap::new();

	pub static GetUtxoRef: Option<UtxoRef> = None;

	pub static LastLockEvent: Option<(UtxoId, u64, Balance)> = None;
	pub static LastReleaseEvent: Option<(UtxoId, bool, Balance)> = None;

	pub static GetBitcoinNetwork: BitcoinNetwork = BitcoinNetwork::Regtest;

	pub static DefaultVaultBitcoinPubkey: PublicKey = "02e3af28965693b9ce1228f9d468149b831d6a0540b25e8a9900f71372c11fb277".parse::<PublicKey>().unwrap();
	pub static DefaultVaultReclaimBitcoinPubkey: PublicKey = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766".parse::<PublicKey>().unwrap();

	pub static MockFeeResult: (Balance, Balance) = (0, 0);

	pub static CurrentTick: Tick = 2;
	pub static PreviousTick: Tick = 1;
	pub static ElapsedTicks: Tick = 0;

	pub static CanceledObligations: Vec<ObligationId> = vec![];

	pub static Obligations: BTreeMap<ObligationId, Obligation<u64, Balance>> = BTreeMap::new();
}

pub struct EventHandler;
impl UtxoLockEvents<u64, Balance> for EventHandler {
	fn utxo_locked(
		utxo_id: UtxoId,
		account_id: &u64,
		amount: Balance,
	) -> Result<(), DispatchError> {
		LastLockEvent::set(Some((utxo_id, *account_id, amount)));
		Ok(())
	}
	fn utxo_released(
		utxo_id: UtxoId,
		remove_pending_mints: bool,
		amount_burned: Balance,
	) -> DispatchResult {
		LastReleaseEvent::set(Some((utxo_id, remove_pending_mints, amount_burned)));

		Ok(())
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	fn get_latest_btc_price_in_us_cents() -> Option<FixedU128> {
		BitcoinPricePerUsd::get()
	}
	fn get_latest_argon_price_in_us_cents() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		ArgonCPI::get()
	}
	fn get_argon_pool_liquidity() -> Option<Balance> {
		todo!()
	}
}

pub struct StaticVaultProvider;

impl BitcoinObligationProvider for StaticVaultProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		if vault_id == 1 {
			return DefaultVault::get().operator_account_id == *account_id
		}
		false
	}

	fn cancel_obligation(
		obligation_id: ObligationId,
	) -> Result<ReleaseFundsResult<Self::Balance>, ObligationError> {
		CanceledObligations::mutate(|a| a.push(obligation_id));
		Obligations::mutate(|a| {
			let obligation = a.remove(&obligation_id).expect("should exist");
			let _ = BitcoinLocks::on_canceled(&obligation);
			DefaultVault::mutate(|v| {
				v.mut_argons(&obligation.fund_type).reserved -= obligation.amount;
			});
		});
		Ok(ReleaseFundsResult { returned_to_beneficiary: 0u128, paid_to_vault: 0u128 })
	}

	fn create_obligation(
		vault_id: VaultId,
		beneficiary: &Self::AccountId,
		amount: Self::Balance,
		expiration: BitcoinHeight,
		_ticks: Tick,
	) -> Result<Obligation<Self::AccountId, Self::Balance>, ObligationError> {
		let fund_type = FundType::LockedBitcoin;
		ensure!(
			DefaultVault::get().mut_argons(&fund_type).allocated >= amount,
			ObligationError::InsufficientVaultFunds
		);
		DefaultVault::mutate(|a| a.mut_argons(&fund_type).reserved += amount);
		let next_obligation_id = NextObligationId::mutate(|a| {
			let id = *a;
			*a += 1;
			id
		});
		let (total_fee, prepaid) = MockFeeResult::get();
		let obligation = Obligation {
			obligation_id: next_obligation_id,
			prepaid_fee: prepaid,
			total_fee,
			beneficiary: *beneficiary,
			amount,
			fund_type,
			vault_id,
			expiration: ObligationExpiration::BitcoinBlock(expiration),
			start_tick: CurrentTick::get(),
			bitcoin_annual_percent_rate: None,
		};
		Obligations::mutate(|a| a.insert(next_obligation_id, obligation.clone()));
		Ok(obligation)
	}

	fn compensate_lost_bitcoin(
		_obligation_id: ObligationId,
		market_rate: Self::Balance,
		release_amount_paid: Self::Balance,
	) -> Result<(Self::Balance, Self::Balance), ObligationError> {
		let rate = release_amount_paid.min(market_rate);
		DefaultVault::mutate(|a| {
			a.locked_bitcoin_argons.destroy_funds(market_rate).expect("should not fail");
		});
		Ok((0, rate))
	}

	fn burn_vault_bitcoin_obligation(
		obligation_id: ObligationId,
		amount_to_burn: Self::Balance,
	) -> Result<Obligation<Self::AccountId, Self::Balance>, ObligationError> {
		DefaultVault::mutate(|a| {
			a.locked_bitcoin_argons.destroy_funds(amount_to_burn).expect("should not fail")
		});

		let mut obligation = Obligations::get().get(&obligation_id).cloned().unwrap();
		obligation.amount = amount_to_burn;

		Ok(obligation)
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_owner_pubkey: CompressedBitcoinPubkey,
		_vault_claim_height: BitcoinHeight,
		_open_claim_height: BitcoinHeight,
		_current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), ObligationError> {
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
	) -> Result<(), ObligationError> {
		Ok(())
	}
}

pub struct StaticBitcoinVerifier;
impl BitcoinVerifier<Test> for StaticBitcoinVerifier {
	fn verify_signature(
		_utxo_releaseer: CosignReleaser,
		_pubkey: CompressedBitcoinPubkey,
		_signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}

pub struct StaticBitcoinUtxoTracker;
impl BitcoinUtxoTracker for StaticBitcoinUtxoTracker {
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
	fn elapsed_ticks() -> Tick {
		ElapsedTicks::get()
	}
	fn voting_schedule() -> VotingSchedule {
		todo!()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 2)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
}

impl pallet_bitcoin_locks::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LockEvents = (EventHandler,);
	type BitcoinUtxoTracker = StaticBitcoinUtxoTracker;
	type PriceProvider = StaticPriceProvider;
	type BitcoinSignatureVerifier = StaticBitcoinVerifier;
	type BitcoinBlockHeight = BitcoinBlockHeight;
	type GetBitcoinNetwork = GetBitcoinNetwork;
	type BitcoinObligationProvider = StaticVaultProvider;
	type ArgonTicksPerDay = ConstU64<1440>;
	type MaxConcurrentlyReleasingLocks = MaxConcurrentlyReleasingLocks;
	type LockDurationBlocks = LockDurationBlocks;
	type LockReclamationBlocks = LockReclamationBlocks;
	type LockReleaseCosignDeadlineBlocks = LockReleaseCosignDeadlineBlocks;
	type TickProvider = StaticTickProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_bitcoin_locks::GenesisConfig::<Test> {
		minimum_bitcoin_lock_satoshis: MinimumLockSatoshis::get(),
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
