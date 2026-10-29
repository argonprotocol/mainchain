use super::prelude::*;
use argon_primitives::{EthereumBlockNumber, MICROGONS_PER_ARGON};
use pallet_transaction_payment::Multiplier;
use polkadot_sdk::sp_runtime::Permill;
use smallvec::smallvec;
use sp_runtime::traits::{AccountIdConversion, One};

pub type AccountData = pallet_balances::AccountData<Balance>;

/// TODO: adjust this to match measured
/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// TODO: measure this
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(10), u64::MAX);

/// The existential deposit.
pub const ARGON_EXISTENTIAL_DEPOSIT: Balance = 10_000;
pub const ARGONOT_EXISTENTIAL_DEPOSIT: Balance = 10_000;

pub const ARGON: Balance = 1_000_000;
pub const CENTS: Balance = ARGON / 100_000;
pub const MILLIGONS: Balance = 1_000;
pub const MICROGONS: Balance = 1;

const FINAL_ARGONS_PER_BLOCK: Balance = 5_000_000;
const INCREMENTAL_REWARD_AMOUNT: Balance = 1_000;
const INCREMENT_TICKS: Tick = 180;

pub type NotaryRecordT =
	NotaryRecordWithState<AccountId, BlockNumber, MaxNotaryHosts, NotebookVerifyError>;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 100 * CENTS + (bytes as Balance) * 5 * MICROGONS
}
parameter_types! {
	// ### ethereum verifier

	pub const EthereumFreeHeadersInterval: u32 = 32;

	// ### block weights

	pub const BlockHashCount: BlockNumber = 4096;
	/// We allow for 60 seconds of compute with a 10 second average block time.
	pub BlockWeights:  frame_system::limits::BlockWeights =  frame_system::limits::BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength::builder()
		.max_length(5 * 1024 * 1024)
		.modify_max_length_for_class(DispatchClass::Normal, |m| *m = NORMAL_DISPATCH_RATIO * *m)
		.build();

	// ### pallet_block_seal
	pub const TargetComputeBlockPercent: FixedU128 = FixedU128::from_rational(49, 100); // aim for less than full compute time so it can wait for notebooks
	pub const TargetBlockVotes: u32 = 50_000;
	pub const SealSpecVoteHistoryForAverage: u32 = 24 * 60; // 24 hours of history
	pub const SealSpecComputeHistoryToTrack: u32 = 10; // track 10 blocks

	pub const DefaultChannelHoldDuration: Tick = 60;
	pub const HistoricalPaymentAddressTicksToKeep: Tick = DefaultChannelHoldDuration::get() + CHANNEL_HOLD_CLAWBACK_TICKS + 10;

	// ### pallet_rewards
	pub const StartingArgonsPerBlock: Balance = 500_000;
	pub const StartingOwnershipTokensPerBlock: Balance = 500_000;
	pub const IncrementalGrowth: GrowthPath<Balance> = (INCREMENTAL_REWARD_AMOUNT, INCREMENT_TICKS, FINAL_ARGONS_PER_BLOCK); // we add 1 milligon every 180 blocks until we reach 5 argons/ownership tokens
	pub const HalvingBeginTick: Tick = INCREMENT_TICKS  * (FINAL_ARGONS_PER_BLOCK as Tick - StartingArgonsPerBlock::get() as Tick) / INCREMENTAL_REWARD_AMOUNT as Tick; // starts after ~ one year of increments
	pub const HalvingTicks: Tick = 2_100_000; // based on bitcoin, but 10x since we're 1 block per minute
	pub const MinerPayoutPercent: FixedU128 = FixedU128::from_rational(75, 100);
	pub const DomainExpirationTicks: Tick = 60 * 24 * 365; // 1 year
	pub const BlockRewardsCohortHistoryToKeep: u32 = FramesPerMiningTerm::get() + 1;
	pub const EpochTicks: Tick = 10 * 1440; // 10 days
	pub const PayoutHistoryBlocks: u32 = 5;
	pub const BlockRewardsDampener: FixedU128 = FixedU128::from_rational(75, 100); // 75% dampener

	// ### pallet_treasury
	pub const MaxActiveArgonotBondLots: u32 = 1_000;
	pub const MaxArgonBondLots: u32 = 15_000;
	pub const MaxVaultsPerPool: u32 = 100;
	pub const TreasuryInternalPalletId: PalletId = PalletId(*b"lqdPools");
	pub TreasuryMiningBidPoolAccount: AccountId =
		TreasuryInternalPalletId::get().into_account_truncating();
	pub TreasuryReservesAccount: AccountId =
		TreasuryInternalPalletId::get().into_sub_account_truncating(*b"treasury-reserve");
	pub const PercentForTreasuryReserves: Percent = Percent::from_percent(20);
	pub const PercentForStakePool: Percent = Percent::from_percent(15);
	pub const PercentForMiningOperatorPool: Percent = Percent::from_percent(6);
	pub const PercentForBitcoinLiquidPool: Percent = Percent::from_percent(3);
	pub const PercentForArgonBondPool: Percent = Percent::from_percent(5);
	pub const PercentForVaultPool: Percent = Percent::from_percent(51);
	pub const MinimumVaultProfitPercent: Percent = Percent::from_percent(1);
	pub const ArgonotSecuritizationMultiplier: u32 = 0;
	pub const DefaultTargetBitcoinPercent: Percent = Percent::from_percent(15);
	pub const MaxArgonotBondedPercentOfCirculation: Percent = Percent::from_percent(60);
	/// Activated upstream Bitcoin securitization, in microgons, required for full vault earnings.
	pub TreasuryUpstreamBitcoinForFullEarnings: Balance =
		OperationalMinimumBitcoinLiquidity::get().saturating_mul(2);
	/// Active upstream ARGON bond principal, in microgons, required for full vault earnings.
	pub TreasuryUpstreamBondsForFullEarnings: Balance =
		OperationalMinimumVaultBondPrincipal::get().saturating_mul(2);
	/// Bitcoin's share of the upstream participation factor applied to vault earnings.
	pub const TreasuryUpstreamBitcoinEarningsWeight: Permill = Permill::from_percent(50);

	// ### pallet_mining_slot
	pub const FramesPerMiningTerm: u32 = 10;
	pub const MinCohortSize: u32 = 10;
	pub const MaxCohortSize: u32 = 144;
	pub const MaxGrandpas: u32 = 1000;
	pub const PricePerSeatAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);
	pub const TargetPricePerSeat: Balance = 1000 * ARGON;
	pub const InitialArgonotsPerSeat: Balance = ARGONOT_EXISTENTIAL_DEPOSIT;
	pub const ArgonotBidCollateralMultiple: u128 = 2;
	pub const GrandpaRotationBlocks: BlockNumber = 260;
	pub const MiningSlotBidIncrement: Balance = 10 * MILLIGONS;

	// ### pallet_vaults
	pub const TicksPerDay: Tick = 1440;
	pub const TicksPerYear: Tick = 1440 * 365;
	pub const MaxVaults: u32 = 10_000;
	pub const MaxPendingCosignsPerVault: u32 = 100;

	const BitcoinBlocksPerDay: BitcoinHeight = 6 * 24;
	pub const BitcoinLockDurationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 365; // 1 year
	pub const BitcoinLockReclamationBlocks: BitcoinHeight = BitcoinBlocksPerDay::get() * 30; // 30 days
	pub const LockReleaseCosignDeadlineFrames: FrameId = 10;
	pub const OrphanedUtxoReleaseExpiryFrames: FrameId = 10;
	pub const TicksPerBitcoinBlock: Tick = 10;

	pub const MaxSetIdSessionEntries: u32 = 2u32;

	pub const MaxPendingTermModificationsPerTick: u32 = 100;

	// ### pallet localchain transfer
	pub const LocalchainTransferPalletId: PalletId = PalletId(*b"transfer");

	// ### pallet_crosschain_transfer
	pub const CrosschainTransferPalletId: PalletId = PalletId(*b"xchaintr");
	pub CrosschainTransferEthereumBurnAccount: AccountId = CrosschainTransferPalletId::get()
		.into_sub_account_truncating((pallet_crosschain_transfer::SourceChain::Ethereum, *b"burn"));

	/// How long a transfer should remain in storage before returning.
	pub const TransferExpirationTicks: u32 = 1400 * 10;

	/// How many transfers out can be queued per block
	pub const MaxPendingTransfersOutPerBlock: u32 = 1000;

	// ### pallet_notary
	pub const MaxActiveNotaries: u32 = 25; // arbitrarily set
	pub const MaxProposalHoldBlocks: u32 = 1440 * 14; // 2 weeks to approve
	pub const MaxProposalsPerBlock: u32 = 10;
	pub const MetaChangesTickDelay: u64 = 6; // delay pubkey changes for minimum of an hour
	pub const MaxTicksForKeyHistory: u32 = 1440 * 2; // keep for 2 days.. only used for notebook submission
	/// Max host ips a notary can provide
	pub const MaxNotaryHosts: u32 = 4;

	// ### pallet_proxy

	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxProxies: u16 = 32;
	pub const MaxPending: u16 = 32;

	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;

	// ### pallet_bitcoin_locks
	pub const MaxConcurrentlyReleasingLocks: u32 = 1000;
	/// Max locks that can expire in a single bitcoin block - effectively the max throughput of locks per bitcoin block (10 minutes)
	pub const MaxConcurrentlyExpiringLocks: u32 = 10_000;
	pub const MaxBtcPriceTickAge: u32 = 30; // 30 minutes of btc prices allowed for locks

	pub const BitcoinLockDuration: u32 = 60 * 24 * 365; // 1 year
	/// Maximum number of active Fissions that may allocate one Lock's funded satoshis.
	pub const MaxFissionsPerLock: u32 = 50;
	/// Minimum percentage rate movement required before a Fission may ratchet.
	pub const MinimumFissionRatchetPercent: Percent = Percent::from_percent(5);
	pub const MaxPendingMintsPerUtxo: u32 = 50;
	pub const MaxPendingMintPayoutWindowSize: u32 = 1_000;
	/// The maximum share of a queued bitcoin mint that may be paid in a single frame.
	pub const BitcoinMintPayoutPercentPerFrame: Percent = Percent::from_percent(10);
	pub const MaxMintHistoryToMaintain: u32 = 10; // keep all active sessions + the rollover
	pub const MaxPossibleMiners: u32 = MaxCohortSize::get() * 10u32;

	pub const MaxDowntimeTicksBeforeReset: Tick = 60; // 1 hour
	pub const MaxHistoryToKeep: u32 = 24 * 60; // 1 day worth of prices
	pub const MaxPriceAgeInTicks: Tick = 24 * 60; // 1 day
	pub const MaxArgonChangePerTickAwayFromTarget: FixedU128 = FixedU128::from_rational(1, 100); // 1 centagon
	pub const MaxArgonTargetChangePerTick: FixedU128 = FixedU128::from_rational(1, 100); // 1 centagon

	pub const SecuritizationHoldBlocks: BitcoinHeight = 6 * 24; // 1 day
	pub const MaxUtxosPerLock: u32 = 100;

	// Fees
	pub FeeMultiplier: Multiplier = Multiplier::one();
	pub const TransactionByteFee: Balance = 1;

	/// Minimum remaining frame horizon required when crosschain queries committed vault
	/// securitization.
	pub const CouncilRotationFrames: FrameId = 10;
	/// Number of frame-level Argonot floor buckets retained by the price index.
	pub const MaxArgonotFloorHistoryFrames: u32 = 10;
	/// Number of frame-level Argonot average buckets retained by the price index.
	pub const MaxArgonotAverageHistoryFrames: u32 = 10;
	/// Number of frame-level Ethereum price buckets retained by the price index.
	pub const MaxEthereumPriceHistoryFrames: u32 = 10;
	/// Maximum size of the active Global Issuance Council member set.
	pub const MaxCouncilMembers: u32 = 100;
	/// Maximum number of contiguous council approval signatures accepted in one extrinsic.
	pub const MaxQueueApprovalsPerCall: u32 = 32;
	/// Ethereum blocks added to the latest verified execution height when opening a
	/// transfer out. This is approximately 10 days at 12-second block times.
	pub const TransferOutValidityEthereumBlocks: EthereumBlockNumber = 72_000;
	/// Maximum age, in ticks, of the verified Ethereum execution anchor used to open a transfer
	/// out.
	pub const MaxVerifiedExecutionBlockAgeTicks: Tick = 60;
	/// Minting authority tip charged on transfer-out requests in basis points.
	pub const TransferOutMintingAuthorityTipBasisPoints: u32 = 150;
	/// Minimum normalized collateral increment accepted for one transfer-out row unless the row
	/// fills the remaining uncovered amount.
	pub const MinTransferCollateralIncrement: Balance = 10_000;
	/// Default minimum normalized microgon value required to register a Minting Authority.
	pub const DefaultMinimumMintingAuthorityMicrogonValue: Balance = 10_000;
	/// Maximum number of non-terminal transfer-out requests tracked for one destination chain.
	pub const MaxPendingTransferOutsPerDestinationChain: u32 = 100;
	/// Maximum number of gateway activities carried by one proven gateway block.
	///
	/// The pallet integrity checks require this envelope to cover the contract's per-block
	/// activity cap, including maximum-size events, encoded proof bytes, and provider weights.
	/// 128 includes 25 worst-case Vault callbacks per activity, plus proof verification,
	/// settlement, and pool validation, below half the available Normal extrinsic weight.
	pub const MaxActivitiesPerGatewayProof: u32 = 128;
	/// Reserved byte budget for the signed extrinsic envelope, excluding its call.
	/// This is runtime policy, not part of the Ethereum proof encoding.
	pub const GatewayProofExtrinsicOverheadBytes: u32 = 512;
	/// Maximum ordered activities sharing a receipt proof.
	pub const MaxActivitiesPerReceiptProof: u32 = 16;
	/// Maximum receipt proofs submitted together, bounded by the runtime's byte envelope.
	/// Six blocks leave room for 4-KiB receipt nodes and 2,688-byte gateway logs.
	pub const MaxReceiptProofsPerExtrinsic: u32 = 6;
	// ## pallet_bootstrap
	/// Maximum number of encrypted bootstrap payload bytes.
	pub const MaxEncryptedBootstrapPayloadLen: u32 = 256;
	// ## pallet_operational_accounts
	/// Maximum number of available operational access codes allowed at once.
	pub const MaxAvailableOperationalAccessCodes: u32 = 3;
	/// Maximum number of accrued operational access codes awarded at one frame boundary.
	pub const MaxOperationalAccessCodeAwardsPerFrame: u32 = 1_000;
	/// Maximum number of queued operational rewards.
	pub const OperationalMaxRewardsQueued: u32 = 1_000;
	/// Minimum Uniswap transfer amount required to register.
	pub const MinimumUniswapTransfer: Balance = 250 * MICROGONS_PER_ARGON;
	/// Minimum Bitcoin-backed ARGON liquidity, in microgons, required to register and activate.
	pub const OperationalMinimumBitcoinLiquidity: Balance = 2_500 * MICROGONS_PER_ARGON;
	/// Bitcoin valuation tolerance for the liquidity minimum at registration and activation.
	pub const OperationalBitcoinLiquidityTolerance: Percent = Percent::from_percent(1);
	/// Minimum active ARGON vault bond principal, in microgons, required to register and activate.
	pub const OperationalMinimumVaultBondPrincipal: Balance = 2_500 * MICROGONS_PER_ARGON;
	/// Minimum total Uniswap transfer amount required for operational certification.
	pub const OperationalMinimumUniswapTransfer: Balance = 3_000 * MICROGONS_PER_ARGON;
	/// Minimum vault securitization (base units) required to become operational.
	pub const OperationalMinimumVaultSecuritization: Balance = 2_500 * MICROGONS_PER_ARGON;
	/// Additional argon amount (base units) required per follow-on access code after an account
	/// becomes operational.
	pub const BitcoinLockSizeForAccessCode: Balance = 5_000 * MICROGONS_PER_ARGON;
	/// Mining seats required to become operational.
	pub const MiningSeatsForOperational: u32 = 2;
	/// Mining seats required per follow-on access code after an account becomes operational.
	pub const MiningSeatsPerAccessCode: u32 = 5;
	/// Number of operational certifications required per bonus reward.
	pub const OperationalCertificationsPerBonusReward: u32 = 5;
	/// Default reward paid when an account becomes operationally certified.
	pub const OperationalCertificationReward: Balance = 500 * MICROGONS_PER_ARGON;
	/// Default bonus reward paid every operational certification threshold.
	pub const OperationalCertificationBonusReward: Balance = 5_000 * MICROGONS_PER_ARGON;
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct ArgonWeightToFee;
impl WeightToFeePolynomial for ArgonWeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let p = ARGON; // microgons
		let q = 10_000 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		// BAB - disabling wage protector for fees. Makes it hard to keep system stable
		// let cpi = PriceIndex::get_argon_cpi().unwrap_or(ArgonCPI::zero());
		// if cpi.is_positive() {
		// 	let cpi = cpi.into_inner() / ArgonCPI::accuracy();
		// 	let adjustment = (p * (cpi as u128) * 1_000).checked_div(1_000).unwrap_or_default();
		// 	p += adjustment;
		// }
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pallet_mining_slot::WeightInfo as MiningWeightInfo;

	#[test]
	fn mining_auction_settlement_refunds_before_treasury_distribution() {
		use argon_runtime::{
			Balances, MiningSlot, Ownership, Runtime, RuntimeEvent, RuntimeGenesisConfig,
			RuntimeOrigin, SessionKeys, System, Treasury,
		};
		use codec::Decode;
		use frame_support::{assert_ok, traits::Hooks};
		use polkadot_sdk::{sp_io::TestExternalities, sp_runtime::BuildStorage};

		TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap())
			.execute_with(|| {
				let miners = (1..=11).map(|id| AccountId::new([id; 32])).collect::<Vec<_>>();
				let staker = AccountId::new([20; 32]);
				for miner in &miners {
					assert_ok!(Balances::mint_into(miner, 1000 * ARGON));
					assert_ok!(Ownership::mint_into(miner, ARGON));
				}
				assert_ok!(Balances::mint_into(&staker, ARGON));
				assert_ok!(Ownership::mint_into(&staker, 1000 * ARGON));
				pallet_operational_accounts::IsOperationalAccountInviteOnly::<Runtime>::put(false);
				pallet_mining_slot::MiningConfig::<Runtime>::put(MiningSlotConfig {
					slot_bidding_start_after_ticks: 0,
					ticks_between_slots: 40,
					ticks_before_bid_end_for_vrf_close: 0,
				});
				pallet_mining_slot::FrameRewardTicksRemaining::<Runtime>::put(40);
				assert_ok!(Treasury::buy_argonot_bonds(RuntimeOrigin::signed(staker.clone()), 100));
				let keys =
					SessionKeys::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes())
						.unwrap();
				let pool_account = TreasuryMiningBidPoolAccount::get();
				let reserves_account = TreasuryReservesAccount::get();
				let initial_issuance = Balances::total_issuance();
				let initial_staker_balance = Balances::balance(&staker);
				let winning_price = 100 * ARGON + MiningSlotBidIncrement::get();
				let final_pool = 10 * winning_price;
				let stake_payout = PercentForStakePool::get().mul_floor(final_pool);
				let reserves_payout = PercentForTreasuryReserves::get().mul_floor(final_pool);
				let burned = final_pool - stake_payout - reserves_payout;

				for block in 1..=41 {
					pallet_ticks::CurrentTick::<Runtime>::put(39 + block as u64);
					System::initialize(&block, &System::parent_hash(), &Digest::default());
					MiningSlot::on_initialize(block);
					match block {
						2 =>
							for (index, miner) in miners[..10].iter().enumerate() {
								let max = if index == 9 { 100 * ARGON } else { 150 * ARGON };
								let hash = MiningSlot::bid_commitment(
									2,
									miner,
									miner,
									90 * ARGON,
									max,
									&keys,
									[index as u8; 32],
								);
								assert_ok!(MiningSlot::bid(
									RuntimeOrigin::signed(miner.clone()),
									90 * ARGON,
									keys.clone(),
									None,
									Some(2),
									Some(hash)
								));
							},
						3 =>
							for (index, miner) in miners[..10].iter().enumerate() {
								let max = if index == 9 { 100 * ARGON } else { 150 * ARGON };
								assert_ok!(MiningSlot::bid_max(
									RuntimeOrigin::signed(miner.clone()),
									max,
									[index as u8; 32],
									None,
									2
								));
							},
						13 => {
							let miner = &miners[10];
							let hash = MiningSlot::bid_commitment(
								2,
								miner,
								miner,
								winning_price,
								125 * ARGON,
								&keys,
								[11; 32],
							);
							assert_ok!(MiningSlot::bid(
								RuntimeOrigin::signed(miner.clone()),
								winning_price,
								keys.clone(),
								None,
								Some(2),
								Some(hash)
							));
						},
						14 => {
							assert_ok!(MiningSlot::bid_max(
								RuntimeOrigin::signed(miners[10].clone()),
								125 * ARGON,
								[11; 32],
								None,
								2
							));
						},
						_ => {},
					}
					MiningSlot::on_finalize(block);
					match block {
						1 => assert_eq!(
							pallet_treasury::CurrentFrameArgonotBondParticipants::<Runtime>::get()
								.unwrap()
								.frame_id,
							1
						),
						12 => {
							assert_eq!(Balances::balance(&pool_account), 900 * ARGON);
							assert_eq!(Balances::balance(&miners[9]), 910 * ARGON);
						},
						23..=40 => {
							let cohort =
								pallet_mining_slot::BidsForNextSlotCohort::<Runtime>::get();
							assert_eq!(cohort.len(), 10);
							assert!(cohort.iter().all(|r| r.bid == winning_price));
							assert!(!cohort.iter().any(|r| r.account_id == miners[9]));
							assert_eq!(Balances::balance(&miners[9]), 1000 * ARGON);
							assert_eq!(Balances::balance(&pool_account), final_pool);
							assert_eq!(Balances::balance(&reserves_account), 0);
							assert_eq!(Balances::balance(&staker), initial_staker_balance);
						},
						_ => {},
					}
				}
				let cohort = pallet_mining_slot::MinersByCohort::<Runtime>::get(2);
				assert_eq!(cohort.len(), 10);
				assert!(cohort.iter().all(|r| r.bid == winning_price));
				assert!(!cohort.iter().any(|r| r.account_id == miners[9]));
				assert_eq!(Balances::balance(&pool_account), 0);
				assert_eq!(Balances::balance(&reserves_account), reserves_payout);
				assert_eq!(Balances::balance(&staker), initial_staker_balance + stake_payout);
				assert_eq!(Balances::total_issuance(), initial_issuance - burned);
				assert!(pallet_mining_slot::PendingBids::<Runtime>::iter().next().is_none());
				assert!(pallet_mining_slot::BidBatches::<Runtime>::iter().next().is_none());
				assert!(System::events().iter().any(|record| matches!(
					&record.event,
					RuntimeEvent::Treasury(pallet_treasury::Event::FrameEarningsDistributed {
						frame_id: 1,
						bid_pool_distributed,
						stake_pool_distributed,
						argon_bond_pool_distributed: 0,
						vault_pool_distributed: 0,
						burned: actual_burn,
						treasury_reserves,
						participating_vaults: 0,
					}) if *bid_pool_distributed == stake_payout
						&& *stake_pool_distributed == stake_payout
						&& *actual_burn == burned && *treasury_reserves == reserves_payout
				)));
			});
	}

	#[test]
	fn mining_batch_and_activation_fit_initialization_budget() {
		type Runtime = argon_runtime::Runtime;
		type MiningWeights = <Runtime as pallet_mining_slot::Config>::WeightInfo;
		let bids = <<Runtime as pallet_mining_slot::Config>::MaxBidsPerBlock as Get<u32>>::get();
		let seats = <Runtime as pallet_mining_slot::Config>::MaxCohortSize::get();
		let settlement = MiningWeights::settle_bids(bids, seats);
		assert!(settlement.ref_time() > 0, "settlement must have measured weights");
		let nonactivating =
			settlement.saturating_add(MiningWeights::on_finalize_grandpa_rotation());
		// Activation waits until every submission block has settled and the buffer has
		// passed. These paths cannot coincide; empty settlement is covered by bookkeeping.
		assert!(MiningWeights::settle_bids(0, seats).is_zero());
		let activation = MiningWeights::start_new_frame(seats)
			.saturating_add(MiningWeights::on_finalize_frame_adjustments())
			.saturating_add(
				<<Runtime as pallet_mining_slot::Config>::OperationalAccountsHook as
					argon_primitives::OperationalAccountsHook<AccountId, Balance>>::Weights::mining_seat_won()
					.saturating_mul(seats.into()),
			);
		// Other pallets' frame callbacks reserve their own weights separately.
		let mining = MiningWeights::auction_bookkeeping(1)
			.saturating_add(MiningWeights::on_finalize_record_block_author())
			.saturating_add(nonactivating.max(activation));
		assert!(
			mining.all_lte(AVERAGE_ON_INITIALIZE_RATIO * MAXIMUM_BLOCK_WEIGHT),
			"maximum mining batch and activation exceed the initialization allowance: {mining:?}"
		);
	}
}
