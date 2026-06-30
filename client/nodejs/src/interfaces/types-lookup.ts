// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/types/lookup';

import type {
  BTreeMap,
  Bytes,
  Compact,
  Enum,
  Null,
  Option,
  Result,
  Struct,
  Text,
  U256,
  U8aFixed,
  Vec,
  bool,
  i128,
  i16,
  i32,
  u128,
  u16,
  u32,
  u64,
  u8,
} from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type {
  AccountId32,
  Call,
  H160,
  H256,
  MultiAddress,
  Permill,
} from '@polkadot/types/interfaces/runtime';
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
      readonly dispatchInfo: FrameSystemDispatchEventInfo;
    } & Struct;
    readonly isExtrinsicFailed: boolean;
    readonly asExtrinsicFailed: {
      readonly dispatchError: SpRuntimeDispatchError;
      readonly dispatchInfo: FrameSystemDispatchEventInfo;
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
    readonly isRejectedInvalidAuthorizedUpgrade: boolean;
    readonly asRejectedInvalidAuthorizedUpgrade: {
      readonly codeHash: H256;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type:
      | 'ExtrinsicSuccess'
      | 'ExtrinsicFailed'
      | 'CodeUpdated'
      | 'NewAccount'
      | 'KilledAccount'
      | 'Remarked'
      | 'UpgradeAuthorized'
      | 'RejectedInvalidAuthorizedUpgrade';
  }

  /** @name FrameSystemDispatchEventInfo (23) */
  interface FrameSystemDispatchEventInfo extends Struct {
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
    readonly isTrie: boolean;
    readonly asTrie: SpRuntimeProvingTrieTrieError;
    readonly type:
      | 'Other'
      | 'CannotLookup'
      | 'BadOrigin'
      | 'Module'
      | 'ConsumerRemaining'
      | 'NoProviders'
      | 'TooManyConsumers'
      | 'Token'
      | 'Arithmetic'
      | 'Transactional'
      | 'Exhausted'
      | 'Corruption'
      | 'Unavailable'
      | 'RootNotAllowed'
      | 'Trie';
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
    readonly type:
      | 'FundsUnavailable'
      | 'OnlyProvider'
      | 'BelowMinimum'
      | 'CannotCreate'
      | 'UnknownAsset'
      | 'Frozen'
      | 'Unsupported'
      | 'CannotCreateHold'
      | 'NotExpendable'
      | 'Blocked';
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

  /** @name SpRuntimeProvingTrieTrieError (31) */
  interface SpRuntimeProvingTrieTrieError extends Enum {
    readonly isInvalidStateRoot: boolean;
    readonly isIncompleteDatabase: boolean;
    readonly isValueAtIncompleteKey: boolean;
    readonly isDecoderError: boolean;
    readonly isInvalidHash: boolean;
    readonly isDuplicateKey: boolean;
    readonly isExtraneousNode: boolean;
    readonly isExtraneousValue: boolean;
    readonly isExtraneousHashReference: boolean;
    readonly isInvalidChildReference: boolean;
    readonly isValueMismatch: boolean;
    readonly isIncompleteProof: boolean;
    readonly isRootMismatch: boolean;
    readonly isDecodeError: boolean;
    readonly type:
      | 'InvalidStateRoot'
      | 'IncompleteDatabase'
      | 'ValueAtIncompleteKey'
      | 'DecoderError'
      | 'InvalidHash'
      | 'DuplicateKey'
      | 'ExtraneousNode'
      | 'ExtraneousValue'
      | 'ExtraneousHashReference'
      | 'InvalidChildReference'
      | 'ValueMismatch'
      | 'IncompleteProof'
      | 'RootMismatch'
      | 'DecodeError';
  }

  /** @name PalletDigestsEvent (32) */
  type PalletDigestsEvent = Null;

  /** @name PalletMultisigEvent (33) */
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
    readonly isDepositPoked: boolean;
    readonly asDepositPoked: {
      readonly who: AccountId32;
      readonly callHash: U8aFixed;
      readonly oldDeposit: u128;
      readonly newDeposit: u128;
    } & Struct;
    readonly type:
      | 'NewMultisig'
      | 'MultisigApproval'
      | 'MultisigExecuted'
      | 'MultisigCancelled'
      | 'DepositPoked';
  }

  /** @name PalletMultisigTimepoint (34) */
  interface PalletMultisigTimepoint extends Struct {
    readonly height: u32;
    readonly index: u32;
  }

  /** @name PalletProxyEvent (37) */
  interface PalletProxyEvent extends Enum {
    readonly isProxyExecuted: boolean;
    readonly asProxyExecuted: {
      readonly result: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly isPureCreated: boolean;
    readonly asPureCreated: {
      readonly pure: AccountId32;
      readonly who: AccountId32;
      readonly proxyType: ArgonRuntimeProxyType;
      readonly disambiguationIndex: u16;
      readonly at: u32;
      readonly extrinsicIndex: u32;
    } & Struct;
    readonly isPureKilled: boolean;
    readonly asPureKilled: {
      readonly pure: AccountId32;
      readonly spawner: AccountId32;
      readonly proxyType: ArgonRuntimeProxyType;
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
      readonly proxyType: ArgonRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isProxyRemoved: boolean;
    readonly asProxyRemoved: {
      readonly delegator: AccountId32;
      readonly delegatee: AccountId32;
      readonly proxyType: ArgonRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isDepositPoked: boolean;
    readonly asDepositPoked: {
      readonly who: AccountId32;
      readonly kind: PalletProxyDepositKind;
      readonly oldDeposit: u128;
      readonly newDeposit: u128;
    } & Struct;
    readonly type:
      | 'ProxyExecuted'
      | 'PureCreated'
      | 'PureKilled'
      | 'Announced'
      | 'ProxyAdded'
      | 'ProxyRemoved'
      | 'DepositPoked';
  }

  /** @name ArgonRuntimeProxyType (38) */
  interface ArgonRuntimeProxyType extends Enum {
    readonly isAny: boolean;
    readonly isNonTransfer: boolean;
    readonly isPriceIndex: boolean;
    readonly isMiningBid: boolean;
    readonly isMiningBidRealPaysFee: boolean;
    readonly isBitcoin: boolean;
    readonly isVaultAdmin: boolean;
    readonly isVaultDelegate: boolean;
    readonly type:
      | 'Any'
      | 'NonTransfer'
      | 'PriceIndex'
      | 'MiningBid'
      | 'MiningBidRealPaysFee'
      | 'Bitcoin'
      | 'VaultAdmin'
      | 'VaultDelegate';
  }

  /** @name PalletProxyDepositKind (40) */
  interface PalletProxyDepositKind extends Enum {
    readonly isProxies: boolean;
    readonly isAnnouncements: boolean;
    readonly type: 'Proxies' | 'Announcements';
  }

  /** @name PalletMiningSlotEvent (41) */
  interface PalletMiningSlotEvent extends Enum {
    readonly isNewMiners: boolean;
    readonly asNewMiners: {
      readonly newMiners: Vec<ArgonPrimitivesBlockSealMiningRegistration>;
      readonly releasedMiners: u32;
      readonly frameId: u64;
    } & Struct;
    readonly isSlotBidderAdded: boolean;
    readonly asSlotBidderAdded: {
      readonly accountId: AccountId32;
      readonly bidAmount: u128;
      readonly index: u32;
    } & Struct;
    readonly isSlotBidderDropped: boolean;
    readonly asSlotBidderDropped: {
      readonly accountId: AccountId32;
      readonly preservedArgonotHold: bool;
    } & Struct;
    readonly isReleaseMinerSeatError: boolean;
    readonly asReleaseMinerSeatError: {
      readonly accountId: AccountId32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isMiningConfigurationUpdated: boolean;
    readonly asMiningConfigurationUpdated: {
      readonly ticksBeforeBidEndForVrfClose: u64;
      readonly ticksBetweenSlots: u64;
      readonly slotBiddingStartAfterTicks: u64;
    } & Struct;
    readonly isMiningBidsClosed: boolean;
    readonly asMiningBidsClosed: {
      readonly frameId: u64;
    } & Struct;
    readonly isReleaseBidError: boolean;
    readonly asReleaseBidError: {
      readonly accountId: AccountId32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type:
      | 'NewMiners'
      | 'SlotBidderAdded'
      | 'SlotBidderDropped'
      | 'ReleaseMinerSeatError'
      | 'MiningConfigurationUpdated'
      | 'MiningBidsClosed'
      | 'ReleaseBidError';
  }

  /** @name ArgonPrimitivesBlockSealMiningRegistration (43) */
  interface ArgonPrimitivesBlockSealMiningRegistration extends Struct {
    readonly accountId: AccountId32;
    readonly externalFundingAccount: Option<AccountId32>;
    readonly bid: Compact<u128>;
    readonly argonots: Compact<u128>;
    readonly authorityKeys: ArgonRuntimeSessionKeys;
    readonly startingFrameId: Compact<u64>;
    readonly bidAtTick: Compact<u64>;
  }

  /** @name ArgonRuntimeSessionKeys (44) */
  interface ArgonRuntimeSessionKeys extends Struct {
    readonly grandpa: SpConsensusGrandpaAppPublic;
    readonly blockSealAuthority: ArgonPrimitivesBlockSealAppPublic;
  }

  /** @name SpConsensusGrandpaAppPublic (45) */
  interface SpConsensusGrandpaAppPublic extends U8aFixed {}

  /** @name ArgonPrimitivesBlockSealAppPublic (46) */
  interface ArgonPrimitivesBlockSealAppPublic extends U8aFixed {}

  /** @name PalletBitcoinUtxosEvent (50) */
  interface PalletBitcoinUtxosEvent extends Enum {
    readonly isUtxoVerified: boolean;
    readonly asUtxoVerified: {
      readonly utxoId: u64;
      readonly satoshisReceived: u64;
    } & Struct;
    readonly isUtxoRejected: boolean;
    readonly asUtxoRejected: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly rejectedReason: ArgonPrimitivesBitcoinBitcoinRejectedReason;
      readonly satoshisReceived: u64;
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
    readonly type:
      | 'UtxoVerified'
      | 'UtxoRejected'
      | 'UtxoSpent'
      | 'UtxoUnwatched'
      | 'UtxoSpentError'
      | 'UtxoVerifiedError'
      | 'UtxoRejectedError';
  }

  /** @name ArgonPrimitivesBitcoinUtxoRef (51) */
  interface ArgonPrimitivesBitcoinUtxoRef extends Struct {
    readonly txid: ArgonPrimitivesBitcoinH256Le;
    readonly outputIndex: Compact<u32>;
  }

  /** @name ArgonPrimitivesBitcoinH256Le (52) */
  interface ArgonPrimitivesBitcoinH256Le extends U8aFixed {}

  /** @name ArgonPrimitivesBitcoinBitcoinRejectedReason (54) */
  interface ArgonPrimitivesBitcoinBitcoinRejectedReason extends Enum {
    readonly isSatoshisOutsideAcceptedRange: boolean;
    readonly isSpent: boolean;
    readonly isVerificationExpired: boolean;
    readonly isAlreadyVerified: boolean;
    readonly type:
      | 'SatoshisOutsideAcceptedRange'
      | 'Spent'
      | 'VerificationExpired'
      | 'AlreadyVerified';
  }

  /** @name PalletVaultsEvent (55) */
  interface PalletVaultsEvent extends Enum {
    readonly isVaultCreated: boolean;
    readonly asVaultCreated: {
      readonly vaultId: u32;
      readonly securitization: u128;
      readonly securitizationRatio: u128;
      readonly operatorAccountId: AccountId32;
      readonly openedTick: u64;
    } & Struct;
    readonly isVaultModified: boolean;
    readonly asVaultModified: {
      readonly vaultId: u32;
      readonly securitization: u128;
      readonly securitizationTarget: u128;
      readonly securitizationRatio: u128;
    } & Struct;
    readonly isVaultTermsChangeScheduled: boolean;
    readonly asVaultTermsChangeScheduled: {
      readonly vaultId: u32;
      readonly changeTick: u64;
    } & Struct;
    readonly isVaultTermsChanged: boolean;
    readonly asVaultTermsChanged: {
      readonly vaultId: u32;
    } & Struct;
    readonly isVaultClosed: boolean;
    readonly asVaultClosed: {
      readonly vaultId: u32;
      readonly securitizationRemaining: u128;
      readonly securitizationReleased: u128;
    } & Struct;
    readonly isVaultBitcoinXpubChange: boolean;
    readonly asVaultBitcoinXpubChange: {
      readonly vaultId: u32;
    } & Struct;
    readonly isVaultRevenueUncollected: boolean;
    readonly asVaultRevenueUncollected: {
      readonly vaultId: u32;
      readonly frameId: u64;
      readonly amount: u128;
    } & Struct;
    readonly isVaultCollected: boolean;
    readonly asVaultCollected: {
      readonly vaultId: u32;
      readonly revenue: u128;
    } & Struct;
    readonly isFundsLocked: boolean;
    readonly asFundsLocked: {
      readonly vaultId: u32;
      readonly locker: AccountId32;
      readonly liquidityPromised: u128;
      readonly isRatchet: bool;
      readonly feeRevenue: u128;
      readonly didUseFeeCoupon: bool;
    } & Struct;
    readonly isFundLockCanceled: boolean;
    readonly asFundLockCanceled: {
      readonly vaultId: u32;
      readonly amount: u128;
    } & Struct;
    readonly isFundsScheduledForRelease: boolean;
    readonly asFundsScheduledForRelease: {
      readonly vaultId: u32;
      readonly securitization: u128;
      readonly releaseHeight: u64;
    } & Struct;
    readonly isLostBitcoinCompensated: boolean;
    readonly asLostBitcoinCompensated: {
      readonly vaultId: u32;
      readonly beneficiary: AccountId32;
      readonly toBeneficiary: u128;
      readonly burned: u128;
    } & Struct;
    readonly isFundsReleased: boolean;
    readonly asFundsReleased: {
      readonly vaultId: u32;
      readonly securitization: u128;
    } & Struct;
    readonly isFundsReleasedError: boolean;
    readonly asFundsReleasedError: {
      readonly vaultId: u32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isTreasuryRecordingError: boolean;
    readonly asTreasuryRecordingError: {
      readonly vaultId: u32;
      readonly frameId: u64;
      readonly vaultEarnings: u128;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isCommittedArgonotsSet: boolean;
    readonly asCommittedArgonotsSet: {
      readonly vaultId: u32;
      readonly operatorAccountId: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly type:
      | 'VaultCreated'
      | 'VaultModified'
      | 'VaultTermsChangeScheduled'
      | 'VaultTermsChanged'
      | 'VaultClosed'
      | 'VaultBitcoinXpubChange'
      | 'VaultRevenueUncollected'
      | 'VaultCollected'
      | 'FundsLocked'
      | 'FundLockCanceled'
      | 'FundsScheduledForRelease'
      | 'LostBitcoinCompensated'
      | 'FundsReleased'
      | 'FundsReleasedError'
      | 'TreasuryRecordingError'
      | 'CommittedArgonotsSet';
  }

  /** @name PalletBitcoinLocksEvent (57) */
  interface PalletBitcoinLocksEvent extends Enum {
    readonly isBitcoinLockCreated: boolean;
    readonly asBitcoinLockCreated: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly liquidityPromised: u128;
      readonly securitization: u128;
      readonly lockedTargetPrice: u128;
      readonly accountId: AccountId32;
      readonly securityFee: u128;
    } & Struct;
    readonly isBitcoinLockRatcheted: boolean;
    readonly asBitcoinLockRatcheted: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly liquidityPromised: u128;
      readonly oldTargetPrice: u128;
      readonly securityFee: u128;
      readonly newTargetPrice: u128;
      readonly amountBurned: u128;
      readonly accountId: AccountId32;
    } & Struct;
    readonly isBitcoinLockBurned: boolean;
    readonly asBitcoinLockBurned: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly wasUtxoSpent: bool;
    } & Struct;
    readonly isBitcoinUtxoCosignRequested: boolean;
    readonly asBitcoinUtxoCosignRequested: {
      readonly utxoId: u64;
      readonly vaultId: u32;
    } & Struct;
    readonly isBitcoinUtxoCosigned: boolean;
    readonly asBitcoinUtxoCosigned: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly signature: Bytes;
    } & Struct;
    readonly isBitcoinSpentAfterRelease: boolean;
    readonly asBitcoinSpentAfterRelease: {
      readonly utxoId: u64;
      readonly vaultId: u32;
    } & Struct;
    readonly isBitcoinCosignPastDue: boolean;
    readonly asBitcoinCosignPastDue: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly compensationAmount: u128;
      readonly compensatedAccountId: AccountId32;
    } & Struct;
    readonly isCosignOverdueError: boolean;
    readonly asCosignOverdueError: {
      readonly utxoId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isLockExpirationError: boolean;
    readonly asLockExpirationError: {
      readonly utxoId: u64;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isOrphanedUtxoReceived: boolean;
    readonly asOrphanedUtxoReceived: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly vaultId: u32;
      readonly satoshis: u64;
    } & Struct;
    readonly isOrphanedUtxoReleaseRequested: boolean;
    readonly asOrphanedUtxoReleaseRequested: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly vaultId: u32;
      readonly accountId: AccountId32;
    } & Struct;
    readonly isOrphanedUtxoCosigned: boolean;
    readonly asOrphanedUtxoCosigned: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly vaultId: u32;
      readonly accountId: AccountId32;
      readonly signature: Bytes;
    } & Struct;
    readonly isUtxoFundedFromCandidate: boolean;
    readonly asUtxoFundedFromCandidate: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly vaultId: u32;
      readonly accountId: AccountId32;
    } & Struct;
    readonly isSecuritizationIncreased: boolean;
    readonly asSecuritizationIncreased: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly newSatoshis: u64;
      readonly accountId: AccountId32;
    } & Struct;
    readonly type:
      | 'BitcoinLockCreated'
      | 'BitcoinLockRatcheted'
      | 'BitcoinLockBurned'
      | 'BitcoinUtxoCosignRequested'
      | 'BitcoinUtxoCosigned'
      | 'BitcoinSpentAfterRelease'
      | 'BitcoinCosignPastDue'
      | 'CosignOverdueError'
      | 'LockExpirationError'
      | 'OrphanedUtxoReceived'
      | 'OrphanedUtxoReleaseRequested'
      | 'OrphanedUtxoCosigned'
      | 'UtxoFundedFromCandidate'
      | 'SecuritizationIncreased';
  }

  /** @name PalletNotariesEvent (60) */
  interface PalletNotariesEvent extends Enum {
    readonly isNotaryProposed: boolean;
    readonly asNotaryProposed: {
      readonly operatorAccount: AccountId32;
      readonly meta: ArgonPrimitivesNotaryNotaryMeta;
      readonly expires: u32;
    } & Struct;
    readonly isNotaryActivated: boolean;
    readonly asNotaryActivated: {
      readonly notary: ArgonPrimitivesNotaryNotaryRecord;
    } & Struct;
    readonly isNotaryMetaUpdateQueued: boolean;
    readonly asNotaryMetaUpdateQueued: {
      readonly notaryId: u32;
      readonly meta: ArgonPrimitivesNotaryNotaryMeta;
      readonly effectiveTick: u64;
    } & Struct;
    readonly isNotaryMetaUpdated: boolean;
    readonly asNotaryMetaUpdated: {
      readonly notaryId: u32;
      readonly meta: ArgonPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly isNotaryMetaUpdateError: boolean;
    readonly asNotaryMetaUpdateError: {
      readonly notaryId: u32;
      readonly error: SpRuntimeDispatchError;
      readonly meta: ArgonPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly type:
      | 'NotaryProposed'
      | 'NotaryActivated'
      | 'NotaryMetaUpdateQueued'
      | 'NotaryMetaUpdated'
      | 'NotaryMetaUpdateError';
  }

  /** @name ArgonPrimitivesNotaryNotaryMeta (61) */
  interface ArgonPrimitivesNotaryNotaryMeta extends Struct {
    readonly name: Bytes;
    readonly public: U8aFixed;
    readonly hosts: Vec<Bytes>;
  }

  /** @name ArgonPrimitivesNotaryNotaryRecord (68) */
  interface ArgonPrimitivesNotaryNotaryRecord extends Struct {
    readonly notaryId: Compact<u32>;
    readonly operatorAccountId: AccountId32;
    readonly activatedBlock: Compact<u32>;
    readonly metaUpdatedBlock: Compact<u32>;
    readonly metaUpdatedTick: Compact<u64>;
    readonly meta: ArgonPrimitivesNotaryNotaryMeta;
  }

  /** @name PalletNotebookEvent (69) */
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
      readonly notebookHash: H256;
      readonly firstFailureReason: ArgonNotaryAuditErrorVerifyError;
    } & Struct;
    readonly isNotebookReadyForReprocess: boolean;
    readonly asNotebookReadyForReprocess: {
      readonly notaryId: u32;
      readonly notebookNumber: u32;
    } & Struct;
    readonly type: 'NotebookSubmitted' | 'NotebookAuditFailure' | 'NotebookReadyForReprocess';
  }

  /** @name ArgonNotaryAuditErrorVerifyError (70) */
  interface ArgonNotaryAuditErrorVerifyError extends Enum {
    readonly isMissingAccountOrigin: boolean;
    readonly asMissingAccountOrigin: {
      readonly accountId: AccountId32;
      readonly accountType: ArgonPrimitivesAccountAccountType;
    } & Struct;
    readonly isHistoryLookupError: boolean;
    readonly asHistoryLookupError: {
      readonly source: ArgonNotaryAuditAccountHistoryLookupError;
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
      readonly message: Bytes;
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
    readonly isInvalidNotebookVersion: boolean;
    readonly isDuplicateChainTransfer: boolean;
    readonly isDuplicatedAccountOriginUid: boolean;
    readonly isInvalidNotarySignature: boolean;
    readonly isInvalidSecretProvided: boolean;
    readonly isNotebookTooOld: boolean;
    readonly isCatchupNotebooksMissing: boolean;
    readonly isDecodeError: boolean;
    readonly isAccountChannelHoldDoesntExist: boolean;
    readonly isAccountAlreadyHasChannelHold: boolean;
    readonly isChannelHoldNotReadyForClaim: boolean;
    readonly asChannelHoldNotReadyForClaim: {
      readonly currentTick: u64;
      readonly claimTick: u64;
    } & Struct;
    readonly isAccountLocked: boolean;
    readonly isMissingChannelHoldNote: boolean;
    readonly isInvalidChannelHoldNote: boolean;
    readonly isInvalidChannelHoldClaimers: boolean;
    readonly isChannelHoldNoteBelowMinimum: boolean;
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
    readonly isInvalidBlockVoteTick: boolean;
    readonly asInvalidBlockVoteTick: {
      readonly tick: u64;
      readonly notebookTick: u64;
    } & Struct;
    readonly isInvalidDefaultBlockVote: boolean;
    readonly isInvalidDefaultBlockVoteAuthor: boolean;
    readonly asInvalidDefaultBlockVoteAuthor: {
      readonly author: AccountId32;
      readonly expected: AccountId32;
    } & Struct;
    readonly isNoDefaultBlockVote: boolean;
    readonly type:
      | 'MissingAccountOrigin'
      | 'HistoryLookupError'
      | 'InvalidAccountChangelist'
      | 'InvalidChainTransfersList'
      | 'InvalidBalanceChangeRoot'
      | 'InvalidHeaderTaxRecorded'
      | 'InvalidPreviousNonce'
      | 'InvalidPreviousBalance'
      | 'InvalidPreviousAccountOrigin'
      | 'InvalidPreviousBalanceChangeNotebook'
      | 'InvalidBalanceChange'
      | 'InvalidBalanceChangeSignature'
      | 'InvalidNoteRecipients'
      | 'BalanceChangeError'
      | 'InvalidNetBalanceChangeset'
      | 'InsufficientBalance'
      | 'ExceededMaxBalance'
      | 'BalanceChangeMismatch'
      | 'BalanceChangeNotNetZero'
      | 'InvalidDomainLeaseAllocation'
      | 'TaxBalanceChangeNotNetZero'
      | 'MissingBalanceProof'
      | 'InvalidPreviousBalanceProof'
      | 'InvalidNotebookHash'
      | 'InvalidNotebookHeaderHash'
      | 'InvalidNotebookVersion'
      | 'DuplicateChainTransfer'
      | 'DuplicatedAccountOriginUid'
      | 'InvalidNotarySignature'
      | 'InvalidSecretProvided'
      | 'NotebookTooOld'
      | 'CatchupNotebooksMissing'
      | 'DecodeError'
      | 'AccountChannelHoldDoesntExist'
      | 'AccountAlreadyHasChannelHold'
      | 'ChannelHoldNotReadyForClaim'
      | 'AccountLocked'
      | 'MissingChannelHoldNote'
      | 'InvalidChannelHoldNote'
      | 'InvalidChannelHoldClaimers'
      | 'ChannelHoldNoteBelowMinimum'
      | 'InvalidTaxNoteAccount'
      | 'InvalidTaxOperation'
      | 'InsufficientTaxIncluded'
      | 'InsufficientBlockVoteTax'
      | 'IneligibleTaxVoter'
      | 'BlockVoteInvalidSignature'
      | 'InvalidBlockVoteAllocation'
      | 'InvalidBlockVoteRoot'
      | 'InvalidBlockVotesCount'
      | 'InvalidBlockVotingPower'
      | 'InvalidBlockVoteList'
      | 'InvalidComputeProof'
      | 'InvalidBlockVoteSource'
      | 'InsufficientBlockVoteMinimum'
      | 'InvalidBlockVoteTick'
      | 'InvalidDefaultBlockVote'
      | 'InvalidDefaultBlockVoteAuthor'
      | 'NoDefaultBlockVote';
  }

  /** @name ArgonPrimitivesAccountAccountType (71) */
  interface ArgonPrimitivesAccountAccountType extends Enum {
    readonly isTax: boolean;
    readonly isDeposit: boolean;
    readonly type: 'Tax' | 'Deposit';
  }

  /** @name ArgonNotaryAuditAccountHistoryLookupError (72) */
  interface ArgonNotaryAuditAccountHistoryLookupError extends Enum {
    readonly isRootNotFound: boolean;
    readonly isLastChangeNotFound: boolean;
    readonly isInvalidTransferToLocalchain: boolean;
    readonly isBlockSpecificationNotFound: boolean;
    readonly type:
      | 'RootNotFound'
      | 'LastChangeNotFound'
      | 'InvalidTransferToLocalchain'
      | 'BlockSpecificationNotFound';
  }

  /** @name PalletLocalchainTransferEvent (76) */
  interface PalletLocalchainTransferEvent extends Enum {
    readonly isTransferToLocalchain: boolean;
    readonly asTransferToLocalchain: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly transferId: u32;
      readonly notaryId: u32;
      readonly expirationTick: u64;
    } & Struct;
    readonly isTransferToLocalchainExpired: boolean;
    readonly asTransferToLocalchainExpired: {
      readonly accountId: AccountId32;
      readonly transferId: u32;
      readonly notaryId: u32;
    } & Struct;
    readonly isTransferFromLocalchain: boolean;
    readonly asTransferFromLocalchain: {
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly notaryId: u32;
    } & Struct;
    readonly isTransferFromLocalchainError: boolean;
    readonly asTransferFromLocalchainError: {
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
    readonly isPossibleInvalidLocalchainTransferAllowed: boolean;
    readonly asPossibleInvalidLocalchainTransferAllowed: {
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
    readonly type:
      | 'TransferToLocalchain'
      | 'TransferToLocalchainExpired'
      | 'TransferFromLocalchain'
      | 'TransferFromLocalchainError'
      | 'TransferToLocalchainRefundError'
      | 'PossibleInvalidLocalchainTransferAllowed'
      | 'TaxationError';
  }

  /** @name PalletBlockSealSpecEvent (77) */
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

  /** @name PalletDomainsEvent (78) */
  interface PalletDomainsEvent extends Enum {
    readonly isZoneRecordUpdated: boolean;
    readonly asZoneRecordUpdated: {
      readonly domainHash: H256;
      readonly zoneRecord: ArgonPrimitivesDomainZoneRecord;
    } & Struct;
    readonly isDomainRegistered: boolean;
    readonly asDomainRegistered: {
      readonly domainHash: H256;
      readonly registration: PalletDomainsDomainRegistration;
    } & Struct;
    readonly isDomainRenewed: boolean;
    readonly asDomainRenewed: {
      readonly domainHash: H256;
    } & Struct;
    readonly isDomainExpired: boolean;
    readonly asDomainExpired: {
      readonly domainHash: H256;
    } & Struct;
    readonly isDomainRegistrationCanceled: boolean;
    readonly asDomainRegistrationCanceled: {
      readonly domainHash: H256;
      readonly registration: PalletDomainsDomainRegistration;
    } & Struct;
    readonly isDomainRegistrationError: boolean;
    readonly asDomainRegistrationError: {
      readonly domainHash: H256;
      readonly accountId: AccountId32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type:
      | 'ZoneRecordUpdated'
      | 'DomainRegistered'
      | 'DomainRenewed'
      | 'DomainExpired'
      | 'DomainRegistrationCanceled'
      | 'DomainRegistrationError';
  }

  /** @name ArgonPrimitivesDomainZoneRecord (79) */
  interface ArgonPrimitivesDomainZoneRecord extends Struct {
    readonly paymentAccount: AccountId32;
    readonly notaryId: u32;
    readonly versions: BTreeMap<ArgonPrimitivesDomainSemver, ArgonPrimitivesDomainVersionHost>;
  }

  /** @name ArgonPrimitivesDomainSemver (81) */
  interface ArgonPrimitivesDomainSemver extends Struct {
    readonly major: u32;
    readonly minor: u32;
    readonly patch: u32;
  }

  /** @name ArgonPrimitivesDomainVersionHost (82) */
  interface ArgonPrimitivesDomainVersionHost extends Struct {
    readonly datastoreId: Bytes;
    readonly host: Bytes;
  }

  /** @name PalletDomainsDomainRegistration (87) */
  interface PalletDomainsDomainRegistration extends Struct {
    readonly accountId: AccountId32;
    readonly registeredAtTick: u64;
  }

  /** @name PalletPriceIndexEvent (88) */
  interface PalletPriceIndexEvent extends Enum {
    readonly isNewIndex: boolean;
    readonly isOperatorChanged: boolean;
    readonly asOperatorChanged: {
      readonly operatorId: AccountId32;
    } & Struct;
    readonly type: 'NewIndex' | 'OperatorChanged';
  }

  /** @name PalletGrandpaEvent (89) */
  interface PalletGrandpaEvent extends Enum {
    readonly isNewAuthorities: boolean;
    readonly asNewAuthorities: {
      readonly authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    } & Struct;
    readonly isPaused: boolean;
    readonly isResumed: boolean;
    readonly type: 'NewAuthorities' | 'Paused' | 'Resumed';
  }

  /** @name PalletBlockRewardsEvent (92) */
  interface PalletBlockRewardsEvent extends Enum {
    readonly isRewardCreated: boolean;
    readonly asRewardCreated: {
      readonly rewards: Vec<ArgonPrimitivesBlockSealBlockPayout>;
    } & Struct;
    readonly isRewardCreateError: boolean;
    readonly asRewardCreateError: {
      readonly accountId: AccountId32;
      readonly argons: Option<u128>;
      readonly ownership: Option<u128>;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'RewardCreated' | 'RewardCreateError';
  }

  /** @name ArgonPrimitivesBlockSealBlockPayout (94) */
  interface ArgonPrimitivesBlockSealBlockPayout extends Struct {
    readonly accountId: AccountId32;
    readonly ownership: Compact<u128>;
    readonly argons: Compact<u128>;
    readonly rewardType: ArgonPrimitivesBlockSealBlockRewardType;
    readonly blockSealAuthority: Option<ArgonPrimitivesBlockSealAppPublic>;
  }

  /** @name ArgonPrimitivesBlockSealBlockRewardType (95) */
  interface ArgonPrimitivesBlockSealBlockRewardType extends Enum {
    readonly isMiner: boolean;
    readonly isVoter: boolean;
    readonly isProfitShare: boolean;
    readonly type: 'Miner' | 'Voter' | 'ProfitShare';
  }

  /** @name PalletMintEvent (98) */
  interface PalletMintEvent extends Enum {
    readonly isBitcoinMint: boolean;
    readonly asBitcoinMint: {
      readonly accountId: AccountId32;
      readonly utxoId: Option<u64>;
      readonly amount: u128;
    } & Struct;
    readonly isMiningMint: boolean;
    readonly asMiningMint: {
      readonly amount: u128;
      readonly perMiner: u128;
      readonly argonCpi: i128;
      readonly liquidity: u128;
    } & Struct;
    readonly isMintError: boolean;
    readonly asMintError: {
      readonly mintType: PalletMintMintType;
      readonly accountId: AccountId32;
      readonly utxoId: Option<u64>;
      readonly amount: u128;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type: 'BitcoinMint' | 'MiningMint' | 'MintError';
  }

  /** @name PalletMintMintType (101) */
  interface PalletMintMintType extends Enum {
    readonly isBitcoin: boolean;
    readonly isMining: boolean;
    readonly type: 'Bitcoin' | 'Mining';
  }

  /** @name PalletBalancesEvent (102) */
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
    readonly isMintedCredit: boolean;
    readonly asMintedCredit: {
      readonly amount: u128;
    } & Struct;
    readonly isBurned: boolean;
    readonly asBurned: {
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isBurnedDebt: boolean;
    readonly asBurnedDebt: {
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
    readonly isHeld: boolean;
    readonly asHeld: {
      readonly reason: ArgonRuntimeRuntimeHoldReason;
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isBurnedHeld: boolean;
    readonly asBurnedHeld: {
      readonly reason: ArgonRuntimeRuntimeHoldReason;
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isTransferOnHold: boolean;
    readonly asTransferOnHold: {
      readonly reason: ArgonRuntimeRuntimeHoldReason;
      readonly source: AccountId32;
      readonly dest: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isTransferAndHold: boolean;
    readonly asTransferAndHold: {
      readonly reason: ArgonRuntimeRuntimeHoldReason;
      readonly source: AccountId32;
      readonly dest: AccountId32;
      readonly transferred: u128;
    } & Struct;
    readonly isReleased: boolean;
    readonly asReleased: {
      readonly reason: ArgonRuntimeRuntimeHoldReason;
      readonly who: AccountId32;
      readonly amount: u128;
    } & Struct;
    readonly isUnexpected: boolean;
    readonly asUnexpected: PalletBalancesUnexpectedKind;
    readonly type:
      | 'Endowed'
      | 'DustLost'
      | 'Transfer'
      | 'BalanceSet'
      | 'Reserved'
      | 'Unreserved'
      | 'ReserveRepatriated'
      | 'Deposit'
      | 'Withdraw'
      | 'Slashed'
      | 'Minted'
      | 'MintedCredit'
      | 'Burned'
      | 'BurnedDebt'
      | 'Suspended'
      | 'Restored'
      | 'Upgraded'
      | 'Issued'
      | 'Rescinded'
      | 'Locked'
      | 'Unlocked'
      | 'Frozen'
      | 'Thawed'
      | 'TotalIssuanceForced'
      | 'Held'
      | 'BurnedHeld'
      | 'TransferOnHold'
      | 'TransferAndHold'
      | 'Released'
      | 'Unexpected';
  }

  /** @name FrameSupportTokensMiscBalanceStatus (103) */
  interface FrameSupportTokensMiscBalanceStatus extends Enum {
    readonly isFree: boolean;
    readonly isReserved: boolean;
    readonly type: 'Free' | 'Reserved';
  }

  /** @name ArgonRuntimeRuntimeHoldReason (104) */
  interface ArgonRuntimeRuntimeHoldReason extends Enum {
    readonly isMiningSlot: boolean;
    readonly asMiningSlot: PalletMiningSlotHoldReason;
    readonly isVaults: boolean;
    readonly asVaults: PalletVaultsHoldReason;
    readonly isBitcoinLocks: boolean;
    readonly asBitcoinLocks: PalletBitcoinLocksHoldReason;
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsHoldReason;
    readonly isTreasury: boolean;
    readonly asTreasury: PalletTreasuryHoldReason;
    readonly isCrosschainTransfer: boolean;
    readonly asCrosschainTransfer: PalletCrosschainTransferHoldReason;
    readonly type:
      | 'MiningSlot'
      | 'Vaults'
      | 'BitcoinLocks'
      | 'BlockRewards'
      | 'Treasury'
      | 'CrosschainTransfer';
  }

  /** @name PalletMiningSlotHoldReason (105) */
  interface PalletMiningSlotHoldReason extends Enum {
    readonly isRegisterAsMiner: boolean;
    readonly type: 'RegisterAsMiner';
  }

  /** @name PalletVaultsHoldReason (106) */
  interface PalletVaultsHoldReason extends Enum {
    readonly isEnterVault: boolean;
    readonly isObligationFee: boolean;
    readonly isPendingCollect: boolean;
    readonly type: 'EnterVault' | 'ObligationFee' | 'PendingCollect';
  }

  /** @name PalletBitcoinLocksHoldReason (107) */
  interface PalletBitcoinLocksHoldReason extends Enum {
    readonly isReleaseBitcoinLock: boolean;
    readonly type: 'ReleaseBitcoinLock';
  }

  /** @name PalletBlockRewardsHoldReason (108) */
  interface PalletBlockRewardsHoldReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletTreasuryHoldReason (109) */
  interface PalletTreasuryHoldReason extends Enum {
    readonly isContributedToTreasury: boolean;
    readonly type: 'ContributedToTreasury';
  }

  /** @name PalletCrosschainTransferHoldReason (110) */
  interface PalletCrosschainTransferHoldReason extends Enum {
    readonly isTransferOutMintingAuthorityTip: boolean;
    readonly isMintingAuthorityActivationRepayment: boolean;
    readonly type: 'TransferOutMintingAuthorityTip' | 'MintingAuthorityActivationRepayment';
  }

  /** @name PalletBalancesUnexpectedKind (111) */
  interface PalletBalancesUnexpectedKind extends Enum {
    readonly isBalanceUpdated: boolean;
    readonly isFailedToMutateAccount: boolean;
    readonly type: 'BalanceUpdated' | 'FailedToMutateAccount';
  }

  /** @name PalletTxPauseEvent (113) */
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

  /** @name PalletTransactionPaymentEvent (116) */
  interface PalletTransactionPaymentEvent extends Enum {
    readonly isTransactionFeePaid: boolean;
    readonly asTransactionFeePaid: {
      readonly who: AccountId32;
      readonly actualFee: u128;
      readonly tip: u128;
    } & Struct;
    readonly type: 'TransactionFeePaid';
  }

  /** @name PalletUtilityEvent (117) */
  interface PalletUtilityEvent extends Enum {
    readonly isBatchInterrupted: boolean;
    readonly asBatchInterrupted: {
      readonly index: u32;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isBatchCompleted: boolean;
    readonly isBatchCompletedWithErrors: boolean;
    readonly isItemCompleted: boolean;
    readonly isItemFailed: boolean;
    readonly asItemFailed: {
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly isDispatchedAs: boolean;
    readonly asDispatchedAs: {
      readonly result: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly isIfElseMainSuccess: boolean;
    readonly isIfElseFallbackCalled: boolean;
    readonly asIfElseFallbackCalled: {
      readonly mainError: SpRuntimeDispatchError;
    } & Struct;
    readonly type:
      | 'BatchInterrupted'
      | 'BatchCompleted'
      | 'BatchCompletedWithErrors'
      | 'ItemCompleted'
      | 'ItemFailed'
      | 'DispatchedAs'
      | 'IfElseMainSuccess'
      | 'IfElseFallbackCalled';
  }

  /** @name PalletSudoEvent (118) */
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

  /** @name PalletTreasuryEvent (119) */
  interface PalletTreasuryEvent extends Enum {
    readonly isCouldNotDistributeEarningsToBondLot: boolean;
    readonly asCouldNotDistributeEarningsToBondLot: {
      readonly frameId: u64;
      readonly vaultId: u32;
      readonly bondLotId: u64;
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly dispatchError: SpRuntimeDispatchError;
    } & Struct;
    readonly isCouldNotDistributeEarningsToArgonotBondLot: boolean;
    readonly asCouldNotDistributeEarningsToArgonotBondLot: {
      readonly frameId: u64;
      readonly bondLotId: u64;
      readonly accountId: AccountId32;
      readonly amount: u128;
      readonly dispatchError: SpRuntimeDispatchError;
    } & Struct;
    readonly isCouldNotTransferToTreasuryReserves: boolean;
    readonly asCouldNotTransferToTreasuryReserves: {
      readonly frameId: u64;
      readonly amount: u128;
      readonly dispatchError: SpRuntimeDispatchError;
    } & Struct;
    readonly isFrameEarningsDistributed: boolean;
    readonly asFrameEarningsDistributed: {
      readonly frameId: u64;
      readonly bidPoolDistributed: u128;
      readonly argonotBondPoolDistributed: u128;
      readonly vaultBidPoolDistributed: u128;
      readonly treasuryRefunds: u128;
      readonly treasuryReserves: u128;
      readonly participatingVaults: u32;
    } & Struct;
    readonly isFrameVaultCapitalLocked: boolean;
    readonly asFrameVaultCapitalLocked: {
      readonly frameId: u64;
      readonly totalEligibleBonds: u128;
      readonly participatingVaults: u32;
    } & Struct;
    readonly isCouldNotReleaseBondLot: boolean;
    readonly asCouldNotReleaseBondLot: {
      readonly frameId: u64;
      readonly programId: PalletTreasuryBondProgramId;
      readonly bondLotId: u64;
      readonly amount: u128;
      readonly accountId: AccountId32;
      readonly dispatchError: SpRuntimeDispatchError;
    } & Struct;
    readonly isBondLotPurchased: boolean;
    readonly asBondLotPurchased: {
      readonly programId: PalletTreasuryBondProgramId;
      readonly bondLotId: u64;
      readonly accountId: AccountId32;
      readonly bonds: u32;
    } & Struct;
    readonly isBondLotReleaseScheduled: boolean;
    readonly asBondLotReleaseScheduled: {
      readonly programId: PalletTreasuryBondProgramId;
      readonly bondLotId: u64;
      readonly accountId: AccountId32;
      readonly bonds: u32;
      readonly releaseFrameId: u64;
      readonly reason: PalletTreasuryBondReleaseReason;
    } & Struct;
    readonly isBondLotReleased: boolean;
    readonly asBondLotReleased: {
      readonly frameId: u64;
      readonly programId: PalletTreasuryBondProgramId;
      readonly bondLotId: u64;
      readonly accountId: AccountId32;
      readonly bonds: u32;
    } & Struct;
    readonly isEncumberedBondMicrogonsBurned: boolean;
    readonly asEncumberedBondMicrogonsBurned: {
      readonly accountId: AccountId32;
      readonly burnedAmount: u128;
      readonly releasedAmount: u128;
    } & Struct;
    readonly type:
      | 'CouldNotDistributeEarningsToBondLot'
      | 'CouldNotDistributeEarningsToArgonotBondLot'
      | 'CouldNotTransferToTreasuryReserves'
      | 'FrameEarningsDistributed'
      | 'FrameVaultCapitalLocked'
      | 'CouldNotReleaseBondLot'
      | 'BondLotPurchased'
      | 'BondLotReleaseScheduled'
      | 'BondLotReleased'
      | 'EncumberedBondMicrogonsBurned';
  }

  /** @name PalletTreasuryBondProgramId (120) */
  interface PalletTreasuryBondProgramId extends Enum {
    readonly isVault: boolean;
    readonly asVault: {
      readonly vaultId: Compact<u32>;
    } & Struct;
    readonly isArgonot: boolean;
    readonly type: 'Vault' | 'Argonot';
  }

  /** @name PalletTreasuryBondReleaseReason (121) */
  interface PalletTreasuryBondReleaseReason extends Enum {
    readonly isUserLiquidation: boolean;
    readonly isBumped: boolean;
    readonly isVaultClosed: boolean;
    readonly type: 'UserLiquidation' | 'Bumped' | 'VaultClosed';
  }

  /** @name PalletFeeControlEvent (122) */
  interface PalletFeeControlEvent extends Enum {
    readonly isFeeSkipped: boolean;
    readonly asFeeSkipped: {
      readonly origin: ArgonRuntimeOriginCaller;
    } & Struct;
    readonly isFeeDelegated: boolean;
    readonly asFeeDelegated: {
      readonly origin: ArgonRuntimeOriginCaller;
      readonly from: AccountId32;
      readonly to: AccountId32;
    } & Struct;
    readonly type: 'FeeSkipped' | 'FeeDelegated';
  }

  /** @name ArgonRuntimeOriginCaller (123) */
  interface ArgonRuntimeOriginCaller extends Enum {
    readonly isSystem: boolean;
    readonly asSystem: FrameSupportDispatchRawOrigin;
    readonly type: 'System';
  }

  /** @name FrameSupportDispatchRawOrigin (124) */
  interface FrameSupportDispatchRawOrigin extends Enum {
    readonly isRoot: boolean;
    readonly isSigned: boolean;
    readonly asSigned: AccountId32;
    readonly isNone: boolean;
    readonly isAuthorized: boolean;
    readonly type: 'Root' | 'Signed' | 'None' | 'Authorized';
  }

  /** @name PalletOperationalAccountsEvent (125) */
  interface PalletOperationalAccountsEvent extends Enum {
    readonly isOperationalAccountRegistered: boolean;
    readonly asOperationalAccountRegistered: {
      readonly operationalAccount: AccountId32;
      readonly vaultAccount: AccountId32;
      readonly miningFundingAccount: AccountId32;
      readonly miningBotAccount: AccountId32;
      readonly sponsor: Option<AccountId32>;
    } & Struct;
    readonly isAccountWentOperational: boolean;
    readonly asAccountWentOperational: {
      readonly account: AccountId32;
    } & Struct;
    readonly isOperationalRewardEarned: boolean;
    readonly asOperationalRewardEarned: {
      readonly account: AccountId32;
      readonly rewardKind: ArgonPrimitivesProvidersOperationalRewardKind;
      readonly amount: u128;
    } & Struct;
    readonly isOperationalRewardsClaimed: boolean;
    readonly asOperationalRewardsClaimed: {
      readonly operationalAccount: AccountId32;
      readonly claimant: AccountId32;
      readonly amount: u128;
      readonly remainingPending: u128;
    } & Struct;
    readonly isRewardsConfigUpdated: boolean;
    readonly asRewardsConfigUpdated: {
      readonly operationalReferralReward: u128;
      readonly referralBonusReward: u128;
    } & Struct;
    readonly isOperationalProgressForced: boolean;
    readonly asOperationalProgressForced: {
      readonly account: AccountId32;
      readonly updateOperationalProgress: bool;
      readonly hasUniswapTransfer: bool;
      readonly vaultCreated: bool;
      readonly hasTreasuryPoolParticipation: bool;
      readonly observedBitcoinTotal: u128;
      readonly observedMiningSeatTotal: u32;
    } & Struct;
    readonly isEncryptedServerUpdated: boolean;
    readonly asEncryptedServerUpdated: {
      readonly sponsor: AccountId32;
      readonly sponsee: AccountId32;
    } & Struct;
    readonly type:
      | 'OperationalAccountRegistered'
      | 'AccountWentOperational'
      | 'OperationalRewardEarned'
      | 'OperationalRewardsClaimed'
      | 'RewardsConfigUpdated'
      | 'OperationalProgressForced'
      | 'EncryptedServerUpdated';
  }

  /** @name ArgonPrimitivesProvidersOperationalRewardKind (126) */
  interface ArgonPrimitivesProvidersOperationalRewardKind extends Enum {
    readonly isActivation: boolean;
    readonly isReferralBonus: boolean;
    readonly type: 'Activation' | 'ReferralBonus';
  }

  /** @name PalletEthereumVerifierEvent (127) */
  interface PalletEthereumVerifierEvent extends Enum {
    readonly isBeaconHeaderImported: boolean;
    readonly asBeaconHeaderImported: {
      readonly blockHash: H256;
      readonly slot: u64;
    } & Struct;
    readonly isExecutionHeaderAnchorImported: boolean;
    readonly asExecutionHeaderAnchorImported: {
      readonly blockHash: H256;
      readonly blockNumber: u64;
    } & Struct;
    readonly isExecutionHeaderAnchorBackfilled: boolean;
    readonly asExecutionHeaderAnchorBackfilled: {
      readonly beaconRoot: H256;
      readonly slot: u64;
      readonly blockHash: H256;
      readonly blockNumber: u64;
    } & Struct;
    readonly isSyncCommitteeUpdated: boolean;
    readonly asSyncCommitteeUpdated: {
      readonly period: u64;
    } & Struct;
    readonly isOperatingModeChanged: boolean;
    readonly asOperatingModeChanged: {
      readonly mode: PalletEthereumVerifierBasicOperatingMode;
    } & Struct;
    readonly type:
      | 'BeaconHeaderImported'
      | 'ExecutionHeaderAnchorImported'
      | 'ExecutionHeaderAnchorBackfilled'
      | 'SyncCommitteeUpdated'
      | 'OperatingModeChanged';
  }

  /** @name PalletEthereumVerifierBasicOperatingMode (128) */
  interface PalletEthereumVerifierBasicOperatingMode extends Enum {
    readonly isNormal: boolean;
    readonly isHalted: boolean;
    readonly type: 'Normal' | 'Halted';
  }

  /** @name PalletCrosschainTransferEvent (129) */
  interface PalletCrosschainTransferEvent extends Enum {
    readonly isTransferToArgonSettled: boolean;
    readonly asTransferToArgonSettled: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly transfer: PalletCrosschainTransferTransferToArgonActivity;
    } & Struct;
    readonly isGlobalIssuanceCouncilForced: boolean;
    readonly asGlobalIssuanceCouncilForced: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly councilHash: H256;
    } & Struct;
    readonly isCouncilSignerRegistered: boolean;
    readonly asCouncilSignerRegistered: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly accountId: AccountId32;
      readonly signer: H160;
    } & Struct;
    readonly isCouncilSignerRotationQueued: boolean;
    readonly asCouncilSignerRotationQueued: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly accountId: AccountId32;
      readonly signer: H160;
    } & Struct;
    readonly isMintingAuthorityRegistered: boolean;
    readonly asMintingAuthorityRegistered: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly destinationSigningKey: H160;
      readonly accountId: AccountId32;
      readonly approvalQueueNonce: u64;
    } & Struct;
    readonly isMintingAuthorityDeactivationQueued: boolean;
    readonly asMintingAuthorityDeactivationQueued: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly destinationSigningKey: H160;
      readonly approvalQueueNonce: u64;
    } & Struct;
    readonly isMinimumMintingAuthorityValueSet: boolean;
    readonly asMinimumMintingAuthorityValueSet: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly minimumValue: u128;
    } & Struct;
    readonly isMintingAuthorityActivationRepaymentPricingSet: boolean;
    readonly asMintingAuthorityActivationRepaymentPricingSet: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
    } & Struct;
    readonly isQueueEntryApprovalRecorded: boolean;
    readonly asQueueEntryApprovalRecorded: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly target: PalletCrosschainTransferCouncilApprovalTargetId;
      readonly approvalQueueNonce: u64;
    } & Struct;
    readonly isQueueEntryApprovalReady: boolean;
    readonly asQueueEntryApprovalReady: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly target: PalletCrosschainTransferCouncilApprovalTargetId;
      readonly approvalQueueNonce: u64;
    } & Struct;
    readonly isMintingAuthorityActivationFinalized: boolean;
    readonly asMintingAuthorityActivationFinalized: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly destinationSigningKey: H160;
    } & Struct;
    readonly isMintingAuthorityActivationCompleted: boolean;
    readonly asMintingAuthorityActivationCompleted: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly destinationSigningKey: H160;
      readonly relayerArgonAccountId: AccountId32;
      readonly repaymentAmount: u128;
    } & Struct;
    readonly isMintingAuthorityDeactivationFinalized: boolean;
    readonly asMintingAuthorityDeactivationFinalized: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly destinationSigningKey: H160;
    } & Struct;
    readonly isGatewaySyncPaused: boolean;
    readonly asGatewaySyncPaused: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly pause: PalletCrosschainTransferGatewaySyncPause;
    } & Struct;
    readonly isGatewayUnpaused: boolean;
    readonly asGatewayUnpaused: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
    } & Struct;
    readonly isGatewayStateAdvanced: boolean;
    readonly asGatewayStateAdvanced: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly gatewayState: PalletCrosschainTransferGatewayState;
    } & Struct;
    readonly isTransferOutStarted: boolean;
    readonly asTransferOutStarted: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly transferId: H256;
      readonly accountId: AccountId32;
      readonly asset: PalletCrosschainTransferAssetKind;
      readonly amount: u128;
      readonly mintingAuthorityTip: u128;
    } & Struct;
    readonly isTransferCollateralized: boolean;
    readonly asTransferCollateralized: {
      readonly transferId: H256;
      readonly destinationSigningKey: H160;
      readonly microgonCollateral: u128;
      readonly micronotCollateral: u128;
    } & Struct;
    readonly isTransferOutReady: boolean;
    readonly asTransferOutReady: {
      readonly transferId: H256;
    } & Struct;
    readonly isTransferOutFinalized: boolean;
    readonly asTransferOutFinalized: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly transferId: H256;
    } & Struct;
    readonly isTransferOutCanceled: boolean;
    readonly asTransferOutCanceled: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly transferId: H256;
    } & Struct;
    readonly isTransferCollateralInvalidated: boolean;
    readonly asTransferCollateralInvalidated: {
      readonly transferId: H256;
      readonly destinationSigningKey: H160;
    } & Struct;
    readonly type:
      | 'TransferToArgonSettled'
      | 'GlobalIssuanceCouncilForced'
      | 'CouncilSignerRegistered'
      | 'CouncilSignerRotationQueued'
      | 'MintingAuthorityRegistered'
      | 'MintingAuthorityDeactivationQueued'
      | 'MinimumMintingAuthorityValueSet'
      | 'MintingAuthorityActivationRepaymentPricingSet'
      | 'QueueEntryApprovalRecorded'
      | 'QueueEntryApprovalReady'
      | 'MintingAuthorityActivationFinalized'
      | 'MintingAuthorityActivationCompleted'
      | 'MintingAuthorityDeactivationFinalized'
      | 'GatewaySyncPaused'
      | 'GatewayUnpaused'
      | 'GatewayStateAdvanced'
      | 'TransferOutStarted'
      | 'TransferCollateralized'
      | 'TransferOutReady'
      | 'TransferOutFinalized'
      | 'TransferOutCanceled'
      | 'TransferCollateralInvalidated';
  }

  /** @name PalletCrosschainTransferSourceChain (130) */
  interface PalletCrosschainTransferSourceChain extends Enum {
    readonly isEthereum: boolean;
    readonly type: 'Ethereum';
  }

  /** @name PalletCrosschainTransferTransferToArgonActivity (131) */
  interface PalletCrosschainTransferTransferToArgonActivity extends Struct {
    readonly gatewayActivityNonce: Compact<u64>;
    readonly from: H160;
    readonly asset: PalletCrosschainTransferAssetKind;
    readonly to: AccountId32;
    readonly amount: Compact<u128>;
  }

  /** @name PalletCrosschainTransferAssetKind (134) */
  interface PalletCrosschainTransferAssetKind extends Enum {
    readonly isArgon: boolean;
    readonly isArgonot: boolean;
    readonly type: 'Argon' | 'Argonot';
  }

  /** @name PalletCrosschainTransferCouncilApprovalTargetId (135) */
  interface PalletCrosschainTransferCouncilApprovalTargetId extends Enum {
    readonly isMintingAuthorityActivation: boolean;
    readonly asMintingAuthorityActivation: H160;
    readonly isMintingAuthorityDeactivation: boolean;
    readonly asMintingAuthorityDeactivation: H160;
    readonly isGlobalIssuanceCouncilRotation: boolean;
    readonly asGlobalIssuanceCouncilRotation: H256;
    readonly type:
      | 'MintingAuthorityActivation'
      | 'MintingAuthorityDeactivation'
      | 'GlobalIssuanceCouncilRotation';
  }

  /** @name PalletCrosschainTransferGatewaySyncPause (136) */
  interface PalletCrosschainTransferGatewaySyncPause extends Struct {
    readonly lastGoodGatewayActivityNonce: Compact<u64>;
    readonly failedGatewayActivityNonce: Compact<u64>;
    readonly reason: PalletCrosschainTransferGatewaySyncPauseReason;
  }

  /** @name PalletCrosschainTransferGatewaySyncPauseReason (137) */
  interface PalletCrosschainTransferGatewaySyncPauseReason extends Enum {
    readonly isManual: boolean;
    readonly isMalformedGatewayActivity: boolean;
    readonly isUnsupportedToken: boolean;
    readonly isMintingAuthorityNotFound: boolean;
    readonly isUnexpectedMintingAuthorityState: boolean;
    readonly isMintingAuthorityMismatch: boolean;
    readonly isMissingMintingAuthorityActivationRepaymentPricing: boolean;
    readonly isMintingAuthorityActivationRepaymentMismatch: boolean;
    readonly isGlobalIssuanceCouncilNotFound: boolean;
    readonly isGatewayStateDrift: boolean;
    readonly type:
      | 'Manual'
      | 'MalformedGatewayActivity'
      | 'UnsupportedToken'
      | 'MintingAuthorityNotFound'
      | 'UnexpectedMintingAuthorityState'
      | 'MintingAuthorityMismatch'
      | 'MissingMintingAuthorityActivationRepaymentPricing'
      | 'MintingAuthorityActivationRepaymentMismatch'
      | 'GlobalIssuanceCouncilNotFound'
      | 'GatewayStateDrift';
  }

  /** @name PalletCrosschainTransferGatewayState (138) */
  interface PalletCrosschainTransferGatewayState extends Struct {
    readonly gatewayActivityNonce: Compact<u64>;
    readonly argonApprovalsNonce: Compact<u64>;
    readonly argonCirculation: u128;
    readonly argonotCirculation: u128;
  }

  /** @name FrameSystemPhase (139) */
  interface FrameSystemPhase extends Enum {
    readonly isApplyExtrinsic: boolean;
    readonly asApplyExtrinsic: u32;
    readonly isFinalization: boolean;
    readonly isInitialization: boolean;
    readonly type: 'ApplyExtrinsic' | 'Finalization' | 'Initialization';
  }

  /** @name FrameSystemLastRuntimeUpgradeInfo (143) */
  interface FrameSystemLastRuntimeUpgradeInfo extends Struct {
    readonly specVersion: Compact<u32>;
    readonly specName: Text;
  }

  /** @name FrameSystemCodeUpgradeAuthorization (146) */
  interface FrameSystemCodeUpgradeAuthorization extends Struct {
    readonly codeHash: H256;
    readonly checkVersion: bool;
  }

  /** @name FrameSystemCall (147) */
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
    readonly type:
      | 'Remark'
      | 'SetHeapPages'
      | 'SetCode'
      | 'SetCodeWithoutChecks'
      | 'SetStorage'
      | 'KillStorage'
      | 'KillPrefix'
      | 'RemarkWithEvent'
      | 'AuthorizeUpgrade'
      | 'AuthorizeUpgradeWithoutChecks'
      | 'ApplyAuthorizedUpgrade';
  }

  /** @name FrameSystemLimitsBlockWeights (151) */
  interface FrameSystemLimitsBlockWeights extends Struct {
    readonly baseBlock: SpWeightsWeightV2Weight;
    readonly maxBlock: SpWeightsWeightV2Weight;
    readonly perClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
  }

  /** @name FrameSupportDispatchPerDispatchClassWeightsPerClass (152) */
  interface FrameSupportDispatchPerDispatchClassWeightsPerClass extends Struct {
    readonly normal: FrameSystemLimitsWeightsPerClass;
    readonly operational: FrameSystemLimitsWeightsPerClass;
    readonly mandatory: FrameSystemLimitsWeightsPerClass;
  }

  /** @name FrameSystemLimitsWeightsPerClass (153) */
  interface FrameSystemLimitsWeightsPerClass extends Struct {
    readonly baseExtrinsic: SpWeightsWeightV2Weight;
    readonly maxExtrinsic: Option<SpWeightsWeightV2Weight>;
    readonly maxTotal: Option<SpWeightsWeightV2Weight>;
    readonly reserved: Option<SpWeightsWeightV2Weight>;
  }

  /** @name FrameSystemLimitsBlockLength (155) */
  interface FrameSystemLimitsBlockLength extends Struct {
    readonly max: FrameSupportDispatchPerDispatchClassU32;
    readonly maxHeaderSize: Option<u32>;
  }

  /** @name FrameSupportDispatchPerDispatchClassU32 (156) */
  interface FrameSupportDispatchPerDispatchClassU32 extends Struct {
    readonly normal: u32;
    readonly operational: u32;
    readonly mandatory: u32;
  }

  /** @name SpWeightsRuntimeDbWeight (158) */
  interface SpWeightsRuntimeDbWeight extends Struct {
    readonly read: u64;
    readonly write: u64;
  }

  /** @name SpVersionRuntimeVersion (159) */
  interface SpVersionRuntimeVersion extends Struct {
    readonly specName: Text;
    readonly implName: Text;
    readonly authoringVersion: u32;
    readonly specVersion: u32;
    readonly implVersion: u32;
    readonly apis: Vec<ITuple<[U8aFixed, u32]>>;
    readonly transactionVersion: u32;
    readonly systemVersion: u8;
  }

  /** @name FrameSystemError (164) */
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
    readonly type:
      | 'InvalidSpecName'
      | 'SpecVersionNeedsToIncrease'
      | 'FailedToExtractRuntimeVersion'
      | 'NonDefaultComposite'
      | 'NonZeroRefCount'
      | 'CallFiltered'
      | 'MultiBlockMigrationsOngoing'
      | 'NothingAuthorized'
      | 'Unauthorized';
  }

  /** @name ArgonPrimitivesDigestsDigestset (165) */
  interface ArgonPrimitivesDigestsDigestset extends Struct {
    readonly author: AccountId32;
    readonly blockVote: ArgonPrimitivesDigestsBlockVoteDigest;
    readonly votingKey: Option<ArgonPrimitivesDigestsParentVotingKeyDigest>;
    readonly forkPower: Option<ArgonPrimitivesForkPower>;
    readonly frameInfo: Option<ArgonPrimitivesDigestsFrameInfo>;
    readonly tick: u64;
    readonly notebooks: ArgonPrimitivesDigestsNotebookDigest;
  }

  /** @name ArgonPrimitivesDigestsBlockVoteDigest (166) */
  interface ArgonPrimitivesDigestsBlockVoteDigest extends Struct {
    readonly votingPower: Compact<u128>;
    readonly votesCount: Compact<u32>;
  }

  /** @name ArgonPrimitivesDigestsParentVotingKeyDigest (168) */
  interface ArgonPrimitivesDigestsParentVotingKeyDigest extends Struct {
    readonly parentVotingKey: Option<H256>;
  }

  /** @name ArgonPrimitivesForkPower (171) */
  interface ArgonPrimitivesForkPower extends Struct {
    readonly isLatestVote: bool;
    readonly notebooks: Compact<u64>;
    readonly votingPower: U256;
    readonly sealStrength: U256;
    readonly totalComputeDifficulty: U256;
    readonly voteCreatedBlocks: Compact<u128>;
    readonly minerNonceScore: Option<U256>;
  }

  /** @name ArgonPrimitivesDigestsFrameInfo (176) */
  interface ArgonPrimitivesDigestsFrameInfo extends Struct {
    readonly frameId: Compact<u64>;
    readonly frameRewardTicksRemaining: Compact<u32>;
    readonly isNewFrame: bool;
  }

  /** @name ArgonPrimitivesDigestsNotebookDigest (178) */
  interface ArgonPrimitivesDigestsNotebookDigest extends Struct {
    readonly notebooks: Vec<ArgonPrimitivesNotebookNotebookAuditResult>;
  }

  /** @name ArgonPrimitivesNotebookNotebookAuditResult (180) */
  interface ArgonPrimitivesNotebookNotebookAuditResult extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly auditFirstFailure: Option<ArgonNotaryAuditErrorVerifyError>;
  }

  /** @name PalletDigestsError (183) */
  interface PalletDigestsError extends Enum {
    readonly isDuplicateBlockVoteDigest: boolean;
    readonly isDuplicateAuthorDigest: boolean;
    readonly isDuplicateTickDigest: boolean;
    readonly isDuplicateParentVotingKeyDigest: boolean;
    readonly isDuplicateNotebookDigest: boolean;
    readonly isDuplicateForkPowerDigest: boolean;
    readonly isMissingBlockVoteDigest: boolean;
    readonly isMissingAuthorDigest: boolean;
    readonly isMissingTickDigest: boolean;
    readonly isMissingParentVotingKeyDigest: boolean;
    readonly isMissingNotebookDigest: boolean;
    readonly isCouldNotDecodeDigest: boolean;
    readonly isDuplicateFrameInfoDigest: boolean;
    readonly type:
      | 'DuplicateBlockVoteDigest'
      | 'DuplicateAuthorDigest'
      | 'DuplicateTickDigest'
      | 'DuplicateParentVotingKeyDigest'
      | 'DuplicateNotebookDigest'
      | 'DuplicateForkPowerDigest'
      | 'MissingBlockVoteDigest'
      | 'MissingAuthorDigest'
      | 'MissingTickDigest'
      | 'MissingParentVotingKeyDigest'
      | 'MissingNotebookDigest'
      | 'CouldNotDecodeDigest'
      | 'DuplicateFrameInfoDigest';
  }

  /** @name PalletTimestampCall (184) */
  interface PalletTimestampCall extends Enum {
    readonly isSet: boolean;
    readonly asSet: {
      readonly now: Compact<u64>;
    } & Struct;
    readonly type: 'Set';
  }

  /** @name PalletMultisigMultisig (186) */
  interface PalletMultisigMultisig extends Struct {
    readonly when: PalletMultisigTimepoint;
    readonly deposit: u128;
    readonly depositor: AccountId32;
    readonly approvals: Vec<AccountId32>;
  }

  /** @name PalletMultisigCall (189) */
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
    readonly isPokeDeposit: boolean;
    readonly asPokeDeposit: {
      readonly threshold: u16;
      readonly otherSignatories: Vec<AccountId32>;
      readonly callHash: U8aFixed;
    } & Struct;
    readonly type:
      | 'AsMultiThreshold1'
      | 'AsMulti'
      | 'ApproveAsMulti'
      | 'CancelAsMulti'
      | 'PokeDeposit';
  }

  /** @name PalletProxyCall (191) */
  interface PalletProxyCall extends Enum {
    readonly isProxy: boolean;
    readonly asProxy: {
      readonly real: MultiAddress;
      readonly forceProxyType: Option<ArgonRuntimeProxyType>;
      readonly call: Call;
    } & Struct;
    readonly isAddProxy: boolean;
    readonly asAddProxy: {
      readonly delegate: MultiAddress;
      readonly proxyType: ArgonRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isRemoveProxy: boolean;
    readonly asRemoveProxy: {
      readonly delegate: MultiAddress;
      readonly proxyType: ArgonRuntimeProxyType;
      readonly delay: u32;
    } & Struct;
    readonly isRemoveProxies: boolean;
    readonly isCreatePure: boolean;
    readonly asCreatePure: {
      readonly proxyType: ArgonRuntimeProxyType;
      readonly delay: u32;
      readonly index: u16;
    } & Struct;
    readonly isKillPure: boolean;
    readonly asKillPure: {
      readonly spawner: MultiAddress;
      readonly proxyType: ArgonRuntimeProxyType;
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
      readonly forceProxyType: Option<ArgonRuntimeProxyType>;
      readonly call: Call;
    } & Struct;
    readonly isPokeDeposit: boolean;
    readonly type:
      | 'Proxy'
      | 'AddProxy'
      | 'RemoveProxy'
      | 'RemoveProxies'
      | 'CreatePure'
      | 'KillPure'
      | 'Announce'
      | 'RemoveAnnouncement'
      | 'RejectAnnouncement'
      | 'ProxyAnnounced'
      | 'PokeDeposit';
  }

  /** @name PalletTicksCall (195) */
  type PalletTicksCall = Null;

  /** @name PalletMiningSlotCall (196) */
  interface PalletMiningSlotCall extends Enum {
    readonly isBid: boolean;
    readonly asBid: {
      readonly bid: u128;
      readonly keys_: ArgonRuntimeSessionKeys;
      readonly miningAccountId: Option<AccountId32>;
    } & Struct;
    readonly isConfigureMiningSlotDelay: boolean;
    readonly asConfigureMiningSlotDelay: {
      readonly miningSlotDelay: Option<u64>;
      readonly ticksBeforeBidEndForVrfClose: Option<u64>;
    } & Struct;
    readonly type: 'Bid' | 'ConfigureMiningSlotDelay';
  }

  /** @name PalletBitcoinUtxosCall (197) */
  interface PalletBitcoinUtxosCall extends Enum {
    readonly isSync: boolean;
    readonly asSync: {
      readonly utxoSync: ArgonPrimitivesInherentsBitcoinUtxoSync;
    } & Struct;
    readonly isSetConfirmedBlock: boolean;
    readonly asSetConfirmedBlock: {
      readonly bitcoinHeight: u64;
      readonly bitcoinBlockHash: ArgonPrimitivesBitcoinH256Le;
    } & Struct;
    readonly isSetOperator: boolean;
    readonly asSetOperator: {
      readonly accountId: AccountId32;
    } & Struct;
    readonly isFundWithUtxoCandidate: boolean;
    readonly asFundWithUtxoCandidate: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
    } & Struct;
    readonly isRejectUtxoCandidate: boolean;
    readonly asRejectUtxoCandidate: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
    } & Struct;
    readonly type:
      | 'Sync'
      | 'SetConfirmedBlock'
      | 'SetOperator'
      | 'FundWithUtxoCandidate'
      | 'RejectUtxoCandidate';
  }

  /** @name ArgonPrimitivesInherentsBitcoinUtxoSync (198) */
  interface ArgonPrimitivesInherentsBitcoinUtxoSync extends Struct {
    readonly spent: Vec<ArgonPrimitivesInherentsBitcoinUtxoSpend>;
    readonly funded: Vec<ArgonPrimitivesInherentsBitcoinUtxoFunding>;
    readonly syncToBlock: ArgonPrimitivesBitcoinBitcoinBlock;
  }

  /** @name ArgonPrimitivesInherentsBitcoinUtxoSpend (200) */
  interface ArgonPrimitivesInherentsBitcoinUtxoSpend extends Struct {
    readonly utxoId: Compact<u64>;
    readonly utxoRef: Option<ArgonPrimitivesBitcoinUtxoRef>;
    readonly bitcoinHeight: Compact<u64>;
  }

  /** @name ArgonPrimitivesInherentsBitcoinUtxoFunding (203) */
  interface ArgonPrimitivesInherentsBitcoinUtxoFunding extends Struct {
    readonly utxoId: Compact<u64>;
    readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
    readonly satoshis: Compact<u64>;
    readonly expectedSatoshis: Compact<u64>;
    readonly bitcoinHeight: Compact<u64>;
  }

  /** @name ArgonPrimitivesBitcoinBitcoinBlock (204) */
  interface ArgonPrimitivesBitcoinBitcoinBlock extends Struct {
    readonly blockHeight: Compact<u64>;
    readonly blockHash: ArgonPrimitivesBitcoinH256Le;
  }

  /** @name PalletVaultsCall (205) */
  interface PalletVaultsCall extends Enum {
    readonly isCreate: boolean;
    readonly asCreate: {
      readonly vaultConfig: PalletVaultsVaultConfig;
    } & Struct;
    readonly isModifyFunding: boolean;
    readonly asModifyFunding: {
      readonly vaultId: u32;
      readonly securitization: u128;
      readonly securitizationRatio: u128;
    } & Struct;
    readonly isModifyTerms: boolean;
    readonly asModifyTerms: {
      readonly vaultId: u32;
      readonly terms: ArgonPrimitivesVaultVaultTerms;
    } & Struct;
    readonly isClose: boolean;
    readonly asClose: {
      readonly vaultId: u32;
    } & Struct;
    readonly isReplaceBitcoinXpub: boolean;
    readonly asReplaceBitcoinXpub: {
      readonly vaultId: u32;
      readonly bitcoinXpub: ArgonPrimitivesBitcoinOpaqueBitcoinXpub;
    } & Struct;
    readonly isCollect: boolean;
    readonly asCollect: {
      readonly vaultId: u32;
    } & Struct;
    readonly isSetDelegateAccount: boolean;
    readonly asSetDelegateAccount: {
      readonly delegateAccountId: Option<AccountId32>;
    } & Struct;
    readonly isSetName: boolean;
    readonly asSetName: {
      readonly name: Option<Bytes>;
    } & Struct;
    readonly isSetCommittedArgonots: boolean;
    readonly asSetCommittedArgonots: {
      readonly amount: Compact<u128>;
    } & Struct;
    readonly type:
      | 'Create'
      | 'ModifyFunding'
      | 'ModifyTerms'
      | 'Close'
      | 'ReplaceBitcoinXpub'
      | 'Collect'
      | 'SetDelegateAccount'
      | 'SetName'
      | 'SetCommittedArgonots';
  }

  /** @name PalletVaultsVaultConfig (206) */
  interface PalletVaultsVaultConfig extends Struct {
    readonly terms: ArgonPrimitivesVaultVaultTerms;
    readonly name: Option<Bytes>;
    readonly delegateAccountId: Option<AccountId32>;
    readonly securitization: Compact<u128>;
    readonly bitcoinXpubkey: ArgonPrimitivesBitcoinOpaqueBitcoinXpub;
    readonly securitizationRatio: Compact<u128>;
  }

  /** @name ArgonPrimitivesVaultVaultTerms (207) */
  interface ArgonPrimitivesVaultVaultTerms extends Struct {
    readonly bitcoinAnnualPercentRate: Compact<u128>;
    readonly bitcoinBaseFee: Compact<u128>;
    readonly treasuryProfitSharing: Compact<Permill>;
    readonly treasuryBonusProfitSharing: Compact<Permill>;
  }

  /** @name ArgonPrimitivesBitcoinOpaqueBitcoinXpub (213) */
  interface ArgonPrimitivesBitcoinOpaqueBitcoinXpub extends U8aFixed {}

  /** @name PalletBitcoinLocksCall (215) */
  interface PalletBitcoinLocksCall extends Enum {
    readonly isInitialize: boolean;
    readonly asInitialize: {
      readonly vaultId: u32;
      readonly satoshis: Compact<u64>;
      readonly bitcoinPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
      readonly options: Option<PalletBitcoinLocksLockOptions>;
    } & Struct;
    readonly isRequestRelease: boolean;
    readonly asRequestRelease: {
      readonly utxoId: u64;
      readonly toScriptPubkey: Bytes;
      readonly bitcoinNetworkFee: u64;
    } & Struct;
    readonly isCosignRelease: boolean;
    readonly asCosignRelease: {
      readonly utxoId: u64;
      readonly signature: Bytes;
    } & Struct;
    readonly isRatchet: boolean;
    readonly asRatchet: {
      readonly utxoId: u64;
      readonly options: Option<PalletBitcoinLocksLockOptions>;
    } & Struct;
    readonly isAdminModifyMinimumLockedSats: boolean;
    readonly asAdminModifyMinimumLockedSats: {
      readonly satoshis: u64;
    } & Struct;
    readonly isRequestOrphanedUtxoRelease: boolean;
    readonly asRequestOrphanedUtxoRelease: {
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly toScriptPubkey: Bytes;
      readonly bitcoinNetworkFee: u64;
    } & Struct;
    readonly isCosignOrphanedUtxoRelease: boolean;
    readonly asCosignOrphanedUtxoRelease: {
      readonly orphanOwner: AccountId32;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly signature: Bytes;
    } & Struct;
    readonly isInitializeFor: boolean;
    readonly asInitializeFor: {
      readonly accountId: AccountId32;
      readonly vaultId: u32;
      readonly satoshis: Compact<u64>;
      readonly bitcoinPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
      readonly options: Option<PalletBitcoinLocksLockOptions>;
    } & Struct;
    readonly isIncreaseSecuritization: boolean;
    readonly asIncreaseSecuritization: {
      readonly utxoId: u64;
      readonly newSatoshis: Compact<u64>;
    } & Struct;
    readonly type:
      | 'Initialize'
      | 'RequestRelease'
      | 'CosignRelease'
      | 'Ratchet'
      | 'AdminModifyMinimumLockedSats'
      | 'RequestOrphanedUtxoRelease'
      | 'CosignOrphanedUtxoRelease'
      | 'InitializeFor'
      | 'IncreaseSecuritization';
  }

  /** @name ArgonPrimitivesBitcoinCompressedBitcoinPubkey (216) */
  interface ArgonPrimitivesBitcoinCompressedBitcoinPubkey extends U8aFixed {}

  /** @name PalletBitcoinLocksLockOptions (219) */
  interface PalletBitcoinLocksLockOptions extends Enum {
    readonly isV1: boolean;
    readonly asV1: {
      readonly microgonsAtTargetPerBtc: Option<u128>;
    } & Struct;
    readonly type: 'V1';
  }

  /** @name PalletNotariesCall (222) */
  interface PalletNotariesCall extends Enum {
    readonly isPropose: boolean;
    readonly asPropose: {
      readonly meta: ArgonPrimitivesNotaryNotaryMeta;
    } & Struct;
    readonly isActivate: boolean;
    readonly asActivate: {
      readonly operatorAccount: AccountId32;
    } & Struct;
    readonly isUpdate: boolean;
    readonly asUpdate: {
      readonly notaryId: Compact<u32>;
      readonly meta: ArgonPrimitivesNotaryNotaryMeta;
      readonly effectiveTick: Compact<u64>;
    } & Struct;
    readonly type: 'Propose' | 'Activate' | 'Update';
  }

  /** @name PalletNotebookCall (223) */
  interface PalletNotebookCall extends Enum {
    readonly isSubmit: boolean;
    readonly asSubmit: {
      readonly notebooks: Vec<ArgonPrimitivesNotebookSignedNotebookHeader>;
    } & Struct;
    readonly isUnlock: boolean;
    readonly asUnlock: {
      readonly notaryId: u32;
    } & Struct;
    readonly type: 'Submit' | 'Unlock';
  }

  /** @name ArgonPrimitivesNotebookSignedNotebookHeader (225) */
  interface ArgonPrimitivesNotebookSignedNotebookHeader extends Struct {
    readonly header: ArgonPrimitivesNotebookNotebookHeader;
    readonly signature: U8aFixed;
  }

  /** @name ArgonPrimitivesNotebookNotebookHeader (226) */
  interface ArgonPrimitivesNotebookNotebookHeader extends Struct {
    readonly version: Compact<u16>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly tax: Compact<u128>;
    readonly notaryId: Compact<u32>;
    readonly chainTransfers: Vec<ArgonPrimitivesNotebookChainTransfer>;
    readonly changedAccountsRoot: H256;
    readonly changedAccountOrigins: Vec<ArgonPrimitivesBalanceChangeAccountOrigin>;
    readonly blockVotesRoot: H256;
    readonly blockVotesCount: Compact<u32>;
    readonly blocksWithVotes: Vec<H256>;
    readonly blockVotingPower: Compact<u128>;
    readonly secretHash: H256;
    readonly parentSecret: Option<H256>;
    readonly domains: Vec<ITuple<[H256, AccountId32]>>;
  }

  /** @name ArgonPrimitivesNotebookChainTransfer (229) */
  interface ArgonPrimitivesNotebookChainTransfer extends Enum {
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

  /** @name ArgonPrimitivesBalanceChangeAccountOrigin (232) */
  interface ArgonPrimitivesBalanceChangeAccountOrigin extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly accountUid: Compact<u32>;
  }

  /** @name PalletLocalchainTransferCall (239) */
  interface PalletLocalchainTransferCall extends Enum {
    readonly isSendToLocalchain: boolean;
    readonly asSendToLocalchain: {
      readonly amount: Compact<u128>;
      readonly notaryId: u32;
    } & Struct;
    readonly type: 'SendToLocalchain';
  }

  /** @name PalletBlockSealSpecCall (240) */
  interface PalletBlockSealSpecCall extends Enum {
    readonly isConfigure: boolean;
    readonly asConfigure: {
      readonly voteMinimum: Option<u128>;
      readonly computeDifficulty: Option<u128>;
    } & Struct;
    readonly type: 'Configure';
  }

  /** @name PalletDomainsCall (241) */
  interface PalletDomainsCall extends Enum {
    readonly isSetZoneRecord: boolean;
    readonly asSetZoneRecord: {
      readonly domainHash: H256;
      readonly zoneRecord: ArgonPrimitivesDomainZoneRecord;
    } & Struct;
    readonly type: 'SetZoneRecord';
  }

  /** @name PalletPriceIndexCall (242) */
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

  /** @name PalletPriceIndexPriceIndex (243) */
  interface PalletPriceIndexPriceIndex extends Struct {
    readonly btcUsdPrice: Compact<u128>;
    readonly argonotUsdPrice: u128;
    readonly argonUsdPrice: Compact<u128>;
    readonly argonUsdTargetPrice: u128;
    readonly argonTimeWeightedAverageLiquidity: u128;
    readonly tick: Compact<u64>;
  }

  /** @name PalletGrandpaCall (244) */
  interface PalletGrandpaCall extends Enum {
    readonly isReportEquivocation: boolean;
    readonly asReportEquivocation: {
      readonly equivocationProof: SpConsensusGrandpaEquivocationProof;
      readonly keyOwnerProof: SpCoreVoid;
    } & Struct;
    readonly isReportEquivocationUnsigned: boolean;
    readonly asReportEquivocationUnsigned: {
      readonly equivocationProof: SpConsensusGrandpaEquivocationProof;
      readonly keyOwnerProof: SpCoreVoid;
    } & Struct;
    readonly isNoteStalled: boolean;
    readonly asNoteStalled: {
      readonly delay: u32;
      readonly bestFinalizedBlockNumber: u32;
    } & Struct;
    readonly type: 'ReportEquivocation' | 'ReportEquivocationUnsigned' | 'NoteStalled';
  }

  /** @name SpConsensusGrandpaEquivocationProof (245) */
  interface SpConsensusGrandpaEquivocationProof extends Struct {
    readonly setId: u64;
    readonly equivocation: SpConsensusGrandpaEquivocation;
  }

  /** @name SpConsensusGrandpaEquivocation (246) */
  interface SpConsensusGrandpaEquivocation extends Enum {
    readonly isPrevote: boolean;
    readonly asPrevote: FinalityGrandpaEquivocationPrevote;
    readonly isPrecommit: boolean;
    readonly asPrecommit: FinalityGrandpaEquivocationPrecommit;
    readonly type: 'Prevote' | 'Precommit';
  }

  /** @name FinalityGrandpaEquivocationPrevote (247) */
  interface FinalityGrandpaEquivocationPrevote extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrevote (248) */
  interface FinalityGrandpaPrevote extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpConsensusGrandpaAppSignature (249) */
  interface SpConsensusGrandpaAppSignature extends U8aFixed {}

  /** @name FinalityGrandpaEquivocationPrecommit (251) */
  interface FinalityGrandpaEquivocationPrecommit extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrecommit (252) */
  interface FinalityGrandpaPrecommit extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpCoreVoid (254) */
  type SpCoreVoid = Null;

  /** @name PalletBlockSealCall (255) */
  interface PalletBlockSealCall extends Enum {
    readonly isApply: boolean;
    readonly asApply: {
      readonly seal: ArgonPrimitivesInherentsBlockSealInherent;
    } & Struct;
    readonly type: 'Apply';
  }

  /** @name ArgonPrimitivesInherentsBlockSealInherent (256) */
  interface ArgonPrimitivesInherentsBlockSealInherent extends Enum {
    readonly isVote: boolean;
    readonly asVote: {
      readonly sealStrength: U256;
      readonly notaryId: Compact<u32>;
      readonly sourceNotebookNumber: Compact<u32>;
      readonly sourceNotebookProof: ArgonPrimitivesBalanceChangeMerkleProof;
      readonly blockVote: ArgonPrimitivesBlockVoteBlockVoteT;
      readonly minerNonceScore: Option<U256>;
    } & Struct;
    readonly isCompute: boolean;
    readonly type: 'Vote' | 'Compute';
  }

  /** @name ArgonPrimitivesBalanceChangeMerkleProof (257) */
  interface ArgonPrimitivesBalanceChangeMerkleProof extends Struct {
    readonly proof: Vec<H256>;
    readonly numberOfLeaves: Compact<u32>;
    readonly leafIndex: Compact<u32>;
  }

  /** @name ArgonPrimitivesBlockVoteBlockVoteT (259) */
  interface ArgonPrimitivesBlockVoteBlockVoteT extends Struct {
    readonly accountId: AccountId32;
    readonly blockHash: H256;
    readonly index: Compact<u32>;
    readonly power: Compact<u128>;
    readonly signature: SpRuntimeMultiSignature;
    readonly blockRewardsAccountId: AccountId32;
    readonly tick: Compact<u64>;
  }

  /** @name SpRuntimeMultiSignature (260) */
  interface SpRuntimeMultiSignature extends Enum {
    readonly isEd25519: boolean;
    readonly asEd25519: U8aFixed;
    readonly isSr25519: boolean;
    readonly asSr25519: U8aFixed;
    readonly isEcdsa: boolean;
    readonly asEcdsa: U8aFixed;
    readonly isEth: boolean;
    readonly asEth: U8aFixed;
    readonly type: 'Ed25519' | 'Sr25519' | 'Ecdsa' | 'Eth';
  }

  /** @name PalletBlockRewardsCall (262) */
  interface PalletBlockRewardsCall extends Enum {
    readonly isSetBlockRewardsPaused: boolean;
    readonly asSetBlockRewardsPaused: {
      readonly paused: bool;
    } & Struct;
    readonly isSetBlockVoterRewardsEnabled: boolean;
    readonly asSetBlockVoterRewardsEnabled: {
      readonly enabled: bool;
    } & Struct;
    readonly type: 'SetBlockRewardsPaused' | 'SetBlockVoterRewardsEnabled';
  }

  /** @name PalletMintCall (263) */
  type PalletMintCall = Null;

  /** @name PalletBalancesCall (264) */
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
    readonly type:
      | 'TransferAllowDeath'
      | 'ForceTransfer'
      | 'TransferKeepAlive'
      | 'TransferAll'
      | 'ForceUnreserve'
      | 'UpgradeAccounts'
      | 'ForceSetBalance'
      | 'ForceAdjustTotalIssuance'
      | 'Burn';
  }

  /** @name PalletBalancesAdjustmentDirection (265) */
  interface PalletBalancesAdjustmentDirection extends Enum {
    readonly isIncrease: boolean;
    readonly isDecrease: boolean;
    readonly type: 'Increase' | 'Decrease';
  }

  /** @name PalletTxPauseCall (267) */
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

  /** @name PalletUtilityCall (268) */
  interface PalletUtilityCall extends Enum {
    readonly isBatch: boolean;
    readonly asBatch: {
      readonly calls: Vec<Call>;
    } & Struct;
    readonly isAsDerivative: boolean;
    readonly asAsDerivative: {
      readonly index: u16;
      readonly call: Call;
    } & Struct;
    readonly isBatchAll: boolean;
    readonly asBatchAll: {
      readonly calls: Vec<Call>;
    } & Struct;
    readonly isDispatchAs: boolean;
    readonly asDispatchAs: {
      readonly asOrigin: ArgonRuntimeOriginCaller;
      readonly call: Call;
    } & Struct;
    readonly isForceBatch: boolean;
    readonly asForceBatch: {
      readonly calls: Vec<Call>;
    } & Struct;
    readonly isWithWeight: boolean;
    readonly asWithWeight: {
      readonly call: Call;
      readonly weight: SpWeightsWeightV2Weight;
    } & Struct;
    readonly isIfElse: boolean;
    readonly asIfElse: {
      readonly main: Call;
      readonly fallback: Call;
    } & Struct;
    readonly isDispatchAsFallible: boolean;
    readonly asDispatchAsFallible: {
      readonly asOrigin: ArgonRuntimeOriginCaller;
      readonly call: Call;
    } & Struct;
    readonly type:
      | 'Batch'
      | 'AsDerivative'
      | 'BatchAll'
      | 'DispatchAs'
      | 'ForceBatch'
      | 'WithWeight'
      | 'IfElse'
      | 'DispatchAsFallible';
  }

  /** @name PalletSudoCall (270) */
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

  /** @name PalletTreasuryCall (271) */
  interface PalletTreasuryCall extends Enum {
    readonly isBuyBonds: boolean;
    readonly asBuyBonds: {
      readonly vaultId: u32;
      readonly bonds: u32;
      readonly bonusApproval: Option<ArgonPrimitivesVaultTreasuryBonusApprovalProof>;
    } & Struct;
    readonly isLiquidateBondLot: boolean;
    readonly asLiquidateBondLot: {
      readonly bondLotId: u64;
    } & Struct;
    readonly isBuyArgonotBonds: boolean;
    readonly asBuyArgonotBonds: {
      readonly bonds: u32;
    } & Struct;
    readonly type: 'BuyBonds' | 'LiquidateBondLot' | 'BuyArgonotBonds';
  }

  /** @name ArgonPrimitivesVaultTreasuryBonusApprovalProof (273) */
  interface ArgonPrimitivesVaultTreasuryBonusApprovalProof extends Struct {
    readonly vaultId: Compact<u32>;
    readonly beneficiary: AccountId32;
    readonly expiresAtFrame: Compact<u64>;
    readonly signature: SpRuntimeMultiSignature;
  }

  /** @name PalletOperationalAccountsCall (274) */
  interface PalletOperationalAccountsCall extends Enum {
    readonly isRegister: boolean;
    readonly asRegister: {
      readonly registration: PalletOperationalAccountsRegistration;
    } & Struct;
    readonly isSetRewardConfig: boolean;
    readonly asSetRewardConfig: {
      readonly operationalReferralReward: u128;
      readonly referralBonusReward: u128;
    } & Struct;
    readonly isForceSetProgress: boolean;
    readonly asForceSetProgress: {
      readonly owner: AccountId32;
      readonly patch: PalletOperationalAccountsOperationalProgressPatch;
      readonly updateOperationalProgress: bool;
    } & Struct;
    readonly isSetEncryptedServerForSponsee: boolean;
    readonly asSetEncryptedServerForSponsee: {
      readonly sponsee: AccountId32;
      readonly encryptedServer: Bytes;
    } & Struct;
    readonly isActivate: boolean;
    readonly isClaimRewards: boolean;
    readonly asClaimRewards: {
      readonly amount: u128;
    } & Struct;
    readonly type:
      | 'Register'
      | 'SetRewardConfig'
      | 'ForceSetProgress'
      | 'SetEncryptedServerForSponsee'
      | 'Activate'
      | 'ClaimRewards';
  }

  /** @name PalletOperationalAccountsRegistration (275) */
  interface PalletOperationalAccountsRegistration extends Enum {
    readonly isV1: boolean;
    readonly asV1: PalletOperationalAccountsRegistrationV1;
    readonly type: 'V1';
  }

  /** @name PalletOperationalAccountsRegistrationV1 (276) */
  interface PalletOperationalAccountsRegistrationV1 extends Struct {
    readonly operationalAccount: AccountId32;
    readonly encryptionPubkey: PalletOperationalAccountsOpaqueEncryptionPubkey;
    readonly operationalAccountProof: PalletOperationalAccountsAccountOwnershipProof;
    readonly vaultAccount: AccountId32;
    readonly miningFundingAccount: AccountId32;
    readonly miningBotAccount: AccountId32;
    readonly vaultAccountProof: PalletOperationalAccountsAccountOwnershipProof;
    readonly miningFundingAccountProof: PalletOperationalAccountsAccountOwnershipProof;
    readonly miningBotAccountProof: PalletOperationalAccountsAccountOwnershipProof;
    readonly referralProof: Option<PalletOperationalAccountsReferralProof>;
  }

  /** @name PalletOperationalAccountsOpaqueEncryptionPubkey (277) */
  interface PalletOperationalAccountsOpaqueEncryptionPubkey extends U8aFixed {}

  /** @name PalletOperationalAccountsAccountOwnershipProof (278) */
  interface PalletOperationalAccountsAccountOwnershipProof extends Struct {
    readonly signature: SpRuntimeMultiSignature;
  }

  /** @name PalletOperationalAccountsReferralProof (280) */
  interface PalletOperationalAccountsReferralProof extends Struct {
    readonly referralCode: U8aFixed;
    readonly referralSignature: U8aFixed;
    readonly sponsor: AccountId32;
    readonly expiresAtFrame: Compact<u64>;
    readonly sponsorSignature: SpRuntimeMultiSignature;
  }

  /** @name PalletOperationalAccountsOperationalProgressPatch (281) */
  interface PalletOperationalAccountsOperationalProgressPatch extends Struct {
    readonly hasUniswapTransfer: Option<bool>;
    readonly vaultCreated: Option<bool>;
    readonly hasTreasuryPoolParticipation: Option<bool>;
    readonly observedBitcoinTotal: Option<u128>;
    readonly observedMiningSeatTotal: Option<u32>;
  }

  /** @name PalletEthereumVerifierCall (283) */
  interface PalletEthereumVerifierCall extends Enum {
    readonly isForceCheckpoint: boolean;
    readonly asForceCheckpoint: {
      readonly update: PalletEthereumVerifierCheckpointUpdate;
      readonly forkVersions: PalletEthereumVerifierForkVersions;
    } & Struct;
    readonly isSubmit: boolean;
    readonly asSubmit: {
      readonly update: PalletEthereumVerifierUpdate;
    } & Struct;
    readonly isImportTrustedExecutionHeaderBackfill: boolean;
    readonly asImportTrustedExecutionHeaderBackfill: {
      readonly expectedBeaconRoot: H256;
      readonly header: SnowbridgeBeaconPrimitivesBeaconHeader;
      readonly executionHeaderProof: PalletEthereumVerifierExecutionHeaderProof;
    } & Struct;
    readonly isSetOperatingMode: boolean;
    readonly asSetOperatingMode: {
      readonly mode: PalletEthereumVerifierBasicOperatingMode;
    } & Struct;
    readonly type:
      | 'ForceCheckpoint'
      | 'Submit'
      | 'ImportTrustedExecutionHeaderBackfill'
      | 'SetOperatingMode';
  }

  /** @name PalletEthereumVerifierCheckpointUpdate (284) */
  interface PalletEthereumVerifierCheckpointUpdate extends Struct {
    readonly header: SnowbridgeBeaconPrimitivesBeaconHeader;
    readonly currentSyncCommittee: PalletEthereumVerifierSyncCommittee;
    readonly currentSyncCommitteeBranch: Vec<H256>;
    readonly validatorsRoot: H256;
    readonly executionHeaderProof: PalletEthereumVerifierExecutionHeaderProof;
  }

  /** @name SnowbridgeBeaconPrimitivesBeaconHeader (285) */
  interface SnowbridgeBeaconPrimitivesBeaconHeader extends Struct {
    readonly slot: u64;
    readonly proposerIndex: u64;
    readonly parentRoot: H256;
    readonly stateRoot: H256;
    readonly bodyRoot: H256;
  }

  /** @name PalletEthereumVerifierSyncCommittee (286) */
  interface PalletEthereumVerifierSyncCommittee extends Struct {
    readonly pubkeys: Vec<SnowbridgeBeaconPrimitivesPublicKey>;
    readonly aggregatePubkey: SnowbridgeBeaconPrimitivesPublicKey;
  }

  /** @name SnowbridgeBeaconPrimitivesPublicKey (288) */
  interface SnowbridgeBeaconPrimitivesPublicKey extends U8aFixed {}

  /** @name PalletEthereumVerifierExecutionHeaderProof (292) */
  interface PalletEthereumVerifierExecutionHeaderProof extends Struct {
    readonly executionHeader: SnowbridgeBeaconPrimitivesVersionedExecutionPayloadHeader;
    readonly executionBranch: Vec<H256>;
  }

  /** @name SnowbridgeBeaconPrimitivesVersionedExecutionPayloadHeader (293) */
  interface SnowbridgeBeaconPrimitivesVersionedExecutionPayloadHeader extends Enum {
    readonly isCapella: boolean;
    readonly asCapella: SnowbridgeBeaconPrimitivesExecutionPayloadHeader;
    readonly isDeneb: boolean;
    readonly asDeneb: SnowbridgeBeaconPrimitivesDenebExecutionPayloadHeader;
    readonly type: 'Capella' | 'Deneb';
  }

  /** @name SnowbridgeBeaconPrimitivesExecutionPayloadHeader (294) */
  interface SnowbridgeBeaconPrimitivesExecutionPayloadHeader extends Struct {
    readonly parentHash: H256;
    readonly feeRecipient: H160;
    readonly stateRoot: H256;
    readonly receiptsRoot: H256;
    readonly logsBloom: Bytes;
    readonly prevRandao: H256;
    readonly blockNumber: u64;
    readonly gasLimit: u64;
    readonly gasUsed: u64;
    readonly timestamp: u64;
    readonly extraData: Bytes;
    readonly baseFeePerGas: U256;
    readonly blockHash: H256;
    readonly transactionsRoot: H256;
    readonly withdrawalsRoot: H256;
  }

  /** @name SnowbridgeBeaconPrimitivesDenebExecutionPayloadHeader (295) */
  interface SnowbridgeBeaconPrimitivesDenebExecutionPayloadHeader extends Struct {
    readonly parentHash: H256;
    readonly feeRecipient: H160;
    readonly stateRoot: H256;
    readonly receiptsRoot: H256;
    readonly logsBloom: Bytes;
    readonly prevRandao: H256;
    readonly blockNumber: u64;
    readonly gasLimit: u64;
    readonly gasUsed: u64;
    readonly timestamp: u64;
    readonly extraData: Bytes;
    readonly baseFeePerGas: U256;
    readonly blockHash: H256;
    readonly transactionsRoot: H256;
    readonly withdrawalsRoot: H256;
    readonly blobGasUsed: u64;
    readonly excessBlobGas: u64;
  }

  /** @name PalletEthereumVerifierForkVersions (296) */
  interface PalletEthereumVerifierForkVersions extends Struct {
    readonly genesis: PalletEthereumVerifierFork;
    readonly altair: PalletEthereumVerifierFork;
    readonly bellatrix: PalletEthereumVerifierFork;
    readonly capella: PalletEthereumVerifierFork;
    readonly deneb: PalletEthereumVerifierFork;
    readonly electra: PalletEthereumVerifierFork;
    readonly fulu: PalletEthereumVerifierFork;
  }

  /** @name PalletEthereumVerifierFork (297) */
  interface PalletEthereumVerifierFork extends Struct {
    readonly version: U8aFixed;
    readonly epoch: Compact<u64>;
  }

  /** @name PalletEthereumVerifierUpdate (298) */
  interface PalletEthereumVerifierUpdate extends Struct {
    readonly attestedHeader: SnowbridgeBeaconPrimitivesBeaconHeader;
    readonly syncAggregate: PalletEthereumVerifierSyncAggregate;
    readonly signatureSlot: Compact<u64>;
    readonly nextSyncCommitteeUpdate: Option<PalletEthereumVerifierNextSyncCommitteeUpdate>;
    readonly finalizedHeader: SnowbridgeBeaconPrimitivesBeaconHeader;
    readonly finalityBranch: Vec<H256>;
    readonly executionHeaderProof: PalletEthereumVerifierExecutionHeaderProof;
  }

  /** @name PalletEthereumVerifierSyncAggregate (299) */
  interface PalletEthereumVerifierSyncAggregate extends Struct {
    readonly syncCommitteeBits: Bytes;
    readonly syncCommitteeSignature: SnowbridgeBeaconPrimitivesSignature;
  }

  /** @name SnowbridgeBeaconPrimitivesSignature (301) */
  interface SnowbridgeBeaconPrimitivesSignature extends U8aFixed {}

  /** @name PalletEthereumVerifierNextSyncCommitteeUpdate (304) */
  interface PalletEthereumVerifierNextSyncCommitteeUpdate extends Struct {
    readonly nextSyncCommittee: PalletEthereumVerifierSyncCommittee;
    readonly nextSyncCommitteeBranch: Vec<H256>;
  }

  /** @name PalletCrosschainTransferCall (305) */
  interface PalletCrosschainTransferCall extends Enum {
    readonly isSetChainConfig: boolean;
    readonly asSetChainConfig: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly config: PalletCrosschainTransferChainConfig;
    } & Struct;
    readonly isForceSetGlobalIssuanceCouncil: boolean;
    readonly asForceSetGlobalIssuanceCouncil: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly afterNonce: Compact<u64>;
      readonly memberAccountIds: Vec<AccountId32>;
    } & Struct;
    readonly isPauseGateway: boolean;
    readonly asPauseGateway: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
    } & Struct;
    readonly isUnpauseGateway: boolean;
    readonly asUnpauseGateway: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
    } & Struct;
    readonly isSetMinimumMintingAuthorityValue: boolean;
    readonly asSetMinimumMintingAuthorityValue: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly minimumValue: Compact<u128>;
    } & Struct;
    readonly isSetMintingAuthorityActivationRepaymentPricing: boolean;
    readonly asSetMintingAuthorityActivationRepaymentPricing: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly pricing: PalletCrosschainTransferMintingAuthorityActivationRepaymentPricing;
    } & Struct;
    readonly isRegisterCouncilSigner: boolean;
    readonly asRegisterCouncilSigner: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly signingKey: H160;
      readonly signature: U8aFixed;
    } & Struct;
    readonly isApproveQueueEntries: boolean;
    readonly asApproveQueueEntries: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly signatures: Vec<U8aFixed>;
    } & Struct;
    readonly isProveGatewayActivity: boolean;
    readonly asProveGatewayActivity: {
      readonly sourceChain: PalletCrosschainTransferSourceChain;
      readonly previousGatewayActivityNonce: Compact<u64>;
      readonly proofBatch: ArgonPrimitivesEthereumEthereumReceiptLogProofBatch;
    } & Struct;
    readonly isRegisterMintingAuthority: boolean;
    readonly asRegisterMintingAuthority: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly destinationSigningKey: H160;
      readonly signature: U8aFixed;
      readonly microgonCollateral: Compact<u128>;
      readonly micronotCollateral: Compact<u128>;
    } & Struct;
    readonly isDeactivateMintingAuthority: boolean;
    readonly asDeactivateMintingAuthority: {
      readonly destinationSigningKey: H160;
    } & Struct;
    readonly isTransferOut: boolean;
    readonly asTransferOut: {
      readonly destinationChain: PalletCrosschainTransferSourceChain;
      readonly asset: PalletCrosschainTransferAssetKind;
      readonly destinationAccount: H160;
      readonly amount: Compact<u128>;
    } & Struct;
    readonly isCollateralizeTransfer: boolean;
    readonly asCollateralizeTransfer: {
      readonly transferId: H256;
      readonly signature: U8aFixed;
      readonly microgonCollateral: Compact<u128>;
      readonly micronotCollateral: Compact<u128>;
    } & Struct;
    readonly type:
      | 'SetChainConfig'
      | 'ForceSetGlobalIssuanceCouncil'
      | 'PauseGateway'
      | 'UnpauseGateway'
      | 'SetMinimumMintingAuthorityValue'
      | 'SetMintingAuthorityActivationRepaymentPricing'
      | 'RegisterCouncilSigner'
      | 'ApproveQueueEntries'
      | 'ProveGatewayActivity'
      | 'RegisterMintingAuthority'
      | 'DeactivateMintingAuthority'
      | 'TransferOut'
      | 'CollateralizeTransfer';
  }

  /** @name PalletCrosschainTransferChainConfig (306) */
  interface PalletCrosschainTransferChainConfig extends Enum {
    readonly isEvm: boolean;
    readonly asEvm: {
      readonly chainId: Compact<u64>;
      readonly gateway: H160;
      readonly argonToken: H160;
      readonly argonotToken: H160;
    } & Struct;
    readonly type: 'Evm';
  }

  /** @name PalletCrosschainTransferMintingAuthorityActivationRepaymentPricing (308) */
  interface PalletCrosschainTransferMintingAuthorityActivationRepaymentPricing extends Struct {
    readonly activationGasCost: Compact<u128>;
    readonly signatureGasCost: Compact<u128>;
    readonly estimatedWeiPerGas: Compact<u128>;
    readonly estimatedMicrogonsPerEth: u128;
  }

  /** @name ArgonPrimitivesEthereumEthereumReceiptLogProofBatch (311) */
  interface ArgonPrimitivesEthereumEthereumReceiptLogProofBatch extends Struct {
    readonly executionBlockProof: ArgonPrimitivesEthereumEthereumExecutionBlockProof;
    readonly blocks: Vec<ArgonPrimitivesEthereumEthereumReceiptLogProofBlock>;
  }

  /** @name ArgonPrimitivesEthereumEthereumExecutionBlockProof (312) */
  interface ArgonPrimitivesEthereumEthereumExecutionBlockProof extends Struct {
    readonly anchorBlockHash: H256;
    readonly targetToAnchorHeaderChain: Vec<ArgonPrimitivesEthereumEthereumExecutionHeader>;
  }

  /** @name ArgonPrimitivesEthereumEthereumExecutionHeader (314) */
  interface ArgonPrimitivesEthereumEthereumExecutionHeader extends Struct {
    readonly rlp: Bytes;
  }

  /** @name ArgonPrimitivesEthereumEthereumReceiptLogProofBlock (318) */
  interface ArgonPrimitivesEthereumEthereumReceiptLogProofBlock extends Struct {
    readonly targetBlockNumber: Compact<u64>;
    readonly receiptProof: ArgonPrimitivesEthereumEthereumCombinedReceiptProof;
    readonly receiptLogs: Vec<ArgonPrimitivesEthereumEthereumReceiptLog>;
  }

  /** @name ArgonPrimitivesEthereumEthereumCombinedReceiptProof (319) */
  interface ArgonPrimitivesEthereumEthereumCombinedReceiptProof extends Struct {
    readonly nodes: Vec<Bytes>;
    readonly receipts: Vec<ArgonPrimitivesEthereumEthereumReceiptProofReceipt>;
  }

  /** @name ArgonPrimitivesEthereumEthereumReceiptProofReceipt (323) */
  interface ArgonPrimitivesEthereumEthereumReceiptProofReceipt extends Struct {
    readonly transactionIndex: Compact<u64>;
    readonly nodeIndexes: Vec<u16>;
  }

  /** @name ArgonPrimitivesEthereumEthereumReceiptLog (328) */
  interface ArgonPrimitivesEthereumEthereumReceiptLog extends Struct {
    readonly transactionIndex: Compact<u64>;
    readonly eventLog: ArgonPrimitivesEthereumEthereumLog;
  }

  /** @name ArgonPrimitivesEthereumEthereumLog (329) */
  interface ArgonPrimitivesEthereumEthereumLog extends Struct {
    readonly address: H160;
    readonly topics: Vec<H256>;
    readonly data: Bytes;
  }

  /** @name PalletMultisigError (335) */
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
    readonly type:
      | 'MinimumThreshold'
      | 'AlreadyApproved'
      | 'NoApprovalsNeeded'
      | 'TooFewSignatories'
      | 'TooManySignatories'
      | 'SignatoriesOutOfOrder'
      | 'SenderInSignatories'
      | 'NotFound'
      | 'NotOwner'
      | 'NoTimepoint'
      | 'WrongTimepoint'
      | 'UnexpectedTimepoint'
      | 'MaxWeightTooLow'
      | 'AlreadyStored';
  }

  /** @name PalletProxyProxyDefinition (338) */
  interface PalletProxyProxyDefinition extends Struct {
    readonly delegate: AccountId32;
    readonly proxyType: ArgonRuntimeProxyType;
    readonly delay: u32;
  }

  /** @name PalletProxyAnnouncement (342) */
  interface PalletProxyAnnouncement extends Struct {
    readonly real: AccountId32;
    readonly callHash: H256;
    readonly height: u32;
  }

  /** @name PalletProxyError (344) */
  interface PalletProxyError extends Enum {
    readonly isTooMany: boolean;
    readonly isNotFound: boolean;
    readonly isNotProxy: boolean;
    readonly isUnproxyable: boolean;
    readonly isDuplicate: boolean;
    readonly isNoPermission: boolean;
    readonly isUnannounced: boolean;
    readonly isNoSelfProxy: boolean;
    readonly type:
      | 'TooMany'
      | 'NotFound'
      | 'NotProxy'
      | 'Unproxyable'
      | 'Duplicate'
      | 'NoPermission'
      | 'Unannounced'
      | 'NoSelfProxy';
  }

  /** @name ArgonPrimitivesTickTicker (345) */
  interface ArgonPrimitivesTickTicker extends Struct {
    readonly tickDurationMillis: Compact<u64>;
    readonly channelHoldExpirationTicks: Compact<u64>;
  }

  /** @name PalletTicksError (347) */
  type PalletTicksError = Null;

  /** @name PalletMiningSlotMinerNonceScoring (350) */
  interface PalletMiningSlotMinerNonceScoring extends Struct {
    readonly nonce: U256;
    readonly lastWinBlock: Option<u32>;
    readonly blocksWonInFrame: u16;
    readonly frameStartBlocksWonSurplus: i16;
  }

  /** @name ArgonPrimitivesBlockSealMiningBidStats (362) */
  interface ArgonPrimitivesBlockSealMiningBidStats extends Struct {
    readonly bidsCount: u32;
    readonly bidAmountMin: u128;
    readonly bidAmountMax: u128;
    readonly bidAmountSum: u128;
  }

  /** @name ArgonPrimitivesBlockSealMiningSlotConfig (366) */
  interface ArgonPrimitivesBlockSealMiningSlotConfig extends Struct {
    readonly ticksBeforeBidEndForVrfClose: Compact<u64>;
    readonly ticksBetweenSlots: Compact<u64>;
    readonly slotBiddingStartAfterTicks: Compact<u64>;
  }

  /** @name PalletMiningSlotError (376) */
  interface PalletMiningSlotError extends Enum {
    readonly isSlotNotTakingBids: boolean;
    readonly isTooManyBlockRegistrants: boolean;
    readonly isInsufficientOwnershipTokens: boolean;
    readonly isBidTooLow: boolean;
    readonly isCannotRegisterOverlappingSessions: boolean;
    readonly isCannotChangeFundingAccount: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isBidCannotBeReduced: boolean;
    readonly isInvalidBidAmount: boolean;
    readonly isOperationalAccountRegistrationRequired: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly type:
      | 'SlotNotTakingBids'
      | 'TooManyBlockRegistrants'
      | 'InsufficientOwnershipTokens'
      | 'BidTooLow'
      | 'CannotRegisterOverlappingSessions'
      | 'CannotChangeFundingAccount'
      | 'InsufficientFunds'
      | 'BidCannotBeReduced'
      | 'InvalidBidAmount'
      | 'OperationalAccountRegistrationRequired'
      | 'UnrecoverableHold';
  }

  /** @name ArgonPrimitivesBitcoinUtxoValue (377) */
  interface ArgonPrimitivesBitcoinUtxoValue extends Struct {
    readonly utxoId: u64;
    readonly scriptPubkey: ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey;
    readonly satoshis: Compact<u64>;
    readonly submittedAtHeight: Compact<u64>;
    readonly watchForSpentUntilHeight: Compact<u64>;
  }

  /** @name ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey (378) */
  interface ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey extends Enum {
    readonly isP2wsh: boolean;
    readonly asP2wsh: {
      readonly wscriptHash: H256;
    } & Struct;
    readonly type: 'P2wsh';
  }

  /** @name ArgonPrimitivesBitcoinBitcoinNetwork (387) */
  interface ArgonPrimitivesBitcoinBitcoinNetwork extends Enum {
    readonly isBitcoin: boolean;
    readonly isTestnet: boolean;
    readonly isSignet: boolean;
    readonly isRegtest: boolean;
    readonly type: 'Bitcoin' | 'Testnet' | 'Signet' | 'Regtest';
  }

  /** @name PalletBitcoinUtxosError (388) */
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
    readonly isDuplicateUtxoId: boolean;
    readonly isMaxCandidateUtxosExceeded: boolean;
    readonly isUtxoNotCandidate: boolean;
    readonly isLockAlreadyFunded: boolean;
    readonly type:
      | 'NoPermissions'
      | 'NoBitcoinConfirmedBlock'
      | 'InsufficientBitcoinAmount'
      | 'NoBitcoinPricesAvailable'
      | 'ScriptPubkeyConflict'
      | 'UtxoNotLocked'
      | 'RedemptionsUnavailable'
      | 'InvalidBitcoinSyncHeight'
      | 'BitcoinHeightNotConfirmed'
      | 'MaxUtxosExceeded'
      | 'InvalidBitcoinScript'
      | 'DuplicateUtxoId'
      | 'MaxCandidateUtxosExceeded'
      | 'UtxoNotCandidate'
      | 'LockAlreadyFunded';
  }

  /** @name ArgonPrimitivesVault (389) */
  interface ArgonPrimitivesVault extends Struct {
    readonly operatorAccountId: AccountId32;
    readonly delegateAccountId: Option<AccountId32>;
    readonly name: Option<Bytes>;
    readonly lastNameChangeTick: Option<u64>;
    readonly securitization: Compact<u128>;
    readonly securitizationTarget: Compact<u128>;
    readonly securitizationLocked: Compact<u128>;
    readonly securitizationPendingActivation: Compact<u128>;
    readonly lockedSatoshis: Compact<u64>;
    readonly securitizedSatoshis: Compact<u64>;
    readonly securitizationReleaseSchedule: BTreeMap<u64, u128>;
    readonly securitizationRatio: Compact<u128>;
    readonly isClosed: bool;
    readonly terms: ArgonPrimitivesVaultVaultTerms;
    readonly pendingTerms: Option<ITuple<[u64, ArgonPrimitivesVaultVaultTerms]>>;
    readonly openedTick: Compact<u64>;
    readonly operationalMinimumReleaseTick: Option<u64>;
  }

  /** @name ArgonPrimitivesVaultVaultArgonotCommitment (396) */
  interface ArgonPrimitivesVaultVaultArgonotCommitment extends Struct {
    readonly committedMicronots: Compact<u128>;
    readonly encumberedMicronots: Compact<u128>;
  }

  /** @name ArgonPrimitivesBitcoinBitcoinXPub (398) */
  interface ArgonPrimitivesBitcoinBitcoinXPub extends Struct {
    readonly publicKey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
    readonly depth: Compact<u8>;
    readonly parentFingerprint: U8aFixed;
    readonly childNumber: Compact<u32>;
    readonly chainCode: U8aFixed;
    readonly network: ArgonPrimitivesBitcoinNetworkKind;
  }

  /** @name ArgonPrimitivesBitcoinNetworkKind (400) */
  interface ArgonPrimitivesBitcoinNetworkKind extends Enum {
    readonly isMain: boolean;
    readonly isTest: boolean;
    readonly type: 'Main' | 'Test';
  }

  /** @name PalletVaultsVaultFrameRevenue (409) */
  interface PalletVaultsVaultFrameRevenue extends Struct {
    readonly frameId: Compact<u64>;
    readonly bitcoinLockFeeRevenue: Compact<u128>;
    readonly bitcoinLockFeeCouponValueUsed: Compact<u128>;
    readonly bitcoinLocksCreated: Compact<u32>;
    readonly bitcoinLocksNewLiquidityPromised: Compact<u128>;
    readonly bitcoinLocksReleasedLiquidity: Compact<u128>;
    readonly bitcoinLocksAddedSatoshis: Compact<u64>;
    readonly bitcoinLocksReleasedSatoshis: Compact<u64>;
    readonly securitizationActivated: Compact<u128>;
    readonly securitizationRelockable: Compact<u128>;
    readonly securitization: Compact<u128>;
    readonly treasuryVaultEarnings: Compact<u128>;
    readonly treasuryTotalEarnings: Compact<u128>;
    readonly treasuryVaultCapital: Compact<u128>;
    readonly treasuryExternalCapital: Compact<u128>;
    readonly uncollectedRevenue: Compact<u128>;
  }

  /** @name PalletVaultsRecentCapacityDrop (412) */
  interface PalletVaultsRecentCapacityDrop extends Struct {
    readonly blockNumber: Compact<u32>;
    readonly availableBeforeDrop: Compact<u128>;
    readonly availableAfterDrop: Compact<u128>;
    readonly noFeeFailuresUsed: Compact<u32>;
  }

  /** @name PalletVaultsError (414) */
  interface PalletVaultsError extends Enum {
    readonly isNoMoreVaultIds: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isAccountBelowMinimumBalance: boolean;
    readonly isVaultClosed: boolean;
    readonly isInvalidVaultAmount: boolean;
    readonly isVaultReductionBelowSecuritization: boolean;
    readonly isInvalidSecuritization: boolean;
    readonly isReusedVaultBitcoinXpub: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly isInvalidXpubkey: boolean;
    readonly isWrongXpubNetwork: boolean;
    readonly isUnsafeXpubkey: boolean;
    readonly isUnableToDeriveVaultXpubChild: boolean;
    readonly isBitcoinConversionFailed: boolean;
    readonly isNoPermissions: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isVaultNotYetActive: boolean;
    readonly isNoVaultBitcoinPubkeysAvailable: boolean;
    readonly isTermsModificationOverflow: boolean;
    readonly isTermsChangeAlreadyScheduled: boolean;
    readonly isInternalError: boolean;
    readonly isUnableToGenerateVaultBitcoinPubkey: boolean;
    readonly isFundingChangeAlreadyScheduled: boolean;
    readonly isInvalidBondSharingTerms: boolean;
    readonly isInvalidVaultName: boolean;
    readonly isPendingCosignsBeforeCollect: boolean;
    readonly isPendingOrphanedUtxoCosignsBeforeCollect: boolean;
    readonly isOverdueCollectBlockersBeforeCollect: boolean;
    readonly isAccountAlreadyHasVault: boolean;
    readonly isOperationalAccountRegistrationRequired: boolean;
    readonly isCommittedArgonotsBelowEncumberedBacking: boolean;
    readonly type:
      | 'NoMoreVaultIds'
      | 'InsufficientFunds'
      | 'InsufficientVaultFunds'
      | 'AccountBelowMinimumBalance'
      | 'VaultClosed'
      | 'InvalidVaultAmount'
      | 'VaultReductionBelowSecuritization'
      | 'InvalidSecuritization'
      | 'ReusedVaultBitcoinXpub'
      | 'InvalidBitcoinScript'
      | 'InvalidXpubkey'
      | 'WrongXpubNetwork'
      | 'UnsafeXpubkey'
      | 'UnableToDeriveVaultXpubChild'
      | 'BitcoinConversionFailed'
      | 'NoPermissions'
      | 'HoldUnexpectedlyModified'
      | 'UnrecoverableHold'
      | 'VaultNotFound'
      | 'VaultNotYetActive'
      | 'NoVaultBitcoinPubkeysAvailable'
      | 'TermsModificationOverflow'
      | 'TermsChangeAlreadyScheduled'
      | 'InternalError'
      | 'UnableToGenerateVaultBitcoinPubkey'
      | 'FundingChangeAlreadyScheduled'
      | 'InvalidBondSharingTerms'
      | 'InvalidVaultName'
      | 'PendingCosignsBeforeCollect'
      | 'PendingOrphanedUtxoCosignsBeforeCollect'
      | 'OverdueCollectBlockersBeforeCollect'
      | 'AccountAlreadyHasVault'
      | 'OperationalAccountRegistrationRequired'
      | 'CommittedArgonotsBelowEncumberedBacking';
  }

  /** @name PalletBitcoinLocksLockedBitcoin (415) */
  interface PalletBitcoinLocksLockedBitcoin extends Struct {
    readonly vaultId: Compact<u32>;
    readonly liquidityPromised: Compact<u128>;
    readonly lockedTargetPrice: Compact<u128>;
    readonly ownerAccount: AccountId32;
    readonly securitizationRatio: u128;
    readonly securityFees: Compact<u128>;
    readonly couponPaidFees: Compact<u128>;
    readonly satoshis: Compact<u64>;
    readonly utxoSatoshis: Option<u64>;
    readonly vaultPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
    readonly vaultClaimPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
    readonly vaultXpubSources: ITuple<[U8aFixed, u32, u32]>;
    readonly ownerPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
    readonly vaultClaimHeight: Compact<u64>;
    readonly openClaimHeight: Compact<u64>;
    readonly createdAtHeight: Compact<u64>;
    readonly utxoScriptPubkey: ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey;
    readonly isFunded: bool;
    readonly fundHoldExtensions: BTreeMap<u64, u128>;
    readonly createdAtArgonBlock: Compact<u32>;
  }

  /** @name PalletBitcoinLocksLockReleaseRequest (418) */
  interface PalletBitcoinLocksLockReleaseRequest extends Struct {
    readonly utxoId: Compact<u64>;
    readonly vaultId: Compact<u32>;
    readonly bitcoinNetworkFee: Compact<u64>;
    readonly cosignDueFrame: Compact<u64>;
    readonly toScriptPubkey: Bytes;
    readonly redemptionAmount: Compact<u128>;
  }

  /** @name PalletBitcoinLocksOrphanedUtxo (420) */
  interface PalletBitcoinLocksOrphanedUtxo extends Struct {
    readonly utxoId: Compact<u64>;
    readonly vaultId: Compact<u32>;
    readonly satoshis: Compact<u64>;
    readonly recordedArgonBlockNumber: Compact<u32>;
    readonly cosignRequest: Option<PalletBitcoinLocksOrphanedUtxoCosignRequest>;
  }

  /** @name PalletBitcoinLocksOrphanedUtxoCosignRequest (422) */
  interface PalletBitcoinLocksOrphanedUtxoCosignRequest extends Struct {
    readonly bitcoinNetworkFee: u64;
    readonly toScriptPubkey: Bytes;
    readonly createdAtArgonBlockNumber: u32;
  }

  /** @name PalletBitcoinLocksError (429) */
  interface PalletBitcoinLocksError extends Enum {
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isAccountWouldGoBelowMinimumBalance: boolean;
    readonly isVaultClosed: boolean;
    readonly isInvalidVaultAmount: boolean;
    readonly isRedemptionNotLocked: boolean;
    readonly isBitcoinReleaseInitiationDeadlinePassed: boolean;
    readonly isBitcoinFeeTooHigh: boolean;
    readonly isBitcoinUtxoNotFound: boolean;
    readonly isBitcoinUnableToBeDecodedForRelease: boolean;
    readonly isBitcoinSignatureUnableToBeDecoded: boolean;
    readonly isBitcoinPubkeyUnableToBeDecoded: boolean;
    readonly isBitcoinInvalidCosignature: boolean;
    readonly isInsufficientSatoshisLocked: boolean;
    readonly isNoBitcoinPricesAvailable: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly isNoPermissions: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isGenericVaultError: boolean;
    readonly asGenericVaultError: ArgonPrimitivesVaultVaultError;
    readonly isLockNotFound: boolean;
    readonly isNoVaultBitcoinPubkeysAvailable: boolean;
    readonly isUnableToGenerateVaultBitcoinPubkey: boolean;
    readonly isVaultNotYetActive: boolean;
    readonly isExpirationAtBlockOverflow: boolean;
    readonly isNoRatchetingAvailable: boolean;
    readonly isLockInProcessOfRelease: boolean;
    readonly isLockPendingFunding: boolean;
    readonly isOverflowError: boolean;
    readonly isIneligibleMicrogonRateRequested: boolean;
    readonly isOrphanedUtxoFundingConflict: boolean;
    readonly isOrphanedUtxoReleaseRequested: boolean;
    readonly isFundingUtxoCannotBeReleased: boolean;
    readonly isMaxOrphanedUtxoReleaseRequestsExceeded: boolean;
    readonly type:
      | 'InsufficientFunds'
      | 'InsufficientVaultFunds'
      | 'AccountWouldGoBelowMinimumBalance'
      | 'VaultClosed'
      | 'InvalidVaultAmount'
      | 'RedemptionNotLocked'
      | 'BitcoinReleaseInitiationDeadlinePassed'
      | 'BitcoinFeeTooHigh'
      | 'BitcoinUtxoNotFound'
      | 'BitcoinUnableToBeDecodedForRelease'
      | 'BitcoinSignatureUnableToBeDecoded'
      | 'BitcoinPubkeyUnableToBeDecoded'
      | 'BitcoinInvalidCosignature'
      | 'InsufficientSatoshisLocked'
      | 'NoBitcoinPricesAvailable'
      | 'InvalidBitcoinScript'
      | 'NoPermissions'
      | 'HoldUnexpectedlyModified'
      | 'UnrecoverableHold'
      | 'VaultNotFound'
      | 'GenericVaultError'
      | 'LockNotFound'
      | 'NoVaultBitcoinPubkeysAvailable'
      | 'UnableToGenerateVaultBitcoinPubkey'
      | 'VaultNotYetActive'
      | 'ExpirationAtBlockOverflow'
      | 'NoRatchetingAvailable'
      | 'LockInProcessOfRelease'
      | 'LockPendingFunding'
      | 'OverflowError'
      | 'IneligibleMicrogonRateRequested'
      | 'OrphanedUtxoFundingConflict'
      | 'OrphanedUtxoReleaseRequested'
      | 'FundingUtxoCannotBeReleased'
      | 'MaxOrphanedUtxoReleaseRequestsExceeded';
  }

  /** @name ArgonPrimitivesVaultVaultError (430) */
  interface ArgonPrimitivesVaultVaultError extends Enum {
    readonly isVaultClosed: boolean;
    readonly isAccountWouldBeBelowMinimum: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientVaultFunds: boolean;
    readonly isHoldUnexpectedlyModified: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly isVaultNotFound: boolean;
    readonly isNoVaultBitcoinPubkeysAvailable: boolean;
    readonly isUnableToGenerateVaultBitcoinPubkey: boolean;
    readonly isInvalidBitcoinScript: boolean;
    readonly isInternalError: boolean;
    readonly isVaultNotYetActive: boolean;
    readonly isCommittedArgonotsBelowEncumberedBacking: boolean;
    readonly type:
      | 'VaultClosed'
      | 'AccountWouldBeBelowMinimum'
      | 'InsufficientFunds'
      | 'InsufficientVaultFunds'
      | 'HoldUnexpectedlyModified'
      | 'UnrecoverableHold'
      | 'VaultNotFound'
      | 'NoVaultBitcoinPubkeysAvailable'
      | 'UnableToGenerateVaultBitcoinPubkey'
      | 'InvalidBitcoinScript'
      | 'InternalError'
      | 'VaultNotYetActive'
      | 'CommittedArgonotsBelowEncumberedBacking';
  }

  /** @name PalletNotariesError (442) */
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
    readonly type:
      | 'ProposalNotFound'
      | 'MaxNotariesExceeded'
      | 'MaxProposalsPerBlockExceeded'
      | 'NotAnActiveNotary'
      | 'InvalidNotaryOperator'
      | 'NoMoreNotaryIds'
      | 'EffectiveTickTooSoon'
      | 'TooManyKeys'
      | 'InvalidNotary';
  }

  /** @name ArgonPrimitivesNotaryNotaryNotebookKeyDetails (446) */
  interface ArgonPrimitivesNotaryNotaryNotebookKeyDetails extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly blockVotesRoot: H256;
    readonly secretHash: H256;
    readonly parentSecret: Option<H256>;
  }

  /** @name PalletNotebookError (449) */
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
    readonly isNotebookSubmittedForLockedNotary: boolean;
    readonly isInvalidReprocessNotebook: boolean;
    readonly isInvalidNotaryOperator: boolean;
    readonly isInvalidNotebookSubmissionTick: boolean;
    readonly type:
      | 'DuplicateNotebookNumber'
      | 'MissingNotebookNumber'
      | 'NotebookTickAlreadyUsed'
      | 'InvalidNotebookSignature'
      | 'InvalidSecretProvided'
      | 'CouldNotDecodeNotebook'
      | 'DuplicateNotebookDigest'
      | 'MissingNotebookDigest'
      | 'InvalidNotebookDigest'
      | 'MultipleNotebookInherentsProvided'
      | 'InternalError'
      | 'NotebookSubmittedForLockedNotary'
      | 'InvalidReprocessNotebook'
      | 'InvalidNotaryOperator'
      | 'InvalidNotebookSubmissionTick';
  }

  /** @name PalletLocalchainTransferQueuedTransferOut (450) */
  interface PalletLocalchainTransferQueuedTransferOut extends Struct {
    readonly accountId: AccountId32;
    readonly amount: u128;
    readonly expirationTick: u64;
    readonly notaryId: u32;
  }

  /** @name FrameSupportPalletId (452) */
  interface FrameSupportPalletId extends U8aFixed {}

  /** @name PalletLocalchainTransferError (453) */
  interface PalletLocalchainTransferError extends Enum {
    readonly isMaxBlockTransfersExceeded: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientNotarizedFunds: boolean;
    readonly isInvalidOrDuplicatedLocalchainTransfer: boolean;
    readonly isNotebookIncludesExpiredLocalchainTransfer: boolean;
    readonly isInvalidNotaryUsedForTransfer: boolean;
    readonly isNotaryLockedForTransfer: boolean;
    readonly isNoAvailableTransferId: boolean;
    readonly type:
      | 'MaxBlockTransfersExceeded'
      | 'InsufficientFunds'
      | 'InsufficientNotarizedFunds'
      | 'InvalidOrDuplicatedLocalchainTransfer'
      | 'NotebookIncludesExpiredLocalchainTransfer'
      | 'InvalidNotaryUsedForTransfer'
      | 'NotaryLockedForTransfer'
      | 'NoAvailableTransferId';
  }

  /** @name ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails (457) */
  interface ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly blockVotesCount: Compact<u32>;
    readonly blockVotingPower: Compact<u128>;
  }

  /** @name PalletBlockSealSpecError (462) */
  interface PalletBlockSealSpecError extends Enum {
    readonly isMaxNotebooksAtTickExceeded: boolean;
    readonly type: 'MaxNotebooksAtTickExceeded';
  }

  /** @name PalletDomainsError (464) */
  interface PalletDomainsError extends Enum {
    readonly isDomainNotRegistered: boolean;
    readonly isNotDomainOwner: boolean;
    readonly isFailedToAddToAddressHistory: boolean;
    readonly isFailedToAddExpiringDomain: boolean;
    readonly isAccountDecodingError: boolean;
    readonly type:
      | 'DomainNotRegistered'
      | 'NotDomainOwner'
      | 'FailedToAddToAddressHistory'
      | 'FailedToAddExpiringDomain'
      | 'AccountDecodingError';
  }

  /** @name PalletPriceIndexCpiMeasurementBucket (466) */
  interface PalletPriceIndexCpiMeasurementBucket extends Struct {
    readonly tickRange: ITuple<[u64, u64]>;
    readonly totalCpi: i128;
    readonly measurementsCount: u32;
  }

  /** @name PalletPriceIndexArgonotAverageFrameAccumulator (470) */
  interface PalletPriceIndexArgonotAverageFrameAccumulator extends Struct {
    readonly frameId: Compact<u64>;
    readonly totalMicrogonsPerArgonot: Compact<u128>;
    readonly sampleCount: Compact<u32>;
  }

  /** @name PalletPriceIndexError (471) */
  interface PalletPriceIndexError extends Enum {
    readonly isNotAuthorizedOperator: boolean;
    readonly isMissingValue: boolean;
    readonly isPricesTooOld: boolean;
    readonly isMaxPriceChangePerTickExceeded: boolean;
    readonly type:
      | 'NotAuthorizedOperator'
      | 'MissingValue'
      | 'PricesTooOld'
      | 'MaxPriceChangePerTickExceeded';
  }

  /** @name PalletGrandpaStoredState (472) */
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

  /** @name PalletGrandpaStoredPendingChange (473) */
  interface PalletGrandpaStoredPendingChange extends Struct {
    readonly scheduledAt: u32;
    readonly delay: u32;
    readonly nextAuthorities: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    readonly forced: Option<u32>;
  }

  /** @name PalletGrandpaError (475) */
  interface PalletGrandpaError extends Enum {
    readonly isPauseFailed: boolean;
    readonly isResumeFailed: boolean;
    readonly isChangePending: boolean;
    readonly isTooSoon: boolean;
    readonly isInvalidKeyOwnershipProof: boolean;
    readonly isInvalidEquivocationProof: boolean;
    readonly isDuplicateOffenceReport: boolean;
    readonly type:
      | 'PauseFailed'
      | 'ResumeFailed'
      | 'ChangePending'
      | 'TooSoon'
      | 'InvalidKeyOwnershipProof'
      | 'InvalidEquivocationProof'
      | 'DuplicateOffenceReport';
  }

  /** @name ArgonPrimitivesProvidersBlockSealerInfo (476) */
  interface ArgonPrimitivesProvidersBlockSealerInfo extends Struct {
    readonly blockAuthorAccountId: AccountId32;
    readonly blockVoteRewardsAccount: Option<AccountId32>;
    readonly blockSealAuthority: Option<ArgonPrimitivesBlockSealAppPublic>;
  }

  /** @name PalletBlockSealError (478) */
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
    readonly isCouldNotDecodeVote: boolean;
    readonly isMaxNotebooksAtTickExceeded: boolean;
    readonly isNoClosestMinerFoundForVote: boolean;
    readonly isBlockVoteInvalidSignature: boolean;
    readonly isInvalidForkPowerParent: boolean;
    readonly isBlockSealDecodeError: boolean;
    readonly isInvalidComputeBlockTick: boolean;
    readonly isInvalidMinerNonceScore: boolean;
    readonly isDuplicateVoteBlockAtTick: boolean;
    readonly type:
      | 'InvalidVoteSealStrength'
      | 'InvalidSubmitter'
      | 'UnableToDecodeVoteAccount'
      | 'UnregisteredBlockAuthor'
      | 'InvalidBlockVoteProof'
      | 'NoGrandparentVoteMinimum'
      | 'DuplicateBlockSealProvided'
      | 'InsufficientVotingPower'
      | 'ParentVotingKeyNotFound'
      | 'InvalidVoteGrandparentHash'
      | 'IneligibleNotebookUsed'
      | 'NoEligibleVotingRoot'
      | 'CouldNotDecodeVote'
      | 'MaxNotebooksAtTickExceeded'
      | 'NoClosestMinerFoundForVote'
      | 'BlockVoteInvalidSignature'
      | 'InvalidForkPowerParent'
      | 'BlockSealDecodeError'
      | 'InvalidComputeBlockTick'
      | 'InvalidMinerNonceScore'
      | 'DuplicateVoteBlockAtTick';
  }

  /** @name PalletBlockRewardsError (482) */
  type PalletBlockRewardsError = Null;

  /** @name PalletMintPendingMintUtxo (483) */
  interface PalletMintPendingMintUtxo extends Struct {
    readonly utxoId: Compact<u64>;
    readonly accountId: AccountId32;
    readonly remainingAmount: Compact<u128>;
    readonly maxAmountPerFrame: Compact<u128>;
  }

  /** @name PalletMintMintQueueCursor (485) */
  interface PalletMintMintQueueCursor extends Struct {
    readonly payoutStartIndex: Compact<u64>;
    readonly payoutCursorIndex: Compact<u64>;
    readonly payoutCursorFrameId: Option<u64>;
  }

  /** @name PalletMintMintAction (488) */
  interface PalletMintMintAction extends Struct {
    readonly argonBurned: u128;
    readonly argonMinted: u128;
    readonly bitcoinMinted: u128;
  }

  /** @name PalletMintError (490) */
  interface PalletMintError extends Enum {
    readonly isTooManyPendingMints: boolean;
    readonly type: 'TooManyPendingMints';
  }

  /** @name PalletBalancesBalanceLock (492) */
  interface PalletBalancesBalanceLock extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
    readonly reasons: PalletBalancesReasons;
  }

  /** @name PalletBalancesReasons (493) */
  interface PalletBalancesReasons extends Enum {
    readonly isFee: boolean;
    readonly isMisc: boolean;
    readonly isAll: boolean;
    readonly type: 'Fee' | 'Misc' | 'All';
  }

  /** @name PalletBalancesReserveData (496) */
  interface PalletBalancesReserveData extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
  }

  /** @name FrameSupportTokensMiscIdAmountRuntimeHoldReason (499) */
  interface FrameSupportTokensMiscIdAmountRuntimeHoldReason extends Struct {
    readonly id: ArgonRuntimeRuntimeHoldReason;
    readonly amount: u128;
  }

  /** @name FrameSupportTokensMiscIdAmountRuntimeFreezeReason (502) */
  interface FrameSupportTokensMiscIdAmountRuntimeFreezeReason extends Struct {
    readonly id: ArgonRuntimeRuntimeFreezeReason;
    readonly amount: u128;
  }

  /** @name ArgonRuntimeRuntimeFreezeReason (503) */
  interface ArgonRuntimeRuntimeFreezeReason extends Enum {
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsFreezeReason;
    readonly type: 'BlockRewards';
  }

  /** @name PalletBlockRewardsFreezeReason (504) */
  interface PalletBlockRewardsFreezeReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletBalancesError (506) */
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
    readonly type:
      | 'VestingBalance'
      | 'LiquidityRestrictions'
      | 'InsufficientBalance'
      | 'ExistentialDeposit'
      | 'Expendability'
      | 'ExistingVestingSchedule'
      | 'DeadAccount'
      | 'TooManyReserves'
      | 'TooManyHolds'
      | 'TooManyFreezes'
      | 'IssuanceDeactivated'
      | 'DeltaZero';
  }

  /** @name PalletTxPauseError (508) */
  interface PalletTxPauseError extends Enum {
    readonly isIsPaused: boolean;
    readonly isIsUnpaused: boolean;
    readonly isUnpausable: boolean;
    readonly isNotFound: boolean;
    readonly type: 'IsPaused' | 'IsUnpaused' | 'Unpausable' | 'NotFound';
  }

  /** @name PalletTransactionPaymentReleases (509) */
  interface PalletTransactionPaymentReleases extends Enum {
    readonly isV1Ancient: boolean;
    readonly isV2: boolean;
    readonly type: 'V1Ancient' | 'V2';
  }

  /** @name FrameSupportStorageNoDrop (510) */
  interface FrameSupportStorageNoDrop extends FrameSupportTokensFungibleImbalance {}

  /** @name FrameSupportTokensFungibleImbalance (511) */
  interface FrameSupportTokensFungibleImbalance extends Struct {
    readonly amount: u128;
  }

  /** @name PalletUtilityError (512) */
  interface PalletUtilityError extends Enum {
    readonly isTooManyCalls: boolean;
    readonly type: 'TooManyCalls';
  }

  /** @name PalletSudoError (513) */
  interface PalletSudoError extends Enum {
    readonly isRequireSudo: boolean;
    readonly type: 'RequireSudo';
  }

  /** @name PalletTreasuryFrameVaultCapital (514) */
  interface PalletTreasuryFrameVaultCapital extends Struct {
    readonly frameId: Compact<u64>;
    readonly vaults: BTreeMap<u32, PalletTreasuryVaultCapital>;
  }

  /** @name PalletTreasuryVaultCapital (516) */
  interface PalletTreasuryVaultCapital extends Struct {
    readonly bondLotAllocations: Vec<PalletTreasuryBondLotAllocation>;
    readonly eligibleBonds: Compact<u32>;
  }

  /** @name PalletTreasuryBondLotAllocation (518) */
  interface PalletTreasuryBondLotAllocation extends Struct {
    readonly bondLotId: Compact<u64>;
    readonly prorata: u128;
  }

  /** @name PalletTreasuryFrameArgonotBondParticipants (523) */
  interface PalletTreasuryFrameArgonotBondParticipants extends Struct {
    readonly frameId: Compact<u64>;
    readonly totalBonds: Compact<u32>;
    readonly bondLots: Vec<PalletTreasuryBondLotSummary>;
  }

  /** @name PalletTreasuryBondLotSummary (525) */
  interface PalletTreasuryBondLotSummary extends Struct {
    readonly bondLotId: Compact<u64>;
    readonly bonds: Compact<u32>;
  }

  /** @name PalletTreasuryBondLot (527) */
  interface PalletTreasuryBondLot extends Struct {
    readonly owner: AccountId32;
    readonly program: PalletTreasuryBondProgram;
    readonly bonds: Compact<u32>;
    readonly createdFrameId: Compact<u64>;
    readonly participatedFrames: Compact<u32>;
    readonly lastFrameEarningsFrameId: Option<u64>;
    readonly lastFrameEarnings: Option<u128>;
    readonly cumulativeEarnings: Compact<u128>;
    readonly releaseFrameId: Option<u64>;
    readonly releaseReason: Option<PalletTreasuryBondReleaseReason>;
  }

  /** @name PalletTreasuryBondProgram (528) */
  interface PalletTreasuryBondProgram extends Enum {
    readonly isVault: boolean;
    readonly asVault: {
      readonly vaultId: Compact<u32>;
      readonly sharingPercent: Compact<Permill>;
      readonly bonusPercent: Compact<Permill>;
    } & Struct;
    readonly isArgonot: boolean;
    readonly type: 'Vault' | 'Argonot';
  }

  /** @name PalletTreasuryError (533) */
  interface PalletTreasuryError extends Enum {
    readonly isBondPurchaseRejected: boolean;
    readonly isVaultNotAcceptingBondPurchases: boolean;
    readonly isBondPurchaseBelowMinimum: boolean;
    readonly isInternalError: boolean;
    readonly isMaxAcceptedBondLotsExceeded: boolean;
    readonly isMaxPendingBondReleasesExceeded: boolean;
    readonly isBondLotNotFound: boolean;
    readonly isNotBondLotOwner: boolean;
    readonly isBondLotAlreadyReleasing: boolean;
    readonly isBondPurchaseAboveSecurity: boolean;
    readonly isActiveBondAmountBelowEncumberedBacking: boolean;
    readonly isBonusApprovalWrongVault: boolean;
    readonly isBonusApprovalWrongAccount: boolean;
    readonly isBonusApprovalExpired: boolean;
    readonly isBonusApprovalExistingBondLot: boolean;
    readonly isInvalidBonusApprovalSignature: boolean;
    readonly isArgonotBondPurchaseBelowCutoff: boolean;
    readonly isArgonotBondPurchaseAboveCap: boolean;
    readonly type:
      | 'BondPurchaseRejected'
      | 'VaultNotAcceptingBondPurchases'
      | 'BondPurchaseBelowMinimum'
      | 'InternalError'
      | 'MaxAcceptedBondLotsExceeded'
      | 'MaxPendingBondReleasesExceeded'
      | 'BondLotNotFound'
      | 'NotBondLotOwner'
      | 'BondLotAlreadyReleasing'
      | 'BondPurchaseAboveSecurity'
      | 'ActiveBondAmountBelowEncumberedBacking'
      | 'BonusApprovalWrongVault'
      | 'BonusApprovalWrongAccount'
      | 'BonusApprovalExpired'
      | 'BonusApprovalExistingBondLot'
      | 'InvalidBonusApprovalSignature'
      | 'ArgonotBondPurchaseBelowCutoff'
      | 'ArgonotBondPurchaseAboveCap';
  }

  /** @name PalletFeeControlError (534) */
  interface PalletFeeControlError extends Enum {
    readonly isSponsoredFeeTooHigh: boolean;
    readonly type: 'SponsoredFeeTooHigh';
  }

  /** @name PalletOperationalAccountsOperationalAccount (535) */
  interface PalletOperationalAccountsOperationalAccount extends Struct {
    readonly vaultAccount: AccountId32;
    readonly miningFundingAccount: AccountId32;
    readonly miningBotAccount: AccountId32;
    readonly encryptionPubkey: PalletOperationalAccountsOpaqueEncryptionPubkey;
    readonly sponsor: Option<AccountId32>;
    readonly hasUniswapTransfer: bool;
    readonly vaultCreated: bool;
    readonly bitcoinAccrual: u128;
    readonly bitcoinAppliedTotal: u128;
    readonly hasTreasuryPoolParticipation: bool;
    readonly miningSeatAccrual: Compact<u32>;
    readonly miningSeatAppliedTotal: Compact<u32>;
    readonly operationalReferralsCount: Compact<u32>;
    readonly referralPending: bool;
    readonly availableReferrals: Compact<u32>;
    readonly rewardsEarnedCount: Compact<u32>;
    readonly rewardsEarnedAmount: u128;
    readonly rewardsCollectedAmount: u128;
    readonly isOperational: bool;
  }

  /** @name PalletOperationalAccountsRewardsConfig (537) */
  interface PalletOperationalAccountsRewardsConfig extends Struct {
    readonly operationalReferralReward: Compact<u128>;
    readonly referralBonusReward: Compact<u128>;
  }

  /** @name PalletOperationalAccountsError (539) */
  interface PalletOperationalAccountsError extends Enum {
    readonly isAlreadyRegistered: boolean;
    readonly isInvalidRegistrationSubmitter: boolean;
    readonly isAccountAlreadyLinked: boolean;
    readonly isInvalidAccountProof: boolean;
    readonly isNotOperationalAccount: boolean;
    readonly isInvalidReferralProof: boolean;
    readonly isReferralProofExpired: boolean;
    readonly isRegistrationInviteRequired: boolean;
    readonly isNoProgressUpdateProvided: boolean;
    readonly isEncryptedServerTooLong: boolean;
    readonly isNotSponsorOfSponsee: boolean;
    readonly isNoPendingRewards: boolean;
    readonly isRewardClaimBelowMinimum: boolean;
    readonly isRewardClaimNotWholeArgon: boolean;
    readonly isRewardClaimExceedsPending: boolean;
    readonly isTreasuryInsufficientFunds: boolean;
    readonly isAlreadyOperational: boolean;
    readonly isNotEligibleForActivation: boolean;
    readonly type:
      | 'AlreadyRegistered'
      | 'InvalidRegistrationSubmitter'
      | 'AccountAlreadyLinked'
      | 'InvalidAccountProof'
      | 'NotOperationalAccount'
      | 'InvalidReferralProof'
      | 'ReferralProofExpired'
      | 'RegistrationInviteRequired'
      | 'NoProgressUpdateProvided'
      | 'EncryptedServerTooLong'
      | 'NotSponsorOfSponsee'
      | 'NoPendingRewards'
      | 'RewardClaimBelowMinimum'
      | 'RewardClaimNotWholeArgon'
      | 'RewardClaimExceedsPending'
      | 'TreasuryInsufficientFunds'
      | 'AlreadyOperational'
      | 'NotEligibleForActivation';
  }

  /** @name PalletEthereumVerifierFinalizedBeaconHeaderState (540) */
  interface PalletEthereumVerifierFinalizedBeaconHeaderState extends Struct {
    readonly slot: Compact<u64>;
  }

  /** @name PalletEthereumVerifierExecutionHeaderAnchor (541) */
  interface PalletEthereumVerifierExecutionHeaderAnchor extends Struct {
    readonly blockNumber: Compact<u64>;
    readonly timestampMillis: Compact<u64>;
    readonly blockHash: H256;
    readonly parentHash: H256;
    readonly stateRoot: H256;
    readonly receiptsRoot: H256;
  }

  /** @name PalletEthereumVerifierSyncCommitteePrepared (542) */
  interface PalletEthereumVerifierSyncCommitteePrepared extends Struct {
    readonly root: H256;
    readonly pubkeys: Vec<SnowbridgeMilagroBlsKeysPublicKey>;
    readonly aggregatePubkey: SnowbridgeMilagroBlsKeysPublicKey;
  }

  /** @name SnowbridgeMilagroBlsKeysPublicKey (544) */
  interface SnowbridgeMilagroBlsKeysPublicKey extends Struct {
    readonly point: SnowbridgeAmclBls381Ecp;
  }

  /** @name SnowbridgeAmclBls381Ecp (545) */
  interface SnowbridgeAmclBls381Ecp extends Struct {
    readonly x: SnowbridgeAmclBls381Fp;
    readonly y: SnowbridgeAmclBls381Fp;
    readonly z: SnowbridgeAmclBls381Fp;
  }

  /** @name SnowbridgeAmclBls381Fp (546) */
  interface SnowbridgeAmclBls381Fp extends Struct {
    readonly x: SnowbridgeAmclBls381Big;
    readonly xes: i32;
  }

  /** @name SnowbridgeAmclBls381Big (547) */
  interface SnowbridgeAmclBls381Big extends Struct {
    readonly w: Vec<i32>;
  }

  /** @name ArgonPrimitivesEthereumEthereumBeaconPreset (551) */
  interface ArgonPrimitivesEthereumEthereumBeaconPreset extends Enum {
    readonly isMainnet: boolean;
    readonly isMinimal: boolean;
    readonly type: 'Mainnet' | 'Minimal';
  }

  /** @name PalletEthereumVerifierError (552) */
  interface PalletEthereumVerifierError extends Enum {
    readonly isSkippedSyncCommitteePeriod: boolean;
    readonly isSyncCommitteeUpdateRequired: boolean;
    readonly isIrrelevantUpdate: boolean;
    readonly isNotBootstrapped: boolean;
    readonly isSyncCommitteeParticipantsNotSupermajority: boolean;
    readonly isInvalidHeaderMerkleProof: boolean;
    readonly isInvalidSyncCommitteeMerkleProof: boolean;
    readonly isInvalidExecutionHeaderProof: boolean;
    readonly isInvalidFinalizedHeaderGap: boolean;
    readonly isInvalidBackfillHeaderRoot: boolean;
    readonly isExecutionHeaderAnchorAlreadyImported: boolean;
    readonly isExecutionHeaderAnchorNotHistorical: boolean;
    readonly isHeaderHashTreeRootFailed: boolean;
    readonly isBlockBodyHashTreeRootFailed: boolean;
    readonly isSyncCommitteeHashTreeRootFailed: boolean;
    readonly isSigningRootHashTreeRootFailed: boolean;
    readonly isForkDataHashTreeRootFailed: boolean;
    readonly isExpectedFinalizedHeaderNotStored: boolean;
    readonly isBlsPreparePublicKeysFailed: boolean;
    readonly isBlsVerificationFailed: boolean;
    readonly isUnexpectedBeaconPreset: boolean;
    readonly isInvalidUpdateSlot: boolean;
    readonly isInvalidSyncCommitteeUpdate: boolean;
    readonly isHalted: boolean;
    readonly type:
      | 'SkippedSyncCommitteePeriod'
      | 'SyncCommitteeUpdateRequired'
      | 'IrrelevantUpdate'
      | 'NotBootstrapped'
      | 'SyncCommitteeParticipantsNotSupermajority'
      | 'InvalidHeaderMerkleProof'
      | 'InvalidSyncCommitteeMerkleProof'
      | 'InvalidExecutionHeaderProof'
      | 'InvalidFinalizedHeaderGap'
      | 'InvalidBackfillHeaderRoot'
      | 'ExecutionHeaderAnchorAlreadyImported'
      | 'ExecutionHeaderAnchorNotHistorical'
      | 'HeaderHashTreeRootFailed'
      | 'BlockBodyHashTreeRootFailed'
      | 'SyncCommitteeHashTreeRootFailed'
      | 'SigningRootHashTreeRootFailed'
      | 'ForkDataHashTreeRootFailed'
      | 'ExpectedFinalizedHeaderNotStored'
      | 'BlsPreparePublicKeysFailed'
      | 'BlsVerificationFailed'
      | 'UnexpectedBeaconPreset'
      | 'InvalidUpdateSlot'
      | 'InvalidSyncCommitteeUpdate'
      | 'Halted';
  }

  /** @name PalletCrosschainTransferGlobalIssuanceCouncil (554) */
  interface PalletCrosschainTransferGlobalIssuanceCouncil extends Struct {
    readonly epochMicrogonsPerArgonot: u128;
    readonly members: BTreeMap<H160, PalletCrosschainTransferGlobalIssuanceCouncilMember>;
    readonly totalWeight: u128;
  }

  /** @name PalletCrosschainTransferGlobalIssuanceCouncilMember (556) */
  interface PalletCrosschainTransferGlobalIssuanceCouncilMember extends Struct {
    readonly accountId: AccountId32;
    readonly signer: H160;
    readonly weight: u128;
  }

  /** @name PalletCrosschainTransferCouncilApprovalQueueEntry (561) */
  interface PalletCrosschainTransferCouncilApprovalQueueEntry extends Struct {
    readonly approvingCouncilHash: H256;
    readonly target: PalletCrosschainTransferCouncilApprovalTargetId;
    readonly targetPayloadHash: H256;
    readonly dueFrameId: Compact<u64>;
    readonly previousApprovalHash: H256;
    readonly approvalHash: H256;
    readonly approvedTotalWeight: u128;
    readonly signatures: BTreeMap<H160, U8aFixed>;
  }

  /** @name PalletCrosschainTransferMintingAuthority (566) */
  interface PalletCrosschainTransferMintingAuthority extends Struct {
    readonly accountId: AccountId32;
    readonly destinationChain: PalletCrosschainTransferSourceChain;
    readonly destinationSigningKey: H160;
    readonly state: PalletCrosschainTransferMintingAuthorityState;
    readonly gatewayRemainingMicrogonCollateral: u128;
    readonly gatewayRemainingMicronotCollateral: u128;
    readonly pendingReservedMicrogonCollateral: u128;
    readonly pendingReservedMicronotCollateral: u128;
    readonly activePendingTransferIds: Vec<H256>;
    readonly activationApprovalQueueNonce: u64;
    readonly activationBaseRepaymentQuote: Compact<u128>;
    readonly activationSignatureRepaymentQuote: Compact<u128>;
    readonly deactivationApprovalQueueNonce: Option<u64>;
  }

  /** @name PalletCrosschainTransferMintingAuthorityState (567) */
  interface PalletCrosschainTransferMintingAuthorityState extends Enum {
    readonly isPendingActivation: boolean;
    readonly isActive: boolean;
    readonly isDeactivating: boolean;
    readonly type: 'PendingActivation' | 'Active' | 'Deactivating';
  }

  /** @name PalletCrosschainTransferTransferOutTransferOutOfArgon (569) */
  interface PalletCrosschainTransferTransferOutTransferOutOfArgon extends Struct {
    readonly argonAccountId: AccountId32;
    readonly argonTransferNonce: Compact<u64>;
    readonly destinationChain: PalletCrosschainTransferSourceChain;
    readonly microgonsPerArgonot: Compact<u128>;
    readonly destinationAccount: H160;
    readonly validUntilEthereumBlock: Compact<u64>;
    readonly asset: PalletCrosschainTransferAssetKind;
    readonly amount: u128;
    readonly mintingAuthorityTip: u128;
    readonly totalAttachedCollateral: u128;
    readonly mintingAuthorityCollateralBySigner: BTreeMap<
      H160,
      PalletCrosschainTransferTransferOutMintingAuthorityTransferReservation
    >;
    readonly state: PalletCrosschainTransferTransferOutTransferOutState;
  }

  /** @name PalletCrosschainTransferTransferOutMintingAuthorityTransferReservation (571) */
  interface PalletCrosschainTransferTransferOutMintingAuthorityTransferReservation extends Struct {
    readonly microgonCollateral: u128;
    readonly micronotCollateral: u128;
    readonly collateralShare: u128;
    readonly signature: U8aFixed;
  }

  /** @name PalletCrosschainTransferTransferOutTransferOutState (575) */
  interface PalletCrosschainTransferTransferOutTransferOutState extends Enum {
    readonly isStarted: boolean;
    readonly isReady: boolean;
    readonly type: 'Started' | 'Ready';
  }

  /** @name PalletCrosschainTransferTransferOutPendingCollateralizationRequest (577) */
  interface PalletCrosschainTransferTransferOutPendingCollateralizationRequest extends Struct {
    readonly transferId: H256;
    readonly remainingCollateral: u128;
    readonly remainingMintingAuthorityTip: u128;
  }

  /** @name PalletCrosschainTransferSourceChainCirculation (579) */
  interface PalletCrosschainTransferSourceChainCirculation extends Struct {
    readonly argonCirculation: u128;
    readonly argonotCirculation: u128;
  }

  /** @name PalletCrosschainTransferError (580) */
  interface PalletCrosschainTransferError extends Enum {
    readonly isInvalidTransferToArgonActivity: boolean;
    readonly isNoGatewayProofBlocksProvided: boolean;
    readonly isNoGatewayActivitiesProvided: boolean;
    readonly isInvalidProof: boolean;
    readonly isUnsupportedSource: boolean;
    readonly isUnsupportedGateway: boolean;
    readonly isUnsupportedToken: boolean;
    readonly isUnexpectedPreviousGatewayActivityNonce: boolean;
    readonly isUnexpectedGatewayActivityNonce: boolean;
    readonly isInvalidChainConfig: boolean;
    readonly isInsufficientLiquidity: boolean;
    readonly isMintingAuthorityNotFound: boolean;
    readonly isMintingAuthorityMismatch: boolean;
    readonly isUnexpectedMintingAuthorityState: boolean;
    readonly isUnknownOwnerVault: boolean;
    readonly isInvalidMintingAuthoritySigningKey: boolean;
    readonly isMintingAuthorityAlreadyRegistered: boolean;
    readonly isInvalidMintingAuthoritySigningKeyProof: boolean;
    readonly isInvalidMintingAuthorityCollateral: boolean;
    readonly isGlobalIssuanceCouncilNotFound: boolean;
    readonly isInvalidGlobalIssuanceCouncil: boolean;
    readonly isDuplicateGlobalIssuanceCouncilSigner: boolean;
    readonly isDuplicateGlobalIssuanceCouncilAccount: boolean;
    readonly isInvalidCouncilSignerProof: boolean;
    readonly isGlobalIssuanceCouncilMemberNotFound: boolean;
    readonly isCouncilSignerNotRegistered: boolean;
    readonly isCouncilApprovalQueueEntryNotFound: boolean;
    readonly isCouncilApprovalAlreadyRecorded: boolean;
    readonly isInvalidForceSetAfterNonce: boolean;
    readonly isCannotForceSetQuorumApprovedQueueEntry: boolean;
    readonly isInvalidCouncilApprovalSignature: boolean;
    readonly isNoCouncilApprovalSignaturesProvided: boolean;
    readonly isInsufficientCommittedMicrogonCollateral: boolean;
    readonly isInsufficientCommittedArgonotCollateral: boolean;
    readonly isInvalidMintingAuthorityDeactivationSignature: boolean;
    readonly isInvalidMicrogonsPerArgonot: boolean;
    readonly isMintingAuthorityCollateralBelowMinimum: boolean;
    readonly isInvalidMintingAuthorityActivationRepaymentPricing: boolean;
    readonly isMissingMintingAuthorityActivationRepaymentPricing: boolean;
    readonly isInvalidGatewayActivity: boolean;
    readonly isGatewaySyncPaused: boolean;
    readonly isInvalidTransferOutAmount: boolean;
    readonly isInvalidTransferOutRecipient: boolean;
    readonly isMissingVerifiedExecutionBlock: boolean;
    readonly isStaleVerifiedExecutionBlock: boolean;
    readonly isTransferOutNotFound: boolean;
    readonly isTransferOutAlreadyReady: boolean;
    readonly isTransferOutExpired: boolean;
    readonly isInvalidTransferCollateral: boolean;
    readonly isInvalidTransferCollateralUpdate: boolean;
    readonly isInvalidTransferCollateralSignature: boolean;
    readonly isInsufficientMintingAuthorityCollateral: boolean;
    readonly isTransferCollateralIncrementTooSmall: boolean;
    readonly isTooManyPendingTransferOuts: boolean;
    readonly type:
      | 'InvalidTransferToArgonActivity'
      | 'NoGatewayProofBlocksProvided'
      | 'NoGatewayActivitiesProvided'
      | 'InvalidProof'
      | 'UnsupportedSource'
      | 'UnsupportedGateway'
      | 'UnsupportedToken'
      | 'UnexpectedPreviousGatewayActivityNonce'
      | 'UnexpectedGatewayActivityNonce'
      | 'InvalidChainConfig'
      | 'InsufficientLiquidity'
      | 'MintingAuthorityNotFound'
      | 'MintingAuthorityMismatch'
      | 'UnexpectedMintingAuthorityState'
      | 'UnknownOwnerVault'
      | 'InvalidMintingAuthoritySigningKey'
      | 'MintingAuthorityAlreadyRegistered'
      | 'InvalidMintingAuthoritySigningKeyProof'
      | 'InvalidMintingAuthorityCollateral'
      | 'GlobalIssuanceCouncilNotFound'
      | 'InvalidGlobalIssuanceCouncil'
      | 'DuplicateGlobalIssuanceCouncilSigner'
      | 'DuplicateGlobalIssuanceCouncilAccount'
      | 'InvalidCouncilSignerProof'
      | 'GlobalIssuanceCouncilMemberNotFound'
      | 'CouncilSignerNotRegistered'
      | 'CouncilApprovalQueueEntryNotFound'
      | 'CouncilApprovalAlreadyRecorded'
      | 'InvalidForceSetAfterNonce'
      | 'CannotForceSetQuorumApprovedQueueEntry'
      | 'InvalidCouncilApprovalSignature'
      | 'NoCouncilApprovalSignaturesProvided'
      | 'InsufficientCommittedMicrogonCollateral'
      | 'InsufficientCommittedArgonotCollateral'
      | 'InvalidMintingAuthorityDeactivationSignature'
      | 'InvalidMicrogonsPerArgonot'
      | 'MintingAuthorityCollateralBelowMinimum'
      | 'InvalidMintingAuthorityActivationRepaymentPricing'
      | 'MissingMintingAuthorityActivationRepaymentPricing'
      | 'InvalidGatewayActivity'
      | 'GatewaySyncPaused'
      | 'InvalidTransferOutAmount'
      | 'InvalidTransferOutRecipient'
      | 'MissingVerifiedExecutionBlock'
      | 'StaleVerifiedExecutionBlock'
      | 'TransferOutNotFound'
      | 'TransferOutAlreadyReady'
      | 'TransferOutExpired'
      | 'InvalidTransferCollateral'
      | 'InvalidTransferCollateralUpdate'
      | 'InvalidTransferCollateralSignature'
      | 'InsufficientMintingAuthorityCollateral'
      | 'TransferCollateralIncrementTooSmall'
      | 'TooManyPendingTransferOuts';
  }

  /** @name FrameSystemExtensionsAuthorizeCall (583) */
  type FrameSystemExtensionsAuthorizeCall = Null;

  /** @name FrameSystemExtensionsCheckNonZeroSender (584) */
  type FrameSystemExtensionsCheckNonZeroSender = Null;

  /** @name FrameSystemExtensionsCheckSpecVersion (585) */
  type FrameSystemExtensionsCheckSpecVersion = Null;

  /** @name FrameSystemExtensionsCheckTxVersion (586) */
  type FrameSystemExtensionsCheckTxVersion = Null;

  /** @name FrameSystemExtensionsCheckGenesis (587) */
  type FrameSystemExtensionsCheckGenesis = Null;

  /** @name FrameSystemExtensionsCheckNonce (590) */
  interface FrameSystemExtensionsCheckNonce extends Compact<u32> {}

  /** @name FrameSystemExtensionsCheckWeight (591) */
  type FrameSystemExtensionsCheckWeight = Null;

  /** @name PalletTransactionPaymentChargeTransactionPayment (592) */
  interface PalletTransactionPaymentChargeTransactionPayment extends Compact<u128> {}

  /** @name FrameMetadataHashExtensionCheckMetadataHash (593) */
  interface FrameMetadataHashExtensionCheckMetadataHash extends Struct {
    readonly mode: FrameMetadataHashExtensionMode;
  }

  /** @name FrameMetadataHashExtensionMode (594) */
  interface FrameMetadataHashExtensionMode extends Enum {
    readonly isDisabled: boolean;
    readonly isEnabled: boolean;
    readonly type: 'Disabled' | 'Enabled';
  }

  /** @name FrameSystemExtensionsWeightReclaim (595) */
  type FrameSystemExtensionsWeightReclaim = Null;

  /** @name ArgonRuntimeRuntime (597) */
  type ArgonRuntimeRuntime = Null;
} // declare module
