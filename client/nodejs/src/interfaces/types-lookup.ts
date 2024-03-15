// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/types/lookup';

import type { BTreeMap, Bytes, Compact, Enum, Null, Option, Result, Struct, Text, U256, U8aFixed, Vec, bool, i128, u128, u16, u32, u64, u8 } from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, Call, H256, MultiAddress } from '@polkadot/types/interfaces/runtime';
import type { Event } from '@polkadot/types/interfaces/system';

declare module '@polkadot/types/lookup' {
  /** @name FrameSystemAccountInfo (3) */
  interface FrameSystemAccountInfo extends Struct {
    readonly nonce: u32;
    readonly consumers: u32;
    readonly providers: u32;
    readonly sufficients: u32;
    readonly data: PalletBalancesAccountData;
  }

  /** @name PalletBalancesAccountData (5) */
  interface PalletBalancesAccountData extends Struct {
    readonly free: u128;
    readonly reserved: u128;
    readonly frozen: u128;
    readonly flags: u128;
  }

  /** @name FrameSupportDispatchPerDispatchClassWeight (8) */
  interface FrameSupportDispatchPerDispatchClassWeight extends Struct {
    readonly normal: SpWeightsWeightV2Weight;
    readonly operational: SpWeightsWeightV2Weight;
    readonly mandatory: SpWeightsWeightV2Weight;
  }

  /** @name SpWeightsWeightV2Weight (9) */
  interface SpWeightsWeightV2Weight extends Struct {
    readonly refTime: Compact<u64>;
    readonly proofSize: Compact<u64>;
  }

  /** @name SpRuntimeDigest (14) */
  interface SpRuntimeDigest extends Struct {
    readonly logs: Vec<SpRuntimeDigestDigestItem>;
  }

  /** @name SpRuntimeDigestDigestItem (16) */
  interface SpRuntimeDigestDigestItem extends Enum {
    readonly isOther: boolean;
    readonly asOther: Bytes;
    readonly isConsensus: boolean;
    readonly asConsensus: ITuple<[U8aFixed, Bytes]>;
    readonly isSeal: boolean;
    readonly asSeal: ITuple<[U8aFixed, Bytes]>;
    readonly isPreRuntime: boolean;
    readonly asPreRuntime: ITuple<[U8aFixed, Bytes]>;
    readonly isRuntimeEnvironmentUpdated: boolean;
    readonly type: 'Other' | 'Consensus' | 'Seal' | 'PreRuntime' | 'RuntimeEnvironmentUpdated';
  }

  /** @name FrameSystemEventRecord (19) */
  interface FrameSystemEventRecord extends Struct {
    readonly phase: FrameSystemPhase;
    readonly event: Event;
    readonly topics: Vec<H256>;
  }

  /** @name FrameSystemEvent (21) */
  interface FrameSystemEvent extends Enum {
    readonly isExtrinsicSuccess: boolean;
    readonly asExtrinsicSuccess: {
      readonly dispatchInfo: FrameSupportDispatchDispatchInfo;
    } & Struct;
    readonly isExtrinsicFailed: boolean;
    readonly asExtrinsicFailed: {
      readonly dispatchError: SpRuntimeDispatchError;
      readonly dispatchInfo: FrameSupportDispatchDispatchInfo;
    } & Struct;
    readonly isCodeUpdated: boolean;
    readonly isNewAccount: boolean;
    readonly asNewAccount: {
      readonly account: AccountId32;
    } & Struct;
    readonly isKilledAccount: boolean;
    readonly asKilledAccount: {
      readonly account: AccountId32;
    } & Struct;
    readonly isRemarked: boolean;
    readonly asRemarked: {
      readonly sender: AccountId32;
      readonly hash_: H256;
    } & Struct;
    readonly isUpgradeAuthorized: boolean;
    readonly asUpgradeAuthorized: {
      readonly codeHash: H256;
      readonly checkVersion: bool;
    } & Struct;
    readonly type: 'ExtrinsicSuccess' | 'ExtrinsicFailed' | 'CodeUpdated' | 'NewAccount' | 'KilledAccount' | 'Remarked' | 'UpgradeAuthorized';
  }

  /** @name FrameSupportDispatchDispatchInfo (22) */
  interface FrameSupportDispatchDispatchInfo extends Struct {
    readonly weight: SpWeightsWeightV2Weight;
    readonly class: FrameSupportDispatchDispatchClass;
    readonly paysFee: FrameSupportDispatchPays;
  }

  /** @name FrameSupportDispatchDispatchClass (23) */
  interface FrameSupportDispatchDispatchClass extends Enum {
    readonly isNormal: boolean;
    readonly isOperational: boolean;
    readonly isMandatory: boolean;
    readonly type: 'Normal' | 'Operational' | 'Mandatory';
  }

  /** @name FrameSupportDispatchPays (24) */
  interface FrameSupportDispatchPays extends Enum {
    readonly isYes: boolean;
    readonly isNo: boolean;
    readonly type: 'Yes' | 'No';
  }

  /** @name SpRuntimeDispatchError (25) */
  interface SpRuntimeDispatchError extends Enum {
    readonly isOther: boolean;
    readonly isCannotLookup: boolean;
    readonly isBadOrigin: boolean;
    readonly isModule: boolean;
    readonly asModule: SpRuntimeModuleError;
    readonly isConsumerRemaining: boolean;
    readonly isNoProviders: boolean;
    readonly isTooManyConsumers: boolean;
    readonly isToken: boolean;
    readonly asToken: SpRuntimeTokenError;
    readonly isArithmetic: boolean;
    readonly asArithmetic: SpArithmeticArithmeticError;
    readonly isTransactional: boolean;
    readonly asTransactional: SpRuntimeTransactionalError;
    readonly isExhausted: boolean;
    readonly isCorruption: boolean;
    readonly isUnavailable: boolean;
    readonly isRootNotAllowed: boolean;
    readonly type: 'Other' | 'CannotLookup' | 'BadOrigin' | 'Module' | 'ConsumerRemaining' | 'NoProviders' | 'TooManyConsumers' | 'Token' | 'Arithmetic' | 'Transactional' | 'Exhausted' | 'Corruption' | 'Unavailable' | 'RootNotAllowed';
  }

  /** @name SpRuntimeModuleError (26) */
  interface SpRuntimeModuleError extends Struct {
    readonly index: u8;
    readonly error: U8aFixed;
  }

  /** @name SpRuntimeTokenError (27) */
  interface SpRuntimeTokenError extends Enum {
    readonly isFundsUnavailable: boolean;
    readonly isOnlyProvider: boolean;
    readonly isBelowMinimum: boolean;
    readonly isCannotCreate: boolean;
    readonly isUnknownAsset: boolean;
    readonly isFrozen: boolean;
    readonly isUnsupported: boolean;
    readonly isCannotCreateHold: boolean;
    readonly isNotExpendable: boolean;
    readonly isBlocked: boolean;
    readonly type: 'FundsUnavailable' | 'OnlyProvider' | 'BelowMinimum' | 'CannotCreate' | 'UnknownAsset' | 'Frozen' | 'Unsupported' | 'CannotCreateHold' | 'NotExpendable' | 'Blocked';
  }

  /** @name SpArithmeticArithmeticError (28) */
  interface SpArithmeticArithmeticError extends Enum {
    readonly isUnderflow: boolean;
    readonly isOverflow: boolean;
    readonly isDivisionByZero: boolean;
    readonly type: 'Underflow' | 'Overflow' | 'DivisionByZero';
  }

  /** @name SpRuntimeTransactionalError (29) */
  interface SpRuntimeTransactionalError extends Enum {
    readonly isLimitReached: boolean;
    readonly isNoLayer: boolean;
    readonly type: 'LimitReached' | 'NoLayer';
  }

  /** @name PalletMiningSlotEvent (31) */
  interface PalletMiningSlotEvent extends Enum {
    readonly isNewMiners: boolean;
    readonly asNewMiners: {
      readonly startIndex: u32;
      readonly newMiners: Vec<UlxPrimitivesBlockSealMiningRegistration>;
    } & Struct;
    readonly isSlotBidderAdded: boolean;
    readonly asSlotBidderAdded: {
      readonly accountId: AccountId32;
      readonly bidAmount: u128;
      readonly index: u32;
    } & Struct;
    readonly isSlotBidderReplaced: boolean;
    readonly asSlotBidderReplaced: {
      readonly accountId: AccountId32;
      readonly bondId: Option<u64>;
      readonly keptOwnershipBond: bool;
    } & Struct;
    readonly isUnbondedMiner: boolean;
    readonly asUnbondedMiner: {
      readonly accountId: AccountId32;
      readonly bondId: Option<u64>;
      readonly keptOwnershipBond: bool;
    } & Struct;
    readonly type: 'NewMiners' | 'SlotBidderAdded' | 'SlotBidderReplaced' | 'UnbondedMiner';
  }

  /** @name UlxPrimitivesBlockSealMiningRegistration (33) */
  interface UlxPrimitivesBlockSealMiningRegistration extends Struct {
    readonly accountId: AccountId32;
    readonly rewardDestination: UlxPrimitivesBlockSealRewardDestination;
    readonly bondId: Option<u64>;
    readonly bondAmount: Compact<u128>;
    readonly ownershipTokens: Compact<u128>;
  }

  /** @name UlxPrimitivesBlockSealRewardDestination (34) */
  interface UlxPrimitivesBlockSealRewardDestination extends Enum {
    readonly isOwner: boolean;
    readonly isAccount: boolean;
    readonly asAccount: AccountId32;
    readonly type: 'Owner' | 'Account';
  }

  /** @name PalletBondEvent (38) */
  interface PalletBondEvent extends Enum {
    readonly isBondFundOffered: boolean;
    readonly asBondFundOffered: {
      readonly bondFundId: u32;
      readonly amountOffered: u128;
      readonly expirationBlock: u32;
      readonly offerAccountId: AccountId32;
    } & Struct;
    readonly isBondFundExtended: boolean;
    readonly asBondFundExtended: {
      readonly bondFundId: u32;
      readonly amountOffered: u128;
      readonly expirationBlock: u32;
    } & Struct;
    readonly isBondFundEnded: boolean;
    readonly asBondFundEnded: {
      readonly bondFundId: u32;
      readonly amountStillBonded: u128;
    } & Struct;
    readonly isBondFundExpired: boolean;
    readonly asBondFundExpired: {
      readonly bondFundId: u32;
      readonly offerAccountId: AccountId32;
    } & Struct;
    readonly isBondedSelf: boolean;
    readonly asBondedSelf: {
      readonly bondId: u64;
      readonly bondedAccountId: AccountId32;
      readonly amount: u128;
      readonly completionBlock: u32;
    } & Struct;
    readonly isBondLeased: boolean;
    readonly asBondLeased: {
      readonly bondFundId: u32;
      readonly bondId: u64;
      readonly bondedAccountId: AccountId32;
      readonly amount: u128;
      readonly totalFee: u128;
      readonly annualPercentRate: u32;
      readonly completionBlock: u32;
    } & Struct;
    readonly isBondExtended: boolean;
    readonly asBondExtended: {
      readonly bondFundId: Option<u32>;
      readonly bondId: u64;
      readonly amount: u128;
      readonly completionBlock: u32;
      readonly feeChange: u128;
      readonly annualPercentRate: u32;
    } & Struct;
    readonly isBondCompleted: boolean;
    readonly asBondCompleted: {
      readonly bondFundId: Option<u32>;
      readonly bondId: u64;
    } & Struct;
    readonly isBondFeeRefund: boolean;
    readonly asBondFeeRefund: {
      readonly bondFundId: u32;
      readonly bondId: u64;
      readonly bondedAccountId: AccountId32;
      readonly bondFundReductionForPayment: u128;
      readonly finalFee: u128;
      readonly refundAmount: u128;
    } & Struct;
    readonly isBondLocked: boolean;
    readonly asBondLocked: {
      readonly bondId: u64;
      readonly bondedAccountId: AccountId32;
    } & Struct;
    readonly isBondUnlocked: boolean;
    readonly asBondUnlocked: {
      readonly bondId: u64;
      readonly bondedAccountId: AccountId32;
    } & Struct;
    readonly type: 'BondFundOffered' | 'BondFundExtended' | 'BondFundEnded' | 'BondFundExpired' | 'BondedSelf' | 'BondLeased' | 'BondExtended' | 'BondCompleted' | 'BondFeeRefund' | 'BondLocked' | 'BondUnlocked';
  }

  /** @name PalletNotariesEvent (40) */
  interface PalletNotariesEvent extends Enum {
    readonly isNotaryProposed: boolean;
    readonly asNotaryProposed: {
      readonly operatorAccount: AccountId32;
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
      readonly expires: u32;
    } & Struct;
    readonly isNotaryActivated: boolean;
    readonly asNotaryActivated: {
      readonly notary: UlxPrimitivesNotaryNotaryRecord;
    } & Struct;
    readonly isNotaryMetaUpdateQueued: boolean;
    readonly asNotaryMetaUpdateQueued: {
      readonly notaryId: u32;
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
      readonly effectiveBlock: u32;
    } & Struct;
    readonly isNotaryMetaUpdated: boolean;
    readonly asNotaryMetaUpdated: {
      readonly notaryId: u32;
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly type: 'NotaryProposed' | 'NotaryActivated' | 'NotaryMetaUpdateQueued' | 'NotaryMetaUpdated';
  }

  /** @name UlxPrimitivesNotaryNotaryMeta (41) */
  interface UlxPrimitivesNotaryNotaryMeta extends Struct {
    readonly public: SpCoreEd25519Public;
    readonly hosts: Vec<UlxPrimitivesHost>;
  }

  /** @name SpCoreEd25519Public (42) */
  interface SpCoreEd25519Public extends U8aFixed {}

  /** @name UlxPrimitivesHost (44) */
  interface UlxPrimitivesHost extends Struct {
    readonly ip: Compact<u32>;
    readonly port: Compact<u16>;
    readonly isSecure: bool;
  }

  /** @name UlxPrimitivesNotaryNotaryRecord (49) */
  interface UlxPrimitivesNotaryNotaryRecord extends Struct {
    readonly notaryId: Compact<u32>;
    readonly operatorAccountId: AccountId32;
    readonly activatedBlock: Compact<u32>;
    readonly metaUpdatedBlock: Compact<u32>;
    readonly meta: UlxPrimitivesNotaryNotaryMeta;
  }

  /** @name PalletNotebookEvent (50) */
  interface PalletNotebookEvent extends Enum {
    readonly isNotebookSubmitted: boolean;
    readonly asNotebookSubmitted: {
      readonly notaryId: u32;
      readonly notebookNumber: u32;
    } & Struct;
    readonly isNotebookAuditFailure: boolean;
    readonly asNotebookAuditFailure: {
      readonly notaryId: u32;
      readonly notebookNumber: u32;
      readonly firstFailureReason: UlxNotaryAuditErrorVerifyError;
    } & Struct;
    readonly type: 'NotebookSubmitted' | 'NotebookAuditFailure';
  }

  /** @name UlxNotaryAuditErrorVerifyError (51) */
  interface UlxNotaryAuditErrorVerifyError extends Enum {
    readonly isMissingAccountOrigin: boolean;
    readonly asMissingAccountOrigin: {
      readonly accountId: AccountId32;
      readonly accountType: UlxPrimitivesAccountAccountType;
    } & Struct;
    readonly isHistoryLookupError: boolean;
    readonly asHistoryLookupError: {
      readonly source: UlxNotaryAuditAccountHistoryLookupError;
    } & Struct;
    readonly isInvalidAccountChangelist: boolean;
    readonly isInvalidChainTransfersList: boolean;
    readonly isInvalidBalanceChangeRoot: boolean;
    readonly isInvalidHeaderTaxRecorded: boolean;
    readonly isInvalidPreviousNonce: boolean;
    readonly isInvalidPreviousBalance: boolean;
    readonly isInvalidPreviousAccountOrigin: boolean;
    readonly isInvalidPreviousBalanceChangeNotebook: boolean;
    readonly isInvalidBalanceChange: boolean;
    readonly isInvalidBalanceChangeSignature: boolean;
    readonly asInvalidBalanceChangeSignature: {
      readonly changeIndex: u16;
    } & Struct;
    readonly isInvalidNoteRecipients: boolean;
    readonly isBalanceChangeError: boolean;
    readonly asBalanceChangeError: {
      readonly changeIndex: u16;
      readonly noteIndex: u16;
      readonly message: Text;
    } & Struct;
    readonly isInvalidNetBalanceChangeset: boolean;
    readonly isInsufficientBalance: boolean;
    readonly asInsufficientBalance: {
      readonly balance: u128;
      readonly amount: u128;
      readonly noteIndex: u16;
      readonly changeIndex: u16;
    } & Struct;
    readonly isExceededMaxBalance: boolean;
    readonly asExceededMaxBalance: {
      readonly balance: u128;
      readonly amount: u128;
      readonly noteIndex: u16;
      readonly changeIndex: u16;
    } & Struct;
    readonly isBalanceChangeMismatch: boolean;
    readonly asBalanceChangeMismatch: {
      readonly changeIndex: u16;
      readonly providedBalance: u128;
      readonly calculatedBalance: i128;
    } & Struct;
    readonly isBalanceChangeNotNetZero: boolean;
    readonly asBalanceChangeNotNetZero: {
      readonly sent: u128;
      readonly claimed: u128;
    } & Struct;
    readonly isInvalidDomainLeaseAllocation: boolean;
    readonly isTaxBalanceChangeNotNetZero: boolean;
    readonly asTaxBalanceChangeNotNetZero: {
      readonly sent: u128;
      readonly claimed: u128;
    } & Struct;
    readonly isMissingBalanceProof: boolean;
    readonly isInvalidPreviousBalanceProof: boolean;
    readonly isInvalidNotebookHash: boolean;
    readonly isInvalidNotebookHeaderHash: boolean;
    readonly isDuplicateChainTransfer: boolean;
    readonly isDuplicatedAccountOriginUid: boolean;
    readonly isInvalidNotarySignature: boolean;
    readonly isNotebookTooOld: boolean;
    readonly isDecodeError: boolean;
    readonly isAccountEscrowHoldDoesntExist: boolean;
    readonly isAccountAlreadyHasEscrowHold: boolean;
    readonly isEscrowHoldNotReadyForClaim: boolean;
    readonly asEscrowHoldNotReadyForClaim: {
      readonly currentTick: u32;
      readonly claimTick: u32;
    } & Struct;
    readonly isAccountLocked: boolean;
    readonly isMissingEscrowHoldNote: boolean;
    readonly isInvalidEscrowHoldNote: boolean;
    readonly isInvalidEscrowClaimers: boolean;
    readonly isEscrowNoteBelowMinimum: boolean;
    readonly isInvalidTaxNoteAccount: boolean;
    readonly isInvalidTaxOperation: boolean;
    readonly isInsufficientTaxIncluded: boolean;
    readonly asInsufficientTaxIncluded: {
      readonly taxSent: u128;
      readonly taxOwed: u128;
      readonly accountId: AccountId32;
    } & Struct;
    readonly isInsufficientBlockVoteTax: boolean;
    readonly isIneligibleTaxVoter: boolean;
    readonly isBlockVoteInvalidSignature: boolean;
    readonly isInvalidBlockVoteAllocation: boolean;
    readonly isInvalidBlockVoteRoot: boolean;
    readonly isInvalidBlockVotesCount: boolean;
    readonly isInvalidBlockVotingPower: boolean;
    readonly isInvalidBlockVoteList: boolean;
    readonly isInvalidComputeProof: boolean;
    readonly isInvalidBlockVoteSource: boolean;
    readonly isInsufficientBlockVoteMinimum: boolean;
    readonly isBlockVoteDataDomainMismatch: boolean;
    readonly isBlockVoteEscrowReused: boolean;
    readonly type: 'MissingAccountOrigin' | 'HistoryLookupError' | 'InvalidAccountChangelist' | 'InvalidChainTransfersList' | 'InvalidBalanceChangeRoot' | 'InvalidHeaderTaxRecorded' | 'InvalidPreviousNonce' | 'InvalidPreviousBalance' | 'InvalidPreviousAccountOrigin' | 'InvalidPreviousBalanceChangeNotebook' | 'InvalidBalanceChange' | 'InvalidBalanceChangeSignature' | 'InvalidNoteRecipients' | 'BalanceChangeError' | 'InvalidNetBalanceChangeset' | 'InsufficientBalance' | 'ExceededMaxBalance' | 'BalanceChangeMismatch' | 'BalanceChangeNotNetZero' | 'InvalidDomainLeaseAllocation' | 'TaxBalanceChangeNotNetZero' | 'MissingBalanceProof' | 'InvalidPreviousBalanceProof' | 'InvalidNotebookHash' | 'InvalidNotebookHeaderHash' | 'DuplicateChainTransfer' | 'DuplicatedAccountOriginUid' | 'InvalidNotarySignature' | 'NotebookTooOld' | 'DecodeError' | 'AccountEscrowHoldDoesntExist' | 'AccountAlreadyHasEscrowHold' | 'EscrowHoldNotReadyForClaim' | 'AccountLocked' | 'MissingEscrowHoldNote' | 'InvalidEscrowHoldNote' | 'InvalidEscrowClaimers' | 'EscrowNoteBelowMinimum' | 'InvalidTaxNoteAccount' | 'InvalidTaxOperation' | 'InsufficientTaxIncluded' | 'InsufficientBlockVoteTax' | 'IneligibleTaxVoter' | 'BlockVoteInvalidSignature' | 'InvalidBlockVoteAllocation' | 'InvalidBlockVoteRoot' | 'InvalidBlockVotesCount' | 'InvalidBlockVotingPower' | 'InvalidBlockVoteList' | 'InvalidComputeProof' | 'InvalidBlockVoteSource' | 'InsufficientBlockVoteMinimum' | 'BlockVoteDataDomainMismatch' | 'BlockVoteEscrowReused';
  }

  /** @name UlxPrimitivesAccountAccountType (52) */
  interface UlxPrimitivesAccountAccountType extends Enum {
    readonly isTax: boolean;
    readonly isDeposit: boolean;
    readonly type: 'Tax' | 'Deposit';
  }

  /** @name UlxNotaryAuditAccountHistoryLookupError (53) */
  interface UlxNotaryAuditAccountHistoryLookupError extends Enum {
    readonly isRootNotFound: boolean;
    readonly isLastChangeNotFound: boolean;
    readonly isInvalidTransferToLocalchain: boolean;
    readonly isBlockSpecificationNotFound: boolean;
    readonly type: 'RootNotFound' | 'LastChangeNotFound' | 'InvalidTransferToLocalchain' | 'BlockSpecificationNotFound';
  }

  /** @name PalletChainTransferEvent (56) */
  interface PalletChainTransferEvent extends Enum {
    readonly isTransferToLocalchain: boolean;
    readonly asTransferToLocalchain: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly accountNonce: u32;
      readonly notaryId: u32;
      readonly expirationBlock: u32;
    } & Struct;
    readonly isTransferToLocalchainExpired: boolean;
    readonly asTransferToLocalchainExpired: {
      readonly accountId: AccountId32;
      readonly accountNonce: u32;
      readonly notaryId: u32;
    } & Struct;
    readonly isTransferIn: boolean;
    readonly asTransferIn: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly notaryId: u32;
    } & Struct;
    readonly type: 'TransferToLocalchain' | 'TransferToLocalchainExpired' | 'TransferIn';
  }

  /** @name PalletBlockSealSpecEvent (57) */
  interface PalletBlockSealSpecEvent extends Enum {
    readonly isVoteMinimumAdjusted: boolean;
    readonly asVoteMinimumAdjusted: {
      readonly expectedBlockVotes: u128;
      readonly actualBlockVotes: u128;
      readonly startVoteMinimum: u128;
      readonly newVoteMinimum: u128;
    } & Struct;
    readonly isComputeDifficultyAdjusted: boolean;
    readonly asComputeDifficultyAdjusted: {
      readonly expectedBlockTime: u64;
      readonly actualBlockTime: u64;
      readonly startDifficulty: u128;
      readonly newDifficulty: u128;
    } & Struct;
    readonly type: 'VoteMinimumAdjusted' | 'ComputeDifficultyAdjusted';
  }

  /** @name PalletDataDomainEvent (58) */
  interface PalletDataDomainEvent extends Enum {
    readonly isZoneRecordUpdated: boolean;
    readonly asZoneRecordUpdated: {
      readonly domainHash: H256;
      readonly zoneRecord: UlxPrimitivesDataDomainZoneRecord;
    } & Struct;
    readonly isDataDomainRegistered: boolean;
    readonly asDataDomainRegistered: {
      readonly domainHash: H256;
      readonly registration: PalletDataDomainDataDomainRegistration;
    } & Struct;
    readonly isDataDomainRenewed: boolean;
    readonly asDataDomainRenewed: {
      readonly domainHash: H256;
    } & Struct;
    readonly isDataDomainExpired: boolean;
    readonly asDataDomainExpired: {
      readonly domainHash: H256;
    } & Struct;
    readonly isDataDomainRegistrationCanceled: boolean;
    readonly asDataDomainRegistrationCanceled: {
      readonly domainHash: H256;
      readonly registration: PalletDataDomainDataDomainRegistration;
    } & Struct;
    readonly type: 'ZoneRecordUpdated' | 'DataDomainRegistered' | 'DataDomainRenewed' | 'DataDomainExpired' | 'DataDomainRegistrationCanceled';
  }

  /** @name UlxPrimitivesDataDomainZoneRecord (59) */
  interface UlxPrimitivesDataDomainZoneRecord extends Struct {
    readonly paymentAccount: AccountId32;
    readonly notaryId: u32;
    readonly versions: BTreeMap<UlxPrimitivesDataDomainSemver, UlxPrimitivesDataDomainVersionHost>;
  }

  /** @name UlxPrimitivesDataDomainSemver (61) */
  interface UlxPrimitivesDataDomainSemver extends Struct {
    readonly major: u32;
    readonly minor: u32;
    readonly patch: u32;
  }

  /** @name UlxPrimitivesDataDomainVersionHost (62) */
  interface UlxPrimitivesDataDomainVersionHost extends Struct {
    readonly datastoreId: Bytes;
    readonly host: UlxPrimitivesHost;
  }

  /** @name PalletDataDomainDataDomainRegistration (66) */
  interface PalletDataDomainDataDomainRegistration extends Struct {
    readonly accountId: AccountId32;
    readonly registeredAtTick: u32;
  }

  /** @name PalletSessionEvent (67) */
  interface PalletSessionEvent extends Enum {
    readonly isNewSession: boolean;
    readonly asNewSession: {
      readonly sessionIndex: u32;
    } & Struct;
    readonly type: 'NewSession';
  }

  /** @name PalletBlockRewardsEvent (68) */
  interface PalletBlockRewardsEvent extends Enum {
    readonly isRewardCreated: boolean;
    readonly asRewardCreated: {
      readonly maturationBlock: u32;
      readonly rewards: Vec<PalletBlockRewardsBlockPayout>;
    } & Struct;
    readonly isRewardUnlocked: boolean;
    readonly asRewardUnlocked: {
      readonly rewards: Vec<PalletBlockRewardsBlockPayout>;
    } & Struct;
    readonly type: 'RewardCreated' | 'RewardUnlocked';
  }

  /** @name PalletBlockRewardsBlockPayout (70) */
  interface PalletBlockRewardsBlockPayout extends Struct {
    readonly accountId: AccountId32;
    readonly ulixees: u128;
    readonly argons: u128;
  }

  /** @name PalletGrandpaEvent (71) */
  interface PalletGrandpaEvent extends Enum {
    readonly isNewAuthorities: boolean;
    readonly asNewAuthorities: {
      readonly authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    } & Struct;
    readonly isPaused: boolean;
    readonly isResumed: boolean;
    readonly type: 'NewAuthorities' | 'Paused' | 'Resumed';
  }

  /** @name SpConsensusGrandpaAppPublic (74) */
  interface SpConsensusGrandpaAppPublic extends SpCoreEd25519Public {}

  /** @name PalletOffencesEvent (75) */
  interface PalletOffencesEvent extends Enum {
    readonly isOffence: boolean;
    readonly asOffence: {
      readonly kind: U8aFixed;
      readonly timeslot: Bytes;
    } & Struct;
    readonly type: 'Offence';
  }

  /** @name PalletBalancesEvent (77) */
  interface PalletBalancesEvent extends Enum {
    readonly isEndowed: boolean;
    readonly asEndowed: {
      readonly account: AccountId32;
      readonly freeBalance: u128;
    } & Struct;
    readonly isDustLost: boolean;
    readonly asDustLost: {
      readonly account: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isTransfer: boolean;
    readonly asTransfer: {
      readonly from: AccountId32;
      readonly to: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isBalanceSet: boolean;
    readonly asBalanceSet: {
      readonly who: AccountId32;
      readonly free: u128;
    } & Struct;
    readonly isReserved: boolean;
    readonly asReserved: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isUnreserved: boolean;
    readonly asUnreserved: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isReserveRepatriated: boolean;
    readonly asReserveRepatriated: {
      readonly from: AccountId32;
      readonly to: AccountId32;
      readonly amount: u128;
      readonly destinationStatus: FrameSupportTokensMiscBalanceStatus;
    } & Struct;
    readonly isDeposit: boolean;
    readonly asDeposit: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isWithdraw: boolean;
    readonly asWithdraw: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isSlashed: boolean;
    readonly asSlashed: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isMinted: boolean;
    readonly asMinted: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isBurned: boolean;
    readonly asBurned: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isSuspended: boolean;
    readonly asSuspended: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isRestored: boolean;
    readonly asRestored: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isUpgraded: boolean;
    readonly asUpgraded: {
      readonly who: AccountId32;
    } & Struct;
    readonly isIssued: boolean;
    readonly asIssued: {
      readonly amount: u128;
    } & Struct;
    readonly isRescinded: boolean;
    readonly asRescinded: {
      readonly amount: u128;
    } & Struct;
    readonly isLocked: boolean;
    readonly asLocked: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isUnlocked: boolean;
    readonly asUnlocked: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isFrozen: boolean;
    readonly asFrozen: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isThawed: boolean;
    readonly asThawed: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly type: 'Endowed' | 'DustLost' | 'Transfer' | 'BalanceSet' | 'Reserved' | 'Unreserved' | 'ReserveRepatriated' | 'Deposit' | 'Withdraw' | 'Slashed' | 'Minted' | 'Burned' | 'Suspended' | 'Restored' | 'Upgraded' | 'Issued' | 'Rescinded' | 'Locked' | 'Unlocked' | 'Frozen' | 'Thawed';
  }

  /** @name FrameSupportTokensMiscBalanceStatus (78) */
  interface FrameSupportTokensMiscBalanceStatus extends Enum {
    readonly isFree: boolean;
    readonly isReserved: boolean;
    readonly type: 'Free' | 'Reserved';
  }

  /** @name PalletMintEvent (79) */
  type PalletMintEvent = Null;

  /** @name PalletTxPauseEvent (81) */
  interface PalletTxPauseEvent extends Enum {
    readonly isCallPaused: boolean;
    readonly asCallPaused: {
      readonly fullName: ITuple<[Bytes, Bytes]>;
    } & Struct;
    readonly isCallUnpaused: boolean;
    readonly asCallUnpaused: {
      readonly fullName: ITuple<[Bytes, Bytes]>;
    } & Struct;
    readonly type: 'CallPaused' | 'CallUnpaused';
  }

  /** @name PalletTransactionPaymentEvent (84) */
  interface PalletTransactionPaymentEvent extends Enum {
    readonly isTransactionFeePaid: boolean;
    readonly asTransactionFeePaid: {
      readonly who: AccountId32;
      readonly actualFee: u128;
      readonly tip: u128;
    } & Struct;
    readonly type: 'TransactionFeePaid';
  }

  /** @name PalletSudoEvent (85) */
  interface PalletSudoEvent extends Enum {
    readonly isSudid: boolean;
    readonly asSudid: {
      readonly sudoResult: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly isKeyChanged: boolean;
    readonly asKeyChanged: {
      readonly old: Option<AccountId32>;
      readonly new_: AccountId32;
    } & Struct;
    readonly isKeyRemoved: boolean;
    readonly isSudoAsDone: boolean;
    readonly asSudoAsDone: {
      readonly sudoResult: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly type: 'Sudid' | 'KeyChanged' | 'KeyRemoved' | 'SudoAsDone';
  }

  /** @name FrameSystemPhase (89) */
  interface FrameSystemPhase extends Enum {
    readonly isApplyExtrinsic: boolean;
    readonly asApplyExtrinsic: u32;
    readonly isFinalization: boolean;
    readonly isInitialization: boolean;
    readonly type: 'ApplyExtrinsic' | 'Finalization' | 'Initialization';
  }

  /** @name FrameSystemLastRuntimeUpgradeInfo (93) */
  interface FrameSystemLastRuntimeUpgradeInfo extends Struct {
    readonly specVersion: Compact<u32>;
    readonly specName: Text;
  }

  /** @name FrameSystemCodeUpgradeAuthorization (94) */
  interface FrameSystemCodeUpgradeAuthorization extends Struct {
    readonly codeHash: H256;
    readonly checkVersion: bool;
  }

  /** @name FrameSystemCall (95) */
  interface FrameSystemCall extends Enum {
    readonly isRemark: boolean;
    readonly asRemark: {
      readonly remark: Bytes;
    } & Struct;
    readonly isSetHeapPages: boolean;
    readonly asSetHeapPages: {
      readonly pages: u64;
    } & Struct;
    readonly isSetCode: boolean;
    readonly asSetCode: {
      readonly code: Bytes;
    } & Struct;
    readonly isSetCodeWithoutChecks: boolean;
    readonly asSetCodeWithoutChecks: {
      readonly code: Bytes;
    } & Struct;
    readonly isSetStorage: boolean;
    readonly asSetStorage: {
      readonly items: Vec<ITuple<[Bytes, Bytes]>>;
    } & Struct;
    readonly isKillStorage: boolean;
    readonly asKillStorage: {
      readonly keys_: Vec<Bytes>;
    } & Struct;
    readonly isKillPrefix: boolean;
    readonly asKillPrefix: {
      readonly prefix: Bytes;
      readonly subkeys: u32;
    } & Struct;
    readonly isRemarkWithEvent: boolean;
    readonly asRemarkWithEvent: {
      readonly remark: Bytes;
    } & Struct;
    readonly isAuthorizeUpgrade: boolean;
    readonly asAuthorizeUpgrade: {
      readonly codeHash: H256;
    } & Struct;
    readonly isAuthorizeUpgradeWithoutChecks: boolean;
    readonly asAuthorizeUpgradeWithoutChecks: {
      readonly codeHash: H256;
    } & Struct;
    readonly isApplyAuthorizedUpgrade: boolean;
    readonly asApplyAuthorizedUpgrade: {
      readonly code: Bytes;
    } & Struct;
    readonly type: 'Remark' | 'SetHeapPages' | 'SetCode' | 'SetCodeWithoutChecks' | 'SetStorage' | 'KillStorage' | 'KillPrefix' | 'RemarkWithEvent' | 'AuthorizeUpgrade' | 'AuthorizeUpgradeWithoutChecks' | 'ApplyAuthorizedUpgrade';
  }

  /** @name FrameSystemLimitsBlockWeights (99) */
  interface FrameSystemLimitsBlockWeights extends Struct {
    readonly baseBlock: SpWeightsWeightV2Weight;
    readonly maxBlock: SpWeightsWeightV2Weight;
    readonly perClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
  }

  /** @name FrameSupportDispatchPerDispatchClassWeightsPerClass (100) */
  interface FrameSupportDispatchPerDispatchClassWeightsPerClass extends Struct {
    readonly normal: FrameSystemLimitsWeightsPerClass;
    readonly operational: FrameSystemLimitsWeightsPerClass;
    readonly mandatory: FrameSystemLimitsWeightsPerClass;
  }

  /** @name FrameSystemLimitsWeightsPerClass (101) */
  interface FrameSystemLimitsWeightsPerClass extends Struct {
    readonly baseExtrinsic: SpWeightsWeightV2Weight;
    readonly maxExtrinsic: Option<SpWeightsWeightV2Weight>;
    readonly maxTotal: Option<SpWeightsWeightV2Weight>;
    readonly reserved: Option<SpWeightsWeightV2Weight>;
  }

  /** @name FrameSystemLimitsBlockLength (103) */
  interface FrameSystemLimitsBlockLength extends Struct {
    readonly max: FrameSupportDispatchPerDispatchClassU32;
  }

  /** @name FrameSupportDispatchPerDispatchClassU32 (104) */
  interface FrameSupportDispatchPerDispatchClassU32 extends Struct {
    readonly normal: u32;
    readonly operational: u32;
    readonly mandatory: u32;
  }

  /** @name SpWeightsRuntimeDbWeight (105) */
  interface SpWeightsRuntimeDbWeight extends Struct {
    readonly read: u64;
    readonly write: u64;
  }

  /** @name SpVersionRuntimeVersion (106) */
  interface SpVersionRuntimeVersion extends Struct {
    readonly specName: Text;
    readonly implName: Text;
    readonly authoringVersion: u32;
    readonly specVersion: u32;
    readonly implVersion: u32;
    readonly apis: Vec<ITuple<[U8aFixed, u32]>>;
    readonly transactionVersion: u32;
    readonly stateVersion: u8;
  }

  /** @name FrameSystemError (111) */
  interface FrameSystemError extends Enum {
    readonly isInvalidSpecName: boolean;
    readonly isSpecVersionNeedsToIncrease: boolean;
    readonly isFailedToExtractRuntimeVersion: boolean;
    readonly isNonDefaultComposite: boolean;
    readonly isNonZeroRefCount: boolean;
    readonly isCallFiltered: boolean;
    readonly isNothingAuthorized: boolean;
    readonly isUnauthorized: boolean;
    readonly type: 'InvalidSpecName' | 'SpecVersionNeedsToIncrease' | 'FailedToExtractRuntimeVersion' | 'NonDefaultComposite' | 'NonZeroRefCount' | 'CallFiltered' | 'NothingAuthorized' | 'Unauthorized';
  }

  /** @name PalletTimestampCall (112) */
  interface PalletTimestampCall extends Enum {
    readonly isSet: boolean;
    readonly asSet: {
      readonly now: Compact<u64>;
    } & Struct;
    readonly type: 'Set';
  }

  /** @name PalletTicksCall (114) */
  type PalletTicksCall = Null;

  /** @name PalletTicksError (115) */
  type PalletTicksError = Null;

  /** @name UlxPrimitivesBlockSealAppPublic (118) */
  interface UlxPrimitivesBlockSealAppPublic extends SpCoreEd25519Public {}

  /** @name PalletMiningSlotCall (124) */
  interface PalletMiningSlotCall extends Enum {
    readonly isBid: boolean;
    readonly asBid: {
      readonly bondId: Option<u64>;
      readonly rewardDestination: UlxPrimitivesBlockSealRewardDestination;
    } & Struct;
    readonly type: 'Bid';
  }

  /** @name PalletMiningSlotError (125) */
  interface PalletMiningSlotError extends Enum {
    readonly isSlotNotTakingBids: boolean;
    readonly isTooManyBlockRegistrants: boolean;
    readonly isUnableToRotateAuthority: boolean;
    readonly isInsufficientOwnershipTokens: boolean;
    readonly isInsufficientBalanceForBid: boolean;
    readonly isBidTooLow: boolean;
    readonly isBadInternalState: boolean;
    readonly isRpcHostsAreRequired: boolean;
    readonly isBidBondDurationTooShort: boolean;
    readonly isCannotRegisteredOverlappingSessions: boolean;
    readonly isBadState: boolean;
    readonly isBondNotFound: boolean;
    readonly isNoMoreBondIds: boolean;
    readonly isBondFundClosed: boolean;
    readonly isMinimumBondAmountNotMet: boolean;
    readonly isLeaseUntilBlockTooSoon: boolean;
    readonly isLeaseUntilPastFundExpiration: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientBondFunds: boolean;
    readonly isExpirationTooSoon: boolean;
    readonly isNoPermissions: boolean;
    readonly isNoBondFundFound: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isBondFundMaximumBondsExceeded: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isBondFundNotFound: boolean;
    readonly isBondAlreadyClosed: boolean;
    readonly isBondAlreadyLocked: boolean;
    readonly isBondLockedCannotModify: boolean;
    readonly isFeeExceedsBondAmount: boolean;
    readonly isAccountWouldBeBelowMinimum: boolean;
    readonly type: 'SlotNotTakingBids' | 'TooManyBlockRegistrants' | 'UnableToRotateAuthority' | 'InsufficientOwnershipTokens' | 'InsufficientBalanceForBid' | 'BidTooLow' | 'BadInternalState' | 'RpcHostsAreRequired' | 'BidBondDurationTooShort' | 'CannotRegisteredOverlappingSessions' | 'BadState' | 'BondNotFound' | 'NoMoreBondIds' | 'BondFundClosed' | 'MinimumBondAmountNotMet' | 'LeaseUntilBlockTooSoon' | 'LeaseUntilPastFundExpiration' | 'ExpirationAtBlockOverflow' | 'InsufficientFunds' | 'InsufficientBondFunds' | 'ExpirationTooSoon' | 'NoPermissions' | 'NoBondFundFound' | 'HoldUnexpectedlyModified' | 'BondFundMaximumBondsExceeded' | 'UnrecoverableHold' | 'BondFundNotFound' | 'BondAlreadyClosed' | 'BondAlreadyLocked' | 'BondLockedCannotModify' | 'FeeExceedsBondAmount' | 'AccountWouldBeBelowMinimum';
  }

  /** @name UlxPrimitivesBondBondFund (126) */
  interface UlxPrimitivesBondBondFund extends Struct {
    readonly leaseAnnualPercentRate: Compact<u32>;
    readonly leaseBaseFee: Compact<u128>;
    readonly offerAccountId: AccountId32;
    readonly amountReserved: Compact<u128>;
    readonly offerExpirationBlock: u32;
    readonly amountBonded: Compact<u128>;
    readonly isEnded: bool;
  }

  /** @name UlxPrimitivesBond (129) */
  interface UlxPrimitivesBond extends Struct {
    readonly bondFundId: Option<u32>;
    readonly bondedAccountId: AccountId32;
    readonly annualPercentRate: Compact<u32>;
    readonly baseFee: Compact<u128>;
    readonly fee: Compact<u128>;
    readonly amount: Compact<u128>;
    readonly startBlock: Compact<u32>;
    readonly completionBlock: Compact<u32>;
    readonly isLocked: bool;
  }

  /** @name PalletBondCall (132) */
  interface PalletBondCall extends Enum {
    readonly isOfferFund: boolean;
    readonly asOfferFund: {
      readonly leaseAnnualPercentRate: Compact<u32>;
      readonly leaseBaseFee: Compact<u128>;
      readonly amountOffered: Compact<u128>;
      readonly expirationBlock: u32;
    } & Struct;
    readonly isEndFund: boolean;
    readonly asEndFund: {
      readonly bondFundId: u32;
    } & Struct;
    readonly isExtendFund: boolean;
    readonly asExtendFund: {
      readonly bondFundId: u32;
      readonly totalAmountOffered: u128;
      readonly expirationBlock: u32;
    } & Struct;
    readonly isBondSelf: boolean;
    readonly asBondSelf: {
      readonly amount: u128;
      readonly bondUntilBlock: u32;
    } & Struct;
    readonly isLease: boolean;
    readonly asLease: {
      readonly bondFundId: u32;
      readonly amount: u128;
      readonly leaseUntilBlock: u32;
    } & Struct;
    readonly isReturnBond: boolean;
    readonly asReturnBond: {
      readonly bondId: u64;
    } & Struct;
    readonly isExtendBond: boolean;
    readonly asExtendBond: {
      readonly bondId: u64;
      readonly totalAmount: u128;
      readonly bondUntilBlock: u32;
    } & Struct;
    readonly type: 'OfferFund' | 'EndFund' | 'ExtendFund' | 'BondSelf' | 'Lease' | 'ReturnBond' | 'ExtendBond';
  }

  /** @name PalletBondError (133) */
  interface PalletBondError extends Enum {
    readonly isBadState: boolean;
    readonly isBondNotFound: boolean;
    readonly isNoMoreBondFundIds: boolean;
    readonly isNoMoreBondIds: boolean;
    readonly isMinimumBondAmountNotMet: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientBondFunds: boolean;
    readonly isTransactionWouldTakeAccountBelowMinimumBalance: boolean;
    readonly isBondFundClosed: boolean;
    readonly isBondFundReductionExceedsAllocatedFunds: boolean;
    readonly isExpirationTooSoon: boolean;
    readonly isLeaseUntilBlockTooSoon: boolean;
    readonly isLeaseUntilPastFundExpiration: boolean;
    readonly isNoPermissions: boolean;
    readonly isNoBondFundFound: boolean;
    readonly isFundExtensionMustBeLater: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isBondFundMaximumBondsExceeded: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isBondFundNotFound: boolean;
    readonly isBondAlreadyLocked: boolean;
    readonly isBondLockedCannotModify: boolean;
    readonly isFeeExceedsBondAmount: boolean;
    readonly type: 'BadState' | 'BondNotFound' | 'NoMoreBondFundIds' | 'NoMoreBondIds' | 'MinimumBondAmountNotMet' | 'ExpirationAtBlockOverflow' | 'InsufficientFunds' | 'InsufficientBondFunds' | 'TransactionWouldTakeAccountBelowMinimumBalance' | 'BondFundClosed' | 'BondFundReductionExceedsAllocatedFunds' | 'ExpirationTooSoon' | 'LeaseUntilBlockTooSoon' | 'LeaseUntilPastFundExpiration' | 'NoPermissions' | 'NoBondFundFound' | 'FundExtensionMustBeLater' | 'HoldUnexpectedlyModified' | 'BondFundMaximumBondsExceeded' | 'UnrecoverableHold' | 'BondFundNotFound' | 'BondAlreadyLocked' | 'BondLockedCannotModify' | 'FeeExceedsBondAmount';
  }

  /** @name PalletNotariesCall (146) */
  interface PalletNotariesCall extends Enum {
    readonly isPropose: boolean;
    readonly asPropose: {
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly isActivate: boolean;
    readonly asActivate: {
      readonly operatorAccount: AccountId32;
    } & Struct;
    readonly isUpdate: boolean;
    readonly asUpdate: {
      readonly notaryId: Compact<u32>;
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly type: 'Propose' | 'Activate' | 'Update';
  }

  /** @name PalletNotariesError (147) */
  interface PalletNotariesError extends Enum {
    readonly isProposalNotFound: boolean;
    readonly isMaxNotariesExceeded: boolean;
    readonly isMaxProposalsPerBlockExceeded: boolean;
    readonly isNotAnActiveNotary: boolean;
    readonly isInvalidNotaryOperator: boolean;
    readonly isNoMoreNotaryIds: boolean;
    readonly type: 'ProposalNotFound' | 'MaxNotariesExceeded' | 'MaxProposalsPerBlockExceeded' | 'NotAnActiveNotary' | 'InvalidNotaryOperator' | 'NoMoreNotaryIds';
  }

  /** @name UlxPrimitivesBalanceChangeAccountOrigin (149) */
  interface UlxPrimitivesBalanceChangeAccountOrigin extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly accountUid: Compact<u32>;
  }

  /** @name UlxPrimitivesNotaryNotaryNotebookKeyDetails (152) */
  interface UlxPrimitivesNotaryNotaryNotebookKeyDetails extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly blockVotesRoot: H256;
    readonly secretHash: H256;
    readonly parentSecret: Option<H256>;
  }

  /** @name UlxPrimitivesDigestsNotebookDigest (155) */
  interface UlxPrimitivesDigestsNotebookDigest extends Struct {
    readonly notebooks: Vec<UlxPrimitivesDigestsNotebookDigestRecord>;
  }

  /** @name UlxPrimitivesDigestsNotebookDigestRecord (157) */
  interface UlxPrimitivesDigestsNotebookDigestRecord extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly auditFirstFailure: Option<UlxNotaryAuditErrorVerifyError>;
  }

  /** @name PalletNotebookCall (160) */
  interface PalletNotebookCall extends Enum {
    readonly isSubmit: boolean;
    readonly asSubmit: {
      readonly notebooks: Vec<UlxPrimitivesNotebookSignedNotebookHeader>;
    } & Struct;
    readonly type: 'Submit';
  }

  /** @name UlxPrimitivesNotebookSignedNotebookHeader (162) */
  interface UlxPrimitivesNotebookSignedNotebookHeader extends Struct {
    readonly header: UlxPrimitivesNotebookNotebookHeader;
    readonly signature: SpCoreEd25519Signature;
  }

  /** @name UlxPrimitivesNotebookNotebookHeader (163) */
  interface UlxPrimitivesNotebookNotebookHeader extends Struct {
    readonly version: Compact<u16>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly finalizedBlockNumber: Compact<u32>;
    readonly tax: Compact<u128>;
    readonly notaryId: Compact<u32>;
    readonly chainTransfers: Vec<UlxPrimitivesNotebookChainTransfer>;
    readonly changedAccountsRoot: H256;
    readonly changedAccountOrigins: Vec<UlxPrimitivesBalanceChangeAccountOrigin>;
    readonly blockVotesRoot: H256;
    readonly blockVotesCount: Compact<u32>;
    readonly blocksWithVotes: Vec<H256>;
    readonly blockVotingPower: Compact<u128>;
    readonly secretHash: H256;
    readonly parentSecret: Option<H256>;
    readonly dataDomains: Vec<ITuple<[H256, AccountId32]>>;
  }

  /** @name UlxPrimitivesNotebookChainTransfer (165) */
  interface UlxPrimitivesNotebookChainTransfer extends Enum {
    readonly isToMainchain: boolean;
    readonly asToMainchain: {
      readonly accountId: AccountId32;
      readonly amount: Compact<u128>;
    } & Struct;
    readonly isToLocalchain: boolean;
    readonly asToLocalchain: {
      readonly accountId: AccountId32;
      readonly accountNonce: Compact<u32>;
    } & Struct;
    readonly type: 'ToMainchain' | 'ToLocalchain';
  }

  /** @name SpCoreEd25519Signature (173) */
  interface SpCoreEd25519Signature extends U8aFixed {}

  /** @name PalletNotebookError (175) */
  interface PalletNotebookError extends Enum {
    readonly isDuplicateNotebookNumber: boolean;
    readonly isMissingNotebookNumber: boolean;
    readonly isNotebookTickAlreadyUsed: boolean;
    readonly isInvalidNotebookSignature: boolean;
    readonly isInvalidSecretProvided: boolean;
    readonly isCouldNotDecodeNotebook: boolean;
    readonly isDuplicateNotebookDigest: boolean;
    readonly isMissingNotebookDigest: boolean;
    readonly isInvalidNotebookDigest: boolean;
    readonly isMultipleNotebookInherentsProvided: boolean;
    readonly isInternalError: boolean;
    readonly type: 'DuplicateNotebookNumber' | 'MissingNotebookNumber' | 'NotebookTickAlreadyUsed' | 'InvalidNotebookSignature' | 'InvalidSecretProvided' | 'CouldNotDecodeNotebook' | 'DuplicateNotebookDigest' | 'MissingNotebookDigest' | 'InvalidNotebookDigest' | 'MultipleNotebookInherentsProvided' | 'InternalError';
  }

  /** @name PalletChainTransferQueuedTransferOut (177) */
  interface PalletChainTransferQueuedTransferOut extends Struct {
    readonly amount: u128;
    readonly expirationBlock: u32;
    readonly notaryId: u32;
  }

  /** @name PalletChainTransferCall (180) */
  interface PalletChainTransferCall extends Enum {
    readonly isSendToLocalchain: boolean;
    readonly asSendToLocalchain: {
      readonly amount: Compact<u128>;
      readonly notaryId: u32;
    } & Struct;
    readonly type: 'SendToLocalchain';
  }

  /** @name FrameSupportPalletId (181) */
  interface FrameSupportPalletId extends U8aFixed {}

  /** @name PalletChainTransferError (182) */
  interface PalletChainTransferError extends Enum {
    readonly isMaxBlockTransfersExceeded: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInvalidAccountNonce: boolean;
    readonly isInsufficientNotarizedFunds: boolean;
    readonly isInvalidOrDuplicatedLocalchainTransfer: boolean;
    readonly isNotebookIncludesExpiredLocalchainTransfer: boolean;
    readonly isInvalidNotaryUsedForTransfer: boolean;
    readonly type: 'MaxBlockTransfersExceeded' | 'InsufficientFunds' | 'InvalidAccountNonce' | 'InsufficientNotarizedFunds' | 'InvalidOrDuplicatedLocalchainTransfer' | 'NotebookIncludesExpiredLocalchainTransfer' | 'InvalidNotaryUsedForTransfer';
  }

  /** @name UlxPrimitivesNotaryNotaryNotebookVoteDigestDetails (187) */
  interface UlxPrimitivesNotaryNotaryNotebookVoteDigestDetails extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly blockVotesCount: Compact<u32>;
    readonly blockVotingPower: Compact<u128>;
  }

  /** @name UlxPrimitivesDigestsBlockVoteDigest (189) */
  interface UlxPrimitivesDigestsBlockVoteDigest extends Struct {
    readonly votingPower: Compact<u128>;
    readonly votesCount: Compact<u32>;
  }

  /** @name PalletBlockSealSpecCall (193) */
  interface PalletBlockSealSpecCall extends Enum {
    readonly isConfigure: boolean;
    readonly asConfigure: {
      readonly voteMinimum: Option<u128>;
      readonly computeDifficulty: Option<u128>;
    } & Struct;
    readonly type: 'Configure';
  }

  /** @name PalletBlockSealSpecError (195) */
  interface PalletBlockSealSpecError extends Enum {
    readonly isMaxNotebooksAtTickExceeded: boolean;
    readonly type: 'MaxNotebooksAtTickExceeded';
  }

  /** @name PalletDataDomainCall (198) */
  interface PalletDataDomainCall extends Enum {
    readonly isSetZoneRecord: boolean;
    readonly asSetZoneRecord: {
      readonly domainHash: H256;
      readonly zoneRecord: UlxPrimitivesDataDomainZoneRecord;
    } & Struct;
    readonly type: 'SetZoneRecord';
  }

  /** @name PalletDataDomainError (199) */
  interface PalletDataDomainError extends Enum {
    readonly isDomainNotRegistered: boolean;
    readonly isNotDomainOwner: boolean;
    readonly type: 'DomainNotRegistered' | 'NotDomainOwner';
  }

  /** @name UlxNodeRuntimeOpaqueSessionKeys (203) */
  interface UlxNodeRuntimeOpaqueSessionKeys extends Struct {
    readonly grandpa: SpConsensusGrandpaAppPublic;
    readonly blockSealAuthority: UlxPrimitivesBlockSealAppPublic;
  }

  /** @name SpCoreCryptoKeyTypeId (205) */
  interface SpCoreCryptoKeyTypeId extends U8aFixed {}

  /** @name PalletSessionCall (206) */
  interface PalletSessionCall extends Enum {
    readonly isSetKeys: boolean;
    readonly asSetKeys: {
      readonly keys_: UlxNodeRuntimeOpaqueSessionKeys;
      readonly proof: Bytes;
    } & Struct;
    readonly isPurgeKeys: boolean;
    readonly type: 'SetKeys' | 'PurgeKeys';
  }

  /** @name PalletSessionError (207) */
  interface PalletSessionError extends Enum {
    readonly isInvalidProof: boolean;
    readonly isNoAssociatedValidatorId: boolean;
    readonly isDuplicatedKey: boolean;
    readonly isNoKeys: boolean;
    readonly isNoAccount: boolean;
    readonly type: 'InvalidProof' | 'NoAssociatedValidatorId' | 'DuplicatedKey' | 'NoKeys' | 'NoAccount';
  }

  /** @name UlxPrimitivesProvidersBlockSealerInfo (208) */
  interface UlxPrimitivesProvidersBlockSealerInfo extends Struct {
    readonly minerRewardsAccount: AccountId32;
    readonly blockVoteRewardsAccount: AccountId32;
  }

  /** @name UlxPrimitivesInherentsBlockSealInherent (209) */
  interface UlxPrimitivesInherentsBlockSealInherent extends Enum {
    readonly isVote: boolean;
    readonly asVote: {
      readonly sealStrength: U256;
      readonly notaryId: Compact<u32>;
      readonly sourceNotebookNumber: Compact<u32>;
      readonly sourceNotebookProof: UlxPrimitivesBalanceChangeMerkleProof;
      readonly blockVote: UlxPrimitivesBlockVoteBlockVoteT;
      readonly minerSignature: UlxPrimitivesBlockSealAppSignature;
    } & Struct;
    readonly isCompute: boolean;
    readonly type: 'Vote' | 'Compute';
  }

  /** @name UlxPrimitivesBalanceChangeMerkleProof (210) */
  interface UlxPrimitivesBalanceChangeMerkleProof extends Struct {
    readonly proof: Vec<H256>;
    readonly numberOfLeaves: Compact<u32>;
    readonly leafIndex: Compact<u32>;
  }

  /** @name UlxPrimitivesBlockVoteBlockVoteT (212) */
  interface UlxPrimitivesBlockVoteBlockVoteT extends Struct {
    readonly accountId: AccountId32;
    readonly blockHash: H256;
    readonly index: Compact<u32>;
    readonly power: Compact<u128>;
    readonly dataDomainHash: H256;
    readonly dataDomainAccount: AccountId32;
    readonly signature: SpRuntimeMultiSignature;
    readonly blockRewardsAccountId: AccountId32;
  }

  /** @name SpRuntimeMultiSignature (213) */
  interface SpRuntimeMultiSignature extends Enum {
    readonly isEd25519: boolean;
    readonly asEd25519: SpCoreEd25519Signature;
    readonly isSr25519: boolean;
    readonly asSr25519: SpCoreSr25519Signature;
    readonly isEcdsa: boolean;
    readonly asEcdsa: SpCoreEcdsaSignature;
    readonly type: 'Ed25519' | 'Sr25519' | 'Ecdsa';
  }

  /** @name SpCoreSr25519Signature (214) */
  interface SpCoreSr25519Signature extends U8aFixed {}

  /** @name SpCoreEcdsaSignature (215) */
  interface SpCoreEcdsaSignature extends U8aFixed {}

  /** @name UlxPrimitivesBlockSealAppSignature (217) */
  interface UlxPrimitivesBlockSealAppSignature extends SpCoreEd25519Signature {}

  /** @name UlxPrimitivesDigestsParentVotingKeyDigest (218) */
  interface UlxPrimitivesDigestsParentVotingKeyDigest extends Struct {
    readonly parentVotingKey: Option<H256>;
  }

  /** @name PalletBlockSealCall (219) */
  interface PalletBlockSealCall extends Enum {
    readonly isApply: boolean;
    readonly asApply: {
      readonly seal: UlxPrimitivesInherentsBlockSealInherent;
    } & Struct;
    readonly type: 'Apply';
  }

  /** @name PalletBlockSealError (220) */
  interface PalletBlockSealError extends Enum {
    readonly isInvalidVoteSealStrength: boolean;
    readonly isInvalidSubmitter: boolean;
    readonly isUnableToDecodeVoteAccount: boolean;
    readonly isUnregisteredBlockAuthor: boolean;
    readonly isInvalidBlockVoteProof: boolean;
    readonly isNoGrandparentVoteMinimum: boolean;
    readonly isDuplicateBlockSealProvided: boolean;
    readonly isInsufficientVotingPower: boolean;
    readonly isParentVotingKeyNotFound: boolean;
    readonly isInvalidVoteGrandparentHash: boolean;
    readonly isIneligibleNotebookUsed: boolean;
    readonly isNoEligibleVotingRoot: boolean;
    readonly isUnregisteredDataDomain: boolean;
    readonly isInvalidDataDomainAccount: boolean;
    readonly isInvalidAuthoritySignature: boolean;
    readonly isCouldNotDecodeVote: boolean;
    readonly isMaxNotebooksAtTickExceeded: boolean;
    readonly isNoClosestMinerFoundForVote: boolean;
    readonly isBlockVoteInvalidSignature: boolean;
    readonly type: 'InvalidVoteSealStrength' | 'InvalidSubmitter' | 'UnableToDecodeVoteAccount' | 'UnregisteredBlockAuthor' | 'InvalidBlockVoteProof' | 'NoGrandparentVoteMinimum' | 'DuplicateBlockSealProvided' | 'InsufficientVotingPower' | 'ParentVotingKeyNotFound' | 'InvalidVoteGrandparentHash' | 'IneligibleNotebookUsed' | 'NoEligibleVotingRoot' | 'UnregisteredDataDomain' | 'InvalidDataDomainAccount' | 'InvalidAuthoritySignature' | 'CouldNotDecodeVote' | 'MaxNotebooksAtTickExceeded' | 'NoClosestMinerFoundForVote' | 'BlockVoteInvalidSignature';
  }

  /** @name PalletBlockRewardsCall (222) */
  type PalletBlockRewardsCall = Null;

  /** @name PalletBlockRewardsError (223) */
  type PalletBlockRewardsError = Null;

  /** @name PalletGrandpaStoredState (224) */
  interface PalletGrandpaStoredState extends Enum {
    readonly isLive: boolean;
    readonly isPendingPause: boolean;
    readonly asPendingPause: {
      readonly scheduledAt: u32;
      readonly delay: u32;
    } & Struct;
    readonly isPaused: boolean;
    readonly isPendingResume: boolean;
    readonly asPendingResume: {
      readonly scheduledAt: u32;
      readonly delay: u32;
    } & Struct;
    readonly type: 'Live' | 'PendingPause' | 'Paused' | 'PendingResume';
  }

  /** @name PalletGrandpaStoredPendingChange (225) */
  interface PalletGrandpaStoredPendingChange extends Struct {
    readonly scheduledAt: u32;
    readonly delay: u32;
    readonly nextAuthorities: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    readonly forced: Option<u32>;
  }

  /** @name PalletGrandpaCall (227) */
  interface PalletGrandpaCall extends Enum {
    readonly isReportEquivocation: boolean;
    readonly asReportEquivocation: {
      readonly equivocationProof: SpConsensusGrandpaEquivocationProof;
      readonly keyOwnerProof: SpSessionMembershipProof;
    } & Struct;
    readonly isReportEquivocationUnsigned: boolean;
    readonly asReportEquivocationUnsigned: {
      readonly equivocationProof: SpConsensusGrandpaEquivocationProof;
      readonly keyOwnerProof: SpSessionMembershipProof;
    } & Struct;
    readonly isNoteStalled: boolean;
    readonly asNoteStalled: {
      readonly delay: u32;
      readonly bestFinalizedBlockNumber: u32;
    } & Struct;
    readonly type: 'ReportEquivocation' | 'ReportEquivocationUnsigned' | 'NoteStalled';
  }

  /** @name SpConsensusGrandpaEquivocationProof (228) */
  interface SpConsensusGrandpaEquivocationProof extends Struct {
    readonly setId: u64;
    readonly equivocation: SpConsensusGrandpaEquivocation;
  }

  /** @name SpConsensusGrandpaEquivocation (229) */
  interface SpConsensusGrandpaEquivocation extends Enum {
    readonly isPrevote: boolean;
    readonly asPrevote: FinalityGrandpaEquivocationPrevote;
    readonly isPrecommit: boolean;
    readonly asPrecommit: FinalityGrandpaEquivocationPrecommit;
    readonly type: 'Prevote' | 'Precommit';
  }

  /** @name FinalityGrandpaEquivocationPrevote (230) */
  interface FinalityGrandpaEquivocationPrevote extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrevote (231) */
  interface FinalityGrandpaPrevote extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpConsensusGrandpaAppSignature (232) */
  interface SpConsensusGrandpaAppSignature extends SpCoreEd25519Signature {}

  /** @name FinalityGrandpaEquivocationPrecommit (234) */
  interface FinalityGrandpaEquivocationPrecommit extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrecommit (235) */
  interface FinalityGrandpaPrecommit extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpSessionMembershipProof (237) */
  interface SpSessionMembershipProof extends Struct {
    readonly session: u32;
    readonly trieNodes: Vec<Bytes>;
    readonly validatorCount: u32;
  }

  /** @name PalletGrandpaError (238) */
  interface PalletGrandpaError extends Enum {
    readonly isPauseFailed: boolean;
    readonly isResumeFailed: boolean;
    readonly isChangePending: boolean;
    readonly isTooSoon: boolean;
    readonly isInvalidKeyOwnershipProof: boolean;
    readonly isInvalidEquivocationProof: boolean;
    readonly isDuplicateOffenceReport: boolean;
    readonly type: 'PauseFailed' | 'ResumeFailed' | 'ChangePending' | 'TooSoon' | 'InvalidKeyOwnershipProof' | 'InvalidEquivocationProof' | 'DuplicateOffenceReport';
  }

  /** @name SpStakingOffenceOffenceDetails (239) */
  interface SpStakingOffenceOffenceDetails extends Struct {
    readonly offender: ITuple<[AccountId32, PalletMiningSlotMinerHistory]>;
    readonly reporters: Vec<AccountId32>;
  }

  /** @name PalletMiningSlotMinerHistory (241) */
  interface PalletMiningSlotMinerHistory extends Struct {
    readonly authorityIndex: u32;
  }

  /** @name PalletBalancesBalanceLock (244) */
  interface PalletBalancesBalanceLock extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
    readonly reasons: PalletBalancesReasons;
  }

  /** @name PalletBalancesReasons (245) */
  interface PalletBalancesReasons extends Enum {
    readonly isFee: boolean;
    readonly isMisc: boolean;
    readonly isAll: boolean;
    readonly type: 'Fee' | 'Misc' | 'All';
  }

  /** @name PalletBalancesReserveData (248) */
  interface PalletBalancesReserveData extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
  }

  /** @name PalletBalancesIdAmountRuntimeHoldReason (251) */
  interface PalletBalancesIdAmountRuntimeHoldReason extends Struct {
    readonly id: UlxNodeRuntimeRuntimeHoldReason;
    readonly amount: u128;
  }

  /** @name UlxNodeRuntimeRuntimeHoldReason (252) */
  interface UlxNodeRuntimeRuntimeHoldReason extends Enum {
    readonly isMiningSlot: boolean;
    readonly asMiningSlot: PalletMiningSlotHoldReason;
    readonly isBond: boolean;
    readonly asBond: PalletBondHoldReason;
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsHoldReason;
    readonly isMint: boolean;
    readonly type: 'MiningSlot' | 'Bond' | 'BlockRewards' | 'Mint';
  }

  /** @name PalletMiningSlotHoldReason (253) */
  interface PalletMiningSlotHoldReason extends Enum {
    readonly isRegisterAsMiner: boolean;
    readonly type: 'RegisterAsMiner';
  }

  /** @name PalletBondHoldReason (254) */
  interface PalletBondHoldReason extends Enum {
    readonly isEnterBondFund: boolean;
    readonly type: 'EnterBondFund';
  }

  /** @name PalletBlockRewardsHoldReason (255) */
  interface PalletBlockRewardsHoldReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletMintHoldReason (256) */
  type PalletMintHoldReason = Null;

  /** @name PalletBalancesIdAmountRuntimeFreezeReason (259) */
  interface PalletBalancesIdAmountRuntimeFreezeReason extends Struct {
    readonly id: UlxNodeRuntimeRuntimeFreezeReason;
    readonly amount: u128;
  }

  /** @name UlxNodeRuntimeRuntimeFreezeReason (260) */
  interface UlxNodeRuntimeRuntimeFreezeReason extends Enum {
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsFreezeReason;
    readonly type: 'BlockRewards';
  }

  /** @name PalletBlockRewardsFreezeReason (261) */
  interface PalletBlockRewardsFreezeReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletBalancesCall (263) */
  interface PalletBalancesCall extends Enum {
    readonly isTransferAllowDeath: boolean;
    readonly asTransferAllowDeath: {
      readonly dest: MultiAddress;
      readonly value: Compact<u128>;
    } & Struct;
    readonly isForceTransfer: boolean;
    readonly asForceTransfer: {
      readonly source: MultiAddress;
      readonly dest: MultiAddress;
      readonly value: Compact<u128>;
    } & Struct;
    readonly isTransferKeepAlive: boolean;
    readonly asTransferKeepAlive: {
      readonly dest: MultiAddress;
      readonly value: Compact<u128>;
    } & Struct;
    readonly isTransferAll: boolean;
    readonly asTransferAll: {
      readonly dest: MultiAddress;
      readonly keepAlive: bool;
    } & Struct;
    readonly isForceUnreserve: boolean;
    readonly asForceUnreserve: {
      readonly who: MultiAddress;
      readonly amount: u128;
    } & Struct;
    readonly isUpgradeAccounts: boolean;
    readonly asUpgradeAccounts: {
      readonly who: Vec<AccountId32>;
    } & Struct;
    readonly isForceSetBalance: boolean;
    readonly asForceSetBalance: {
      readonly who: MultiAddress;
      readonly newFree: Compact<u128>;
    } & Struct;
    readonly type: 'TransferAllowDeath' | 'ForceTransfer' | 'TransferKeepAlive' | 'TransferAll' | 'ForceUnreserve' | 'UpgradeAccounts' | 'ForceSetBalance';
  }

  /** @name PalletBalancesError (267) */
  interface PalletBalancesError extends Enum {
    readonly isVestingBalance: boolean;
    readonly isLiquidityRestrictions: boolean;
    readonly isInsufficientBalance: boolean;
    readonly isExistentialDeposit: boolean;
    readonly isExpendability: boolean;
    readonly isExistingVestingSchedule: boolean;
    readonly isDeadAccount: boolean;
    readonly isTooManyReserves: boolean;
    readonly isTooManyHolds: boolean;
    readonly isTooManyFreezes: boolean;
    readonly type: 'VestingBalance' | 'LiquidityRestrictions' | 'InsufficientBalance' | 'ExistentialDeposit' | 'Expendability' | 'ExistingVestingSchedule' | 'DeadAccount' | 'TooManyReserves' | 'TooManyHolds' | 'TooManyFreezes';
  }

  /** @name PalletMintCall (268) */
  type PalletMintCall = Null;

  /** @name PalletMintError (269) */
  type PalletMintError = Null;

  /** @name PalletTxPauseCall (273) */
  interface PalletTxPauseCall extends Enum {
    readonly isPause: boolean;
    readonly asPause: {
      readonly fullName: ITuple<[Bytes, Bytes]>;
    } & Struct;
    readonly isUnpause: boolean;
    readonly asUnpause: {
      readonly ident: ITuple<[Bytes, Bytes]>;
    } & Struct;
    readonly type: 'Pause' | 'Unpause';
  }

  /** @name PalletTxPauseError (274) */
  interface PalletTxPauseError extends Enum {
    readonly isIsPaused: boolean;
    readonly isIsUnpaused: boolean;
    readonly isUnpausable: boolean;
    readonly isNotFound: boolean;
    readonly type: 'IsPaused' | 'IsUnpaused' | 'Unpausable' | 'NotFound';
  }

  /** @name PalletTransactionPaymentReleases (276) */
  interface PalletTransactionPaymentReleases extends Enum {
    readonly isV1Ancient: boolean;
    readonly isV2: boolean;
    readonly type: 'V1Ancient' | 'V2';
  }

  /** @name PalletSudoCall (277) */
  interface PalletSudoCall extends Enum {
    readonly isSudo: boolean;
    readonly asSudo: {
      readonly call: Call;
    } & Struct;
    readonly isSudoUncheckedWeight: boolean;
    readonly asSudoUncheckedWeight: {
      readonly call: Call;
      readonly weight: SpWeightsWeightV2Weight;
    } & Struct;
    readonly isSetKey: boolean;
    readonly asSetKey: {
      readonly new_: MultiAddress;
    } & Struct;
    readonly isSudoAs: boolean;
    readonly asSudoAs: {
      readonly who: MultiAddress;
      readonly call: Call;
    } & Struct;
    readonly isRemoveKey: boolean;
    readonly type: 'Sudo' | 'SudoUncheckedWeight' | 'SetKey' | 'SudoAs' | 'RemoveKey';
  }

  /** @name PalletSudoError (279) */
  interface PalletSudoError extends Enum {
    readonly isRequireSudo: boolean;
    readonly type: 'RequireSudo';
  }

  /** @name FrameSystemExtensionsCheckNonZeroSender (282) */
  type FrameSystemExtensionsCheckNonZeroSender = Null;

  /** @name FrameSystemExtensionsCheckSpecVersion (283) */
  type FrameSystemExtensionsCheckSpecVersion = Null;

  /** @name FrameSystemExtensionsCheckTxVersion (284) */
  type FrameSystemExtensionsCheckTxVersion = Null;

  /** @name FrameSystemExtensionsCheckGenesis (285) */
  type FrameSystemExtensionsCheckGenesis = Null;

  /** @name FrameSystemExtensionsCheckNonce (288) */
  interface FrameSystemExtensionsCheckNonce extends Compact<u32> {}

  /** @name FrameSystemExtensionsCheckWeight (289) */
  type FrameSystemExtensionsCheckWeight = Null;

  /** @name PalletTransactionPaymentChargeTransactionPayment (290) */
  interface PalletTransactionPaymentChargeTransactionPayment extends Compact<u128> {}

  /** @name UlxNodeRuntimeRuntime (291) */
  type UlxNodeRuntimeRuntime = Null;

} // declare module
