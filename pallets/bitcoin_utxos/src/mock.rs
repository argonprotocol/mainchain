use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types};
use sp_runtime::{BuildStorage, DispatchResult};

use ulx_primitives::{
	bitcoin::{BitcoinRejectedReason, UtxoId},
	BitcoinUtxoEvents,
};

use crate as pallet_bitcoin_utxos;

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
	pub const MinBitcoinSatoshiAmount: u64 = 100_000_000; // 1 bitcoin minimum

	pub const MaxPendingConfirmationUtxos: u32 = 10;

	pub const MaxPendingConfirmationBlocks: u32 = 10;
	pub const MaxUtxoBirthBlocksOld: u32 = 10;
	pub static UtxoVerifiedCallback: Option<fn(UtxoId) -> DispatchResult> = None;
}

pub struct StaticEventHandler;
impl BitcoinUtxoEvents for StaticEventHandler {
	fn utxo_verified(_utxo_id: UtxoId) -> DispatchResult {
		if let Some(callback) = UtxoVerifiedCallback::get() {
			callback(_utxo_id)
		} else {
			Ok(())
		}
	}

	fn utxo_rejected(_utxo_id: UtxoId, _reason: BitcoinRejectedReason) -> DispatchResult {
		Ok(())
	}

	fn utxo_spent(_utxo_id: UtxoId) -> DispatchResult {
		Ok(())
	}

	fn utxo_expired(_utxo_id: UtxoId) -> DispatchResult {
		Ok(())
	}
}

impl pallet_bitcoin_utxos::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxPendingConfirmationUtxos = MaxPendingConfirmationUtxos;
	type MaxPendingConfirmationBlocks = MaxPendingConfirmationBlocks;
	type EventHandler = StaticEventHandler;
	type MaxUtxoBirthBlocksOld = MaxUtxoBirthBlocksOld;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
