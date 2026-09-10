use pallet_prelude::*;

use crate as pallet_bitcoin_utxos;
use argon_primitives::{bitcoin::BitcoinLockId, BitcoinUtxoEvents, BitcoinUtxoTracker};
use pallet_prelude::argon_primitives::bitcoin::{Satoshis, UtxoRef};

type UtxoDetectedCallbackFn = fn((BitcoinLockId, UtxoRef, Satoshis)) -> DispatchResult;
type SpentCallbackFn = fn((BitcoinLockId, UtxoRef)) -> DispatchResult;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BitcoinUtxos: pallet_bitcoin_utxos
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = frame_system::mocking::MockBlock<Test>;
}

parameter_types! {
	pub const BitcoinBondDuration: u32 = 60 * 24 * 365; // 1 year
	pub static MinimumSatoshisPerUtxo: u64 = 100_000_000; // 1 bitcoin minimum

	pub const MaxUtxosPerLock: u32 = 10;
	pub static UtxoDetectedCallback: Option<UtxoDetectedCallbackFn> = None;
	pub static UtxoDetectionCount: u32 = 0;
	pub static SpentCallback: Option<SpentCallbackFn> = None;
	pub static LastSpent: Option<(BitcoinLockId, UtxoRef)> = None;
}

pub struct StaticEventHandler;
impl BitcoinUtxoEvents<u64> for StaticEventHandler {
	type Weights = ();

	fn utxo_detected(
		lock_id: BitcoinLockId,
		utxo_ref: UtxoRef,
		received_satoshis: Satoshis,
		_bitcoin_height: u64,
	) -> sp_runtime::DispatchResult {
		UtxoDetectionCount::mutate(|count| *count = count.saturating_add(1));
		if let Some(callback) = UtxoDetectedCallback::get() {
			callback((lock_id, utxo_ref, received_satoshis))
		} else {
			Ok(())
		}
	}
	fn spent(lock_id: BitcoinLockId, utxo_ref: UtxoRef) -> DispatchResult {
		LastSpent::set(Some((lock_id, utxo_ref.clone())));
		if let Some(callback) = SpentCallback::get() {
			callback((lock_id, utxo_ref))
		} else {
			BitcoinUtxos::unwatch_utxo(lock_id, &utxo_ref);
			Ok(())
		}
	}
}

impl pallet_bitcoin_utxos::Config for Test {
	type WeightInfo = ();
	type MaxUtxosPerLock = MaxUtxosPerLock;
	type EventHandler = StaticEventHandler;
	type MinimumSatoshisPerUtxo = MinimumSatoshisPerUtxo;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
