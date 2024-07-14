// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/types/lookup';

import type { BTreeMap, Bytes, Compact, Enum, Null, Option, Result, Struct, Text, U256, U8aFixed, Vec, bool, i128, u128, u16, u32, u64, u8 } from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, Call, H256, MultiAddress, Percent } from '@polkadot/types/interfaces/runtime';
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

  /** @name FrameSupportDispatchPerDispatchClassWeight (9) */
  interface FrameSupportDispatchPerDispatchClassWeight extends Struct {
    readonly normal: SpWeightsWeightV2Weight;
    readonly operational: SpWeightsWeightV2Weight;
    readonly mandatory: SpWeightsWeightV2Weight;
  }

  /** @name SpWeightsWeightV2Weight (10) */
  interface SpWeightsWeightV2Weight extends Struct {
    readonly refTime: Compact<u64>;
    readonly proofSize: Compact<u64>;
  }

  /** @name SpRuntimeDigest (15) */
  interface SpRuntimeDigest extends Struct {
    readonly logs: Vec<SpRuntimeDigestDigestItem>;
  }

  /** @name SpRuntimeDigestDigestItem (17) */
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

  /** @name FrameSystemEventRecord (20) */
  interface FrameSystemEventRecord extends Struct {
    readonly phase: FrameSystemPhase;
    readonly event: Event;
    readonly topics: Vec<H256>;
  }

  /** @name FrameSystemEvent (22) */
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

  /** @name FrameSupportDispatchDispatchInfo (23) */
  interface FrameSupportDispatchDispatchInfo extends Struct {
    readonly weight: SpWeightsWeightV2Weight;
    readonly class: FrameSupportDispatchDispatchClass;
    readonly paysFee: FrameSupportDispatchPays;
  }

  /** @name FrameSupportDispatchDispatchClass (24) */
  interface FrameSupportDispatchDispatchClass extends Enum {
    readonly isNormal: boolean;
    readonly isOperational: boolean;
    readonly isMandatory: boolean;
    readonly type: 'Normal' | 'Operational' | 'Mandatory';
  }

  /** @name FrameSupportDispatchPays (25) */
  interface FrameSupportDispatchPays extends Enum {
    readonly isYes: boolean;
    readonly isNo: boolean;
    readonly type: 'Yes' | 'No';
  }

  /** @name SpRuntimeDispatchError (26) */
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

  /** @name SpRuntimeModuleError (27) */
  interface SpRuntimeModuleError extends Struct {
    readonly index: u8;
    readonly error: U8aFixed;
  }

  /** @name SpRuntimeTokenError (28) */
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

  /** @name SpArithmeticArithmeticError (29) */
  interface SpArithmeticArithmeticError extends Enum {
    readonly isUnderflow: boolean;
    readonly isOverflow: boolean;
    readonly isDivisionByZero: boolean;
    readonly type: 'Underflow' | 'Overflow' | 'DivisionByZero';
  }

  /** @name SpRuntimeTransactionalError (30) */
  interface SpRuntimeTransactionalError extends Enum {
    readonly isLimitReached: boolean;
    readonly isNoLayer: boolean;
    readonly type: 'LimitReached' | 'NoLayer';
  }

  /** @name PalletMultisigEvent (31) */
  interface PalletMultisigEvent extends Enum {
    readonly isNewMultisig: boolean;
    readonly asNewMultisig: {
      readonly approving: AccountId32;
      readonly multisig: AccountId32;
      readonly callHash: U8aFixed;
    } & Struct;
    readonly isMultisigApproval: boolean;
    readonly asMultisigApproval: {
      readonly approving: AccountId32;
      readonly timepoint: PalletMultisigTimepoint;
      readonly multisig: AccountId32;
      readonly callHash: U8aFixed;
    } & Struct;
    readonly isMultisigExecuted: boolean;
    readonly asMultisigExecuted: {
      readonly approving: AccountId32;
      readonly timepoint: PalletMultisigTimepoint;
      readonly multisig: AccountId32;
      readonly callHash: U8aFixed;
      readonly result: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly isMultisigCancelled: boolean;
    readonly asMultisigCancelled: {
      readonly cancelling: AccountId32;
      readonly timepoint: PalletMultisigTimepoint;
      readonly multisig: AccountId32;
      readonly callHash: U8aFixed;
    } & Struct;
    readonly type: 'NewMultisig' | 'MultisigApproval' | 'MultisigExecuted' | 'MultisigCancelled';
  }

  /** @name PalletMultisigTimepoint (32) */
  interface PalletMultisigTimepoint extends Struct {
    readonly height: u32;
    readonly index: u32;
  }

  /** @name PalletProxyEvent (35) */
  interface PalletProxyEvent extends Enum {
    readonly isProxyExecuted: boolean;
    readonly asProxyExecuted: {
      readonly result: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly isPureCreated: boolean;
    readonly asPureCreated: {
      readonly pure: AccountId32;
      readonly who: AccountId32;
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly disambiguationIndex: u16;
    } & Struct;
    readonly isAnnounced: boolean;
    readonly asAnnounced: {
      readonly real: AccountId32;
      readonly proxy: AccountId32;
      readonly callHash: H256;
    } & Struct;
    readonly isProxyAdded: boolean;
    readonly asProxyAdded: {
      readonly delegator: AccountId32;
      readonly delegatee: AccountId32;
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isProxyRemoved: boolean;
    readonly asProxyRemoved: {
      readonly delegator: AccountId32;
      readonly delegatee: AccountId32;
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly type: 'ProxyExecuted' | 'PureCreated' | 'Announced' | 'ProxyAdded' | 'ProxyRemoved';
  }

  /** @name UlxNodeRuntimeProxyType (36) */
  interface UlxNodeRuntimeProxyType extends Enum {
    readonly isAny: boolean;
    readonly isNonTransfer: boolean;
    readonly isPriceIndex: boolean;
    readonly type: 'Any' | 'NonTransfer' | 'PriceIndex';
  }

  /** @name PalletMiningSlotEvent (38) */
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
    readonly isUnbondMinerError: boolean;
    readonly asUnbondMinerError: {
      readonly accountId: AccountId32;
      readonly bondId: Option<u64>;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'NewMiners' | 'SlotBidderAdded' | 'SlotBidderReplaced' | 'UnbondedMiner' | 'UnbondMinerError';
  }

  /** @name UlxPrimitivesBlockSealMiningRegistration (40) */
  interface UlxPrimitivesBlockSealMiningRegistration extends Struct {
    readonly accountId: AccountId32;
    readonly rewardDestination: UlxPrimitivesBlockSealRewardDestination;
    readonly bondId: Option<u64>;
    readonly bondAmount: Compact<u128>;
    readonly ownershipTokens: Compact<u128>;
    readonly rewardSharing: Option<UlxPrimitivesBlockSealRewardSharing>;
  }

  /** @name UlxPrimitivesBlockSealRewardDestination (41) */
  interface UlxPrimitivesBlockSealRewardDestination extends Enum {
    readonly isOwner: boolean;
    readonly isAccount: boolean;
    readonly asAccount: AccountId32;
    readonly type: 'Owner' | 'Account';
  }

  /** @name UlxPrimitivesBlockSealRewardSharing (45) */
  interface UlxPrimitivesBlockSealRewardSharing extends Struct {
    readonly accountId: AccountId32;
    readonly percentTake: Compact<Percent>;
  }

  /** @name PalletBitcoinUtxosEvent (49) */
  interface PalletBitcoinUtxosEvent extends Enum {
    readonly isUtxoVerified: boolean;
    readonly asUtxoVerified: {
      readonly utxoId: u64;
    } & Struct;
    readonly isUtxoRejected: boolean;
    readonly asUtxoRejected: {
      readonly utxoId: u64;
      readonly rejectedReason: UlxPrimitivesBitcoinBitcoinRejectedReason;
    } & Struct;
    readonly isUtxoSpent: boolean;
    readonly asUtxoSpent: {
      readonly utxoId: u64;
      readonly blockHeight: u64;
    } & Struct;
    readonly isUtxoUnwatched: boolean;
    readonly asUtxoUnwatched: {
      readonly utxoId: u64;
    } & Struct;
    readonly isUtxoSpentError: boolean;
    readonly asUtxoSpentError: {
      readonly utxoId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isUtxoVerifiedError: boolean;
    readonly asUtxoVerifiedError: {
      readonly utxoId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isUtxoRejectedError: boolean;
    readonly asUtxoRejectedError: {
      readonly utxoId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isUtxoExpiredError: boolean;
    readonly asUtxoExpiredError: {
      readonly utxoRef: UlxPrimitivesBitcoinUtxoRef;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'UtxoVerified' | 'UtxoRejected' | 'UtxoSpent' | 'UtxoUnwatched' | 'UtxoSpentError' | 'UtxoVerifiedError' | 'UtxoRejectedError' | 'UtxoExpiredError';
  }

  /** @name UlxPrimitivesBitcoinBitcoinRejectedReason (50) */
  interface UlxPrimitivesBitcoinBitcoinRejectedReason extends Enum {
    readonly isSatoshisMismatch: boolean;
    readonly isSpent: boolean;
    readonly isLookupExpired: boolean;
    readonly isDuplicateUtxo: boolean;
    readonly type: 'SatoshisMismatch' | 'Spent' | 'LookupExpired' | 'DuplicateUtxo';
  }

  /** @name UlxPrimitivesBitcoinUtxoRef (51) */
  interface UlxPrimitivesBitcoinUtxoRef extends Struct {
    readonly txid: UlxPrimitivesBitcoinH256Le;
    readonly outputIndex: Compact<u32>;
  }

  /** @name UlxPrimitivesBitcoinH256Le (52) */
  interface UlxPrimitivesBitcoinH256Le extends U8aFixed {}

  /** @name PalletVaultsEvent (54) */
  interface PalletVaultsEvent extends Enum {
    readonly isVaultCreated: boolean;
    readonly asVaultCreated: {
      readonly vaultId: u32;
      readonly bitcoinArgons: u128;
      readonly miningArgons: u128;
      readonly securitizationPercent: u128;
      readonly operatorAccountId: AccountId32;
    } & Struct;
    readonly isVaultModified: boolean;
    readonly asVaultModified: {
      readonly vaultId: u32;
      readonly bitcoinArgons: u128;
      readonly miningArgons: u128;
      readonly securitizationPercent: u128;
    } & Struct;
    readonly isVaultTermsChangeScheduled: boolean;
    readonly asVaultTermsChangeScheduled: {
      readonly vaultId: u32;
      readonly changeBlock: u32;
    } & Struct;
    readonly isVaultTermsChanged: boolean;
    readonly asVaultTermsChanged: {
      readonly vaultId: u32;
    } & Struct;
    readonly isVaultClosed: boolean;
    readonly asVaultClosed: {
      readonly vaultId: u32;
      readonly bitcoinAmountStillBonded: u128;
      readonly miningAmountStillBonded: u128;
      readonly securitizationStillBonded: u128;
    } & Struct;
    readonly type: 'VaultCreated' | 'VaultModified' | 'VaultTermsChangeScheduled' | 'VaultTermsChanged' | 'VaultClosed';
  }

  /** @name PalletBondEvent (56) */
  interface PalletBondEvent extends Enum {
    readonly isBondCreated: boolean;
    readonly asBondCreated: {
      readonly vaultId: u32;
      readonly bondId: u64;
      readonly bondType: UlxPrimitivesBondBondType;
      readonly bondedAccountId: AccountId32;
      readonly utxoId: Option<u64>;
      readonly amount: u128;
      readonly expiration: UlxPrimitivesBondBondExpiration;
    } & Struct;
    readonly isBondCompleted: boolean;
    readonly asBondCompleted: {
      readonly vaultId: u32;
      readonly bondId: u64;
    } & Struct;
    readonly isBondCanceled: boolean;
    readonly asBondCanceled: {
      readonly vaultId: u32;
      readonly bondId: u64;
      readonly bondedAccountId: AccountId32;
      readonly bondType: UlxPrimitivesBondBondType;
      readonly returnedFee: u128;
    } & Struct;
    readonly isBitcoinBondBurned: boolean;
    readonly asBitcoinBondBurned: {
      readonly vaultId: u32;
      readonly bondId: u64;
      readonly utxoId: u64;
      readonly amountBurned: u128;
      readonly amountHeld: u128;
      readonly wasUtxoSpent: bool;
    } & Struct;
    readonly isBitcoinUtxoCosignRequested: boolean;
    readonly asBitcoinUtxoCosignRequested: {
      readonly bondId: u64;
      readonly vaultId: u32;
      readonly utxoId: u64;
    } & Struct;
    readonly isBitcoinUtxoCosigned: boolean;
    readonly asBitcoinUtxoCosigned: {
      readonly bondId: u64;
      readonly vaultId: u32;
      readonly utxoId: u64;
      readonly pubkey: UlxPrimitivesBitcoinCompressedBitcoinPubkey;
      readonly signature: Bytes;
    } & Struct;
    readonly isBitcoinCosignPastDue: boolean;
    readonly asBitcoinCosignPastDue: {
      readonly bondId: u64;
      readonly vaultId: u32;
      readonly utxoId: u64;
      readonly compensationAmount: u128;
      readonly compensationStillOwed: u128;
      readonly compensatedAccountId: AccountId32;
    } & Struct;
    readonly isBondCompletionError: boolean;
    readonly asBondCompletionError: {
      readonly bondId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isCosignOverdueError: boolean;
    readonly asCosignOverdueError: {
      readonly utxoId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'BondCreated' | 'BondCompleted' | 'BondCanceled' | 'BitcoinBondBurned' | 'BitcoinUtxoCosignRequested' | 'BitcoinUtxoCosigned' | 'BitcoinCosignPastDue' | 'BondCompletionError' | 'CosignOverdueError';
  }

  /** @name UlxPrimitivesBondBondType (57) */
  interface UlxPrimitivesBondBondType extends Enum {
    readonly isMining: boolean;
    readonly isBitcoin: boolean;
    readonly type: 'Mining' | 'Bitcoin';
  }

  /** @name UlxPrimitivesBondBondExpiration (58) */
  interface UlxPrimitivesBondBondExpiration extends Enum {
    readonly isUlixeeBlock: boolean;
    readonly asUlixeeBlock: Compact<u32>;
    readonly isBitcoinBlock: boolean;
    readonly asBitcoinBlock: Compact<u64>;
    readonly type: 'UlixeeBlock' | 'BitcoinBlock';
  }

  /** @name UlxPrimitivesBitcoinCompressedBitcoinPubkey (59) */
  interface UlxPrimitivesBitcoinCompressedBitcoinPubkey extends U8aFixed {}

  /** @name PalletNotariesEvent (63) */
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
      readonly effectiveTick: u32;
    } & Struct;
    readonly isNotaryMetaUpdated: boolean;
    readonly asNotaryMetaUpdated: {
      readonly notaryId: u32;
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly isNotaryMetaUpdateError: boolean;
    readonly asNotaryMetaUpdateError: {
      readonly notaryId: u32;
      readonly error: SpRuntimeDispatchError;
      readonly meta: UlxPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly type: 'NotaryProposed' | 'NotaryActivated' | 'NotaryMetaUpdateQueued' | 'NotaryMetaUpdated' | 'NotaryMetaUpdateError';
  }

  /** @name UlxPrimitivesNotaryNotaryMeta (64) */
  interface UlxPrimitivesNotaryNotaryMeta extends Struct {
    readonly name: Bytes;
    readonly public: U8aFixed;
    readonly hosts: Vec<Bytes>;
  }

  /** @name UlxPrimitivesNotaryNotaryRecord (71) */
  interface UlxPrimitivesNotaryNotaryRecord extends Struct {
    readonly notaryId: Compact<u32>;
    readonly operatorAccountId: AccountId32;
    readonly activatedBlock: Compact<u32>;
    readonly metaUpdatedBlock: Compact<u32>;
    readonly metaUpdatedTick: Compact<u32>;
    readonly meta: UlxPrimitivesNotaryNotaryMeta;
  }

  /** @name PalletNotebookEvent (72) */
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

  /** @name UlxNotaryAuditErrorVerifyError (73) */
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
    readonly isInvalidSecretProvided: boolean;
    readonly isNotebookTooOld: boolean;
    readonly isCatchupNotebooksMissing: boolean;
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
    readonly type: 'MissingAccountOrigin' | 'HistoryLookupError' | 'InvalidAccountChangelist' | 'InvalidChainTransfersList' | 'InvalidBalanceChangeRoot' | 'InvalidHeaderTaxRecorded' | 'InvalidPreviousNonce' | 'InvalidPreviousBalance' | 'InvalidPreviousAccountOrigin' | 'InvalidPreviousBalanceChangeNotebook' | 'InvalidBalanceChange' | 'InvalidBalanceChangeSignature' | 'InvalidNoteRecipients' | 'BalanceChangeError' | 'InvalidNetBalanceChangeset' | 'InsufficientBalance' | 'ExceededMaxBalance' | 'BalanceChangeMismatch' | 'BalanceChangeNotNetZero' | 'InvalidDomainLeaseAllocation' | 'TaxBalanceChangeNotNetZero' | 'MissingBalanceProof' | 'InvalidPreviousBalanceProof' | 'InvalidNotebookHash' | 'InvalidNotebookHeaderHash' | 'DuplicateChainTransfer' | 'DuplicatedAccountOriginUid' | 'InvalidNotarySignature' | 'InvalidSecretProvided' | 'NotebookTooOld' | 'CatchupNotebooksMissing' | 'DecodeError' | 'AccountEscrowHoldDoesntExist' | 'AccountAlreadyHasEscrowHold' | 'EscrowHoldNotReadyForClaim' | 'AccountLocked' | 'MissingEscrowHoldNote' | 'InvalidEscrowHoldNote' | 'InvalidEscrowClaimers' | 'EscrowNoteBelowMinimum' | 'InvalidTaxNoteAccount' | 'InvalidTaxOperation' | 'InsufficientTaxIncluded' | 'InsufficientBlockVoteTax' | 'IneligibleTaxVoter' | 'BlockVoteInvalidSignature' | 'InvalidBlockVoteAllocation' | 'InvalidBlockVoteRoot' | 'InvalidBlockVotesCount' | 'InvalidBlockVotingPower' | 'InvalidBlockVoteList' | 'InvalidComputeProof' | 'InvalidBlockVoteSource' | 'InsufficientBlockVoteMinimum' | 'BlockVoteDataDomainMismatch' | 'BlockVoteEscrowReused';
  }

  /** @name UlxPrimitivesAccountAccountType (74) */
  interface UlxPrimitivesAccountAccountType extends Enum {
    readonly isTax: boolean;
    readonly isDeposit: boolean;
    readonly type: 'Tax' | 'Deposit';
  }

  /** @name UlxNotaryAuditAccountHistoryLookupError (75) */
  interface UlxNotaryAuditAccountHistoryLookupError extends Enum {
    readonly isRootNotFound: boolean;
    readonly isLastChangeNotFound: boolean;
    readonly isInvalidTransferToLocalchain: boolean;
    readonly isBlockSpecificationNotFound: boolean;
    readonly type: 'RootNotFound' | 'LastChangeNotFound' | 'InvalidTransferToLocalchain' | 'BlockSpecificationNotFound';
  }

  /** @name PalletChainTransferEvent (78) */
  interface PalletChainTransferEvent extends Enum {
    readonly isTransferToLocalchain: boolean;
    readonly asTransferToLocalchain: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly transferId: u32;
      readonly notaryId: u32;
      readonly expirationTick: u32;
    } & Struct;
    readonly isTransferToLocalchainExpired: boolean;
    readonly asTransferToLocalchainExpired: {
      readonly accountId: AccountId32;
      readonly transferId: u32;
      readonly notaryId: u32;
    } & Struct;
    readonly isTransferIn: boolean;
    readonly asTransferIn: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly notaryId: u32;
    } & Struct;
    readonly isTransferInError: boolean;
    readonly asTransferInError: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly notaryId: u32;
      readonly notebookNumber: u32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isTransferToLocalchainRefundError: boolean;
    readonly asTransferToLocalchainRefundError: {
      readonly accountId: AccountId32;
      readonly transferId: u32;
      readonly notaryId: u32;
      readonly notebookNumber: u32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isPossibleInvalidTransferAllowed: boolean;
    readonly asPossibleInvalidTransferAllowed: {
      readonly transferId: u32;
      readonly notaryId: u32;
      readonly notebookNumber: u32;
    } & Struct;
    readonly isTaxationError: boolean;
    readonly asTaxationError: {
      readonly notaryId: u32;
      readonly notebookNumber: u32;
      readonly tax: u128;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'TransferToLocalchain' | 'TransferToLocalchainExpired' | 'TransferIn' | 'TransferInError' | 'TransferToLocalchainRefundError' | 'PossibleInvalidTransferAllowed' | 'TaxationError';
  }

  /** @name PalletBlockSealSpecEvent (79) */
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

  /** @name PalletDataDomainEvent (80) */
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
    readonly isDataDomainRegistrationError: boolean;
    readonly asDataDomainRegistrationError: {
      readonly domainHash: H256;
      readonly accountId: AccountId32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'ZoneRecordUpdated' | 'DataDomainRegistered' | 'DataDomainRenewed' | 'DataDomainExpired' | 'DataDomainRegistrationCanceled' | 'DataDomainRegistrationError';
  }

  /** @name UlxPrimitivesDataDomainZoneRecord (81) */
  interface UlxPrimitivesDataDomainZoneRecord extends Struct {
    readonly paymentAccount: AccountId32;
    readonly notaryId: u32;
    readonly versions: BTreeMap<UlxPrimitivesDataDomainSemver, UlxPrimitivesDataDomainVersionHost>;
  }

  /** @name UlxPrimitivesDataDomainSemver (83) */
  interface UlxPrimitivesDataDomainSemver extends Struct {
    readonly major: u32;
    readonly minor: u32;
    readonly patch: u32;
  }

  /** @name UlxPrimitivesDataDomainVersionHost (84) */
  interface UlxPrimitivesDataDomainVersionHost extends Struct {
    readonly datastoreId: Bytes;
    readonly host: Bytes;
  }

  /** @name PalletDataDomainDataDomainRegistration (88) */
  interface PalletDataDomainDataDomainRegistration extends Struct {
    readonly accountId: AccountId32;
    readonly registeredAtTick: u32;
  }

  /** @name PalletPriceIndexEvent (89) */
  interface PalletPriceIndexEvent extends Enum {
    readonly isNewIndex: boolean;
    readonly isOperatorChanged: boolean;
    readonly asOperatorChanged: {
      readonly operatorId: AccountId32;
    } & Struct;
    readonly type: 'NewIndex' | 'OperatorChanged';
  }

  /** @name PalletSessionEvent (90) */
  interface PalletSessionEvent extends Enum {
    readonly isNewSession: boolean;
    readonly asNewSession: {
      readonly sessionIndex: u32;
    } & Struct;
    readonly type: 'NewSession';
  }

  /** @name PalletBlockRewardsEvent (91) */
  interface PalletBlockRewardsEvent extends Enum {
    readonly isRewardCreated: boolean;
    readonly asRewardCreated: {
      readonly maturationBlock: u32;
      readonly rewards: Vec<UlxPrimitivesBlockSealBlockPayout>;
    } & Struct;
    readonly isRewardUnlocked: boolean;
    readonly asRewardUnlocked: {
      readonly rewards: Vec<UlxPrimitivesBlockSealBlockPayout>;
    } & Struct;
    readonly isRewardUnlockError: boolean;
    readonly asRewardUnlockError: {
      readonly accountId: AccountId32;
      readonly argons: Option<u128>;
      readonly ulixees: Option<u128>;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isRewardCreateError: boolean;
    readonly asRewardCreateError: {
      readonly accountId: AccountId32;
      readonly argons: Option<u128>;
      readonly ulixees: Option<u128>;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'RewardCreated' | 'RewardUnlocked' | 'RewardUnlockError' | 'RewardCreateError';
  }

  /** @name UlxPrimitivesBlockSealBlockPayout (93) */
  interface UlxPrimitivesBlockSealBlockPayout extends Struct {
    readonly accountId: AccountId32;
    readonly ulixees: Compact<u128>;
    readonly argons: Compact<u128>;
  }

  /** @name PalletGrandpaEvent (95) */
  interface PalletGrandpaEvent extends Enum {
    readonly isNewAuthorities: boolean;
    readonly asNewAuthorities: {
      readonly authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    } & Struct;
    readonly isPaused: boolean;
    readonly isResumed: boolean;
    readonly type: 'NewAuthorities' | 'Paused' | 'Resumed';
  }

  /** @name SpConsensusGrandpaAppPublic (98) */
  interface SpConsensusGrandpaAppPublic extends U8aFixed {}

  /** @name PalletOffencesEvent (99) */
  interface PalletOffencesEvent extends Enum {
    readonly isOffence: boolean;
    readonly asOffence: {
      readonly kind: U8aFixed;
      readonly timeslot: Bytes;
    } & Struct;
    readonly type: 'Offence';
  }

  /** @name PalletMintEvent (101) */
  interface PalletMintEvent extends Enum {
    readonly isArgonsMinted: boolean;
    readonly asArgonsMinted: {
      readonly mintType: PalletMintMintType;
      readonly accountId: AccountId32;
      readonly utxoId: Option<u64>;
      readonly amount: u128;
    } & Struct;
    readonly isMintError: boolean;
    readonly asMintError: {
      readonly mintType: PalletMintMintType;
      readonly accountId: AccountId32;
      readonly utxoId: Option<u64>;
      readonly amount: u128;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'ArgonsMinted' | 'MintError';
  }

  /** @name PalletMintMintType (102) */
  interface PalletMintMintType extends Enum {
    readonly isBitcoin: boolean;
    readonly isUlixee: boolean;
    readonly type: 'Bitcoin' | 'Ulixee';
  }

  /** @name PalletBalancesEvent (103) */
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
    readonly isTotalIssuanceForced: boolean;
    readonly asTotalIssuanceForced: {
      readonly old: u128;
      readonly new_: u128;
    } & Struct;
    readonly type: 'Endowed' | 'DustLost' | 'Transfer' | 'BalanceSet' | 'Reserved' | 'Unreserved' | 'ReserveRepatriated' | 'Deposit' | 'Withdraw' | 'Slashed' | 'Minted' | 'Burned' | 'Suspended' | 'Restored' | 'Upgraded' | 'Issued' | 'Rescinded' | 'Locked' | 'Unlocked' | 'Frozen' | 'Thawed' | 'TotalIssuanceForced';
  }

  /** @name FrameSupportTokensMiscBalanceStatus (104) */
  interface FrameSupportTokensMiscBalanceStatus extends Enum {
    readonly isFree: boolean;
    readonly isReserved: boolean;
    readonly type: 'Free' | 'Reserved';
  }

  /** @name PalletTxPauseEvent (106) */
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

  /** @name PalletTransactionPaymentEvent (109) */
  interface PalletTransactionPaymentEvent extends Enum {
    readonly isTransactionFeePaid: boolean;
    readonly asTransactionFeePaid: {
      readonly who: AccountId32;
      readonly actualFee: u128;
      readonly tip: u128;
    } & Struct;
    readonly type: 'TransactionFeePaid';
  }

  /** @name PalletSudoEvent (110) */
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

  /** @name FrameSystemPhase (112) */
  interface FrameSystemPhase extends Enum {
    readonly isApplyExtrinsic: boolean;
    readonly asApplyExtrinsic: u32;
    readonly isFinalization: boolean;
    readonly isInitialization: boolean;
    readonly type: 'ApplyExtrinsic' | 'Finalization' | 'Initialization';
  }

  /** @name FrameSystemLastRuntimeUpgradeInfo (116) */
  interface FrameSystemLastRuntimeUpgradeInfo extends Struct {
    readonly specVersion: Compact<u32>;
    readonly specName: Text;
  }

  /** @name FrameSystemCodeUpgradeAuthorization (117) */
  interface FrameSystemCodeUpgradeAuthorization extends Struct {
    readonly codeHash: H256;
    readonly checkVersion: bool;
  }

  /** @name FrameSystemCall (118) */
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

  /** @name FrameSystemLimitsBlockWeights (122) */
  interface FrameSystemLimitsBlockWeights extends Struct {
    readonly baseBlock: SpWeightsWeightV2Weight;
    readonly maxBlock: SpWeightsWeightV2Weight;
    readonly perClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
  }

  /** @name FrameSupportDispatchPerDispatchClassWeightsPerClass (123) */
  interface FrameSupportDispatchPerDispatchClassWeightsPerClass extends Struct {
    readonly normal: FrameSystemLimitsWeightsPerClass;
    readonly operational: FrameSystemLimitsWeightsPerClass;
    readonly mandatory: FrameSystemLimitsWeightsPerClass;
  }

  /** @name FrameSystemLimitsWeightsPerClass (124) */
  interface FrameSystemLimitsWeightsPerClass extends Struct {
    readonly baseExtrinsic: SpWeightsWeightV2Weight;
    readonly maxExtrinsic: Option<SpWeightsWeightV2Weight>;
    readonly maxTotal: Option<SpWeightsWeightV2Weight>;
    readonly reserved: Option<SpWeightsWeightV2Weight>;
  }

  /** @name FrameSystemLimitsBlockLength (126) */
  interface FrameSystemLimitsBlockLength extends Struct {
    readonly max: FrameSupportDispatchPerDispatchClassU32;
  }

  /** @name FrameSupportDispatchPerDispatchClassU32 (127) */
  interface FrameSupportDispatchPerDispatchClassU32 extends Struct {
    readonly normal: u32;
    readonly operational: u32;
    readonly mandatory: u32;
  }

  /** @name SpWeightsRuntimeDbWeight (128) */
  interface SpWeightsRuntimeDbWeight extends Struct {
    readonly read: u64;
    readonly write: u64;
  }

  /** @name SpVersionRuntimeVersion (129) */
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

  /** @name FrameSystemError (134) */
  interface FrameSystemError extends Enum {
    readonly isInvalidSpecName: boolean;
    readonly isSpecVersionNeedsToIncrease: boolean;
    readonly isFailedToExtractRuntimeVersion: boolean;
    readonly isNonDefaultComposite: boolean;
    readonly isNonZeroRefCount: boolean;
    readonly isCallFiltered: boolean;
    readonly isMultiBlockMigrationsOngoing: boolean;
    readonly isNothingAuthorized: boolean;
    readonly isUnauthorized: boolean;
    readonly type: 'InvalidSpecName' | 'SpecVersionNeedsToIncrease' | 'FailedToExtractRuntimeVersion' | 'NonDefaultComposite' | 'NonZeroRefCount' | 'CallFiltered' | 'MultiBlockMigrationsOngoing' | 'NothingAuthorized' | 'Unauthorized';
  }

  /** @name PalletTimestampCall (135) */
  interface PalletTimestampCall extends Enum {
    readonly isSet: boolean;
    readonly asSet: {
      readonly now: Compact<u64>;
    } & Struct;
    readonly type: 'Set';
  }

  /** @name PalletMultisigMultisig (137) */
  interface PalletMultisigMultisig extends Struct {
    readonly when: PalletMultisigTimepoint;
    readonly deposit: u128;
    readonly depositor: AccountId32;
    readonly approvals: Vec<AccountId32>;
  }

  /** @name PalletMultisigCall (140) */
  interface PalletMultisigCall extends Enum {
    readonly isAsMultiThreshold1: boolean;
    readonly asAsMultiThreshold1: {
      readonly otherSignatories: Vec<AccountId32>;
      readonly call: Call;
    } & Struct;
    readonly isAsMulti: boolean;
    readonly asAsMulti: {
      readonly threshold: u16;
      readonly otherSignatories: Vec<AccountId32>;
      readonly maybeTimepoint: Option<PalletMultisigTimepoint>;
      readonly call: Call;
      readonly maxWeight: SpWeightsWeightV2Weight;
    } & Struct;
    readonly isApproveAsMulti: boolean;
    readonly asApproveAsMulti: {
      readonly threshold: u16;
      readonly otherSignatories: Vec<AccountId32>;
      readonly maybeTimepoint: Option<PalletMultisigTimepoint>;
      readonly callHash: U8aFixed;
      readonly maxWeight: SpWeightsWeightV2Weight;
    } & Struct;
    readonly isCancelAsMulti: boolean;
    readonly asCancelAsMulti: {
      readonly threshold: u16;
      readonly otherSignatories: Vec<AccountId32>;
      readonly timepoint: PalletMultisigTimepoint;
      readonly callHash: U8aFixed;
    } & Struct;
    readonly type: 'AsMultiThreshold1' | 'AsMulti' | 'ApproveAsMulti' | 'CancelAsMulti';
  }

  /** @name PalletProxyCall (142) */
  interface PalletProxyCall extends Enum {
    readonly isProxy: boolean;
    readonly asProxy: {
      readonly real: MultiAddress;
      readonly forceProxyType: Option<UlxNodeRuntimeProxyType>;
      readonly call: Call;
    } & Struct;
    readonly isAddProxy: boolean;
    readonly asAddProxy: {
      readonly delegate: MultiAddress;
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isRemoveProxy: boolean;
    readonly asRemoveProxy: {
      readonly delegate: MultiAddress;
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isRemoveProxies: boolean;
    readonly isCreatePure: boolean;
    readonly asCreatePure: {
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly delay: u32;
      readonly index: u16;
    } & Struct;
    readonly isKillPure: boolean;
    readonly asKillPure: {
      readonly spawner: MultiAddress;
      readonly proxyType: UlxNodeRuntimeProxyType;
      readonly index: u16;
      readonly height: Compact<u32>;
      readonly extIndex: Compact<u32>;
    } & Struct;
    readonly isAnnounce: boolean;
    readonly asAnnounce: {
      readonly real: MultiAddress;
      readonly callHash: H256;
    } & Struct;
    readonly isRemoveAnnouncement: boolean;
    readonly asRemoveAnnouncement: {
      readonly real: MultiAddress;
      readonly callHash: H256;
    } & Struct;
    readonly isRejectAnnouncement: boolean;
    readonly asRejectAnnouncement: {
      readonly delegate: MultiAddress;
      readonly callHash: H256;
    } & Struct;
    readonly isProxyAnnounced: boolean;
    readonly asProxyAnnounced: {
      readonly delegate: MultiAddress;
      readonly real: MultiAddress;
      readonly forceProxyType: Option<UlxNodeRuntimeProxyType>;
      readonly call: Call;
    } & Struct;
    readonly type: 'Proxy' | 'AddProxy' | 'RemoveProxy' | 'RemoveProxies' | 'CreatePure' | 'KillPure' | 'Announce' | 'RemoveAnnouncement' | 'RejectAnnouncement' | 'ProxyAnnounced';
  }

  /** @name PalletTicksCall (147) */
  type PalletTicksCall = Null;

  /** @name PalletMiningSlotCall (148) */
  interface PalletMiningSlotCall extends Enum {
    readonly isBid: boolean;
    readonly asBid: {
      readonly bondInfo: Option<PalletMiningSlotMiningSlotBid>;
      readonly rewardDestination: UlxPrimitivesBlockSealRewardDestination;
    } & Struct;
    readonly type: 'Bid';
  }

  /** @name PalletMiningSlotMiningSlotBid (150) */
  interface PalletMiningSlotMiningSlotBid extends Struct {
    readonly vaultId: u32;
    readonly amount: u128;
  }

  /** @name PalletBitcoinUtxosCall (151) */
  interface PalletBitcoinUtxosCall extends Enum {
    readonly isSync: boolean;
    readonly asSync: {
      readonly utxoSync: UlxPrimitivesInherentsBitcoinUtxoSync;
    } & Struct;
    readonly isSetConfirmedBlock: boolean;
    readonly asSetConfirmedBlock: {
      readonly bitcoinHeight: u64;
      readonly bitcoinBlockHash: UlxPrimitivesBitcoinH256Le;
    } & Struct;
    readonly isSetOperator: boolean;
    readonly asSetOperator: {
      readonly accountId: AccountId32;
    } & Struct;
    readonly type: 'Sync' | 'SetConfirmedBlock' | 'SetOperator';
  }

  /** @name UlxPrimitivesInherentsBitcoinUtxoSync (152) */
  interface UlxPrimitivesInherentsBitcoinUtxoSync extends Struct {
    readonly spent: BTreeMap<u64, u64>;
    readonly verified: BTreeMap<u64, UlxPrimitivesBitcoinUtxoRef>;
    readonly invalid: BTreeMap<u64, UlxPrimitivesBitcoinBitcoinRejectedReason>;
    readonly syncToBlock: UlxPrimitivesBitcoinBitcoinBlock;
  }

  /** @name UlxPrimitivesBitcoinBitcoinBlock (162) */
  interface UlxPrimitivesBitcoinBitcoinBlock extends Struct {
    readonly blockHeight: Compact<u64>;
    readonly blockHash: UlxPrimitivesBitcoinH256Le;
  }

  /** @name PalletVaultsCall (163) */
  interface PalletVaultsCall extends Enum {
    readonly isCreate: boolean;
    readonly asCreate: {
      readonly vaultConfig: PalletVaultsVaultConfig;
    } & Struct;
    readonly isModifyFunding: boolean;
    readonly asModifyFunding: {
      readonly vaultId: u32;
      readonly totalMiningAmountOffered: u128;
      readonly totalBitcoinAmountOffered: u128;
      readonly securitizationPercent: u128;
    } & Struct;
    readonly isModifyTerms: boolean;
    readonly asModifyTerms: {
      readonly vaultId: u32;
      readonly terms: UlxPrimitivesBondVaultTerms;
    } & Struct;
    readonly isClose: boolean;
    readonly asClose: {
      readonly vaultId: u32;
    } & Struct;
    readonly isAddBitcoinPubkeyHashes: boolean;
    readonly asAddBitcoinPubkeyHashes: {
      readonly vaultId: u32;
      readonly bitcoinPubkeyHashes: Vec<UlxPrimitivesBitcoinBitcoinPubkeyHash>;
    } & Struct;
    readonly type: 'Create' | 'ModifyFunding' | 'ModifyTerms' | 'Close' | 'AddBitcoinPubkeyHashes';
  }

  /** @name PalletVaultsVaultConfig (164) */
  interface PalletVaultsVaultConfig extends Struct {
    readonly terms: UlxPrimitivesBondVaultTerms;
    readonly bitcoinAmountAllocated: Compact<u128>;
    readonly bitcoinPubkeyHashes: Vec<UlxPrimitivesBitcoinBitcoinPubkeyHash>;
    readonly miningAmountAllocated: Compact<u128>;
    readonly securitizationPercent: Compact<u128>;
  }

  /** @name UlxPrimitivesBondVaultTerms (165) */
  interface UlxPrimitivesBondVaultTerms extends Struct {
    readonly bitcoinAnnualPercentRate: Compact<u128>;
    readonly bitcoinBaseFee: Compact<u128>;
    readonly miningAnnualPercentRate: Compact<u128>;
    readonly miningBaseFee: Compact<u128>;
    readonly miningRewardSharingPercentTake: Compact<Percent>;
  }

  /** @name UlxPrimitivesBitcoinBitcoinPubkeyHash (168) */
  interface UlxPrimitivesBitcoinBitcoinPubkeyHash extends U8aFixed {}

  /** @name PalletBondCall (170) */
  interface PalletBondCall extends Enum {
    readonly isBondBitcoin: boolean;
    readonly asBondBitcoin: {
      readonly vaultId: u32;
      readonly satoshis: Compact<u64>;
      readonly bitcoinPubkeyHash: UlxPrimitivesBitcoinBitcoinPubkeyHash;
    } & Struct;
    readonly isUnlockBitcoinBond: boolean;
    readonly asUnlockBitcoinBond: {
      readonly bondId: u64;
      readonly toScriptPubkey: Bytes;
      readonly bitcoinNetworkFee: u64;
    } & Struct;
    readonly isCosignBitcoinUnlock: boolean;
    readonly asCosignBitcoinUnlock: {
      readonly bondId: u64;
      readonly pubkey: UlxPrimitivesBitcoinCompressedBitcoinPubkey;
      readonly signature: Bytes;
    } & Struct;
    readonly type: 'BondBitcoin' | 'UnlockBitcoinBond' | 'CosignBitcoinUnlock';
  }

  /** @name PalletNotariesCall (173) */
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
      readonly effectiveTick: Compact<u32>;
    } & Struct;
    readonly type: 'Propose' | 'Activate' | 'Update';
  }

  /** @name PalletNotebookCall (174) */
  interface PalletNotebookCall extends Enum {
    readonly isSubmit: boolean;
    readonly asSubmit: {
      readonly notebooks: Vec<UlxPrimitivesNotebookSignedNotebookHeader>;
    } & Struct;
    readonly type: 'Submit';
  }

  /** @name UlxPrimitivesNotebookSignedNotebookHeader (176) */
  interface UlxPrimitivesNotebookSignedNotebookHeader extends Struct {
    readonly header: UlxPrimitivesNotebookNotebookHeader;
    readonly signature: U8aFixed;
  }

  /** @name UlxPrimitivesNotebookNotebookHeader (177) */
  interface UlxPrimitivesNotebookNotebookHeader extends Struct {
    readonly version: Compact<u16>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
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

  /** @name UlxPrimitivesNotebookChainTransfer (180) */
  interface UlxPrimitivesNotebookChainTransfer extends Enum {
    readonly isToMainchain: boolean;
    readonly asToMainchain: {
      readonly accountId: AccountId32;
      readonly amount: Compact<u128>;
    } & Struct;
    readonly isToLocalchain: boolean;
    readonly asToLocalchain: {
      readonly transferId: Compact<u32>;
    } & Struct;
    readonly type: 'ToMainchain' | 'ToLocalchain';
  }

  /** @name UlxPrimitivesBalanceChangeAccountOrigin (183) */
  interface UlxPrimitivesBalanceChangeAccountOrigin extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly accountUid: Compact<u32>;
  }

  /** @name PalletChainTransferCall (191) */
  interface PalletChainTransferCall extends Enum {
    readonly isSendToLocalchain: boolean;
    readonly asSendToLocalchain: {
      readonly amount: Compact<u128>;
      readonly notaryId: u32;
    } & Struct;
    readonly type: 'SendToLocalchain';
  }

  /** @name PalletBlockSealSpecCall (192) */
  interface PalletBlockSealSpecCall extends Enum {
    readonly isConfigure: boolean;
    readonly asConfigure: {
      readonly voteMinimum: Option<u128>;
      readonly computeDifficulty: Option<u128>;
    } & Struct;
    readonly type: 'Configure';
  }

  /** @name PalletDataDomainCall (193) */
  interface PalletDataDomainCall extends Enum {
    readonly isSetZoneRecord: boolean;
    readonly asSetZoneRecord: {
      readonly domainHash: H256;
      readonly zoneRecord: UlxPrimitivesDataDomainZoneRecord;
    } & Struct;
    readonly type: 'SetZoneRecord';
  }

  /** @name PalletPriceIndexCall (194) */
  interface PalletPriceIndexCall extends Enum {
    readonly isSubmit: boolean;
    readonly asSubmit: {
      readonly index: PalletPriceIndexPriceIndex;
    } & Struct;
    readonly isSetOperator: boolean;
    readonly asSetOperator: {
      readonly accountId: AccountId32;
    } & Struct;
    readonly type: 'Submit' | 'SetOperator';
  }

  /** @name PalletPriceIndexPriceIndex (195) */
  interface PalletPriceIndexPriceIndex extends Struct {
    readonly btcUsdPrice: Compact<u128>;
    readonly argonUsdPrice: Compact<u128>;
    readonly argonUsdTargetPrice: u128;
    readonly tick: Compact<u32>;
  }

  /** @name PalletSessionCall (196) */
  interface PalletSessionCall extends Enum {
    readonly isSetKeys: boolean;
    readonly asSetKeys: {
      readonly keys_: UlxNodeRuntimeOpaqueSessionKeys;
      readonly proof: Bytes;
    } & Struct;
    readonly isPurgeKeys: boolean;
    readonly type: 'SetKeys' | 'PurgeKeys';
  }

  /** @name UlxNodeRuntimeOpaqueSessionKeys (197) */
  interface UlxNodeRuntimeOpaqueSessionKeys extends Struct {
    readonly grandpa: SpConsensusGrandpaAppPublic;
    readonly blockSealAuthority: UlxPrimitivesBlockSealAppPublic;
  }

  /** @name UlxPrimitivesBlockSealAppPublic (198) */
  interface UlxPrimitivesBlockSealAppPublic extends U8aFixed {}

  /** @name PalletBlockSealCall (199) */
  interface PalletBlockSealCall extends Enum {
    readonly isApply: boolean;
    readonly asApply: {
      readonly seal: UlxPrimitivesInherentsBlockSealInherent;
    } & Struct;
    readonly type: 'Apply';
  }

  /** @name UlxPrimitivesInherentsBlockSealInherent (200) */
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

  /** @name UlxPrimitivesBalanceChangeMerkleProof (203) */
  interface UlxPrimitivesBalanceChangeMerkleProof extends Struct {
    readonly proof: Vec<H256>;
    readonly numberOfLeaves: Compact<u32>;
    readonly leafIndex: Compact<u32>;
  }

  /** @name UlxPrimitivesBlockVoteBlockVoteT (205) */
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

  /** @name SpRuntimeMultiSignature (206) */
  interface SpRuntimeMultiSignature extends Enum {
    readonly isEd25519: boolean;
    readonly asEd25519: U8aFixed;
    readonly isSr25519: boolean;
    readonly asSr25519: U8aFixed;
    readonly isEcdsa: boolean;
    readonly asEcdsa: U8aFixed;
    readonly type: 'Ed25519' | 'Sr25519' | 'Ecdsa';
  }

  /** @name UlxPrimitivesBlockSealAppSignature (208) */
  interface UlxPrimitivesBlockSealAppSignature extends U8aFixed {}

  /** @name PalletBlockRewardsCall (209) */
  type PalletBlockRewardsCall = Null;

  /** @name PalletGrandpaCall (210) */
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

  /** @name SpConsensusGrandpaEquivocationProof (211) */
  interface SpConsensusGrandpaEquivocationProof extends Struct {
    readonly setId: u64;
    readonly equivocation: SpConsensusGrandpaEquivocation;
  }

  /** @name SpConsensusGrandpaEquivocation (212) */
  interface SpConsensusGrandpaEquivocation extends Enum {
    readonly isPrevote: boolean;
    readonly asPrevote: FinalityGrandpaEquivocationPrevote;
    readonly isPrecommit: boolean;
    readonly asPrecommit: FinalityGrandpaEquivocationPrecommit;
    readonly type: 'Prevote' | 'Precommit';
  }

  /** @name FinalityGrandpaEquivocationPrevote (213) */
  interface FinalityGrandpaEquivocationPrevote extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrevote (214) */
  interface FinalityGrandpaPrevote extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpConsensusGrandpaAppSignature (215) */
  interface SpConsensusGrandpaAppSignature extends U8aFixed {}

  /** @name FinalityGrandpaEquivocationPrecommit (217) */
  interface FinalityGrandpaEquivocationPrecommit extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrecommit (218) */
  interface FinalityGrandpaPrecommit extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpSessionMembershipProof (220) */
  interface SpSessionMembershipProof extends Struct {
    readonly session: u32;
    readonly trieNodes: Vec<Bytes>;
    readonly validatorCount: u32;
  }

  /** @name PalletMintCall (221) */
  type PalletMintCall = Null;

  /** @name PalletBalancesCall (222) */
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
    readonly isForceAdjustTotalIssuance: boolean;
    readonly asForceAdjustTotalIssuance: {
      readonly direction: PalletBalancesAdjustmentDirection;
      readonly delta: Compact<u128>;
    } & Struct;
    readonly isBurn: boolean;
    readonly asBurn: {
      readonly value: Compact<u128>;
      readonly keepAlive: bool;
    } & Struct;
    readonly type: 'TransferAllowDeath' | 'ForceTransfer' | 'TransferKeepAlive' | 'TransferAll' | 'ForceUnreserve' | 'UpgradeAccounts' | 'ForceSetBalance' | 'ForceAdjustTotalIssuance' | 'Burn';
  }

  /** @name PalletBalancesAdjustmentDirection (223) */
  interface PalletBalancesAdjustmentDirection extends Enum {
    readonly isIncrease: boolean;
    readonly isDecrease: boolean;
    readonly type: 'Increase' | 'Decrease';
  }

  /** @name PalletTxPauseCall (225) */
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

  /** @name PalletSudoCall (226) */
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

  /** @name PalletMultisigError (228) */
  interface PalletMultisigError extends Enum {
    readonly isMinimumThreshold: boolean;
    readonly isAlreadyApproved: boolean;
    readonly isNoApprovalsNeeded: boolean;
    readonly isTooFewSignatories: boolean;
    readonly isTooManySignatories: boolean;
    readonly isSignatoriesOutOfOrder: boolean;
    readonly isSenderInSignatories: boolean;
    readonly isNotFound: boolean;
    readonly isNotOwner: boolean;
    readonly isNoTimepoint: boolean;
    readonly isWrongTimepoint: boolean;
    readonly isUnexpectedTimepoint: boolean;
    readonly isMaxWeightTooLow: boolean;
    readonly isAlreadyStored: boolean;
    readonly type: 'MinimumThreshold' | 'AlreadyApproved' | 'NoApprovalsNeeded' | 'TooFewSignatories' | 'TooManySignatories' | 'SignatoriesOutOfOrder' | 'SenderInSignatories' | 'NotFound' | 'NotOwner' | 'NoTimepoint' | 'WrongTimepoint' | 'UnexpectedTimepoint' | 'MaxWeightTooLow' | 'AlreadyStored';
  }

  /** @name PalletProxyProxyDefinition (231) */
  interface PalletProxyProxyDefinition extends Struct {
    readonly delegate: AccountId32;
    readonly proxyType: UlxNodeRuntimeProxyType;
    readonly delay: u32;
  }

  /** @name PalletProxyAnnouncement (235) */
  interface PalletProxyAnnouncement extends Struct {
    readonly real: AccountId32;
    readonly callHash: H256;
    readonly height: u32;
  }

  /** @name PalletProxyError (237) */
  interface PalletProxyError extends Enum {
    readonly isTooMany: boolean;
    readonly isNotFound: boolean;
    readonly isNotProxy: boolean;
    readonly isUnproxyable: boolean;
    readonly isDuplicate: boolean;
    readonly isNoPermission: boolean;
    readonly isUnannounced: boolean;
    readonly isNoSelfProxy: boolean;
    readonly type: 'TooMany' | 'NotFound' | 'NotProxy' | 'Unproxyable' | 'Duplicate' | 'NoPermission' | 'Unannounced' | 'NoSelfProxy';
  }

  /** @name PalletTicksError (239) */
  type PalletTicksError = Null;

  /** @name PalletMiningSlotError (247) */
  interface PalletMiningSlotError extends Enum {
    readonly isSlotNotTakingBids: boolean;
    readonly isTooManyBlockRegistrants: boolean;
    readonly isInsufficientOwnershipTokens: boolean;
    readonly isBidTooLow: boolean;
    readonly isCannotRegisterOverlappingSessions: boolean;
    readonly isBondNotFound: boolean;
    readonly isNoMoreBondIds: boolean;
    readonly isVaultClosed: boolean;
    readonly isMinimumBondAmountNotMet: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isExpirationTooSoon: boolean;
    readonly isNoPermissions: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isBondAlreadyClosed: boolean;
    readonly isFeeExceedsBondAmount: boolean;
    readonly isAccountWouldBeBelowMinimum: boolean;
    readonly isGenericBondError: boolean;
    readonly asGenericBondError: UlxPrimitivesBondBondError;
    readonly type: 'SlotNotTakingBids' | 'TooManyBlockRegistrants' | 'InsufficientOwnershipTokens' | 'BidTooLow' | 'CannotRegisterOverlappingSessions' | 'BondNotFound' | 'NoMoreBondIds' | 'VaultClosed' | 'MinimumBondAmountNotMet' | 'ExpirationAtBlockOverflow' | 'InsufficientFunds' | 'InsufficientVaultFunds' | 'ExpirationTooSoon' | 'NoPermissions' | 'HoldUnexpectedlyModified' | 'UnrecoverableHold' | 'VaultNotFound' | 'BondAlreadyClosed' | 'FeeExceedsBondAmount' | 'AccountWouldBeBelowMinimum' | 'GenericBondError';
  }

  /** @name UlxPrimitivesBondBondError (248) */
  interface UlxPrimitivesBondBondError extends Enum {
    readonly isBondNotFound: boolean;
    readonly isNoMoreBondIds: boolean;
    readonly isMinimumBondAmountNotMet: boolean;
    readonly isVaultClosed: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isAccountWouldBeBelowMinimum: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isInsufficientBitcoinsForMining: boolean;
    readonly isExpirationTooSoon: boolean;
    readonly isNoPermissions: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isNoVaultBitcoinPubkeysAvailable: boolean;
    readonly isFeeExceedsBondAmount: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly type: 'BondNotFound' | 'NoMoreBondIds' | 'MinimumBondAmountNotMet' | 'VaultClosed' | 'ExpirationAtBlockOverflow' | 'AccountWouldBeBelowMinimum' | 'InsufficientFunds' | 'InsufficientVaultFunds' | 'InsufficientBitcoinsForMining' | 'ExpirationTooSoon' | 'NoPermissions' | 'HoldUnexpectedlyModified' | 'UnrecoverableHold' | 'VaultNotFound' | 'NoVaultBitcoinPubkeysAvailable' | 'FeeExceedsBondAmount' | 'InvalidBitcoinScript';
  }

  /** @name UlxPrimitivesBitcoinUtxoValue (249) */
  interface UlxPrimitivesBitcoinUtxoValue extends Struct {
    readonly utxoId: u64;
    readonly scriptPubkey: UlxPrimitivesBitcoinBitcoinCosignScriptPubkey;
    readonly satoshis: Compact<u64>;
    readonly submittedAtHeight: Compact<u64>;
    readonly watchForSpentUntilHeight: Compact<u64>;
  }

  /** @name UlxPrimitivesBitcoinBitcoinCosignScriptPubkey (250) */
  interface UlxPrimitivesBitcoinBitcoinCosignScriptPubkey extends Enum {
    readonly isP2wsh: boolean;
    readonly asP2wsh: {
      readonly wscriptHash: H256;
    } & Struct;
    readonly type: 'P2wsh';
  }

  /** @name PalletBitcoinUtxosError (257) */
  interface PalletBitcoinUtxosError extends Enum {
    readonly isNoPermissions: boolean;
    readonly isNoBitcoinConfirmedBlock: boolean;
    readonly isInsufficientBitcoinAmount: boolean;
    readonly isNoBitcoinPricesAvailable: boolean;
    readonly isScriptPubkeyConflict: boolean;
    readonly isUtxoNotLocked: boolean;
    readonly isRedemptionsUnavailable: boolean;
    readonly isInvalidBitcoinSyncHeight: boolean;
    readonly isBitcoinHeightNotConfirmed: boolean;
    readonly isMaxUtxosExceeded: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly type: 'NoPermissions' | 'NoBitcoinConfirmedBlock' | 'InsufficientBitcoinAmount' | 'NoBitcoinPricesAvailable' | 'ScriptPubkeyConflict' | 'UtxoNotLocked' | 'RedemptionsUnavailable' | 'InvalidBitcoinSyncHeight' | 'BitcoinHeightNotConfirmed' | 'MaxUtxosExceeded' | 'InvalidBitcoinScript';
  }

  /** @name UlxPrimitivesBondVault (258) */
  interface UlxPrimitivesBondVault extends Struct {
    readonly operatorAccountId: AccountId32;
    readonly bitcoinArgons: UlxPrimitivesBondVaultArgons;
    readonly securitizationPercent: Compact<u128>;
    readonly securitizedArgons: Compact<u128>;
    readonly miningArgons: UlxPrimitivesBondVaultArgons;
    readonly miningRewardSharingPercentTake: Compact<Percent>;
    readonly isClosed: bool;
    readonly pendingTerms: Option<ITuple<[u32, UlxPrimitivesBondVaultTerms]>>;
  }

  /** @name UlxPrimitivesBondVaultArgons (259) */
  interface UlxPrimitivesBondVaultArgons extends Struct {
    readonly annualPercentRate: Compact<u128>;
    readonly allocated: Compact<u128>;
    readonly bonded: Compact<u128>;
    readonly baseFee: Compact<u128>;
  }

  /** @name PalletVaultsError (263) */
  interface PalletVaultsError extends Enum {
    readonly isBondNotFound: boolean;
    readonly isNoMoreVaultIds: boolean;
    readonly isNoMoreBondIds: boolean;
    readonly isMinimumBondAmountNotMet: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isInsufficientBitcoinsForMining: boolean;
    readonly isAccountBelowMinimumBalance: boolean;
    readonly isVaultClosed: boolean;
    readonly isInvalidVaultAmount: boolean;
    readonly isVaultReductionBelowAllocatedFunds: boolean;
    readonly isInvalidSecuritization: boolean;
    readonly isMaxPendingVaultBitcoinPubkeys: boolean;
    readonly isMaxSecuritizationPercentExceeded: boolean;
    readonly isInvalidBondType: boolean;
    readonly isBitcoinUtxoNotFound: boolean;
    readonly isInsufficientSatoshisBonded: boolean;
    readonly isNoBitcoinPricesAvailable: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly isExpirationTooSoon: boolean;
    readonly isNoPermissions: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isFeeExceedsBondAmount: boolean;
    readonly isNoVaultBitcoinPubkeysAvailable: boolean;
    readonly isTermsModificationOverflow: boolean;
    readonly isTermsChangeAlreadyScheduled: boolean;
    readonly type: 'BondNotFound' | 'NoMoreVaultIds' | 'NoMoreBondIds' | 'MinimumBondAmountNotMet' | 'ExpirationAtBlockOverflow' | 'InsufficientFunds' | 'InsufficientVaultFunds' | 'InsufficientBitcoinsForMining' | 'AccountBelowMinimumBalance' | 'VaultClosed' | 'InvalidVaultAmount' | 'VaultReductionBelowAllocatedFunds' | 'InvalidSecuritization' | 'MaxPendingVaultBitcoinPubkeys' | 'MaxSecuritizationPercentExceeded' | 'InvalidBondType' | 'BitcoinUtxoNotFound' | 'InsufficientSatoshisBonded' | 'NoBitcoinPricesAvailable' | 'InvalidBitcoinScript' | 'ExpirationTooSoon' | 'NoPermissions' | 'HoldUnexpectedlyModified' | 'UnrecoverableHold' | 'VaultNotFound' | 'FeeExceedsBondAmount' | 'NoVaultBitcoinPubkeysAvailable' | 'TermsModificationOverflow' | 'TermsChangeAlreadyScheduled';
  }

  /** @name UlxPrimitivesBond (264) */
  interface UlxPrimitivesBond extends Struct {
    readonly bondType: UlxPrimitivesBondBondType;
    readonly vaultId: Compact<u32>;
    readonly utxoId: Option<u64>;
    readonly bondedAccountId: AccountId32;
    readonly totalFee: Compact<u128>;
    readonly prepaidFee: Compact<u128>;
    readonly amount: Compact<u128>;
    readonly startBlock: Compact<u32>;
    readonly expiration: UlxPrimitivesBondBondExpiration;
  }

  /** @name PalletBondUtxoState (267) */
  interface PalletBondUtxoState extends Struct {
    readonly bondId: u64;
    readonly satoshis: u64;
    readonly vaultPubkeyHash: UlxPrimitivesBitcoinBitcoinPubkeyHash;
    readonly ownerPubkeyHash: UlxPrimitivesBitcoinBitcoinPubkeyHash;
    readonly vaultClaimHeight: u64;
    readonly openClaimHeight: u64;
    readonly registerBlock: u64;
    readonly utxoScriptPubkey: UlxPrimitivesBitcoinBitcoinCosignScriptPubkey;
    readonly isVerified: bool;
  }

  /** @name PalletBondUtxoCosignRequest (270) */
  interface PalletBondUtxoCosignRequest extends Struct {
    readonly bitcoinNetworkFee: u64;
    readonly cosignDueBlock: u64;
    readonly toScriptPubkey: Bytes;
    readonly redemptionPrice: u128;
  }

  /** @name PalletBondError (274) */
  interface PalletBondError extends Enum {
    readonly isBondNotFound: boolean;
    readonly isNoMoreBondIds: boolean;
    readonly isMinimumBondAmountNotMet: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isInsufficientBitcoinsForMining: boolean;
    readonly isAccountWouldGoBelowMinimumBalance: boolean;
    readonly isVaultClosed: boolean;
    readonly isInvalidVaultAmount: boolean;
    readonly isBondRedemptionNotLocked: boolean;
    readonly isBitcoinUnlockInitiationDeadlinePassed: boolean;
    readonly isBitcoinFeeTooHigh: boolean;
    readonly isInvalidBondType: boolean;
    readonly isBitcoinUtxoNotFound: boolean;
    readonly isInsufficientSatoshisBonded: boolean;
    readonly isNoBitcoinPricesAvailable: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly isExpirationTooSoon: boolean;
    readonly isNoPermissions: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isFeeExceedsBondAmount: boolean;
    readonly isGenericBondError: boolean;
    readonly asGenericBondError: UlxPrimitivesBondBondError;
    readonly type: 'BondNotFound' | 'NoMoreBondIds' | 'MinimumBondAmountNotMet' | 'ExpirationAtBlockOverflow' | 'InsufficientFunds' | 'InsufficientVaultFunds' | 'InsufficientBitcoinsForMining' | 'AccountWouldGoBelowMinimumBalance' | 'VaultClosed' | 'InvalidVaultAmount' | 'BondRedemptionNotLocked' | 'BitcoinUnlockInitiationDeadlinePassed' | 'BitcoinFeeTooHigh' | 'InvalidBondType' | 'BitcoinUtxoNotFound' | 'InsufficientSatoshisBonded' | 'NoBitcoinPricesAvailable' | 'InvalidBitcoinScript' | 'ExpirationTooSoon' | 'NoPermissions' | 'HoldUnexpectedlyModified' | 'UnrecoverableHold' | 'VaultNotFound' | 'FeeExceedsBondAmount' | 'GenericBondError';
  }

  /** @name PalletNotariesError (286) */
  interface PalletNotariesError extends Enum {
    readonly isProposalNotFound: boolean;
    readonly isMaxNotariesExceeded: boolean;
    readonly isMaxProposalsPerBlockExceeded: boolean;
    readonly isNotAnActiveNotary: boolean;
    readonly isInvalidNotaryOperator: boolean;
    readonly isNoMoreNotaryIds: boolean;
    readonly isEffectiveTickTooSoon: boolean;
    readonly isTooManyKeys: boolean;
    readonly isInvalidNotary: boolean;
    readonly type: 'ProposalNotFound' | 'MaxNotariesExceeded' | 'MaxProposalsPerBlockExceeded' | 'NotAnActiveNotary' | 'InvalidNotaryOperator' | 'NoMoreNotaryIds' | 'EffectiveTickTooSoon' | 'TooManyKeys' | 'InvalidNotary';
  }

  /** @name UlxPrimitivesNotaryNotaryNotebookKeyDetails (290) */
  interface UlxPrimitivesNotaryNotaryNotebookKeyDetails extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly blockVotesRoot: H256;
    readonly secretHash: H256;
    readonly parentSecret: Option<H256>;
  }

  /** @name UlxPrimitivesDigestsNotebookDigest (292) */
  interface UlxPrimitivesDigestsNotebookDigest extends Struct {
    readonly notebooks: Vec<UlxPrimitivesDigestsNotebookDigestRecord>;
  }

  /** @name UlxPrimitivesDigestsNotebookDigestRecord (294) */
  interface UlxPrimitivesDigestsNotebookDigestRecord extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly auditFirstFailure: Option<UlxNotaryAuditErrorVerifyError>;
  }

  /** @name PalletNotebookError (297) */
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

  /** @name PalletChainTransferQueuedTransferOut (298) */
  interface PalletChainTransferQueuedTransferOut extends Struct {
    readonly accountId: AccountId32;
    readonly amount: u128;
    readonly expirationTick: u32;
    readonly notaryId: u32;
  }

  /** @name FrameSupportPalletId (303) */
  interface FrameSupportPalletId extends U8aFixed {}

  /** @name PalletChainTransferError (304) */
  interface PalletChainTransferError extends Enum {
    readonly isMaxBlockTransfersExceeded: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientNotarizedFunds: boolean;
    readonly isInvalidOrDuplicatedLocalchainTransfer: boolean;
    readonly isNotebookIncludesExpiredLocalchainTransfer: boolean;
    readonly isInvalidNotaryUsedForTransfer: boolean;
    readonly isNotaryLocked: boolean;
    readonly type: 'MaxBlockTransfersExceeded' | 'InsufficientFunds' | 'InsufficientNotarizedFunds' | 'InvalidOrDuplicatedLocalchainTransfer' | 'NotebookIncludesExpiredLocalchainTransfer' | 'InvalidNotaryUsedForTransfer' | 'NotaryLocked';
  }

  /** @name UlxPrimitivesNotaryNotaryNotebookVoteDigestDetails (309) */
  interface UlxPrimitivesNotaryNotaryNotebookVoteDigestDetails extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u32>;
    readonly blockVotesCount: Compact<u32>;
    readonly blockVotingPower: Compact<u128>;
  }

  /** @name UlxPrimitivesDigestsBlockVoteDigest (311) */
  interface UlxPrimitivesDigestsBlockVoteDigest extends Struct {
    readonly votingPower: Compact<u128>;
    readonly votesCount: Compact<u32>;
  }

  /** @name PalletBlockSealSpecError (315) */
  interface PalletBlockSealSpecError extends Enum {
    readonly isMaxNotebooksAtTickExceeded: boolean;
    readonly type: 'MaxNotebooksAtTickExceeded';
  }

  /** @name PalletDataDomainError (318) */
  interface PalletDataDomainError extends Enum {
    readonly isDomainNotRegistered: boolean;
    readonly isNotDomainOwner: boolean;
    readonly isFailedToAddToAddressHistory: boolean;
    readonly isFailedToAddExpiringDomain: boolean;
    readonly isAccountDecodingError: boolean;
    readonly type: 'DomainNotRegistered' | 'NotDomainOwner' | 'FailedToAddToAddressHistory' | 'FailedToAddExpiringDomain' | 'AccountDecodingError';
  }

  /** @name PalletPriceIndexError (319) */
  interface PalletPriceIndexError extends Enum {
    readonly isNotAuthorizedOperator: boolean;
    readonly isMissingValue: boolean;
    readonly isPricesTooOld: boolean;
    readonly isMaxPriceChangePerTickExceeded: boolean;
    readonly type: 'NotAuthorizedOperator' | 'MissingValue' | 'PricesTooOld' | 'MaxPriceChangePerTickExceeded';
  }

  /** @name SpCoreCryptoKeyTypeId (324) */
  interface SpCoreCryptoKeyTypeId extends U8aFixed {}

  /** @name PalletSessionError (325) */
  interface PalletSessionError extends Enum {
    readonly isInvalidProof: boolean;
    readonly isNoAssociatedValidatorId: boolean;
    readonly isDuplicatedKey: boolean;
    readonly isNoKeys: boolean;
    readonly isNoAccount: boolean;
    readonly type: 'InvalidProof' | 'NoAssociatedValidatorId' | 'DuplicatedKey' | 'NoKeys' | 'NoAccount';
  }

  /** @name UlxPrimitivesProvidersBlockSealerInfo (326) */
  interface UlxPrimitivesProvidersBlockSealerInfo extends Struct {
    readonly blockAuthorAccountId: AccountId32;
    readonly blockVoteRewardsAccount: Option<AccountId32>;
  }

  /** @name UlxPrimitivesDigestsParentVotingKeyDigest (327) */
  interface UlxPrimitivesDigestsParentVotingKeyDigest extends Struct {
    readonly parentVotingKey: Option<H256>;
  }

  /** @name PalletBlockSealError (328) */
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

  /** @name PalletBlockRewardsError (330) */
  type PalletBlockRewardsError = Null;

  /** @name PalletGrandpaStoredState (331) */
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

  /** @name PalletGrandpaStoredPendingChange (332) */
  interface PalletGrandpaStoredPendingChange extends Struct {
    readonly scheduledAt: u32;
    readonly delay: u32;
    readonly nextAuthorities: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    readonly forced: Option<u32>;
  }

  /** @name PalletGrandpaError (335) */
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

  /** @name SpStakingOffenceOffenceDetails (336) */
  interface SpStakingOffenceOffenceDetails extends Struct {
    readonly offender: ITuple<[AccountId32, PalletMiningSlotMinerHistory]>;
    readonly reporters: Vec<AccountId32>;
  }

  /** @name PalletMiningSlotMinerHistory (338) */
  interface PalletMiningSlotMinerHistory extends Struct {
    readonly authorityIndex: u32;
  }

  /** @name PalletMintError (343) */
  interface PalletMintError extends Enum {
    readonly isTooManyPendingMints: boolean;
    readonly type: 'TooManyPendingMints';
  }

  /** @name PalletBalancesBalanceLock (345) */
  interface PalletBalancesBalanceLock extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
    readonly reasons: PalletBalancesReasons;
  }

  /** @name PalletBalancesReasons (346) */
  interface PalletBalancesReasons extends Enum {
    readonly isFee: boolean;
    readonly isMisc: boolean;
    readonly isAll: boolean;
    readonly type: 'Fee' | 'Misc' | 'All';
  }

  /** @name PalletBalancesReserveData (349) */
  interface PalletBalancesReserveData extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
  }

  /** @name PalletBalancesIdAmountRuntimeHoldReason (352) */
  interface PalletBalancesIdAmountRuntimeHoldReason extends Struct {
    readonly id: UlxNodeRuntimeRuntimeHoldReason;
    readonly amount: u128;
  }

  /** @name UlxNodeRuntimeRuntimeHoldReason (353) */
  interface UlxNodeRuntimeRuntimeHoldReason extends Enum {
    readonly isMiningSlot: boolean;
    readonly asMiningSlot: PalletMiningSlotHoldReason;
    readonly isVaults: boolean;
    readonly asVaults: PalletVaultsHoldReason;
    readonly isBonds: boolean;
    readonly asBonds: PalletBondHoldReason;
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsHoldReason;
    readonly type: 'MiningSlot' | 'Vaults' | 'Bonds' | 'BlockRewards';
  }

  /** @name PalletMiningSlotHoldReason (354) */
  interface PalletMiningSlotHoldReason extends Enum {
    readonly isRegisterAsMiner: boolean;
    readonly type: 'RegisterAsMiner';
  }

  /** @name PalletVaultsHoldReason (355) */
  interface PalletVaultsHoldReason extends Enum {
    readonly isEnterVault: boolean;
    readonly isBondFee: boolean;
    readonly type: 'EnterVault' | 'BondFee';
  }

  /** @name PalletBondHoldReason (356) */
  interface PalletBondHoldReason extends Enum {
    readonly isUnlockingBitcoin: boolean;
    readonly type: 'UnlockingBitcoin';
  }

  /** @name PalletBlockRewardsHoldReason (357) */
  interface PalletBlockRewardsHoldReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletBalancesIdAmountRuntimeFreezeReason (360) */
  interface PalletBalancesIdAmountRuntimeFreezeReason extends Struct {
    readonly id: UlxNodeRuntimeRuntimeFreezeReason;
    readonly amount: u128;
  }

  /** @name UlxNodeRuntimeRuntimeFreezeReason (361) */
  interface UlxNodeRuntimeRuntimeFreezeReason extends Enum {
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsFreezeReason;
    readonly type: 'BlockRewards';
  }

  /** @name PalletBlockRewardsFreezeReason (362) */
  interface PalletBlockRewardsFreezeReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletBalancesError (364) */
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
    readonly isIssuanceDeactivated: boolean;
    readonly isDeltaZero: boolean;
    readonly type: 'VestingBalance' | 'LiquidityRestrictions' | 'InsufficientBalance' | 'ExistentialDeposit' | 'Expendability' | 'ExistingVestingSchedule' | 'DeadAccount' | 'TooManyReserves' | 'TooManyHolds' | 'TooManyFreezes' | 'IssuanceDeactivated' | 'DeltaZero';
  }

  /** @name PalletTxPauseError (366) */
  interface PalletTxPauseError extends Enum {
    readonly isIsPaused: boolean;
    readonly isIsUnpaused: boolean;
    readonly isUnpausable: boolean;
    readonly isNotFound: boolean;
    readonly type: 'IsPaused' | 'IsUnpaused' | 'Unpausable' | 'NotFound';
  }

  /** @name PalletTransactionPaymentReleases (367) */
  interface PalletTransactionPaymentReleases extends Enum {
    readonly isV1Ancient: boolean;
    readonly isV2: boolean;
    readonly type: 'V1Ancient' | 'V2';
  }

  /** @name PalletSudoError (368) */
  interface PalletSudoError extends Enum {
    readonly isRequireSudo: boolean;
    readonly type: 'RequireSudo';
  }

  /** @name FrameSystemExtensionsCheckNonZeroSender (371) */
  type FrameSystemExtensionsCheckNonZeroSender = Null;

  /** @name FrameSystemExtensionsCheckSpecVersion (372) */
  type FrameSystemExtensionsCheckSpecVersion = Null;

  /** @name FrameSystemExtensionsCheckTxVersion (373) */
  type FrameSystemExtensionsCheckTxVersion = Null;

  /** @name FrameSystemExtensionsCheckGenesis (374) */
  type FrameSystemExtensionsCheckGenesis = Null;

  /** @name FrameSystemExtensionsCheckNonce (377) */
  interface FrameSystemExtensionsCheckNonce extends Compact<u32> {}

  /** @name FrameSystemExtensionsCheckWeight (378) */
  type FrameSystemExtensionsCheckWeight = Null;

  /** @name PalletTransactionPaymentChargeTransactionPayment (379) */
  interface PalletTransactionPaymentChargeTransactionPayment extends Compact<u128> {}

  /** @name UlxNodeRuntimeRuntime (380) */
  type UlxNodeRuntimeRuntime = Null;

} // declare module
