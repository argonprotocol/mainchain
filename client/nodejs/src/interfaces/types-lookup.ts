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
    readonly isBitcoinInitializeFor: boolean;
    readonly type:
      | 'Any'
      | 'NonTransfer'
      | 'PriceIndex'
      | 'MiningBid'
      | 'MiningBidRealPaysFee'
      | 'Bitcoin'
      | 'VaultAdmin'
      | 'BitcoinInitializeFor';
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

  /** @name ArgonPrimitivesBitcoinBitcoinRejectedReason (51) */
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

  /** @name PalletVaultsEvent (52) */
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
      | 'TreasuryRecordingError';
  }

  /** @name PalletBitcoinLocksEvent (54) */
  interface PalletBitcoinLocksEvent extends Enum {
    readonly isBitcoinLockCreated: boolean;
    readonly asBitcoinLockCreated: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly liquidityPromised: u128;
      readonly securitization: u128;
      readonly lockedMarketRate: u128;
      readonly accountId: AccountId32;
      readonly securityFee: u128;
    } & Struct;
    readonly isBitcoinLockRatcheted: boolean;
    readonly asBitcoinLockRatcheted: {
      readonly utxoId: u64;
      readonly vaultId: u32;
      readonly liquidityPromised: u128;
      readonly originalMarketRate: u128;
      readonly securityFee: u128;
      readonly newLockedMarketRate: u128;
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
    readonly isOrphanedUtxoCosigned: boolean;
    readonly asOrphanedUtxoCosigned: {
      readonly utxoId: u64;
      readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
      readonly vaultId: u32;
      readonly signature: Bytes;
    } & Struct;
    readonly type:
      | 'BitcoinLockCreated'
      | 'BitcoinLockRatcheted'
      | 'BitcoinLockBurned'
      | 'BitcoinUtxoCosignRequested'
      | 'BitcoinUtxoCosigned'
      | 'BitcoinCosignPastDue'
      | 'CosignOverdueError'
      | 'LockExpirationError'
      | 'OrphanedUtxoCosigned';
  }

  /** @name ArgonPrimitivesBitcoinUtxoRef (57) */
  interface ArgonPrimitivesBitcoinUtxoRef extends Struct {
    readonly txid: ArgonPrimitivesBitcoinH256Le;
    readonly outputIndex: Compact<u32>;
  }

  /** @name ArgonPrimitivesBitcoinH256Le (58) */
  interface ArgonPrimitivesBitcoinH256Le extends U8aFixed {}

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

  /** @name PalletChainTransferEvent (76) */
  interface PalletChainTransferEvent extends Enum {
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
      | 'Burned'
      | 'Suspended'
      | 'Restored'
      | 'Upgraded'
      | 'Issued'
      | 'Rescinded'
      | 'Locked'
      | 'Unlocked'
      | 'Frozen'
      | 'Thawed'
      | 'TotalIssuanceForced';
  }

  /** @name FrameSupportTokensMiscBalanceStatus (103) */
  interface FrameSupportTokensMiscBalanceStatus extends Enum {
    readonly isFree: boolean;
    readonly isReserved: boolean;
    readonly type: 'Free' | 'Reserved';
  }

  /** @name PalletTxPauseEvent (105) */
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

  /** @name PalletTransactionPaymentEvent (108) */
  interface PalletTransactionPaymentEvent extends Enum {
    readonly isTransactionFeePaid: boolean;
    readonly asTransactionFeePaid: {
      readonly who: AccountId32;
      readonly actualFee: u128;
      readonly tip: u128;
    } & Struct;
    readonly type: 'TransactionFeePaid';
  }

  /** @name PalletUtilityEvent (109) */
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

  /** @name PalletIsmpEvent (111) */
  interface PalletIsmpEvent extends Enum {
    readonly isStateMachineUpdated: boolean;
    readonly asStateMachineUpdated: {
      readonly stateMachineId: IsmpConsensusStateMachineId;
      readonly latestHeight: u64;
    } & Struct;
    readonly isStateCommitmentVetoed: boolean;
    readonly asStateCommitmentVetoed: {
      readonly height: IsmpConsensusStateMachineHeight;
      readonly fisherman: Bytes;
    } & Struct;
    readonly isConsensusClientCreated: boolean;
    readonly asConsensusClientCreated: {
      readonly consensusClientId: U8aFixed;
    } & Struct;
    readonly isConsensusClientFrozen: boolean;
    readonly asConsensusClientFrozen: {
      readonly consensusClientId: U8aFixed;
    } & Struct;
    readonly isResponse: boolean;
    readonly asResponse: {
      readonly destChain: IsmpHostStateMachine;
      readonly sourceChain: IsmpHostStateMachine;
      readonly requestNonce: u64;
      readonly commitment: H256;
      readonly reqCommitment: H256;
    } & Struct;
    readonly isRequest: boolean;
    readonly asRequest: {
      readonly destChain: IsmpHostStateMachine;
      readonly sourceChain: IsmpHostStateMachine;
      readonly requestNonce: u64;
      readonly commitment: H256;
    } & Struct;
    readonly isErrors: boolean;
    readonly asErrors: {
      readonly errors: Vec<PalletIsmpErrorsHandlingError>;
    } & Struct;
    readonly isPostRequestHandled: boolean;
    readonly asPostRequestHandled: IsmpEventsRequestResponseHandled;
    readonly isPostResponseHandled: boolean;
    readonly asPostResponseHandled: IsmpEventsRequestResponseHandled;
    readonly isGetRequestHandled: boolean;
    readonly asGetRequestHandled: IsmpEventsRequestResponseHandled;
    readonly isPostRequestTimeoutHandled: boolean;
    readonly asPostRequestTimeoutHandled: IsmpEventsTimeoutHandled;
    readonly isPostResponseTimeoutHandled: boolean;
    readonly asPostResponseTimeoutHandled: IsmpEventsTimeoutHandled;
    readonly isGetRequestTimeoutHandled: boolean;
    readonly asGetRequestTimeoutHandled: IsmpEventsTimeoutHandled;
    readonly type:
      | 'StateMachineUpdated'
      | 'StateCommitmentVetoed'
      | 'ConsensusClientCreated'
      | 'ConsensusClientFrozen'
      | 'Response'
      | 'Request'
      | 'Errors'
      | 'PostRequestHandled'
      | 'PostResponseHandled'
      | 'GetRequestHandled'
      | 'PostRequestTimeoutHandled'
      | 'PostResponseTimeoutHandled'
      | 'GetRequestTimeoutHandled';
  }

  /** @name IsmpConsensusStateMachineId (112) */
  interface IsmpConsensusStateMachineId extends Struct {
    readonly stateId: IsmpHostStateMachine;
    readonly consensusStateId: U8aFixed;
  }

  /** @name IsmpHostStateMachine (113) */
  interface IsmpHostStateMachine extends Enum {
    readonly isEvm: boolean;
    readonly asEvm: u32;
    readonly isPolkadot: boolean;
    readonly asPolkadot: u32;
    readonly isKusama: boolean;
    readonly asKusama: u32;
    readonly isSubstrate: boolean;
    readonly asSubstrate: U8aFixed;
    readonly isTendermint: boolean;
    readonly asTendermint: U8aFixed;
    readonly isRelay: boolean;
    readonly asRelay: {
      readonly relay: U8aFixed;
      readonly paraId: u32;
    } & Struct;
    readonly type: 'Evm' | 'Polkadot' | 'Kusama' | 'Substrate' | 'Tendermint' | 'Relay';
  }

  /** @name IsmpConsensusStateMachineHeight (114) */
  interface IsmpConsensusStateMachineHeight extends Struct {
    readonly id: IsmpConsensusStateMachineId;
    readonly height: u64;
  }

  /** @name PalletIsmpErrorsHandlingError (117) */
  interface PalletIsmpErrorsHandlingError extends Struct {
    readonly message: Bytes;
  }

  /** @name IsmpEventsRequestResponseHandled (119) */
  interface IsmpEventsRequestResponseHandled extends Struct {
    readonly commitment: H256;
    readonly relayer: Bytes;
  }

  /** @name IsmpEventsTimeoutHandled (120) */
  interface IsmpEventsTimeoutHandled extends Struct {
    readonly commitment: H256;
    readonly source: IsmpHostStateMachine;
    readonly dest: IsmpHostStateMachine;
  }

  /** @name IsmpGrandpaEvent (121) */
  interface IsmpGrandpaEvent extends Enum {
    readonly isStateMachineAdded: boolean;
    readonly asStateMachineAdded: {
      readonly stateMachines: Vec<IsmpHostStateMachine>;
    } & Struct;
    readonly isStateMachineRemoved: boolean;
    readonly asStateMachineRemoved: {
      readonly stateMachines: Vec<IsmpHostStateMachine>;
    } & Struct;
    readonly type: 'StateMachineAdded' | 'StateMachineRemoved';
  }

  /** @name PalletHyperbridgeEvent (123) */
  interface PalletHyperbridgeEvent extends Enum {
    readonly isHostParamsUpdated: boolean;
    readonly asHostParamsUpdated: {
      readonly old: PalletHyperbridgeVersionedHostParams;
      readonly new_: PalletHyperbridgeVersionedHostParams;
    } & Struct;
    readonly isRelayerFeeWithdrawn: boolean;
    readonly asRelayerFeeWithdrawn: {
      readonly amount: u128;
      readonly account: AccountId32;
    } & Struct;
    readonly type: 'HostParamsUpdated' | 'RelayerFeeWithdrawn';
  }

  /** @name PalletHyperbridgeVersionedHostParams (124) */
  interface PalletHyperbridgeVersionedHostParams extends Enum {
    readonly isV1: boolean;
    readonly asV1: PalletHyperbridgeSubstrateHostParams;
    readonly type: 'V1';
  }

  /** @name PalletHyperbridgeSubstrateHostParams (125) */
  interface PalletHyperbridgeSubstrateHostParams extends Struct {
    readonly defaultPerByteFee: u128;
    readonly perByteFees: BTreeMap<IsmpHostStateMachine, u128>;
    readonly assetRegistrationFee: u128;
  }

  /** @name PalletTokenGatewayEvent (129) */
  interface PalletTokenGatewayEvent extends Enum {
    readonly isAssetTeleported: boolean;
    readonly asAssetTeleported: {
      readonly from: AccountId32;
      readonly to: H256;
      readonly amount: u128;
      readonly dest: IsmpHostStateMachine;
      readonly commitment: H256;
    } & Struct;
    readonly isAssetReceived: boolean;
    readonly asAssetReceived: {
      readonly beneficiary: AccountId32;
      readonly amount: u128;
      readonly source: IsmpHostStateMachine;
    } & Struct;
    readonly isAssetRefunded: boolean;
    readonly asAssetRefunded: {
      readonly beneficiary: AccountId32;
      readonly amount: u128;
      readonly source: IsmpHostStateMachine;
    } & Struct;
    readonly isErc6160AssetRegistrationDispatched: boolean;
    readonly asErc6160AssetRegistrationDispatched: {
      readonly commitment: H256;
    } & Struct;
    readonly isAssetRegisteredLocally: boolean;
    readonly asAssetRegisteredLocally: {
      readonly localId: u32;
      readonly assetId: H256;
    } & Struct;
    readonly type:
      | 'AssetTeleported'
      | 'AssetReceived'
      | 'AssetRefunded'
      | 'Erc6160AssetRegistrationDispatched'
      | 'AssetRegisteredLocally';
  }

  /** @name PalletTreasuryEvent (130) */
  interface PalletTreasuryEvent extends Enum {
    readonly isCouldNotDistributeBidPool: boolean;
    readonly asCouldNotDistributeBidPool: {
      readonly accountId: AccountId32;
      readonly frameId: u64;
      readonly vaultId: u32;
      readonly amount: u128;
      readonly dispatchError: SpRuntimeDispatchError;
      readonly isForVault: bool;
    } & Struct;
    readonly isCouldNotBurnBidPool: boolean;
    readonly asCouldNotBurnBidPool: {
      readonly frameId: u64;
      readonly amount: u128;
      readonly dispatchError: SpRuntimeDispatchError;
    } & Struct;
    readonly isBidPoolDistributed: boolean;
    readonly asBidPoolDistributed: {
      readonly frameId: u64;
      readonly bidPoolDistributed: u128;
      readonly bidPoolBurned: u128;
      readonly bidPoolShares: u32;
    } & Struct;
    readonly isNextBidPoolCapitalLocked: boolean;
    readonly asNextBidPoolCapitalLocked: {
      readonly frameId: u64;
      readonly totalActivatedCapital: u128;
      readonly participatingVaults: u32;
    } & Struct;
    readonly isErrorRefundingTreasuryCapital: boolean;
    readonly asErrorRefundingTreasuryCapital: {
      readonly frameId: u64;
      readonly vaultId: u32;
      readonly amount: u128;
      readonly accountId: AccountId32;
      readonly dispatchError: SpRuntimeDispatchError;
    } & Struct;
    readonly isRefundedTreasuryCapital: boolean;
    readonly asRefundedTreasuryCapital: {
      readonly frameId: u64;
      readonly vaultId: u32;
      readonly amount: u128;
      readonly accountId: AccountId32;
    } & Struct;
    readonly isVaultOperatorPrebond: boolean;
    readonly asVaultOperatorPrebond: {
      readonly vaultId: u32;
      readonly accountId: AccountId32;
      readonly amountPerFrame: u128;
    } & Struct;
    readonly type:
      | 'CouldNotDistributeBidPool'
      | 'CouldNotBurnBidPool'
      | 'BidPoolDistributed'
      | 'NextBidPoolCapitalLocked'
      | 'ErrorRefundingTreasuryCapital'
      | 'RefundedTreasuryCapital'
      | 'VaultOperatorPrebond';
  }

  /** @name PalletFeeControlEvent (131) */
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

  /** @name ArgonRuntimeOriginCaller (132) */
  interface ArgonRuntimeOriginCaller extends Enum {
    readonly isSystem: boolean;
    readonly asSystem: FrameSupportDispatchRawOrigin;
    readonly type: 'System';
  }

  /** @name FrameSupportDispatchRawOrigin (133) */
  interface FrameSupportDispatchRawOrigin extends Enum {
    readonly isRoot: boolean;
    readonly isSigned: boolean;
    readonly asSigned: AccountId32;
    readonly isNone: boolean;
    readonly isAuthorized: boolean;
    readonly type: 'Root' | 'Signed' | 'None' | 'Authorized';
  }

  /** @name FrameSystemPhase (134) */
  interface FrameSystemPhase extends Enum {
    readonly isApplyExtrinsic: boolean;
    readonly asApplyExtrinsic: u32;
    readonly isFinalization: boolean;
    readonly isInitialization: boolean;
    readonly type: 'ApplyExtrinsic' | 'Finalization' | 'Initialization';
  }

  /** @name FrameSystemLastRuntimeUpgradeInfo (138) */
  interface FrameSystemLastRuntimeUpgradeInfo extends Struct {
    readonly specVersion: Compact<u32>;
    readonly specName: Text;
  }

  /** @name FrameSystemCodeUpgradeAuthorization (141) */
  interface FrameSystemCodeUpgradeAuthorization extends Struct {
    readonly codeHash: H256;
    readonly checkVersion: bool;
  }

  /** @name FrameSystemCall (142) */
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

  /** @name FrameSystemLimitsBlockWeights (146) */
  interface FrameSystemLimitsBlockWeights extends Struct {
    readonly baseBlock: SpWeightsWeightV2Weight;
    readonly maxBlock: SpWeightsWeightV2Weight;
    readonly perClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
  }

  /** @name FrameSupportDispatchPerDispatchClassWeightsPerClass (147) */
  interface FrameSupportDispatchPerDispatchClassWeightsPerClass extends Struct {
    readonly normal: FrameSystemLimitsWeightsPerClass;
    readonly operational: FrameSystemLimitsWeightsPerClass;
    readonly mandatory: FrameSystemLimitsWeightsPerClass;
  }

  /** @name FrameSystemLimitsWeightsPerClass (148) */
  interface FrameSystemLimitsWeightsPerClass extends Struct {
    readonly baseExtrinsic: SpWeightsWeightV2Weight;
    readonly maxExtrinsic: Option<SpWeightsWeightV2Weight>;
    readonly maxTotal: Option<SpWeightsWeightV2Weight>;
    readonly reserved: Option<SpWeightsWeightV2Weight>;
  }

  /** @name FrameSystemLimitsBlockLength (150) */
  interface FrameSystemLimitsBlockLength extends Struct {
    readonly max: FrameSupportDispatchPerDispatchClassU32;
  }

  /** @name FrameSupportDispatchPerDispatchClassU32 (151) */
  interface FrameSupportDispatchPerDispatchClassU32 extends Struct {
    readonly normal: u32;
    readonly operational: u32;
    readonly mandatory: u32;
  }

  /** @name SpWeightsRuntimeDbWeight (152) */
  interface SpWeightsRuntimeDbWeight extends Struct {
    readonly read: u64;
    readonly write: u64;
  }

  /** @name SpVersionRuntimeVersion (153) */
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

  /** @name FrameSystemError (158) */
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

  /** @name ArgonPrimitivesDigestsDigestset (159) */
  interface ArgonPrimitivesDigestsDigestset extends Struct {
    readonly author: AccountId32;
    readonly blockVote: ArgonPrimitivesDigestsBlockVoteDigest;
    readonly votingKey: Option<ArgonPrimitivesDigestsParentVotingKeyDigest>;
    readonly forkPower: Option<ArgonPrimitivesForkPower>;
    readonly frameInfo: Option<ArgonPrimitivesDigestsFrameInfo>;
    readonly tick: u64;
    readonly notebooks: ArgonPrimitivesDigestsNotebookDigest;
  }

  /** @name ArgonPrimitivesDigestsBlockVoteDigest (160) */
  interface ArgonPrimitivesDigestsBlockVoteDigest extends Struct {
    readonly votingPower: Compact<u128>;
    readonly votesCount: Compact<u32>;
  }

  /** @name ArgonPrimitivesDigestsParentVotingKeyDigest (162) */
  interface ArgonPrimitivesDigestsParentVotingKeyDigest extends Struct {
    readonly parentVotingKey: Option<H256>;
  }

  /** @name ArgonPrimitivesForkPower (165) */
  interface ArgonPrimitivesForkPower extends Struct {
    readonly isLatestVote: bool;
    readonly notebooks: Compact<u64>;
    readonly votingPower: U256;
    readonly sealStrength: U256;
    readonly totalComputeDifficulty: U256;
    readonly voteCreatedBlocks: Compact<u128>;
    readonly minerNonceScore: Option<U256>;
  }

  /** @name ArgonPrimitivesDigestsFrameInfo (170) */
  interface ArgonPrimitivesDigestsFrameInfo extends Struct {
    readonly frameId: Compact<u64>;
    readonly frameRewardTicksRemaining: Compact<u32>;
    readonly isNewFrame: bool;
  }

  /** @name ArgonPrimitivesDigestsNotebookDigest (172) */
  interface ArgonPrimitivesDigestsNotebookDigest extends Struct {
    readonly notebooks: Vec<ArgonPrimitivesNotebookNotebookAuditResult>;
  }

  /** @name ArgonPrimitivesNotebookNotebookAuditResult (174) */
  interface ArgonPrimitivesNotebookNotebookAuditResult extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly auditFirstFailure: Option<ArgonNotaryAuditErrorVerifyError>;
  }

  /** @name PalletDigestsError (177) */
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

  /** @name PalletTimestampCall (178) */
  interface PalletTimestampCall extends Enum {
    readonly isSet: boolean;
    readonly asSet: {
      readonly now: Compact<u64>;
    } & Struct;
    readonly type: 'Set';
  }

  /** @name PalletMultisigMultisig (180) */
  interface PalletMultisigMultisig extends Struct {
    readonly when: PalletMultisigTimepoint;
    readonly deposit: u128;
    readonly depositor: AccountId32;
    readonly approvals: Vec<AccountId32>;
  }

  /** @name PalletMultisigCall (183) */
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

  /** @name PalletProxyCall (185) */
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

  /** @name PalletTicksCall (190) */
  type PalletTicksCall = Null;

  /** @name PalletMiningSlotCall (191) */
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

  /** @name PalletBitcoinUtxosCall (192) */
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
    readonly type: 'Sync' | 'SetConfirmedBlock' | 'SetOperator';
  }

  /** @name ArgonPrimitivesInherentsBitcoinUtxoSync (193) */
  interface ArgonPrimitivesInherentsBitcoinUtxoSync extends Struct {
    readonly spent: BTreeMap<u64, u64>;
    readonly funded: Vec<ArgonPrimitivesInherentsBitcoinUtxoFunding>;
    readonly syncToBlock: ArgonPrimitivesBitcoinBitcoinBlock;
  }

  /** @name ArgonPrimitivesInherentsBitcoinUtxoFunding (198) */
  interface ArgonPrimitivesInherentsBitcoinUtxoFunding extends Struct {
    readonly utxoId: Compact<u64>;
    readonly utxoRef: ArgonPrimitivesBitcoinUtxoRef;
    readonly satoshis: Compact<u64>;
    readonly expectedSatoshis: Compact<u64>;
    readonly bitcoinHeight: Compact<u64>;
  }

  /** @name ArgonPrimitivesBitcoinBitcoinBlock (199) */
  interface ArgonPrimitivesBitcoinBitcoinBlock extends Struct {
    readonly blockHeight: Compact<u64>;
    readonly blockHash: ArgonPrimitivesBitcoinH256Le;
  }

  /** @name PalletVaultsCall (200) */
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
    readonly type:
      | 'Create'
      | 'ModifyFunding'
      | 'ModifyTerms'
      | 'Close'
      | 'ReplaceBitcoinXpub'
      | 'Collect';
  }

  /** @name PalletVaultsVaultConfig (201) */
  interface PalletVaultsVaultConfig extends Struct {
    readonly terms: ArgonPrimitivesVaultVaultTerms;
    readonly securitization: Compact<u128>;
    readonly bitcoinXpubkey: ArgonPrimitivesBitcoinOpaqueBitcoinXpub;
    readonly securitizationRatio: Compact<u128>;
  }

  /** @name ArgonPrimitivesVaultVaultTerms (202) */
  interface ArgonPrimitivesVaultVaultTerms extends Struct {
    readonly bitcoinAnnualPercentRate: Compact<u128>;
    readonly bitcoinBaseFee: Compact<u128>;
    readonly treasuryProfitSharing: Compact<Permill>;
  }

  /** @name ArgonPrimitivesBitcoinOpaqueBitcoinXpub (206) */
  interface ArgonPrimitivesBitcoinOpaqueBitcoinXpub extends U8aFixed {}

  /** @name PalletBitcoinLocksCall (208) */
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
    readonly isRegisterFeeCoupon: boolean;
    readonly asRegisterFeeCoupon: {
      readonly public: U8aFixed;
      readonly maxSatoshis: Compact<u64>;
      readonly maxFeePlusTip: Option<u128>;
    } & Struct;
    readonly isInitializeFor: boolean;
    readonly asInitializeFor: {
      readonly accountId: AccountId32;
      readonly vaultId: u32;
      readonly satoshis: Compact<u64>;
      readonly bitcoinPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
      readonly options: Option<PalletBitcoinLocksLockOptions>;
    } & Struct;
    readonly type:
      | 'Initialize'
      | 'RequestRelease'
      | 'CosignRelease'
      | 'Ratchet'
      | 'AdminModifyMinimumLockedSats'
      | 'RequestOrphanedUtxoRelease'
      | 'CosignOrphanedUtxoRelease'
      | 'RegisterFeeCoupon'
      | 'InitializeFor';
  }

  /** @name ArgonPrimitivesBitcoinCompressedBitcoinPubkey (209) */
  interface ArgonPrimitivesBitcoinCompressedBitcoinPubkey extends U8aFixed {}

  /** @name PalletBitcoinLocksLockOptions (212) */
  interface PalletBitcoinLocksLockOptions extends Enum {
    readonly isV1: boolean;
    readonly asV1: {
      readonly microgonsPerBtc: Option<u128>;
      readonly feeCouponProof: Option<PalletBitcoinLocksFeeCouponProof>;
    } & Struct;
    readonly type: 'V1';
  }

  /** @name PalletBitcoinLocksFeeCouponProof (214) */
  interface PalletBitcoinLocksFeeCouponProof extends Struct {
    readonly public: U8aFixed;
    readonly signature: U8aFixed;
  }

  /** @name PalletNotariesCall (218) */
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

  /** @name PalletNotebookCall (219) */
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

  /** @name ArgonPrimitivesNotebookSignedNotebookHeader (221) */
  interface ArgonPrimitivesNotebookSignedNotebookHeader extends Struct {
    readonly header: ArgonPrimitivesNotebookNotebookHeader;
    readonly signature: U8aFixed;
  }

  /** @name ArgonPrimitivesNotebookNotebookHeader (222) */
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

  /** @name ArgonPrimitivesNotebookChainTransfer (225) */
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

  /** @name ArgonPrimitivesBalanceChangeAccountOrigin (228) */
  interface ArgonPrimitivesBalanceChangeAccountOrigin extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly accountUid: Compact<u32>;
  }

  /** @name PalletChainTransferCall (234) */
  interface PalletChainTransferCall extends Enum {
    readonly isSendToLocalchain: boolean;
    readonly asSendToLocalchain: {
      readonly amount: Compact<u128>;
      readonly notaryId: u32;
    } & Struct;
    readonly type: 'SendToLocalchain';
  }

  /** @name PalletBlockSealSpecCall (235) */
  interface PalletBlockSealSpecCall extends Enum {
    readonly isConfigure: boolean;
    readonly asConfigure: {
      readonly voteMinimum: Option<u128>;
      readonly computeDifficulty: Option<u128>;
    } & Struct;
    readonly type: 'Configure';
  }

  /** @name PalletDomainsCall (236) */
  interface PalletDomainsCall extends Enum {
    readonly isSetZoneRecord: boolean;
    readonly asSetZoneRecord: {
      readonly domainHash: H256;
      readonly zoneRecord: ArgonPrimitivesDomainZoneRecord;
    } & Struct;
    readonly type: 'SetZoneRecord';
  }

  /** @name PalletPriceIndexCall (237) */
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

  /** @name PalletPriceIndexPriceIndex (238) */
  interface PalletPriceIndexPriceIndex extends Struct {
    readonly btcUsdPrice: Compact<u128>;
    readonly argonotUsdPrice: u128;
    readonly argonUsdPrice: Compact<u128>;
    readonly argonUsdTargetPrice: u128;
    readonly argonTimeWeightedAverageLiquidity: u128;
    readonly tick: Compact<u64>;
  }

  /** @name PalletGrandpaCall (239) */
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

  /** @name SpConsensusGrandpaEquivocationProof (240) */
  interface SpConsensusGrandpaEquivocationProof extends Struct {
    readonly setId: u64;
    readonly equivocation: SpConsensusGrandpaEquivocation;
  }

  /** @name SpConsensusGrandpaEquivocation (241) */
  interface SpConsensusGrandpaEquivocation extends Enum {
    readonly isPrevote: boolean;
    readonly asPrevote: FinalityGrandpaEquivocationPrevote;
    readonly isPrecommit: boolean;
    readonly asPrecommit: FinalityGrandpaEquivocationPrecommit;
    readonly type: 'Prevote' | 'Precommit';
  }

  /** @name FinalityGrandpaEquivocationPrevote (242) */
  interface FinalityGrandpaEquivocationPrevote extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrevote (243) */
  interface FinalityGrandpaPrevote extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpConsensusGrandpaAppSignature (244) */
  interface SpConsensusGrandpaAppSignature extends U8aFixed {}

  /** @name FinalityGrandpaEquivocationPrecommit (246) */
  interface FinalityGrandpaEquivocationPrecommit extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrecommit (247) */
  interface FinalityGrandpaPrecommit extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpCoreVoid (249) */
  type SpCoreVoid = Null;

  /** @name PalletBlockSealCall (250) */
  interface PalletBlockSealCall extends Enum {
    readonly isApply: boolean;
    readonly asApply: {
      readonly seal: ArgonPrimitivesInherentsBlockSealInherent;
    } & Struct;
    readonly type: 'Apply';
  }

  /** @name ArgonPrimitivesInherentsBlockSealInherent (251) */
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

  /** @name ArgonPrimitivesBalanceChangeMerkleProof (252) */
  interface ArgonPrimitivesBalanceChangeMerkleProof extends Struct {
    readonly proof: Vec<H256>;
    readonly numberOfLeaves: Compact<u32>;
    readonly leafIndex: Compact<u32>;
  }

  /** @name ArgonPrimitivesBlockVoteBlockVoteT (254) */
  interface ArgonPrimitivesBlockVoteBlockVoteT extends Struct {
    readonly accountId: AccountId32;
    readonly blockHash: H256;
    readonly index: Compact<u32>;
    readonly power: Compact<u128>;
    readonly signature: SpRuntimeMultiSignature;
    readonly blockRewardsAccountId: AccountId32;
    readonly tick: Compact<u64>;
  }

  /** @name SpRuntimeMultiSignature (255) */
  interface SpRuntimeMultiSignature extends Enum {
    readonly isEd25519: boolean;
    readonly asEd25519: U8aFixed;
    readonly isSr25519: boolean;
    readonly asSr25519: U8aFixed;
    readonly isEcdsa: boolean;
    readonly asEcdsa: U8aFixed;
    readonly type: 'Ed25519' | 'Sr25519' | 'Ecdsa';
  }

  /** @name PalletBlockRewardsCall (257) */
  interface PalletBlockRewardsCall extends Enum {
    readonly isSetBlockRewardsPaused: boolean;
    readonly asSetBlockRewardsPaused: {
      readonly paused: bool;
    } & Struct;
    readonly type: 'SetBlockRewardsPaused';
  }

  /** @name PalletMintCall (258) */
  type PalletMintCall = Null;

  /** @name PalletBalancesCall (259) */
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

  /** @name PalletBalancesAdjustmentDirection (260) */
  interface PalletBalancesAdjustmentDirection extends Enum {
    readonly isIncrease: boolean;
    readonly isDecrease: boolean;
    readonly type: 'Increase' | 'Decrease';
  }

  /** @name PalletTxPauseCall (262) */
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

  /** @name PalletUtilityCall (263) */
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

  /** @name PalletSudoCall (265) */
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

  /** @name PalletIsmpCall (266) */
  interface PalletIsmpCall extends Enum {
    readonly isHandleUnsigned: boolean;
    readonly asHandleUnsigned: {
      readonly messages: Vec<IsmpMessagingMessage>;
    } & Struct;
    readonly isCreateConsensusClient: boolean;
    readonly asCreateConsensusClient: {
      readonly message: IsmpMessagingCreateConsensusState;
    } & Struct;
    readonly isUpdateConsensusState: boolean;
    readonly asUpdateConsensusState: {
      readonly message: PalletIsmpUtilsUpdateConsensusState;
    } & Struct;
    readonly isFundMessage: boolean;
    readonly asFundMessage: {
      readonly message: PalletIsmpUtilsFundMessageParams;
    } & Struct;
    readonly type:
      | 'HandleUnsigned'
      | 'CreateConsensusClient'
      | 'UpdateConsensusState'
      | 'FundMessage';
  }

  /** @name IsmpMessagingMessage (268) */
  interface IsmpMessagingMessage extends Enum {
    readonly isConsensus: boolean;
    readonly asConsensus: IsmpMessagingConsensusMessage;
    readonly isFraudProof: boolean;
    readonly asFraudProof: IsmpMessagingFraudProofMessage;
    readonly isRequest: boolean;
    readonly asRequest: IsmpMessagingRequestMessage;
    readonly isResponse: boolean;
    readonly asResponse: IsmpMessagingResponseMessage;
    readonly isTimeout: boolean;
    readonly asTimeout: IsmpMessagingTimeoutMessage;
    readonly type: 'Consensus' | 'FraudProof' | 'Request' | 'Response' | 'Timeout';
  }

  /** @name IsmpMessagingConsensusMessage (269) */
  interface IsmpMessagingConsensusMessage extends Struct {
    readonly consensusProof: Bytes;
    readonly consensusStateId: U8aFixed;
    readonly signer: Bytes;
  }

  /** @name IsmpMessagingFraudProofMessage (270) */
  interface IsmpMessagingFraudProofMessage extends Struct {
    readonly proof1: Bytes;
    readonly proof2: Bytes;
    readonly consensusStateId: U8aFixed;
    readonly signer: Bytes;
  }

  /** @name IsmpMessagingRequestMessage (271) */
  interface IsmpMessagingRequestMessage extends Struct {
    readonly requests: Vec<IsmpRouterPostRequest>;
    readonly proof: IsmpMessagingProof;
    readonly signer: Bytes;
  }

  /** @name IsmpRouterPostRequest (273) */
  interface IsmpRouterPostRequest extends Struct {
    readonly source: IsmpHostStateMachine;
    readonly dest: IsmpHostStateMachine;
    readonly nonce: u64;
    readonly from: Bytes;
    readonly to: Bytes;
    readonly timeoutTimestamp: u64;
    readonly body: Bytes;
  }

  /** @name IsmpMessagingProof (274) */
  interface IsmpMessagingProof extends Struct {
    readonly height: IsmpConsensusStateMachineHeight;
    readonly proof: Bytes;
  }

  /** @name IsmpMessagingResponseMessage (275) */
  interface IsmpMessagingResponseMessage extends Struct {
    readonly datagram: IsmpRouterRequestResponse;
    readonly proof: IsmpMessagingProof;
    readonly signer: Bytes;
  }

  /** @name IsmpRouterRequestResponse (276) */
  interface IsmpRouterRequestResponse extends Enum {
    readonly isRequest: boolean;
    readonly asRequest: Vec<IsmpRouterRequest>;
    readonly isResponse: boolean;
    readonly asResponse: Vec<IsmpRouterResponse>;
    readonly type: 'Request' | 'Response';
  }

  /** @name IsmpRouterRequest (278) */
  interface IsmpRouterRequest extends Enum {
    readonly isPost: boolean;
    readonly asPost: IsmpRouterPostRequest;
    readonly isGet: boolean;
    readonly asGet: IsmpRouterGetRequest;
    readonly type: 'Post' | 'Get';
  }

  /** @name IsmpRouterGetRequest (279) */
  interface IsmpRouterGetRequest extends Struct {
    readonly source: IsmpHostStateMachine;
    readonly dest: IsmpHostStateMachine;
    readonly nonce: u64;
    readonly from: Bytes;
    readonly keys_: Vec<Bytes>;
    readonly height: u64;
    readonly context: Bytes;
    readonly timeoutTimestamp: u64;
  }

  /** @name IsmpRouterResponse (281) */
  interface IsmpRouterResponse extends Enum {
    readonly isPost: boolean;
    readonly asPost: IsmpRouterPostResponse;
    readonly isGet: boolean;
    readonly asGet: IsmpRouterGetResponse;
    readonly type: 'Post' | 'Get';
  }

  /** @name IsmpRouterPostResponse (282) */
  interface IsmpRouterPostResponse extends Struct {
    readonly post: IsmpRouterPostRequest;
    readonly response: Bytes;
    readonly timeoutTimestamp: u64;
  }

  /** @name IsmpRouterGetResponse (283) */
  interface IsmpRouterGetResponse extends Struct {
    readonly get_: IsmpRouterGetRequest;
    readonly values_: Vec<IsmpRouterStorageValue>;
  }

  /** @name IsmpRouterStorageValue (285) */
  interface IsmpRouterStorageValue extends Struct {
    readonly key: Bytes;
    readonly value: Option<Bytes>;
  }

  /** @name IsmpMessagingTimeoutMessage (287) */
  interface IsmpMessagingTimeoutMessage extends Enum {
    readonly isPost: boolean;
    readonly asPost: {
      readonly requests: Vec<IsmpRouterRequest>;
      readonly timeoutProof: IsmpMessagingProof;
    } & Struct;
    readonly isPostResponse: boolean;
    readonly asPostResponse: {
      readonly responses: Vec<IsmpRouterPostResponse>;
      readonly timeoutProof: IsmpMessagingProof;
    } & Struct;
    readonly isGet: boolean;
    readonly asGet: {
      readonly requests: Vec<IsmpRouterRequest>;
    } & Struct;
    readonly type: 'Post' | 'PostResponse' | 'Get';
  }

  /** @name IsmpMessagingCreateConsensusState (289) */
  interface IsmpMessagingCreateConsensusState extends Struct {
    readonly consensusState: Bytes;
    readonly consensusClientId: U8aFixed;
    readonly consensusStateId: U8aFixed;
    readonly unbondingPeriod: u64;
    readonly challengePeriods: BTreeMap<IsmpHostStateMachine, u64>;
    readonly stateMachineCommitments: Vec<
      ITuple<[IsmpConsensusStateMachineId, IsmpMessagingStateCommitmentHeight]>
    >;
  }

  /** @name IsmpMessagingStateCommitmentHeight (295) */
  interface IsmpMessagingStateCommitmentHeight extends Struct {
    readonly commitment: IsmpConsensusStateCommitment;
    readonly height: u64;
  }

  /** @name IsmpConsensusStateCommitment (296) */
  interface IsmpConsensusStateCommitment extends Struct {
    readonly timestamp: u64;
    readonly overlayRoot: Option<H256>;
    readonly stateRoot: H256;
  }

  /** @name PalletIsmpUtilsUpdateConsensusState (297) */
  interface PalletIsmpUtilsUpdateConsensusState extends Struct {
    readonly consensusStateId: U8aFixed;
    readonly unbondingPeriod: Option<u64>;
    readonly challengePeriods: BTreeMap<IsmpHostStateMachine, u64>;
  }

  /** @name PalletIsmpUtilsFundMessageParams (298) */
  interface PalletIsmpUtilsFundMessageParams extends Struct {
    readonly commitment: PalletIsmpUtilsMessageCommitment;
    readonly amount: u128;
  }

  /** @name PalletIsmpUtilsMessageCommitment (299) */
  interface PalletIsmpUtilsMessageCommitment extends Enum {
    readonly isRequest: boolean;
    readonly asRequest: H256;
    readonly isResponse: boolean;
    readonly asResponse: H256;
    readonly type: 'Request' | 'Response';
  }

  /** @name IsmpGrandpaCall (300) */
  interface IsmpGrandpaCall extends Enum {
    readonly isAddStateMachines: boolean;
    readonly asAddStateMachines: {
      readonly newStateMachines: Vec<IsmpGrandpaAddStateMachine>;
    } & Struct;
    readonly isRemoveStateMachines: boolean;
    readonly asRemoveStateMachines: {
      readonly stateMachines: Vec<IsmpHostStateMachine>;
    } & Struct;
    readonly type: 'AddStateMachines' | 'RemoveStateMachines';
  }

  /** @name IsmpGrandpaAddStateMachine (302) */
  interface IsmpGrandpaAddStateMachine extends Struct {
    readonly stateMachine: IsmpHostStateMachine;
    readonly slotDuration: u64;
  }

  /** @name PalletTokenGatewayCall (303) */
  interface PalletTokenGatewayCall extends Enum {
    readonly isTeleport: boolean;
    readonly asTeleport: {
      readonly params: PalletTokenGatewayTeleportParams;
    } & Struct;
    readonly isSetTokenGatewayAddresses: boolean;
    readonly asSetTokenGatewayAddresses: {
      readonly addresses: BTreeMap<IsmpHostStateMachine, Bytes>;
    } & Struct;
    readonly isCreateErc6160Asset: boolean;
    readonly asCreateErc6160Asset: {
      readonly asset: PalletTokenGatewayAssetRegistration;
    } & Struct;
    readonly isUpdateErc6160Asset: boolean;
    readonly asUpdateErc6160Asset: {
      readonly asset: TokenGatewayPrimitivesGatewayAssetUpdate;
    } & Struct;
    readonly isUpdateAssetPrecision: boolean;
    readonly asUpdateAssetPrecision: {
      readonly update: PalletTokenGatewayPrecisionUpdate;
    } & Struct;
    readonly isRegisterAssetLocally: boolean;
    readonly asRegisterAssetLocally: {
      readonly asset: PalletTokenGatewayAssetRegistration;
    } & Struct;
    readonly type:
      | 'Teleport'
      | 'SetTokenGatewayAddresses'
      | 'CreateErc6160Asset'
      | 'UpdateErc6160Asset'
      | 'UpdateAssetPrecision'
      | 'RegisterAssetLocally';
  }

  /** @name PalletTokenGatewayTeleportParams (304) */
  interface PalletTokenGatewayTeleportParams extends Struct {
    readonly assetId: u32;
    readonly destination: IsmpHostStateMachine;
    readonly recepient: H256;
    readonly amount: u128;
    readonly timeout: u64;
    readonly tokenGateway: Bytes;
    readonly relayerFee: u128;
    readonly callData: Option<Bytes>;
    readonly redeem: bool;
  }

  /** @name PalletTokenGatewayAssetRegistration (308) */
  interface PalletTokenGatewayAssetRegistration extends Struct {
    readonly localId: u32;
    readonly reg: TokenGatewayPrimitivesGatewayAssetRegistration;
    readonly native: bool;
    readonly precision: BTreeMap<IsmpHostStateMachine, u8>;
  }

  /** @name TokenGatewayPrimitivesGatewayAssetRegistration (309) */
  interface TokenGatewayPrimitivesGatewayAssetRegistration extends Struct {
    readonly name: Bytes;
    readonly symbol: Bytes;
    readonly chains: Vec<IsmpHostStateMachine>;
    readonly minimumBalance: Option<u128>;
  }

  /** @name TokenGatewayPrimitivesGatewayAssetUpdate (314) */
  interface TokenGatewayPrimitivesGatewayAssetUpdate extends Struct {
    readonly assetId: H256;
    readonly addChains: Vec<IsmpHostStateMachine>;
    readonly removeChains: Vec<IsmpHostStateMachine>;
    readonly newAdmins: Vec<ITuple<[IsmpHostStateMachine, H160]>>;
  }

  /** @name PalletTokenGatewayPrecisionUpdate (320) */
  interface PalletTokenGatewayPrecisionUpdate extends Struct {
    readonly assetId: u32;
    readonly precisions: BTreeMap<IsmpHostStateMachine, u8>;
  }

  /** @name PalletTreasuryCall (321) */
  interface PalletTreasuryCall extends Enum {
    readonly isBondArgons: boolean;
    readonly asBondArgons: {
      readonly vaultId: u32;
      readonly amount: u128;
    } & Struct;
    readonly isUnbondArgons: boolean;
    readonly asUnbondArgons: {
      readonly vaultId: u32;
      readonly frameId: u64;
    } & Struct;
    readonly isVaultOperatorPrebond: boolean;
    readonly asVaultOperatorPrebond: {
      readonly vaultId: u32;
      readonly maxAmountPerFrame: u128;
    } & Struct;
    readonly type: 'BondArgons' | 'UnbondArgons' | 'VaultOperatorPrebond';
  }

  /** @name PalletMultisigError (323) */
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

  /** @name PalletProxyProxyDefinition (326) */
  interface PalletProxyProxyDefinition extends Struct {
    readonly delegate: AccountId32;
    readonly proxyType: ArgonRuntimeProxyType;
    readonly delay: u32;
  }

  /** @name PalletProxyAnnouncement (330) */
  interface PalletProxyAnnouncement extends Struct {
    readonly real: AccountId32;
    readonly callHash: H256;
    readonly height: u32;
  }

  /** @name PalletProxyError (332) */
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

  /** @name ArgonPrimitivesTickTicker (333) */
  interface ArgonPrimitivesTickTicker extends Struct {
    readonly tickDurationMillis: Compact<u64>;
    readonly channelHoldExpirationTicks: Compact<u64>;
  }

  /** @name PalletTicksError (335) */
  type PalletTicksError = Null;

  /** @name PalletMiningSlotMinerNonceScoring (338) */
  interface PalletMiningSlotMinerNonceScoring extends Struct {
    readonly nonce: U256;
    readonly lastWinBlock: Option<u32>;
    readonly blocksWonInFrame: u16;
    readonly frameStartBlocksWonSurplus: i16;
  }

  /** @name ArgonPrimitivesBlockSealMiningBidStats (351) */
  interface ArgonPrimitivesBlockSealMiningBidStats extends Struct {
    readonly bidsCount: u32;
    readonly bidAmountMin: u128;
    readonly bidAmountMax: u128;
    readonly bidAmountSum: u128;
  }

  /** @name ArgonPrimitivesBlockSealMiningSlotConfig (355) */
  interface ArgonPrimitivesBlockSealMiningSlotConfig extends Struct {
    readonly ticksBeforeBidEndForVrfClose: Compact<u64>;
    readonly ticksBetweenSlots: Compact<u64>;
    readonly slotBiddingStartAfterTicks: Compact<u64>;
  }

  /** @name PalletMiningSlotError (363) */
  interface PalletMiningSlotError extends Enum {
    readonly isSlotNotTakingBids: boolean;
    readonly isTooManyBlockRegistrants: boolean;
    readonly isInsufficientOwnershipTokens: boolean;
    readonly isBidTooLow: boolean;
    readonly isCannotRegisterOverlappingSessions: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isBidCannotBeReduced: boolean;
    readonly isInvalidBidAmount: boolean;
    readonly isUnrecoverableHold: boolean;
    readonly type:
      | 'SlotNotTakingBids'
      | 'TooManyBlockRegistrants'
      | 'InsufficientOwnershipTokens'
      | 'BidTooLow'
      | 'CannotRegisterOverlappingSessions'
      | 'InsufficientFunds'
      | 'BidCannotBeReduced'
      | 'InvalidBidAmount'
      | 'UnrecoverableHold';
  }

  /** @name ArgonPrimitivesBitcoinUtxoValue (364) */
  interface ArgonPrimitivesBitcoinUtxoValue extends Struct {
    readonly utxoId: u64;
    readonly scriptPubkey: ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey;
    readonly satoshis: Compact<u64>;
    readonly submittedAtHeight: Compact<u64>;
    readonly watchForSpentUntilHeight: Compact<u64>;
  }

  /** @name ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey (365) */
  interface ArgonPrimitivesBitcoinBitcoinCosignScriptPubkey extends Enum {
    readonly isP2wsh: boolean;
    readonly asP2wsh: {
      readonly wscriptHash: H256;
    } & Struct;
    readonly type: 'P2wsh';
  }

  /** @name ArgonPrimitivesBitcoinBitcoinNetwork (370) */
  interface ArgonPrimitivesBitcoinBitcoinNetwork extends Enum {
    readonly isBitcoin: boolean;
    readonly isTestnet: boolean;
    readonly isSignet: boolean;
    readonly isRegtest: boolean;
    readonly type: 'Bitcoin' | 'Testnet' | 'Signet' | 'Regtest';
  }

  /** @name PalletBitcoinUtxosError (371) */
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
      | 'DuplicateUtxoId';
  }

  /** @name ArgonPrimitivesVault (372) */
  interface ArgonPrimitivesVault extends Struct {
    readonly operatorAccountId: AccountId32;
    readonly securitization: Compact<u128>;
    readonly securitizationTarget: Compact<u128>;
    readonly securitizationLocked: Compact<u128>;
    readonly securitizationPendingActivation: Compact<u128>;
    readonly securitizationReleaseSchedule: BTreeMap<u64, u128>;
    readonly securitizationRatio: Compact<u128>;
    readonly isClosed: bool;
    readonly terms: ArgonPrimitivesVaultVaultTerms;
    readonly pendingTerms: Option<ITuple<[u64, ArgonPrimitivesVaultVaultTerms]>>;
    readonly openedTick: Compact<u64>;
  }

  /** @name ArgonPrimitivesBitcoinBitcoinXPub (380) */
  interface ArgonPrimitivesBitcoinBitcoinXPub extends Struct {
    readonly publicKey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey;
    readonly depth: Compact<u8>;
    readonly parentFingerprint: U8aFixed;
    readonly childNumber: Compact<u32>;
    readonly chainCode: U8aFixed;
    readonly network: ArgonPrimitivesBitcoinNetworkKind;
  }

  /** @name ArgonPrimitivesBitcoinNetworkKind (382) */
  interface ArgonPrimitivesBitcoinNetworkKind extends Enum {
    readonly isMain: boolean;
    readonly isTest: boolean;
    readonly type: 'Main' | 'Test';
  }

  /** @name PalletVaultsVaultFrameRevenue (391) */
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

  /** @name PalletVaultsError (393) */
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
    readonly isPendingCosignsBeforeCollect: boolean;
    readonly isAccountAlreadyHasVault: boolean;
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
      | 'PendingCosignsBeforeCollect'
      | 'AccountAlreadyHasVault';
  }

  /** @name PalletBitcoinLocksLockedBitcoin (394) */
  interface PalletBitcoinLocksLockedBitcoin extends Struct {
    readonly vaultId: Compact<u32>;
    readonly liquidityPromised: Compact<u128>;
    readonly lockedMarketRate: Compact<u128>;
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
    readonly isVerified: bool;
    readonly fundHoldExtensions: BTreeMap<u64, u128>;
    readonly createdAtArgonBlock: Compact<u32>;
  }

  /** @name PalletBitcoinLocksLockReleaseRequest (396) */
  interface PalletBitcoinLocksLockReleaseRequest extends Struct {
    readonly utxoId: Compact<u64>;
    readonly vaultId: Compact<u32>;
    readonly bitcoinNetworkFee: Compact<u64>;
    readonly cosignDueFrame: Compact<u64>;
    readonly toScriptPubkey: Bytes;
    readonly redemptionPrice: Compact<u128>;
  }

  /** @name PalletBitcoinLocksOrphanedUtxo (398) */
  interface PalletBitcoinLocksOrphanedUtxo extends Struct {
    readonly utxoId: Compact<u64>;
    readonly vaultId: Compact<u32>;
    readonly satoshis: Compact<u64>;
    readonly recordedArgonBlockNumber: Compact<u32>;
    readonly cosignRequest: Option<PalletBitcoinLocksOrphanedUtxoCosignRequest>;
  }

  /** @name PalletBitcoinLocksOrphanedUtxoCosignRequest (400) */
  interface PalletBitcoinLocksOrphanedUtxoCosignRequest extends Struct {
    readonly bitcoinNetworkFee: u64;
    readonly toScriptPubkey: Bytes;
    readonly createdAtArgonBlockNumber: u32;
  }

  /** @name PalletBitcoinLocksFeeCoupon (404) */
  interface PalletBitcoinLocksFeeCoupon extends Struct {
    readonly vaultId: Compact<u32>;
    readonly maxSatoshis: Compact<u64>;
    readonly expirationFrame: Compact<u64>;
    readonly maxFeePlusTip: Option<u128>;
  }

  /** @name PalletBitcoinLocksError (406) */
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
    readonly isUnverifiedLock: boolean;
    readonly isOverflowError: boolean;
    readonly isIneligibleMicrogonRateRequested: boolean;
    readonly isInvalidFeeCoupon: boolean;
    readonly isInvalidFeeCouponProof: boolean;
    readonly isMaxFeeCouponSatoshisExceeded: boolean;
    readonly isFeeCouponAlreadyExists: boolean;
    readonly isFeeCouponRequired: boolean;
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
      | 'UnverifiedLock'
      | 'OverflowError'
      | 'IneligibleMicrogonRateRequested'
      | 'InvalidFeeCoupon'
      | 'InvalidFeeCouponProof'
      | 'MaxFeeCouponSatoshisExceeded'
      | 'FeeCouponAlreadyExists'
      | 'FeeCouponRequired';
  }

  /** @name ArgonPrimitivesVaultVaultError (407) */
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
      | 'VaultNotYetActive';
  }

  /** @name PalletNotariesError (419) */
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

  /** @name ArgonPrimitivesNotaryNotaryNotebookKeyDetails (423) */
  interface ArgonPrimitivesNotaryNotaryNotebookKeyDetails extends Struct {
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly blockVotesRoot: H256;
    readonly secretHash: H256;
    readonly parentSecret: Option<H256>;
  }

  /** @name PalletNotebookError (426) */
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

  /** @name PalletChainTransferQueuedTransferOut (427) */
  interface PalletChainTransferQueuedTransferOut extends Struct {
    readonly accountId: AccountId32;
    readonly amount: u128;
    readonly expirationTick: u64;
    readonly notaryId: u32;
  }

  /** @name FrameSupportPalletId (433) */
  interface FrameSupportPalletId extends U8aFixed {}

  /** @name PalletChainTransferError (434) */
  interface PalletChainTransferError extends Enum {
    readonly isMaxBlockTransfersExceeded: boolean;
    readonly isInsufficientFunds: boolean;
    readonly isInsufficientNotarizedFunds: boolean;
    readonly isInvalidOrDuplicatedLocalchainTransfer: boolean;
    readonly isNotebookIncludesExpiredLocalchainTransfer: boolean;
    readonly isInvalidNotaryUsedForTransfer: boolean;
    readonly type:
      | 'MaxBlockTransfersExceeded'
      | 'InsufficientFunds'
      | 'InsufficientNotarizedFunds'
      | 'InvalidOrDuplicatedLocalchainTransfer'
      | 'NotebookIncludesExpiredLocalchainTransfer'
      | 'InvalidNotaryUsedForTransfer';
  }

  /** @name ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails (438) */
  interface ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails extends Struct {
    readonly notaryId: Compact<u32>;
    readonly notebookNumber: Compact<u32>;
    readonly tick: Compact<u64>;
    readonly blockVotesCount: Compact<u32>;
    readonly blockVotingPower: Compact<u128>;
  }

  /** @name PalletBlockSealSpecError (443) */
  interface PalletBlockSealSpecError extends Enum {
    readonly isMaxNotebooksAtTickExceeded: boolean;
    readonly type: 'MaxNotebooksAtTickExceeded';
  }

  /** @name PalletDomainsError (445) */
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

  /** @name PalletPriceIndexCpiMeasurementBucket (447) */
  interface PalletPriceIndexCpiMeasurementBucket extends Struct {
    readonly tickRange: ITuple<[u64, u64]>;
    readonly totalCpi: i128;
    readonly measurementsCount: u32;
  }

  /** @name PalletPriceIndexError (449) */
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

  /** @name PalletGrandpaStoredState (450) */
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

  /** @name PalletGrandpaStoredPendingChange (451) */
  interface PalletGrandpaStoredPendingChange extends Struct {
    readonly scheduledAt: u32;
    readonly delay: u32;
    readonly nextAuthorities: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    readonly forced: Option<u32>;
  }

  /** @name PalletGrandpaError (453) */
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

  /** @name ArgonPrimitivesProvidersBlockSealerInfo (454) */
  interface ArgonPrimitivesProvidersBlockSealerInfo extends Struct {
    readonly blockAuthorAccountId: AccountId32;
    readonly blockVoteRewardsAccount: Option<AccountId32>;
    readonly blockSealAuthority: Option<ArgonPrimitivesBlockSealAppPublic>;
  }

  /** @name PalletBlockSealError (456) */
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

  /** @name PalletBlockRewardsError (460) */
  type PalletBlockRewardsError = Null;

  /** @name PalletMintMintAction (466) */
  interface PalletMintMintAction extends Struct {
    readonly argonBurned: u128;
    readonly argonMinted: u128;
    readonly bitcoinMinted: u128;
  }

  /** @name PalletMintError (467) */
  interface PalletMintError extends Enum {
    readonly isTooManyPendingMints: boolean;
    readonly type: 'TooManyPendingMints';
  }

  /** @name PalletBalancesBalanceLock (469) */
  interface PalletBalancesBalanceLock extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
    readonly reasons: PalletBalancesReasons;
  }

  /** @name PalletBalancesReasons (470) */
  interface PalletBalancesReasons extends Enum {
    readonly isFee: boolean;
    readonly isMisc: boolean;
    readonly isAll: boolean;
    readonly type: 'Fee' | 'Misc' | 'All';
  }

  /** @name PalletBalancesReserveData (473) */
  interface PalletBalancesReserveData extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
  }

  /** @name FrameSupportTokensMiscIdAmountRuntimeHoldReason (476) */
  interface FrameSupportTokensMiscIdAmountRuntimeHoldReason extends Struct {
    readonly id: ArgonRuntimeRuntimeHoldReason;
    readonly amount: u128;
  }

  /** @name ArgonRuntimeRuntimeHoldReason (477) */
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
    readonly type: 'MiningSlot' | 'Vaults' | 'BitcoinLocks' | 'BlockRewards' | 'Treasury';
  }

  /** @name PalletMiningSlotHoldReason (478) */
  interface PalletMiningSlotHoldReason extends Enum {
    readonly isRegisterAsMiner: boolean;
    readonly type: 'RegisterAsMiner';
  }

  /** @name PalletVaultsHoldReason (479) */
  interface PalletVaultsHoldReason extends Enum {
    readonly isEnterVault: boolean;
    readonly isObligationFee: boolean;
    readonly isPendingCollect: boolean;
    readonly type: 'EnterVault' | 'ObligationFee' | 'PendingCollect';
  }

  /** @name PalletBitcoinLocksHoldReason (480) */
  interface PalletBitcoinLocksHoldReason extends Enum {
    readonly isReleaseBitcoinLock: boolean;
    readonly type: 'ReleaseBitcoinLock';
  }

  /** @name PalletBlockRewardsHoldReason (481) */
  interface PalletBlockRewardsHoldReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletTreasuryHoldReason (482) */
  interface PalletTreasuryHoldReason extends Enum {
    readonly isContributedToTreasury: boolean;
    readonly type: 'ContributedToTreasury';
  }

  /** @name FrameSupportTokensMiscIdAmountRuntimeFreezeReason (485) */
  interface FrameSupportTokensMiscIdAmountRuntimeFreezeReason extends Struct {
    readonly id: ArgonRuntimeRuntimeFreezeReason;
    readonly amount: u128;
  }

  /** @name ArgonRuntimeRuntimeFreezeReason (486) */
  interface ArgonRuntimeRuntimeFreezeReason extends Enum {
    readonly isBlockRewards: boolean;
    readonly asBlockRewards: PalletBlockRewardsFreezeReason;
    readonly type: 'BlockRewards';
  }

  /** @name PalletBlockRewardsFreezeReason (487) */
  interface PalletBlockRewardsFreezeReason extends Enum {
    readonly isMaturationPeriod: boolean;
    readonly type: 'MaturationPeriod';
  }

  /** @name PalletBalancesError (489) */
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

  /** @name PalletTxPauseError (491) */
  interface PalletTxPauseError extends Enum {
    readonly isIsPaused: boolean;
    readonly isIsUnpaused: boolean;
    readonly isUnpausable: boolean;
    readonly isNotFound: boolean;
    readonly type: 'IsPaused' | 'IsUnpaused' | 'Unpausable' | 'NotFound';
  }

  /** @name PalletTransactionPaymentReleases (492) */
  interface PalletTransactionPaymentReleases extends Enum {
    readonly isV1Ancient: boolean;
    readonly isV2: boolean;
    readonly type: 'V1Ancient' | 'V2';
  }

  /** @name PalletUtilityError (493) */
  interface PalletUtilityError extends Enum {
    readonly isTooManyCalls: boolean;
    readonly type: 'TooManyCalls';
  }

  /** @name PalletSudoError (494) */
  interface PalletSudoError extends Enum {
    readonly isRequireSudo: boolean;
    readonly type: 'RequireSudo';
  }

  /** @name PalletIsmpError (495) */
  interface PalletIsmpError extends Enum {
    readonly isInvalidMessage: boolean;
    readonly isMessageNotFound: boolean;
    readonly isConsensusClientCreationFailed: boolean;
    readonly isUnbondingPeriodUpdateFailed: boolean;
    readonly isChallengePeriodUpdateFailed: boolean;
    readonly isErrorChargingFee: boolean;
    readonly type:
      | 'InvalidMessage'
      | 'MessageNotFound'
      | 'ConsensusClientCreationFailed'
      | 'UnbondingPeriodUpdateFailed'
      | 'ChallengePeriodUpdateFailed'
      | 'ErrorChargingFee';
  }

  /** @name PalletHyperbridgeError (496) */
  type PalletHyperbridgeError = Null;

  /** @name PalletTokenGatewayError (498) */
  interface PalletTokenGatewayError extends Enum {
    readonly isUnregisteredAsset: boolean;
    readonly isAssetTeleportError: boolean;
    readonly isCoprocessorNotConfigured: boolean;
    readonly isDispatchError: boolean;
    readonly isAssetCreationError: boolean;
    readonly isAssetDecimalsNotFound: boolean;
    readonly isNotInitialized: boolean;
    readonly isUnknownAsset: boolean;
    readonly isNotAssetOwner: boolean;
    readonly type:
      | 'UnregisteredAsset'
      | 'AssetTeleportError'
      | 'CoprocessorNotConfigured'
      | 'DispatchError'
      | 'AssetCreationError'
      | 'AssetDecimalsNotFound'
      | 'NotInitialized'
      | 'UnknownAsset'
      | 'NotAssetOwner';
  }

  /** @name PalletTreasuryTreasuryPool (500) */
  interface PalletTreasuryTreasuryPool extends Struct {
    readonly bondHolders: Vec<ITuple<[AccountId32, PalletTreasuryBondHolder]>>;
    readonly doNotRenew: Vec<AccountId32>;
    readonly isRolledOver: bool;
    readonly distributedEarnings: Option<u128>;
    readonly vaultSharingPercent: Compact<Permill>;
  }

  /** @name PalletTreasuryBondHolder (503) */
  interface PalletTreasuryBondHolder extends Struct {
    readonly startingBalance: Compact<u128>;
    readonly earnings: Compact<u128>;
    readonly keepEarningsInPool: bool;
  }

  /** @name PalletTreasuryTreasuryCapital (510) */
  interface PalletTreasuryTreasuryCapital extends Struct {
    readonly vaultId: Compact<u32>;
    readonly activatedCapital: Compact<u128>;
    readonly frameId: Compact<u64>;
  }

  /** @name PalletTreasuryPrebondedArgons (512) */
  interface PalletTreasuryPrebondedArgons extends Struct {
    readonly vaultId: Compact<u32>;
    readonly accountId: AccountId32;
    readonly amountUnbonded: Compact<u128>;
    readonly startingFrameId: Compact<u64>;
    readonly bondedByStartOffset: Vec<u128>;
    readonly maxAmountPerFrame: Compact<u128>;
  }

  /** @name PalletTreasuryError (513) */
  interface PalletTreasuryError extends Enum {
    readonly isContributionTooLow: boolean;
    readonly isVaultNotAcceptingMiningBonds: boolean;
    readonly isBelowMinimum: boolean;
    readonly isNotAFundContributor: boolean;
    readonly isInternalError: boolean;
    readonly isCouldNotFindTreasury: boolean;
    readonly isMaxContributorsExceeded: boolean;
    readonly isActivatedSecuritizationExceeded: boolean;
    readonly isMaxVaultsExceeded: boolean;
    readonly isAlreadyRenewed: boolean;
    readonly isNotAVaultOperator: boolean;
    readonly isMaxAmountBelowMinimum: boolean;
    readonly type:
      | 'ContributionTooLow'
      | 'VaultNotAcceptingMiningBonds'
      | 'BelowMinimum'
      | 'NotAFundContributor'
      | 'InternalError'
      | 'CouldNotFindTreasury'
      | 'MaxContributorsExceeded'
      | 'ActivatedSecuritizationExceeded'
      | 'MaxVaultsExceeded'
      | 'AlreadyRenewed'
      | 'NotAVaultOperator'
      | 'MaxAmountBelowMinimum';
  }

  /** @name PalletFeeControlError (514) */
  interface PalletFeeControlError extends Enum {
    readonly isSponsoredFeeTooHigh: boolean;
    readonly type: 'SponsoredFeeTooHigh';
  }

  /** @name FrameSystemExtensionsAuthorizeCall (517) */
  type FrameSystemExtensionsAuthorizeCall = Null;

  /** @name FrameSystemExtensionsCheckNonZeroSender (518) */
  type FrameSystemExtensionsCheckNonZeroSender = Null;

  /** @name FrameSystemExtensionsCheckSpecVersion (519) */
  type FrameSystemExtensionsCheckSpecVersion = Null;

  /** @name FrameSystemExtensionsCheckTxVersion (520) */
  type FrameSystemExtensionsCheckTxVersion = Null;

  /** @name FrameSystemExtensionsCheckGenesis (521) */
  type FrameSystemExtensionsCheckGenesis = Null;

  /** @name FrameSystemExtensionsCheckNonce (524) */
  interface FrameSystemExtensionsCheckNonce extends Compact<u32> {}

  /** @name FrameSystemExtensionsCheckWeight (525) */
  type FrameSystemExtensionsCheckWeight = Null;

  /** @name PalletTransactionPaymentChargeTransactionPayment (526) */
  interface PalletTransactionPaymentChargeTransactionPayment extends Compact<u128> {}

  /** @name FrameMetadataHashExtensionCheckMetadataHash (527) */
  interface FrameMetadataHashExtensionCheckMetadataHash extends Struct {
    readonly mode: FrameMetadataHashExtensionMode;
  }

  /** @name FrameMetadataHashExtensionMode (528) */
  interface FrameMetadataHashExtensionMode extends Enum {
    readonly isDisabled: boolean;
    readonly isEnabled: boolean;
    readonly type: 'Disabled' | 'Enabled';
  }

  /** @name FrameSystemExtensionsWeightReclaim (529) */
  type FrameSystemExtensionsWeightReclaim = Null;

  /** @name ArgonRuntimeRuntime (531) */
  type ArgonRuntimeRuntime = Null;
} // declare module
