use alloc::vec::Vec;
use argon_primitives::{
	treasury::{
		BitcoinLockPosition, NetworkPositionQuantities, PositionQuantities, PositionQuantity,
		TreasuryPositionProvider, UpstreamPosition,
	},
	VaultId,
};
use codec::{Decode, Encode};
use env_logger::{Builder, Env};
pub use frame_support::{derive_impl, parameter_types, weights::constants::RocksDbWeight};
use polkadot_sdk::*;

use frame_support::storage::unhashed;
pub use frame_support::{
	assert_err, assert_err_ignore_postinfo, assert_error_encoded_size, assert_noop, assert_ok,
	assert_storage_noop, storage_alias,
};
pub use frame_system::mocking::*;
pub use sp_core::{storage::Storage, Blake2Hasher};
pub use sp_io::TestExternalities as TestState;
pub use sp_keyring::*;
pub use sp_runtime::{traits::IdentityLookup, BuildStorage};
use sp_runtime::{DispatchError, DispatchResult};

// Build genesis storage according to the mock runtime.
pub fn new_test_with_genesis<Test>(apply: impl FnOnce(&mut Storage)) -> TestState
where
	Test: polkadot_sdk::frame_system::Config,
{
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	apply(&mut t);
	TestState::new(t)
}

/// Records source contributions in test storage so a failed operation rolls them back.
pub struct MockTreasuryPositionProvider;

#[derive(Default, Decode, Encode)]
struct PositionUpdates {
	fissions: Vec<(u64, u128, u128)>,
	bitcoin: Vec<BitcoinPositionUpdate>,
	vault_coverage: Vec<(u128, u128)>,
	rejected_fission_update: Option<(u128, u128)>,
	reject_bitcoin_updates: bool,
	network_fission_liquidity: Option<u128>,
}

#[derive(Decode, Encode)]
struct BitcoinPositionUpdate {
	account: u64,
	vault_id: VaultId,
	// Activated and allocated securitization, in that order.
	previous: [u128; 2],
	current: [u128; 2],
}

const POSITION_UPDATES_KEY: &[u8] = b"test:treasury-position-updates";

impl MockTreasuryPositionProvider {
	pub fn fission_liquidity(account: u64) -> u128 {
		Self::updates()
			.fissions
			.into_iter()
			.filter(|(owner, _, _)| *owner == account)
			.fold(0, |total, (_, previous, current)| total - previous + current)
	}

	pub fn vault_securitized_satoshis() -> u128 {
		Self::updates()
			.vault_coverage
			.into_iter()
			.fold(0, |total, (previous, current)| total - previous + current)
	}

	pub fn bitcoin_position(account: u64, vault_id: VaultId) -> BitcoinLockPosition<u128> {
		let mut position = BitcoinLockPosition::default();
		for update in Self::updates().bitcoin {
			if (update.account, update.vault_id) == (account, vault_id) {
				let [previous_activated, previous_allocated] = update.previous;
				let [activated, allocated] = update.current;
				position.activated_securitization =
					position.activated_securitization - previous_activated + activated;
				position.allocated_securitization =
					position.allocated_securitization - previous_allocated + allocated;
			}
		}
		position
	}

	pub fn reject_fission_update(update: Option<(u128, u128)>) {
		Self::update_storage(|updates| updates.rejected_fission_update = update);
	}

	pub fn reject_bitcoin_updates() {
		Self::update_storage(|updates| updates.reject_bitcoin_updates = true);
	}

	pub fn override_network_fission_liquidity(liquidity: Option<u128>) {
		Self::update_storage(|updates| updates.network_fission_liquidity = liquidity);
	}

	fn updates() -> PositionUpdates {
		unhashed::get_or_default(POSITION_UPDATES_KEY)
	}

	fn update_storage(update: impl FnOnce(&mut PositionUpdates)) {
		let mut updates = Self::updates();
		update(&mut updates);
		unhashed::put(POSITION_UPDATES_KEY, &updates);
	}
}

impl TreasuryPositionProvider<u64, u128> for MockTreasuryPositionProvider {
	type Weights = ();

	fn account_quantity_updated<Amount: Into<u128>>(
		account: &u64,
		quantity: PositionQuantity,
		previous: Amount,
		current: Amount,
	) -> DispatchResult {
		assert_eq!(quantity, PositionQuantity::FissionLiquidity);
		let (previous, current) = (previous.into(), current.into());
		if Self::updates().rejected_fission_update == Some((previous, current)) {
			return Err(DispatchError::Unavailable);
		}
		Self::update_storage(|updates| updates.fissions.push((*account, previous, current)));
		Ok(())
	}

	fn account_quantities(account: &u64) -> PositionQuantities<u128> {
		let fission_liquidity = Self::fission_liquidity(*account);
		PositionQuantities { fission_liquidity, ..Default::default() }
	}

	fn network_totals() -> NetworkPositionQuantities<u128> {
		let updates = Self::updates();
		let fission_liquidity = updates.network_fission_liquidity.unwrap_or_else(|| {
			updates
				.fissions
				.into_iter()
				.fold(0, |total, (_, previous, current)| total - previous + current)
		});
		NetworkPositionQuantities {
			account_quantities: PositionQuantities { fission_liquidity, ..Default::default() },
			vault_securitized_satoshis: Self::vault_securitized_satoshis(),
		}
	}

	fn network_securitized_satoshis_updated(previous: u128, current: u128) -> DispatchResult {
		Self::update_storage(|updates| updates.vault_coverage.push((previous, current)));
		Ok(())
	}

	fn bitcoin_position_updated(
		account: &u64,
		vault_id: VaultId,
		previous: BitcoinLockPosition<u128>,
		current: BitcoinLockPosition<u128>,
	) -> DispatchResult {
		if Self::updates().reject_bitcoin_updates {
			return Err(DispatchError::Unavailable);
		}
		Self::update_storage(|updates| {
			updates.bitcoin.push(BitcoinPositionUpdate {
				account: *account,
				vault_id,
				previous: [previous.activated_securitization, previous.allocated_securitization],
				current: [current.activated_securitization, current.allocated_securitization],
			});
		});
		Ok(())
	}

	fn operational_account_registered(_: &u64) -> DispatchResult {
		Ok(())
	}

	fn bond_principal(_: &u64) -> u128 {
		0
	}

	fn upstream_position(_: &u64) -> Option<UpstreamPosition<u128>> {
		None
	}

	fn set_upstream_position(_: &u64, _: Option<UpstreamPosition<u128>>) {}

	fn bond_position_updated(_: &u64, _: VaultId, _: u128, _: u128) -> DispatchResult {
		Ok(())
	}
}
