//! Benchmark-only runtime stubs and helpers.
#![cfg(feature = "runtime-benchmarks")]

use crate::prelude::{Vec, vec};
use codec::Decode;
use core::marker::PhantomData;
use polkadot_sdk::{
	frame_support::traits::{Get, SortedMembers},
	sp_arithmetic::FixedU128,
	sp_runtime::{
		DispatchError,
		traits::{AtLeast32BitUnsigned, SaturatedConversion},
	},
};

use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	MiningSlotProvider, TreasuryPoolProvider, UniswapTransferRequirementProvider, VaultId,
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, UtxoId,
	},
	vault::{
		BitcoinVaultProvider, LockExtension, RegistrationVaultData, Securitization, VaultError,
	},
};
use pallet_bitcoin_locks::BitcoinVerifier;
pub use pallet_prelude::benchmarking::{
	BenchmarkAuthorityProvider, BenchmarkBitcoinBlockHeightChange,
	BenchmarkBitcoinLocksRuntimeState, BenchmarkBitcoinNetwork, BenchmarkBitcoinUtxoTracker,
	BenchmarkBitcoinUtxoTrackerState, BenchmarkBitcoinVaultProvider,
	BenchmarkBitcoinVaultProviderState, BenchmarkBlockRewardAccountsProvider,
	BenchmarkBlockSealerProvider, BenchmarkCurrentFrameId, BenchmarkCurrentTick,
	BenchmarkDidStartNewFrame, BenchmarkNotaryProvider, BenchmarkNotebookProvider,
	BenchmarkOperationalAccountsProviderState, BenchmarkOperationalRewardsPayer,
	BenchmarkOperationalRewardsProvider, BenchmarkOperationalRewardsProviderState,
	BenchmarkPriceProvider, BenchmarkPriceProviderState, BenchmarkTickProvider,
	BenchmarkUtxoLockEvents, benchmark_operational_accounts_provider_state,
	benchmark_operational_rewards_provider_state, reset_benchmark_bitcoin_locks_runtime_state,
	reset_benchmark_bitcoin_utxo_tracker_state, reset_benchmark_bitcoin_vault_provider_state,
	reset_benchmark_operational_rewards_provider_state, reset_benchmark_price_provider_state,
	reset_benchmark_utxo_lock_events_state, set_benchmark_bitcoin_locks_runtime_state,
	set_benchmark_bitcoin_utxo_tracker_state, set_benchmark_bitcoin_vault_provider_state,
	set_benchmark_operational_accounts_provider_state,
	set_benchmark_operational_rewards_provider_state, set_benchmark_price_provider_state,
};

fn benchmark_token_admin_account_id<AccountId: Decode>() -> AccountId {
	let bytes = [3u8; 32];
	AccountId::decode(&mut &bytes[..]).expect("benchmark token admin account id must decode")
}

pub struct BenchmarkTokenAdmin<AccountId>(PhantomData<AccountId>);

impl<AccountId: Decode> Get<AccountId> for BenchmarkTokenAdmin<AccountId> {
	fn get() -> AccountId {
		benchmark_token_admin_account_id::<AccountId>()
	}
}

pub struct BenchmarkTokenAdmins<AccountId>(PhantomData<AccountId>);

impl<AccountId> SortedMembers<AccountId> for BenchmarkTokenAdmins<AccountId>
where
	AccountId: Decode + Ord,
{
	fn sorted_members() -> Vec<AccountId> {
		vec![benchmark_token_admin_account_id::<AccountId>()]
	}

	fn contains(t: &AccountId) -> bool {
		*t == benchmark_token_admin_account_id::<AccountId>()
	}

	fn count() -> usize {
		1
	}

	fn add(_t: &AccountId) {}
}

pub struct BenchmarkBitcoinSignatureVerifier;
impl<T: pallet_bitcoin_locks::Config> BitcoinVerifier<T> for BenchmarkBitcoinSignatureVerifier {
	fn verify_signature(
		_utxo_releaser: CosignReleaser,
		_pubkey: CompressedBitcoinPubkey,
		_signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}

pub struct BenchmarkOperationalAccountsVaultProvider<Balance, AccountId>(
	PhantomData<(Balance, AccountId)>,
);

impl<Balance, AccountId> BitcoinVaultProvider
	for BenchmarkOperationalAccountsVaultProvider<Balance, AccountId>
where
	Balance: codec::Codec
		+ Copy
		+ scale_info::TypeInfo
		+ codec::MaxEncodedLen
		+ Default
		+ AtLeast32BitUnsigned,
	AccountId: codec::Codec,
{
	type Weights = ();
	type Balance = Balance;
	type AccountId = AccountId;

	fn is_owner(_vault_id: VaultId, _account_id: &Self::AccountId) -> bool {
		false
	}

	fn can_initialize_bitcoin_locks(_vault_id: VaultId, _account_id: &Self::AccountId) -> bool {
		false
	}

	fn get_vault_operator(_vault_id: VaultId) -> Option<Self::AccountId> {
		None
	}

	fn get_vault_id(_account_id: &Self::AccountId) -> Option<VaultId> {
		None
	}

	fn get_registration_vault_data(
		_account_id: &Self::AccountId,
	) -> Option<RegistrationVaultData<Self::Balance>> {
		let mut state = benchmark_operational_accounts_provider_state();
		state.call_counters.get_registration_vault_data =
			state.call_counters.get_registration_vault_data.saturating_add(1);
		let result = state.vault_registration_data.clone().map(|data| RegistrationVaultData {
			vault_id: data.vault_id,
			activated_securitization: data.activated_securitization.saturated_into(),
			securitization: data.securitization.saturated_into(),
		});
		set_benchmark_operational_accounts_provider_state(state);
		result
	}

	fn account_became_operational(_vault_operator_account: &Self::AccountId) {
		let mut state = benchmark_operational_accounts_provider_state();
		state.call_counters.account_became_operational =
			state.call_counters.account_became_operational.saturating_add(1);
		set_benchmark_operational_accounts_provider_state(state);
	}

	fn get_securitization_ratio(_vault_id: VaultId) -> Result<FixedU128, VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn add_securitized_satoshis(
		_vault_id: VaultId,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_securitization_ratio: FixedU128,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn reduce_securitized_satoshis(
		_vault_id: VaultId,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_securitization_ratio: FixedU128,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn lock(
		_vault_id: VaultId,
		_locker: &Self::AccountId,
		_securitization: &Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_extension: Option<(FixedU128, &mut LockExtension<Self::Balance>)>,
		_has_fee_coupon: bool,
	) -> Result<Self::Balance, VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn schedule_for_release(
		_vault_id: VaultId,
		_securitization: &Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_lock_extension: &LockExtension<Self::Balance>,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn cancel(
		_vault_id: VaultId,
		_securitization: &Securitization<Self::Balance>,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn burn(
		_vault_id: VaultId,
		_securitization: &Securitization<Self::Balance>,
		_market_rate: Self::Balance,
		_lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn compensate_lost_bitcoin(
		_vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		_securitization: &Securitization<Self::Balance>,
		_market_rate: Self::Balance,
		_lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_owner_pubkey: CompressedBitcoinPubkey,
		_vault_claim_height: BitcoinHeight,
		_open_claim_height: BitcoinHeight,
		_current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn remove_pending(
		_vault_id: VaultId,
		_securitization: &Securitization<Self::Balance>,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn update_pending_cosign_list(
		_vault_id: VaultId,
		_utxo_id: UtxoId,
		_should_remove: bool,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn update_orphan_cosign_list(
		_vault_id: VaultId,
		_utxo_id: UtxoId,
		_account_id: &Self::AccountId,
		_should_remove: bool,
	) -> Result<(), VaultError> {
		Err(VaultError::VaultNotFound)
	}

	fn consume_recent_capacity_drop_budget(
		_vault_id: VaultId,
		_required_collateral: Self::Balance,
	) -> Result<bool, VaultError> {
		Ok(false)
	}
}

pub struct BenchmarkOperationalAccountsMiningSlotProvider<AccountId>(PhantomData<AccountId>);

impl<AccountId> MiningSlotProvider<AccountId>
	for BenchmarkOperationalAccountsMiningSlotProvider<AccountId>
{
	type Weights = ();

	fn has_active_rewards_account_seat(_account_id: &AccountId) -> bool {
		let mut state = benchmark_operational_accounts_provider_state();
		state.call_counters.has_active_rewards_account_seat =
			state.call_counters.has_active_rewards_account_seat.saturating_add(1);
		let result = state.has_active_rewards_account_seat;
		set_benchmark_operational_accounts_provider_state(state);
		result
	}
}

pub struct BenchmarkOperationalAccountsTreasuryPoolProvider<AccountId>(PhantomData<AccountId>);

impl<AccountId> TreasuryPoolProvider<AccountId>
	for BenchmarkOperationalAccountsTreasuryPoolProvider<AccountId>
{
	type Weights = ();

	fn has_bond_participation(_vault_id: VaultId, _account_id: &AccountId) -> bool {
		let mut state = benchmark_operational_accounts_provider_state();
		state.call_counters.has_bond_participation =
			state.call_counters.has_bond_participation.saturating_add(1);
		let result = state.has_bond_participation;
		set_benchmark_operational_accounts_provider_state(state);
		result
	}
}

pub struct BenchmarkOperationalAccountsUniswapTransferRequirementProvider;

impl UniswapTransferRequirementProvider
	for BenchmarkOperationalAccountsUniswapTransferRequirementProvider
{
	type Weights = ();

	fn requires_uniswap_transfer() -> bool {
		let mut state = benchmark_operational_accounts_provider_state();
		state.call_counters.requires_uniswap_transfer =
			state.call_counters.requires_uniswap_transfer.saturating_add(1);
		let result = state.requires_uniswap_transfer;
		set_benchmark_operational_accounts_provider_state(state);
		result
	}
}
