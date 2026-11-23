use crate::{
	self as pallet_treasury, ArgonotBondLots, BondLotIdsByAccount, BondLotSummary, Bonds,
	TargetBitcoinPercent,
};
use argon_primitives::{
	providers::BitcoinMintedProvider,
	treasury::{
		EarningsSettlementProvider, StabilizerPoolFrameClaim, StabilizerPoolRewardProvider,
		TreasuryPositionProvider,
	},
	vault::{
		TreasuryBonusApprovalProof, TreasuryVaultProvider, VaultError, VaultParticipationCapacity,
		VaultSecuritization, TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY,
	},
	ArgonCPI, MiningFrameProvider, OperationalAccountsHook, PriceProvider, Signature,
	TreasuryPoolProvider, MICROGONS_PER_ARGON,
};
use frame_support::traits::{
	fungible::{Credit, Unbalanced},
	tokens::Imbalance,
	Currency, StorageMapShim,
};
use pallet_prelude::{
	argon_primitives::{vault::VaultTreasuryFrameEarnings, MiningFrameTransitionProvider},
	*,
};
use sp_core::{blake2_256, crypto::AccountId32, sr25519, Pair};
use sp_runtime::{traits::IdentifyAccount, MultiSigner};
use std::collections::{BTreeMap, HashMap};

type Block = frame_system::mocking::MockBlock<Test>;
pub type TestAccountId = AccountId32;

pub struct TestOperationalAccountsHook;

parameter_types! {
	pub static CirculatingArgon: Balance = 0;
	pub static CollectClaimFrames: Vec<StabilizerPoolFrameClaim> = Vec::new();
	pub storage SettledClaimFrame: u64 = 0;
}

pub struct MockStabilizerPoolSettlement;
impl EarningsSettlementProvider<TestAccountId> for MockStabilizerPoolSettlement {
	type Weights = ();

	fn settle_account(account: &TestAccountId) -> DispatchResult {
		let claims = CollectClaimFrames::get();
		if claims.is_empty() || SettledClaimFrame::get() > 0 {
			return Ok(());
		}
		let amount = Treasury::claim_rewards(&claims)?;
		TreasuryPositions::accrue(
			account,
			argon_primitives::treasury::CollectSource::StabilizerPools,
			amount,
		)?;
		SettledClaimFrame::set(&claims.last().unwrap().frame_id);
		Ok(())
	}
}

impl OperationalAccountsHook<TestAccountId, Balance> for TestOperationalAccountsHook {
	type Weights = ();
	fn account_vault_bond_total_updated(account_id: &TestAccountId, amount: Balance) {
		LastOperationalBondTotal::set(Some((account_id.clone(), amount)));
	}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		TreasuryPositions: pallet_treasury_positions,
		Treasury: pallet_treasury,
		Balances: pallet_balances::<Instance1>,
		Ownership: pallet_balances::<Instance2>,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = TestAccountId;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
	type Lookup = IdentityLookup<TestAccountId>;
}

parameter_types! {
	pub const BidIncrements: u128 = 10_000; // 1 cent

	pub static ExistentialDeposit: Balance = 1;
	pub static BidPoolAccountId: TestAccountId = AccountId32::new([250; 32]);
	pub static NewlyStartedFrameId: Option<FrameId> = None;
}

pub(crate) type ArgonToken = pallet_balances::Instance1;
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
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

pub(crate) type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		Self::AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

pub(crate) fn account_pair_from_seed(seed: u64) -> sr25519::Pair {
	sr25519::Pair::from_seed(&[(seed & 0xff) as u8; 32])
}

pub(crate) fn account_id_from_seed(seed: u64) -> TestAccountId {
	MultiSigner::from(account_pair_from_seed(seed).public()).into_account()
}

pub(crate) trait IntoTestAccountId {
	fn into_test_account_id(self) -> TestAccountId;
}

impl IntoTestAccountId for u64 {
	fn into_test_account_id(self) -> TestAccountId {
		account_id_from_seed(self)
	}
}

impl IntoTestAccountId for TestAccountId {
	fn into_test_account_id(self) -> TestAccountId {
		self
	}
}

impl IntoTestAccountId for &TestAccountId {
	fn into_test_account_id(self) -> TestAccountId {
		self.clone()
	}
}

pub(crate) fn set_argons(account_id: impl IntoTestAccountId, amount: Balance) {
	let account_id = account_id.into_test_account_id();
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

pub(crate) fn set_ownership(account_id: impl IntoTestAccountId, amount: Balance) {
	let account_id = account_id.into_test_account_id();
	let _ = Ownership::make_free_balance_be(&account_id, amount);
	drop(Ownership::issue(amount));
}

parameter_types! {
	pub const NextSlot: BlockNumberFor<Test> = 100;
	pub const MiningWindowBlocks: BlockNumberFor<Test> = 100;

	pub const LastBidPoolDistribution: (FrameId, Tick) = (0, 0);

	pub static MinimumArgonsPerContributor: u128 = 100_000_000;
	pub static MaxActiveArgonotBondLots: u32 = 1_000;
	pub static MaxBondCollateralLots: u32 = 10;
	pub static MaxBondCollateralSettlementsPerBlock: u32 = 2;
	pub static MaxVaultsPerPool: u32 = 100;
	pub static MaxPendingUnlocksPerFrame: u32 = 100;
	pub static TreasuryExitDelayFrames: FrameId = 10;
	pub const VaultPalletId: PalletId = PalletId(*b"bidPools");

	pub const PercentForMarketing: Percent = Percent::from_percent(10);
	pub const PercentForStakePool: Percent = Percent::from_percent(15);
	pub const PercentForMiningOperatorPool: Percent = Percent::from_percent(6);
	pub const PercentForBitcoinLiquidPool: Percent = Percent::from_percent(3);
	pub const PercentForArgonBondPool: Percent = Percent::from_percent(5);
	pub const PercentForStabilizerPool: Percent = Percent::from_percent(10);
	pub const PercentForVaultPool: Percent = Percent::from_percent(51);
	pub const MinimumVaultProfitPercent: Percent = Percent::from_percent(5);
	pub const VaultBitcoinRewardPercent: Permill = Permill::from_parts(428_350);
	pub static ArgonotSecuritizationMultiplier: u32 = 2;
	pub const DefaultTargetBitcoinPercent: Percent = Percent::from_percent(15);
	pub const DefaultTargetStabilizerPoolPercent: Percent = Percent::from_percent(5);
	pub static MaxArgonotBondedPercentOfCirculation: Percent = Percent::from_percent(60);
	pub static CurrentFrameId: FrameId = 1;

	pub static VaultsById: HashMap<VaultId, TestVault> = HashMap::new();
	pub static VaultBitcoinSatoshis: HashMap<VaultId, u64> = HashMap::new();
	pub static VaultSecuritizedSatoshis: HashMap<VaultId, u64> = HashMap::new();
	pub static VaultArgonotMicronots: HashMap<VaultId, Balance> = HashMap::new();
	pub static VaultRewardCommittedMicronots: HashMap<VaultId, Balance> = HashMap::new();
	pub static BitcoinPriceInUsd: FixedU128 = FixedU128::from_u32(10);
	pub static AverageArgonCpi: ArgonCPI = ArgonCPI::zero();
	pub static ArgonPriceInUsd: FixedU128 = FixedU128::from_u32(1);
	pub static ArgonotPriceInUsd: FixedU128 = FixedU128::from_u32(1);
	pub static AverageArgonotPriceInMicrogons: Balance = MICROGONS_PER_ARGON;
	pub static LastAverageArgonotPriceFrame: Option<FrameId> = None;
	pub static MintedBitcoinMicrogons: Balance = 0;

	pub storage LastVaultProfits: Vec<VaultTreasuryFrameEarnings<Balance, TestAccountId>> = vec![];
	pub static LastOperationalBondTotal: Option<(TestAccountId, Balance)> = None;
}

#[derive(Clone)]
pub struct TestVault {
	pub securitization: Balance,
	pub exit_notice_amount: Balance,
	pub committed_microgons: Balance,
	pub activated_securitization: Balance,
	pub account_id: TestAccountId,
	pub delegate_account_id: Option<TestAccountId>,
	pub is_closed: bool,
}

pub(crate) fn insert_vault(vault_id: VaultId, vault: TestVault) {
	let securitization = if vault.is_closed { 0 } else { vault.securitization };
	VaultsById::mutate(|x| {
		x.insert(vault_id, vault);
	});
	Treasury::vault_securitization_changed(vault_id, securitization).unwrap();
}

pub struct StaticTreasuryVaultProvider;
impl TreasuryVaultProvider for StaticTreasuryVaultProvider {
	type Currency = Balances;
	type Weights = ();
	type Balance = Balance;
	type AccountId = TestAccountId;

	fn get_participation_capacity(
		vault_id: VaultId,
	) -> Option<VaultParticipationCapacity<Self::Balance>> {
		let vault = VaultsById::get().get(&vault_id)?.clone();
		if vault.is_closed {
			return None;
		}
		Some(VaultParticipationCapacity {
			available_securitization_space: AvailableSecuritizationSpace::get()
				.get(&vault_id)
				.copied()
				.unwrap_or(vault.securitization),
			regular_bond_capacity: vault.securitization.saturating_sub(vault.exit_notice_amount),
		})
	}
	fn get_vault_securitization(vault_id: VaultId) -> Option<Self::Balance> {
		VaultsById::get()
			.get(&vault_id)
			.filter(|vault| !vault.is_closed)
			.map(|vault| vault.securitization)
	}

	fn commit_securitization_for_bonds(
		vault_id: VaultId,
		regular_bond_microgons: Self::Balance,
	) -> Result<(), VaultError> {
		let mut vaults = VaultsById::get();
		let vault = vaults.get_mut(&vault_id).ok_or(VaultError::VaultNotFound)?;
		if vault.is_closed {
			return Err(VaultError::VaultClosed);
		}
		if regular_bond_microgons > vault.securitization.saturating_sub(vault.exit_notice_amount) {
			return Err(VaultError::InsufficientVaultFunds);
		}
		vault.committed_microgons = vault.committed_microgons.max(regular_bond_microgons);
		VaultsById::set(vaults);
		Ok(())
	}

	fn get_top_vaults_by_securitization(
		max_vaults: u32,
	) -> (Vec<VaultSecuritization<Self::Balance, Self::AccountId>>, Self::Balance) {
		let mut positions: Vec<_> = VaultsById::get()
			.into_iter()
			.filter_map(|(vault_id, vault)| {
				if vault.is_closed || vault.securitization.is_zero() {
					return None;
				}
				Some(VaultSecuritization {
					vault_id,
					operator_account_id: vault.account_id,
					securitization: vault.securitization,
					activated_securitization: vault.activated_securitization,
					bitcoin_locked_satoshis: VaultBitcoinSatoshis::get()
						.get(&vault_id)
						.copied()
						.unwrap_or_default(),
					securitized_satoshis: VaultSecuritizedSatoshis::get()
						.get(&vault_id)
						.copied()
						.unwrap_or_default(),
					securitization_micronots: VaultArgonotMicronots::get()
						.get(&vault_id)
						.copied()
						.unwrap_or_default(),
				})
			})
			.collect();
		let total = positions
			.iter()
			.fold(0u128, |sum, vault| sum.saturating_add(vault.securitization));
		positions.sort_by(|a, b| {
			b.securitization
				.cmp(&a.securitization)
				.then_with(|| a.vault_id.cmp(&b.vault_id))
		});
		positions.truncate(max_vaults as usize);
		(positions, total)
	}

	fn commit_securitization_for_rewards(vault_id: VaultId, micronots: Self::Balance) {
		VaultsById::mutate(|vaults| {
			if let Some(vault) = vaults.get_mut(&vault_id) {
				vault.committed_microgons = vault.securitization;
			}
		});
		VaultRewardCommittedMicronots::mutate(|commitments| {
			let amount = commitments.entry(vault_id).or_default();
			*amount = (*amount).max(micronots);
		});
	}

	fn get_vault_operator(vault_id: VaultId) -> Option<Self::AccountId> {
		VaultsById::get().get(&vault_id).map(|a| a.account_id.clone())
	}

	fn get_vault_delegate(vault_id: VaultId) -> Option<Self::AccountId> {
		VaultsById::get().get(&vault_id).and_then(|a| a.delegate_account_id.clone())
	}

	fn is_vault_open(vault_id: VaultId) -> bool {
		VaultsById::get().get(&vault_id).map(|a| !a.is_closed).unwrap_or_default()
	}

	fn record_vault_frame_earnings(
		funding: &mut Credit<Self::AccountId, Self::Currency>,
		profit: VaultTreasuryFrameEarnings<Self::Balance, Self::AccountId>,
	) -> DispatchResult {
		drop(funding.extract(profit.earnings_for_vault));
		let mut profits = LastVaultProfits::get();
		profits.push(profit);
		LastVaultProfits::set(&profits);
		Ok(())
	}
}

pub struct StaticBitcoinMintedProvider;
impl BitcoinMintedProvider<Balance> for StaticBitcoinMintedProvider {
	fn minted_bitcoin_microgons() -> Balance {
		MintedBitcoinMicrogons::get()
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	type Weights = ();

	fn get_average_microgons_per_argonot(frame_id: FrameId) -> Option<Balance> {
		LastAverageArgonotPriceFrame::set(Some(frame_id));
		Some(AverageArgonotPriceInMicrogons::get())
	}

	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		Some(BitcoinPriceInUsd::get())
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

	fn get_argon_cpi() -> Option<ArgonCPI> {
		None
	}

	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> ArgonCPI {
		AverageArgonCpi::get()
	}

	fn get_circulation() -> Balance {
		CirculatingArgon::get()
	}

	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
}

pub struct StaticMiningFrameTransitionProvider;
impl MiningFrameTransitionProvider for StaticMiningFrameTransitionProvider {
	type Weights = ();

	fn get_current_frame_id() -> FrameId {
		CurrentFrameId::get()
	}

	fn is_new_frame_started() -> Option<FrameId> {
		NewlyStartedFrameId::get()
	}
}

impl MiningFrameProvider for StaticMiningFrameTransitionProvider {
	fn get_next_frame_tick() -> Tick {
		CurrentFrameId::get().saturating_add(1).saturating_mul(1_440)
	}
	fn is_seat_bidding_started() -> bool {
		false
	}
	fn get_tick_range_for_frame(frame_id: FrameId) -> Option<(Tick, Tick)> {
		Some((frame_id * 1_440, (frame_id + 1) * 1_440))
	}
}

impl pallet_treasury::Config for Test {
	type PositionProvider = TreasuryPositions;
	type StabilizerPoolRewardHistoryFrames = ConstU32<13>;
	type OperationalAccountProvider = MockUpstreamAccounts;
	type UpstreamBitcoinTarget = UpstreamBitcoinTarget;
	type UpstreamBondTarget = UpstreamBondTarget;
	type UpstreamBitcoinWeight = UpstreamBitcoinWeight;
	type WeightInfo = ();
	type Balance = Balance;
	type Currency = Balances;
	type OwnershipCurrency = Ownership;
	type RuntimeHoldReason = RuntimeHoldReason;
	type TreasuryVaultProvider = StaticTreasuryVaultProvider;
	type BitcoinMintedProvider = StaticBitcoinMintedProvider;
	type PriceProvider = StaticPriceProvider;
	type MinimumArgonsPerContributor = MinimumArgonsPerContributor;
	type MaxActiveArgonotBondLots = MaxActiveArgonotBondLots;
	type MaxBondCollateralLots = MaxBondCollateralLots;
	type MaxBondCollateralSettlementsPerBlock = MaxBondCollateralSettlementsPerBlock;
	type MaxArgonotBondedPercentOfCirculation = MaxArgonotBondedPercentOfCirculation;
	type PalletId = VaultPalletId;
	type MiningBidPoolAccount = BidPoolAccountId;
	type PercentForMarketing = PercentForMarketing;
	type PercentForStakePool = PercentForStakePool;
	type PercentForMiningOperatorPool = PercentForMiningOperatorPool;
	type PercentForBitcoinLiquidPool = PercentForBitcoinLiquidPool;
	type PercentForArgonBondPool = PercentForArgonBondPool;
	type PercentForStabilizerPool = PercentForStabilizerPool;
	type PercentForVaultPool = PercentForVaultPool;
	type MinimumVaultProfitPercent = MinimumVaultProfitPercent;
	type VaultBitcoinRewardPercent = VaultBitcoinRewardPercent;
	type ArgonotSecuritizationMultiplier = ArgonotSecuritizationMultiplier;
	type DefaultTargetBitcoinPercent = DefaultTargetBitcoinPercent;
	type DefaultTargetStabilizerPoolPercent = DefaultTargetStabilizerPoolPercent;
	type MaxVaultsPerPool = MaxVaultsPerPool;
	type MaxPendingUnlocksPerFrame = MaxPendingUnlocksPerFrame;
	type TreasuryExitDelayFrames = TreasuryExitDelayFrames;
	type MiningFrameTransitionProvider = StaticMiningFrameTransitionProvider;
	type OperationalAccountsHook = TestOperationalAccountsHook;
}

pub(crate) fn new_test_ext() -> TestState {
	CurrentFrameId::set(1);
	CollectClaimFrames::set(Vec::new());
	ExistentialDeposit::set(1);
	NewlyStartedFrameId::set(None);
	CirculatingArgon::set(0);
	ArgonotSecuritizationMultiplier::set(2);
	VaultsById::set(HashMap::new());
	VaultBitcoinSatoshis::set(HashMap::new());
	VaultArgonotMicronots::set(HashMap::new());
	VaultRewardCommittedMicronots::set(HashMap::new());
	ArgonPriceInUsd::set(FixedU128::one());
	ArgonotPriceInUsd::set(FixedU128::one());
	AverageArgonotPriceInMicrogons::set(MICROGONS_PER_ARGON);
	AverageArgonCpi::set(ArgonCPI::zero());
	UpstreamBitcoinTarget::set(5_000 * MICROGONS_PER_ARGON);
	UpstreamBondTarget::set(5_000 * MICROGONS_PER_ARGON);
	Upstreams::set(BTreeMap::new());
	AvailableSecuritizationSpace::set(BTreeMap::new());
	VaultSecuritizedSatoshis::set(HashMap::new());
	let mut ext = new_test_with_genesis::<Test>(|_t| {});
	ext.execute_with(|| {
		<TreasuryPositions as frame_support::traits::OnGenesis>::on_genesis();
		// Keep allocation tests untapered. Taper tests clear this override to use production
		// defaults.
		crate::RewardTaperingPolicy::<Test>::mutate(|policy| {
			policy.argon_bonds.excess_reward_payout_percent = Percent::from_percent(100);
			policy.argonot_stakes.excess_reward_payout_percent = Percent::from_percent(100);
			policy.vault_revenue.excess_reward_payout_percent = Percent::from_percent(100);
		});
	});
	ext
}

impl pallet_treasury_positions::Config for Test {
	type OwnershipCurrency = Ownership;
	type BlockRewardAccountsProvider = ();
	type BitcoinPositionProvider = ();
	type TreasuryPoolProvider = Treasury;
	type EarningsSettlementProvider = MockStabilizerPoolSettlement;
	type VaultEarningsProvider = ();
	type MiningFrameTransitionProvider = StaticMiningFrameTransitionProvider;
	type MaxEarningChangesPerCall = ConstU32<32>;
	type Currency = Balances;
	type PalletId = TreasuryPositionsPalletId;
	type OperationalAccountProvider = MockUpstreamAccounts;
	type Balance = Balance;
	type WeightInfo = ();
}

parameter_types! {
	pub const TreasuryPositionsPalletId: frame_support::PalletId = frame_support::PalletId(*b"arg/colt");
	pub static Upstreams: BTreeMap<TestAccountId, (TestAccountId, VaultId)> = BTreeMap::new();
	pub static AvailableSecuritizationSpace: BTreeMap<VaultId, Balance> = BTreeMap::new();
	pub static UpstreamBitcoinTarget: Balance = 5_000 * MICROGONS_PER_ARGON;
	pub static UpstreamBondTarget: Balance = 5_000 * MICROGONS_PER_ARGON;
	pub const UpstreamBitcoinWeight: Permill = Permill::from_percent(50);
}
pub struct MockUpstreamAccounts;
impl argon_primitives::OperationalAccountProvider<TestAccountId> for MockUpstreamAccounts {
	type Weights = ();
	fn is_eligible(_: &TestAccountId) -> bool {
		true
	}
	fn upstream_vault(account: &TestAccountId) -> Option<(TestAccountId, VaultId)> {
		Upstreams::get().get(account).cloned()
	}
}

pub fn test_vault(account_id: u64, securitization: u64) -> TestVault {
	TestVault {
		account_id: account_id_from_seed(account_id),
		securitization: securitization as Balance,
		exit_notice_amount: 0,
		committed_microgons: 0,
		activated_securitization: 0,
		delegate_account_id: None,
		is_closed: false,
	}
}

/// Exercise the same explicit allocation, funding and wallet collection steps as callers.
pub fn collect_frame(frame: FrameId, owners: &[u64]) {
	assert_ok!(Treasury::distribute_bid_pool(frame));
	for owner in owners {
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(account_id_from_seed(*owner))));
	}
}

pub(crate) fn account_bond_lot_ids(account_id: u64) -> Vec<u64> {
	BondLotIdsByAccount::<Test>::iter_key_prefix(account(account_id)).collect()
}

pub(crate) fn account(seed: u64) -> TestAccountId {
	account_id_from_seed(seed)
}

pub(crate) fn origin(seed: u64) -> RuntimeOrigin {
	RuntimeOrigin::signed(account(seed))
}

pub(crate) fn argonot_bond_lots() -> Vec<BondLotSummary> {
	ArgonotBondLots::<Test>::get().into_inner()
}

pub(crate) fn set_target_securitization(amount: Balance) {
	TargetBitcoinPercent::<Test>::put(Percent::from_percent(100));
	MintedBitcoinMicrogons::set(0);
	Balances::set_total_issuance(amount);
}

pub(crate) fn set_vault_securitized_satoshis(vault_id: VaultId, satoshis: u64) {
	let previous = VaultSecuritizedSatoshis::get().get(&vault_id).copied().unwrap_or_default();
	assert_ok!(TreasuryPositions::network_securitized_satoshis_updated(
		previous as u128,
		satoshis as u128,
	));
	VaultSecuritizedSatoshis::mutate(|bitcoin| {
		bitcoin.insert(vault_id, satoshis);
	});
}

pub(crate) fn bonus_approval(
	vault_id: u32,
	beneficiary: u64,
	bonus_percent: Permill,
	expires_at_frame: FrameId,
	bond_space_to_unreserve: Bonds,
	nonce: u64,
) -> TreasuryBonusApprovalProof {
	let beneficiary = account(beneficiary);
	let message = (
		TREASURY_BONUS_APPROVAL_PROOF_MESSAGE_KEY,
		vault_id,
		beneficiary.clone(),
		bonus_percent,
		expires_at_frame,
		bond_space_to_unreserve,
		nonce,
	)
		.using_encoded(blake2_256);
	let signature: Signature = account_pair_from_seed(10).sign(message.as_slice()).into();
	TreasuryBonusApprovalProof {
		vault_id,
		beneficiary,
		bonus_percent,
		expires_at_frame,
		bond_space_to_unreserve,
		nonce,
		signature,
	}
}
