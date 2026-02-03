use pallet_prelude::*;

use crate as pallet_bitcoin_utxos;
use argon_primitives::{BitcoinUtxoEvents, bitcoin::UtxoId};
use pallet_prelude::argon_primitives::bitcoin::{Satoshis, UtxoRef};

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
	pub static MinimumSatoshisPerCandidateUtxo: u64 = 100_000_000; // 1 bitcoin minimum

	pub const MaxPendingConfirmationUtxos: u32 = 10;
	pub const MaxCandidateUtxosPerLock: u32 = 10;

	pub const MaxPendingConfirmationBlocks: u32 = 10;

pub const MaximumSatoshiThresholdFromExpected: Satoshis = 10_000;
pub static UtxoVerifiedCallback: Option<fn((UtxoId, Satoshis)) -> DispatchResult> = None;
pub static OrphanDetectedCallback: Option<fn((UtxoId, UtxoRef, Satoshis)) -> DispatchResult> = None;
pub static LastOrphanDetected: Option<(UtxoId, UtxoRef, Satoshis)> = None;
}

pub struct StaticEventHandler;
impl BitcoinUtxoEvents<u64, u64> for StaticEventHandler {
	fn funding_received(
		utxo_id: UtxoId,
		received_satoshis: Satoshis,
	) -> sp_runtime::DispatchResult {
		if let Some(callback) = UtxoVerifiedCallback::get() {
			callback((utxo_id, received_satoshis))
		} else {
			Ok(())
		}
	}
	fn spent(_utxo_id: UtxoId) -> DispatchResult {
		Ok(())
	}

	fn funding_promoted_by_account(
		_utxo_id: UtxoId,
		_received_satoshis: Satoshis,
		_account_id: &u64,
		_utxo_ref: &UtxoRef,
	) -> DispatchResult {
		Ok(())
	}

	fn orphaned_utxo_detected(
		_utxo_id: UtxoId,
		_satoshis: Satoshis,
		_utxo_ref: UtxoRef,
	) -> DispatchResult {
		if let Some(callback) = OrphanDetectedCallback::get() {
			callback((_utxo_id, _utxo_ref, _satoshis))
		} else {
			Ok(())
		}
	}

	fn timeout_waiting_for_funding(_utxo_id: UtxoId) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

impl pallet_bitcoin_utxos::Config for Test {
	type WeightInfo = ();
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
	type MaxCandidateUtxosPerLock = MaxCandidateUtxosPerLock;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
	type EventHandler = StaticEventHandler;
	type MaximumSatoshiThresholdFromExpected = MaximumSatoshiThresholdFromExpected;
	type MinimumSatoshisPerCandidateUtxo = MinimumSatoshisPerCandidateUtxo;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
