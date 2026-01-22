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
	pub static MinimumSatoshisPerTrackedUtxo: u64 = 100_000_000; // 1 bitcoin minimum

	pub const MaxPendingConfirmationUtxos: u32 = 10;

	pub const MaxPendingConfirmationBlocks: u32 = 10;

	pub const MaximumSatoshiThresholdFromExpected: Satoshis = 10_000;
	pub static UtxoVerifiedCallback: Option<fn((UtxoId, Satoshis)) -> DispatchResult> = None;
	pub static LastInvalidUtxo: Option<(UtxoId, UtxoRef, Satoshis)> = None;
}

pub struct StaticEventHandler;
impl BitcoinUtxoEvents for StaticEventHandler {
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
	fn invalid_utxo_received(
		utxo_id: UtxoId,
		utxo_ref: UtxoRef,
		satoshis: Satoshis,
	) -> sp_runtime::DispatchResult {
		LastInvalidUtxo::set(Some((utxo_id, utxo_ref, satoshis)));
		Ok(())
	}
	fn spent(_utxo_id: UtxoId) -> DispatchResult {
		Ok(())
	}

	fn timeout_waiting_for_funding(_utxo_id: UtxoId) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

impl pallet_bitcoin_utxos::Config for Test {
	type WeightInfo = ();
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
	type EventHandler = StaticEventHandler;
	type MaximumSatoshiThresholdFromExpected = MaximumSatoshiThresholdFromExpected;
	type MinimumSatoshisPerTrackedUtxo = MinimumSatoshisPerTrackedUtxo;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
