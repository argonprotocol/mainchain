use crate as pallet_crosschain_transfer;
use argon_primitives::{
	tick::Ticker,
	vault::{BitcoinVaultProvider, RegistrationVaultData, VaultArgonotCommitment, VaultError},
	EthereumBlockNumber, EthereumReceiptLog, EthereumReceiptLogProofBatch, EthereumVerifyError,
	EthereumVerifyProvider, OperationalAccountsHook, PriceProvider, TickProvider,
	TreasuryPoolProvider, VaultId, VotingSchedule,
};
use frame_support::traits::StorageMapShim;
use pallet_prelude::*;
use sp_arithmetic::FixedU128;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use std::collections::BTreeMap;

const LEGACY_TOKEN_GATEWAY_PALLET_ID: [u8; 8] = [0xa0, 0x9b, 0x1c, 0x60, 0xe8, 0x65, 0x02, 0x45];

type Block = frame_system::mocking::MockBlock<Test>;
pub type TestAccountId = AccountId32;
type ArgonToken = pallet_balances::Instance1;
type OwnershipToken = pallet_balances::Instance2;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Ownership: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
		CrosschainTransfer: pallet_crosschain_transfer,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = TestAccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
	pub static OwnershipExistentialDeposit: Balance = 1;
	pub const CrosschainTransferPalletId: PalletId = PalletId(*b"xchaintr");
	pub CrosschainTransferEthereumBurnAccount: TestAccountId = CrosschainTransferPalletId::get()
		.into_sub_account_truncating((pallet_crosschain_transfer::SourceChain::Ethereum, *b"burn"));
	pub const RecentTransferRetentionTicks: Tick = 5;
	pub const MaxActivitiesPerReceiptProof: u32 = 16;
	pub const MaxReceiptProofsPerExtrinsic: u32 = 10;
	pub const MaxCouncilMembers: u32 = 100;
	pub const MaxQueueApprovalsPerCall: u32 = 32;
	pub const TransferOutValidityEthereumBlocks: EthereumBlockNumber = 72_000;
	pub const MaxVerifiedExecutionBlockAgeTicks: Tick = 60;
	pub const TransferOutMintingAuthorityTipBasisPoints: u32 = 10;
	pub const MinTransferCollateralIncrement: Balance = 10_000;
	pub const DefaultMinimumMintingAuthorityMicrogonValue: Balance = 10_000;
	pub const MaxPendingTransferOutsPerDestinationChain: u32 = 100;
	pub const CouncilRotationFrames: FrameId = 10;
	pub static CurrentFrameId: FrameId = 1;
	pub static CurrentTick: Tick = 0;
	pub static CurrentTicker: Ticker = Ticker::new(1_000, 2);
	pub static ArgonPriceInUsd: FixedU128 = FixedU128::one();
	pub static ArgonotPriceInUsd: FixedU128 = FixedU128::one();
	pub static LowestMicrogonsPerArgonot: Option<Balance> = None;
	pub static ProofVerificationAllowed: bool = true;
	pub static ProofVerificationRejectedTransactionIndexes: Vec<u64> = Vec::new();
	pub static LatestExecutionBlockNumber: Option<EthereumBlockNumber> = Some(1_000);
	pub static LatestExecutionBlockTimestamp: Option<u64> = Some(0);
	pub static ConfirmedTransfers: Vec<(TestAccountId, Balance)> = Vec::new();
pub static RegistrationVaultDataByOperator:
	BTreeMap<TestAccountId, RegistrationVaultData<Balance>> = BTreeMap::new();
pub static ArgonotCommitmentByOperator:
	BTreeMap<TestAccountId, VaultArgonotCommitment<Balance>> = BTreeMap::new();
pub static ActiveBondAmountsByVaultAndAccount:
	BTreeMap<(VaultId, TestAccountId), Balance> = BTreeMap::new();
pub static EncumberedBondMicrogonsByAccount:
	BTreeMap<TestAccountId, Balance> = BTreeMap::new();
}

pub struct MockPriceProvider;
impl PriceProvider<Balance> for MockPriceProvider {
	type Weights = ();

	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		Some(FixedU128::one())
	}

	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		Some(ArgonPriceInUsd::get())
	}

	fn get_argonot_price_in_usd() -> Option<FixedU128> {
		Some(ArgonotPriceInUsd::get())
	}

	fn get_target_argon_price_in_usd() -> Option<FixedU128> {
		Some(ArgonPriceInUsd::get())
	}

	fn get_lowest_microgons_per_argonot(_frames: FrameId) -> Option<Balance> {
		LowestMicrogonsPerArgonot::get().or_else(Self::get_microgons_per_argonot)
	}

	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		None
	}

	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> argon_primitives::ArgonCPI {
		Default::default()
	}

	fn get_circulation() -> Balance {
		0
	}

	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
}

impl pallet_balances::Config<ArgonToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

impl pallet_balances::Config<OwnershipToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = OwnershipExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		TestAccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

pub struct MockEthereumVerifier;
impl EthereumVerifyProvider for MockEthereumVerifier {
	type Weights = ();

	fn verify_receipt_logs<MaxProofBlocks, MaxReceiptLogs>(
		proof_batch: &EthereumReceiptLogProofBatch<MaxProofBlocks, MaxReceiptLogs>,
	) -> Result<(), EthereumVerifyError>
	where
		MaxProofBlocks: Get<u32>,
		MaxReceiptLogs: Get<u32>,
	{
		for proof_block in &proof_batch.blocks {
			Self::verify_receipt_logs_internal(&proof_block.receipt_logs)?;
		}

		Ok(())
	}

	fn latest_execution_block_number() -> Option<EthereumBlockNumber> {
		LatestExecutionBlockNumber::get()
	}

	fn latest_execution_block_timestamp() -> Option<u64> {
		LatestExecutionBlockTimestamp::get()
	}
}

pub struct MockTickProvider;
impl TickProvider<Block> for MockTickProvider {
	type Weights = ();

	fn previous_tick() -> Tick {
		CurrentTick::get().saturating_sub(1)
	}

	fn current_tick() -> Tick {
		CurrentTick::get()
	}

	fn elapsed_ticks() -> Tick {
		CurrentTick::get()
	}

	fn voting_schedule() -> VotingSchedule {
		todo!()
	}

	fn ticker() -> Ticker {
		CurrentTicker::get()
	}

	fn blocks_at_tick(_tick: Tick) -> Vec<<Block as sp_runtime::traits::Block>::Hash> {
		Vec::new()
	}
}

impl MockEthereumVerifier {
	fn verify_receipt_logs_internal(
		receipt_logs: &[EthereumReceiptLog],
	) -> Result<(), EthereumVerifyError> {
		if !ProofVerificationAllowed::get() {
			return Err(EthereumVerifyError::InvalidProof);
		}

		let rejected_indexes = ProofVerificationRejectedTransactionIndexes::get();
		if receipt_logs
			.iter()
			.any(|receipt_log| rejected_indexes.contains(&receipt_log.transaction_index))
		{
			return Err(EthereumVerifyError::InvalidProof);
		}

		Ok(())
	}
}

pub struct MockOperationalAccountsHook;
impl OperationalAccountsHook<TestAccountId, Balance> for MockOperationalAccountsHook {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn treasury_pool_participated_weight() -> Weight {
		Weight::zero()
	}

	fn uniswap_transfer_confirmed_weight() -> Weight {
		Weight::zero()
	}

	fn uniswap_transfer_confirmed(account_id: &TestAccountId, amount: Balance) {
		ConfirmedTransfers::mutate(|confirmed| confirmed.push((account_id.clone(), amount)));
	}
}

pub struct MockVaultProvider;
impl BitcoinVaultProvider for MockVaultProvider {
	type Weights = ();
	type Balance = Balance;
	type AccountId = TestAccountId;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		RegistrationVaultDataByOperator::get()
			.get(account_id)
			.map(|entry| entry.vault_id == vault_id)
			.unwrap_or(false)
	}

	fn can_initialize_bitcoin_locks(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		Self::is_owner(vault_id, account_id)
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		RegistrationVaultDataByOperator::get().iter().find_map(|(account_id, entry)| {
			(entry.vault_id == vault_id).then_some(account_id.clone())
		})
	}

	fn get_vault_id(account_id: &Self::AccountId) -> Option<VaultId> {
		RegistrationVaultDataByOperator::get()
			.get(account_id)
			.map(|entry| entry.vault_id)
	}

	fn get_registration_vault_data(
		account_id: &Self::AccountId,
	) -> Option<RegistrationVaultData<Self::Balance>> {
		RegistrationVaultDataByOperator::get().get(account_id).cloned()
	}

	fn get_committed_securitization(
		account_id: &Self::AccountId,
		_min_frames_remaining: FrameId,
	) -> Option<Self::Balance> {
		Self::get_registration_vault_data(account_id).map(|entry| entry.activated_securitization)
	}

	fn get_committed_argonots(account_id: &Self::AccountId) -> Option<Self::Balance> {
		Self::get_vault_id(account_id).map(|_| {
			ArgonotCommitmentByOperator::get()
				.get(account_id)
				.map(|commitment| commitment.committed_micronots)
				.unwrap_or_default()
		})
	}

	fn encumber_argonots(
		account_id: &Self::AccountId,
		amount: Self::Balance,
	) -> Result<(), VaultError> {
		Self::get_vault_id(account_id).ok_or(VaultError::VaultNotFound)?;
		let mut commitment =
			ArgonotCommitmentByOperator::get().get(account_id).cloned().unwrap_or_default();
		let Some(next_encumbered) = commitment.encumbered_micronots.checked_add(amount) else {
			return Err(VaultError::InternalError);
		};
		if next_encumbered > commitment.committed_micronots {
			return Err(VaultError::CommittedArgonotsBelowEncumberedBacking);
		}
		commitment.encumbered_micronots = next_encumbered;
		ArgonotCommitmentByOperator::mutate(|entries| {
			entries.insert(account_id.clone(), commitment);
		});
		Ok(())
	}

	fn release_encumbered_argonots(
		account_id: &Self::AccountId,
		amount: Self::Balance,
	) -> Result<(), VaultError> {
		Self::get_vault_id(account_id).ok_or(VaultError::VaultNotFound)?;
		let mut commitment =
			ArgonotCommitmentByOperator::get().get(account_id).cloned().unwrap_or_default();
		commitment.encumbered_micronots = commitment.encumbered_micronots.saturating_sub(amount);
		ArgonotCommitmentByOperator::mutate(|entries| {
			entries.insert(account_id.clone(), commitment);
		});
		Ok(())
	}

	fn burn_encumbered_argonots(
		account_id: &Self::AccountId,
		amount: Self::Balance,
	) -> Result<(), VaultError> {
		Self::get_vault_id(account_id).ok_or(VaultError::VaultNotFound)?;
		let mut commitment =
			ArgonotCommitmentByOperator::get().get(account_id).cloned().unwrap_or_default();
		commitment.committed_micronots = commitment
			.committed_micronots
			.checked_sub(amount)
			.ok_or(VaultError::CommittedArgonotsBelowEncumberedBacking)?;
		commitment.encumbered_micronots = commitment
			.encumbered_micronots
			.checked_sub(amount)
			.ok_or(VaultError::CommittedArgonotsBelowEncumberedBacking)?;
		ArgonotCommitmentByOperator::mutate(|entries| {
			entries.insert(account_id.clone(), commitment);
		});
		Ok(())
	}

	fn get_securitization_ratio(
		_vault_id: VaultId,
	) -> Result<sp_runtime::FixedU128, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn add_securitized_satoshis(
		_vault_id: VaultId,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_securitization_ratio: sp_runtime::FixedU128,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn reduce_securitized_satoshis(
		_vault_id: VaultId,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_securitization_ratio: sp_runtime::FixedU128,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn consume_recent_capacity_drop_budget(
		_vault_id: VaultId,
		_required_collateral: Self::Balance,
	) -> Result<bool, argon_primitives::vault::VaultError> {
		Ok(false)
	}

	fn lock(
		_vault_id: VaultId,
		_locker: &Self::AccountId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_extension: Option<(
			sp_runtime::FixedU128,
			&mut argon_primitives::vault::LockExtension<Self::Balance>,
		)>,
		_vault_covers_fee: bool,
	) -> Result<Self::Balance, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn schedule_for_release(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_satoshis: argon_primitives::bitcoin::Satoshis,
		_lock_extension: &argon_primitives::vault::LockExtension<Self::Balance>,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn cancel(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn burn(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_market_rate: Self::Balance,
		_lock_extension: &argon_primitives::vault::LockExtension<Self::Balance>,
	) -> Result<Self::Balance, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn compensate_lost_bitcoin(
		_vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
		_market_rate: Self::Balance,
		_lock_extension: &argon_primitives::vault::LockExtension<Self::Balance>,
	) -> Result<Self::Balance, argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_owner_pubkey: argon_primitives::bitcoin::CompressedBitcoinPubkey,
		_vault_claim_height: argon_primitives::bitcoin::BitcoinHeight,
		_open_claim_height: argon_primitives::bitcoin::BitcoinHeight,
		_current_height: argon_primitives::bitcoin::BitcoinHeight,
	) -> Result<
		(
			argon_primitives::bitcoin::BitcoinXPub,
			argon_primitives::bitcoin::BitcoinXPub,
			argon_primitives::bitcoin::BitcoinCosignScriptPubkey,
		),
		argon_primitives::vault::VaultError,
	> {
		unimplemented!()
	}

	fn remove_pending(
		_vault_id: VaultId,
		_securitization: &argon_primitives::vault::Securitization<Self::Balance>,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn update_pending_cosign_list(
		_vault_id: VaultId,
		_utxo_id: argon_primitives::bitcoin::UtxoId,
		_should_remove: bool,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}

	fn update_orphan_cosign_list(
		_vault_id: VaultId,
		_utxo_id: argon_primitives::bitcoin::UtxoId,
		_account_id: &Self::AccountId,
		_should_remove: bool,
	) -> Result<(), argon_primitives::vault::VaultError> {
		unimplemented!()
	}
}

pub struct MockTreasuryPoolProvider;
impl TreasuryPoolProvider<TestAccountId> for MockTreasuryPoolProvider {
	type Weights = ();
	type Balance = Balance;

	fn has_bond_participation(vault_id: VaultId, account_id: &TestAccountId) -> bool {
		ActiveBondAmountsByVaultAndAccount::get().contains_key(&(vault_id, account_id.clone()))
	}

	fn encumber_bond_microgons(
		account_id: &TestAccountId,
		microgon_amount: Self::Balance,
	) -> DispatchResult {
		if microgon_amount.is_zero() {
			return Ok(());
		}

		let active_bond_microgons = ActiveBondAmountsByVaultAndAccount::get()
			.iter()
			.filter(|((_, entry_account_id), _)| entry_account_id == account_id)
			.fold(0, |total, (_, amount)| total.saturating_add(*amount));
		let next_encumbered = encumbered_bond_microgons(account_id).saturating_add(microgon_amount);
		if active_bond_microgons < next_encumbered {
			return Err(DispatchError::Other("ActiveBondAmountBelowEncumberedBacking"));
		}

		EncumberedBondMicrogonsByAccount::mutate(|entries| {
			let entry = entries.entry(account_id.clone()).or_default();
			*entry = entry.saturating_add(microgon_amount);
		});
		Ok(())
	}

	fn release_encumbered_bond_microgons(
		account_id: &TestAccountId,
		microgon_amount: Self::Balance,
	) -> DispatchResult {
		if microgon_amount.is_zero() {
			return Ok(());
		}

		EncumberedBondMicrogonsByAccount::mutate(|entries| {
			let entry = entries.entry(account_id.clone()).or_default();
			*entry = entry.saturating_sub(microgon_amount);
		});
		Ok(())
	}

	fn burn_encumbered_bond_microgons(
		account_id: &TestAccountId,
		microgon_amount: Self::Balance,
	) -> DispatchResult {
		if microgon_amount.is_zero() {
			return Ok(());
		}

		let current_encumbered = encumbered_bond_microgons(account_id);
		ensure!(current_encumbered >= microgon_amount, DispatchError::Other("InsufficientBond"));
		EncumberedBondMicrogonsByAccount::mutate(|entries| {
			let entry = entries.entry(account_id.clone()).or_default();
			*entry = entry.saturating_sub(microgon_amount);
		});
		ActiveBondAmountsByVaultAndAccount::mutate(|entries| {
			let mut remaining = microgon_amount;
			for ((_, entry_account_id), amount) in entries.iter_mut() {
				if *entry_account_id != *account_id || remaining.is_zero() {
					continue;
				}

				let burned = (*amount).min(remaining);
				*amount = amount.saturating_sub(burned);
				remaining = remaining.saturating_sub(burned);
			}
		});
		Ok(())
	}
}

impl pallet_crosschain_transfer::Config for Test {
	type Balance = Balance;
	type EthereumBurnAccount = CrosschainTransferEthereumBurnAccount;
	type NativeCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type RuntimeHoldReason = RuntimeHoldReason;
	type EthereumVerifier = MockEthereumVerifier;
	type OperationalAccountsHook = MockOperationalAccountsHook;
	type VaultProvider = MockVaultProvider;
	type TreasuryPoolProvider = MockTreasuryPoolProvider;
	type PriceProvider = MockPriceProvider;
	type CurrentFrameId = CurrentFrameId;
	type CurrentTick = CurrentTick;
	type TickProvider = MockTickProvider;
	type RecentTransferRetentionTicks = RecentTransferRetentionTicks;
	type MaxActivitiesPerReceiptProof = MaxActivitiesPerReceiptProof;
	type MaxReceiptProofsPerExtrinsic = MaxReceiptProofsPerExtrinsic;
	type MaxCouncilMembers = MaxCouncilMembers;
	type MaxQueueApprovalsPerCall = MaxQueueApprovalsPerCall;
	type TransferOutValidityEthereumBlocks = TransferOutValidityEthereumBlocks;
	type MaxVerifiedExecutionBlockAgeTicks = MaxVerifiedExecutionBlockAgeTicks;
	type TransferOutMintingAuthorityTipBasisPoints = TransferOutMintingAuthorityTipBasisPoints;
	type MinTransferCollateralIncrement = MinTransferCollateralIncrement;
	type DefaultMinimumMintingAuthorityMicrogonValue = DefaultMinimumMintingAuthorityMicrogonValue;
	type MaxPendingTransferOutsPerDestinationChain = MaxPendingTransferOutsPerDestinationChain;
	type CouncilRotationFrames = CouncilRotationFrames;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestState {
	CurrentFrameId::set(1);
	CurrentTick::set(0);
	CurrentTicker::set(Ticker::new(1_000, 2));
	ArgonPriceInUsd::set(FixedU128::one());
	ArgonotPriceInUsd::set(FixedU128::one());
	LowestMicrogonsPerArgonot::set(None);
	ProofVerificationAllowed::set(true);
	ProofVerificationRejectedTransactionIndexes::set(Vec::new());
	LatestExecutionBlockNumber::set(Some(1_000));
	LatestExecutionBlockTimestamp::set(Some(0));
	ConfirmedTransfers::set(Vec::new());
	RegistrationVaultDataByOperator::set(BTreeMap::new());
	ArgonotCommitmentByOperator::set(BTreeMap::new());
	ActiveBondAmountsByVaultAndAccount::set(BTreeMap::new());
	EncumberedBondMicrogonsByAccount::set(BTreeMap::new());

	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_balances::GenesisConfig::<Test, ArgonToken> {
			balances: vec![(account(1), 1_000_000), (legacy_token_gateway_account(), 200_000_000)],
			dev_accounts: None,
		}
		.assimilate_storage(t)
		.unwrap();

		pallet_balances::GenesisConfig::<Test, OwnershipToken> {
			balances: vec![(legacy_token_gateway_account(), 500_000)],
			dev_accounts: None,
		}
		.assimilate_storage(t)
		.unwrap();
	})
}

pub fn account(byte: u8) -> TestAccountId {
	AccountId32::new([byte; 32])
}

pub fn h160(byte: u8) -> H160 {
	H160::repeat_byte(byte)
}

pub fn legacy_token_gateway_account() -> TestAccountId {
	PalletId(LEGACY_TOKEN_GATEWAY_PALLET_ID).into_account_truncating()
}

pub fn register_vault_operator(
	operator_account: TestAccountId,
	vault_id: VaultId,
	activated_securitization: Balance,
) {
	let operator_account_clone = operator_account.clone();
	let operator_account_for_commitment = operator_account_clone.clone();
	RegistrationVaultDataByOperator::mutate(|entries| {
		entries.insert(
			operator_account,
			RegistrationVaultData {
				vault_id,
				activated_securitization,
				securitization: activated_securitization,
			},
		);
	});
	ActiveBondAmountsByVaultAndAccount::mutate(|entries| {
		entries.insert((vault_id, operator_account_clone), activated_securitization);
	});
	ArgonotCommitmentByOperator::mutate(|entries| {
		entries.entry(operator_account_for_commitment).or_default();
	});
}

pub fn set_committed_argonots(operator_account: TestAccountId, amount: Balance) -> DispatchResult {
	let encumbered = ArgonotCommitmentByOperator::get()
		.get(&operator_account)
		.map(|entry| entry.encumbered_micronots)
		.unwrap_or_default();
	ensure!(amount >= encumbered, TokenError::Frozen);
	ArgonotCommitmentByOperator::mutate(|entries| {
		let entry = entries.entry(operator_account).or_default();
		entry.committed_micronots = amount;
	});
	Ok(())
}

pub fn set_active_bond_amount(vault_id: VaultId, operator_account: TestAccountId, amount: Balance) {
	ActiveBondAmountsByVaultAndAccount::mutate(|entries| {
		entries.insert((vault_id, operator_account), amount);
	});
}

pub fn active_bond_microgons(operator_account: &TestAccountId) -> Balance {
	ActiveBondAmountsByVaultAndAccount::get()
		.iter()
		.filter(|((_, entry_account_id), _)| entry_account_id == operator_account)
		.fold(0, |total, (_, amount)| total.saturating_add(*amount))
}

pub fn encumbered_bond_microgons(operator_account: &TestAccountId) -> Balance {
	EncumberedBondMicrogonsByAccount::get()
		.get(operator_account)
		.copied()
		.unwrap_or_default()
}

pub fn committed_argonot_micronots(operator_account: &TestAccountId) -> Balance {
	ArgonotCommitmentByOperator::get()
		.get(operator_account)
		.map(|entry| entry.committed_micronots)
		.unwrap_or_default()
}

pub fn encumbered_argonot_micronots(operator_account: &TestAccountId) -> Balance {
	ArgonotCommitmentByOperator::get()
		.get(operator_account)
		.map(|entry| entry.encumbered_micronots)
		.unwrap_or_default()
}
