// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/events';

import type { ApiTypes, AugmentedEvent } from '@polkadot/api-base/types';
import type { Bytes, Null, Option, Result, U8aFixed, Vec, bool, u128, u32, u64 } from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, H256 } from '@polkadot/types/interfaces/runtime';
import type { FrameSupportDispatchDispatchInfo, FrameSupportTokensMiscBalanceStatus, PalletBlockRewardsBlockPayout, PalletDataDomainDataDomainRegistration, SpConsensusGrandpaAppPublic, SpRuntimeDispatchError, UlxNotaryAuditErrorVerifyError, UlxPrimitivesBlockSealMiningRegistration, UlxPrimitivesDataDomain, UlxPrimitivesDataDomainZoneRecord, UlxPrimitivesNotaryNotaryMeta, UlxPrimitivesNotaryNotaryRecord } from '@polkadot/types/lookup';

export type __AugmentedEvent<ApiType extends ApiTypes> = AugmentedEvent<ApiType>;

declare module '@polkadot/api-base/types/events' {
  interface AugmentedEvents<ApiType extends ApiTypes> {
    argonBalances: {
      /**
       * A balance was set by root.
       **/
      BalanceSet: AugmentedEvent<ApiType, [who: AccountId32, free: u128], { who: AccountId32, free: u128 }>;
      /**
       * Some amount was burned from an account.
       **/
      Burned: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was deposited (e.g. for transaction fees).
       **/
      Deposit: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * An account was removed whose balance was non-zero but below ExistentialDeposit,
       * resulting in an outright loss.
       **/
      DustLost: AugmentedEvent<ApiType, [account: AccountId32, amount: u128], { account: AccountId32, amount: u128 }>;
      /**
       * An account was created with some free balance.
       **/
      Endowed: AugmentedEvent<ApiType, [account: AccountId32, freeBalance: u128], { account: AccountId32, freeBalance: u128 }>;
      /**
       * Some balance was frozen.
       **/
      Frozen: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Total issuance was increased by `amount`, creating a credit to be balanced.
       **/
      Issued: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was locked.
       **/
      Locked: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was minted into an account.
       **/
      Minted: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Total issuance was decreased by `amount`, creating a debt to be balanced.
       **/
      Rescinded: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was reserved (moved from free to reserved).
       **/
      Reserved: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some balance was moved from the reserve of the first account to the second account.
       * Final argument indicates the destination balance type.
       **/
      ReserveRepatriated: AugmentedEvent<ApiType, [from: AccountId32, to: AccountId32, amount: u128, destinationStatus: FrameSupportTokensMiscBalanceStatus], { from: AccountId32, to: AccountId32, amount: u128, destinationStatus: FrameSupportTokensMiscBalanceStatus }>;
      /**
       * Some amount was restored into an account.
       **/
      Restored: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was removed from the account (e.g. for misbehavior).
       **/
      Slashed: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was suspended from an account (it can be restored later).
       **/
      Suspended: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some balance was thawed.
       **/
      Thawed: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Transfer succeeded.
       **/
      Transfer: AugmentedEvent<ApiType, [from: AccountId32, to: AccountId32, amount: u128], { from: AccountId32, to: AccountId32, amount: u128 }>;
      /**
       * Some balance was unlocked.
       **/
      Unlocked: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some balance was unreserved (moved from reserved to free).
       **/
      Unreserved: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * An account was upgraded.
       **/
      Upgraded: AugmentedEvent<ApiType, [who: AccountId32], { who: AccountId32 }>;
      /**
       * Some amount was withdrawn from the account (e.g. for transaction fees).
       **/
      Withdraw: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
    };
    blockRewards: {
      RewardCreated: AugmentedEvent<ApiType, [maturationBlock: u32, rewards: Vec<PalletBlockRewardsBlockPayout>], { maturationBlock: u32, rewards: Vec<PalletBlockRewardsBlockPayout> }>;
      RewardUnlocked: AugmentedEvent<ApiType, [rewards: Vec<PalletBlockRewardsBlockPayout>], { rewards: Vec<PalletBlockRewardsBlockPayout> }>;
    };
    blockSealSpec: {
      ComputeDifficultyAdjusted: AugmentedEvent<ApiType, [expectedBlockTime: u64, actualBlockTime: u64, startDifficulty: u128, newDifficulty: u128], { expectedBlockTime: u64, actualBlockTime: u64, startDifficulty: u128, newDifficulty: u128 }>;
      VoteMinimumAdjusted: AugmentedEvent<ApiType, [expectedBlockVotes: u128, actualBlockVotes: u128, startVoteMinimum: u128, newVoteMinimum: u128], { expectedBlockVotes: u128, actualBlockVotes: u128, startVoteMinimum: u128, newVoteMinimum: u128 }>;
    };
    bond: {
      BondCompleted: AugmentedEvent<ApiType, [bondFundId: Option<u32>, bondId: u64], { bondFundId: Option<u32>, bondId: u64 }>;
      BondedSelf: AugmentedEvent<ApiType, [bondId: u64, bondedAccountId: AccountId32, amount: u128, completionBlock: u32], { bondId: u64, bondedAccountId: AccountId32, amount: u128, completionBlock: u32 }>;
      BondExtended: AugmentedEvent<ApiType, [bondFundId: Option<u32>, bondId: u64, amount: u128, completionBlock: u32, feeChange: u128, annualPercentRate: u32], { bondFundId: Option<u32>, bondId: u64, amount: u128, completionBlock: u32, feeChange: u128, annualPercentRate: u32 }>;
      BondFeeRefund: AugmentedEvent<ApiType, [bondFundId: u32, bondId: u64, bondedAccountId: AccountId32, bondFundReductionForPayment: u128, finalFee: u128, refundAmount: u128], { bondFundId: u32, bondId: u64, bondedAccountId: AccountId32, bondFundReductionForPayment: u128, finalFee: u128, refundAmount: u128 }>;
      BondFundEnded: AugmentedEvent<ApiType, [bondFundId: u32, amountStillBonded: u128], { bondFundId: u32, amountStillBonded: u128 }>;
      BondFundExpired: AugmentedEvent<ApiType, [bondFundId: u32, offerAccountId: AccountId32], { bondFundId: u32, offerAccountId: AccountId32 }>;
      BondFundExtended: AugmentedEvent<ApiType, [bondFundId: u32, amountOffered: u128, expirationBlock: u32], { bondFundId: u32, amountOffered: u128, expirationBlock: u32 }>;
      BondFundOffered: AugmentedEvent<ApiType, [bondFundId: u32, amountOffered: u128, expirationBlock: u32, offerAccountId: AccountId32], { bondFundId: u32, amountOffered: u128, expirationBlock: u32, offerAccountId: AccountId32 }>;
      BondLeased: AugmentedEvent<ApiType, [bondFundId: u32, bondId: u64, bondedAccountId: AccountId32, amount: u128, totalFee: u128, annualPercentRate: u32, completionBlock: u32], { bondFundId: u32, bondId: u64, bondedAccountId: AccountId32, amount: u128, totalFee: u128, annualPercentRate: u32, completionBlock: u32 }>;
      BondLocked: AugmentedEvent<ApiType, [bondId: u64, bondedAccountId: AccountId32], { bondId: u64, bondedAccountId: AccountId32 }>;
      BondUnlocked: AugmentedEvent<ApiType, [bondId: u64, bondedAccountId: AccountId32], { bondId: u64, bondedAccountId: AccountId32 }>;
    };
    chainTransfer: {
      TransferIn: AugmentedEvent<ApiType, [accountId: AccountId32, amount: u128, notaryId: u32], { accountId: AccountId32, amount: u128, notaryId: u32 }>;
      TransferToLocalchain: AugmentedEvent<ApiType, [accountId: AccountId32, amount: u128, accountNonce: u32, notaryId: u32, expirationBlock: u32], { accountId: AccountId32, amount: u128, accountNonce: u32, notaryId: u32, expirationBlock: u32 }>;
      TransferToLocalchainExpired: AugmentedEvent<ApiType, [accountId: AccountId32, accountNonce: u32, notaryId: u32], { accountId: AccountId32, accountNonce: u32, notaryId: u32 }>;
    };
    dataDomain: {
      /**
       * A data domain was expired
       **/
      DataDomainExpired: AugmentedEvent<ApiType, [domain: UlxPrimitivesDataDomain], { domain: UlxPrimitivesDataDomain }>;
      /**
       * A data domain was registered
       **/
      DataDomainRegistered: AugmentedEvent<ApiType, [domain: UlxPrimitivesDataDomain, registration: PalletDataDomainDataDomainRegistration], { domain: UlxPrimitivesDataDomain, registration: PalletDataDomainDataDomainRegistration }>;
      /**
       * A data domain registration was canceled due to a conflicting registration in the same
       * tick
       **/
      DataDomainRegistrationCanceled: AugmentedEvent<ApiType, [domain: UlxPrimitivesDataDomain, registration: PalletDataDomainDataDomainRegistration], { domain: UlxPrimitivesDataDomain, registration: PalletDataDomainDataDomainRegistration }>;
      /**
       * A data domain was registered
       **/
      DataDomainRenewed: AugmentedEvent<ApiType, [domain: UlxPrimitivesDataDomain], { domain: UlxPrimitivesDataDomain }>;
      /**
       * A data domain zone record was updated
       **/
      ZoneRecordUpdated: AugmentedEvent<ApiType, [domain: UlxPrimitivesDataDomain, zoneRecord: UlxPrimitivesDataDomainZoneRecord], { domain: UlxPrimitivesDataDomain, zoneRecord: UlxPrimitivesDataDomainZoneRecord }>;
    };
    grandpa: {
      /**
       * New authority set has been applied.
       **/
      NewAuthorities: AugmentedEvent<ApiType, [authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>], { authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>> }>;
      /**
       * Current authority set has been paused.
       **/
      Paused: AugmentedEvent<ApiType, []>;
      /**
       * Current authority set has been resumed.
       **/
      Resumed: AugmentedEvent<ApiType, []>;
    };
    miningSlot: {
      NewMiners: AugmentedEvent<ApiType, [startIndex: u32, newMiners: Vec<UlxPrimitivesBlockSealMiningRegistration>], { startIndex: u32, newMiners: Vec<UlxPrimitivesBlockSealMiningRegistration> }>;
      SlotBidderAdded: AugmentedEvent<ApiType, [accountId: AccountId32, bidAmount: u128, index: u32], { accountId: AccountId32, bidAmount: u128, index: u32 }>;
      SlotBidderReplaced: AugmentedEvent<ApiType, [accountId: AccountId32, bondId: Option<u64>, keptOwnershipBond: bool], { accountId: AccountId32, bondId: Option<u64>, keptOwnershipBond: bool }>;
      UnbondedMiner: AugmentedEvent<ApiType, [accountId: AccountId32, bondId: Option<u64>, keptOwnershipBond: bool], { accountId: AccountId32, bondId: Option<u64>, keptOwnershipBond: bool }>;
    };
    mint: {
    };
    notaries: {
      /**
       * A notary proposal has been accepted
       **/
      NotaryActivated: AugmentedEvent<ApiType, [notary: UlxPrimitivesNotaryNotaryRecord], { notary: UlxPrimitivesNotaryNotaryRecord }>;
      /**
       * Notary metadata updated
       **/
      NotaryMetaUpdated: AugmentedEvent<ApiType, [notaryId: u32, meta: UlxPrimitivesNotaryNotaryMeta], { notaryId: u32, meta: UlxPrimitivesNotaryNotaryMeta }>;
      /**
       * Notary metadata queued for update
       **/
      NotaryMetaUpdateQueued: AugmentedEvent<ApiType, [notaryId: u32, meta: UlxPrimitivesNotaryNotaryMeta, effectiveBlock: u32], { notaryId: u32, meta: UlxPrimitivesNotaryNotaryMeta, effectiveBlock: u32 }>;
      /**
       * A user has proposed operating as a notary
       **/
      NotaryProposed: AugmentedEvent<ApiType, [operatorAccount: AccountId32, meta: UlxPrimitivesNotaryNotaryMeta, expires: u32], { operatorAccount: AccountId32, meta: UlxPrimitivesNotaryNotaryMeta, expires: u32 }>;
    };
    notebook: {
      NotebookAuditFailure: AugmentedEvent<ApiType, [notaryId: u32, notebookNumber: u32, firstFailureReason: UlxNotaryAuditErrorVerifyError], { notaryId: u32, notebookNumber: u32, firstFailureReason: UlxNotaryAuditErrorVerifyError }>;
      NotebookSubmitted: AugmentedEvent<ApiType, [notaryId: u32, notebookNumber: u32], { notaryId: u32, notebookNumber: u32 }>;
    };
    offences: {
      /**
       * There is an offence reported of the given `kind` happened at the `session_index` and
       * (kind-specific) time slot. This event is not deposited for duplicate slashes.
       * \[kind, timeslot\].
       **/
      Offence: AugmentedEvent<ApiType, [kind: U8aFixed, timeslot: Bytes], { kind: U8aFixed, timeslot: Bytes }>;
    };
    session: {
      /**
       * New session has happened. Note that the argument is the session index, not the
       * block number as the type might suggest.
       **/
      NewSession: AugmentedEvent<ApiType, [sessionIndex: u32], { sessionIndex: u32 }>;
    };
    sudo: {
      /**
       * The sudo key has been updated.
       **/
      KeyChanged: AugmentedEvent<ApiType, [old: Option<AccountId32>, new_: AccountId32], { old: Option<AccountId32>, new_: AccountId32 }>;
      /**
       * The key was permanently removed.
       **/
      KeyRemoved: AugmentedEvent<ApiType, []>;
      /**
       * A sudo call just took place.
       **/
      Sudid: AugmentedEvent<ApiType, [sudoResult: Result<Null, SpRuntimeDispatchError>], { sudoResult: Result<Null, SpRuntimeDispatchError> }>;
      /**
       * A [sudo_as](Pallet::sudo_as) call just took place.
       **/
      SudoAsDone: AugmentedEvent<ApiType, [sudoResult: Result<Null, SpRuntimeDispatchError>], { sudoResult: Result<Null, SpRuntimeDispatchError> }>;
    };
    system: {
      /**
       * `:code` was updated.
       **/
      CodeUpdated: AugmentedEvent<ApiType, []>;
      /**
       * An extrinsic failed.
       **/
      ExtrinsicFailed: AugmentedEvent<ApiType, [dispatchError: SpRuntimeDispatchError, dispatchInfo: FrameSupportDispatchDispatchInfo], { dispatchError: SpRuntimeDispatchError, dispatchInfo: FrameSupportDispatchDispatchInfo }>;
      /**
       * An extrinsic completed successfully.
       **/
      ExtrinsicSuccess: AugmentedEvent<ApiType, [dispatchInfo: FrameSupportDispatchDispatchInfo], { dispatchInfo: FrameSupportDispatchDispatchInfo }>;
      /**
       * An account was reaped.
       **/
      KilledAccount: AugmentedEvent<ApiType, [account: AccountId32], { account: AccountId32 }>;
      /**
       * A new account was created.
       **/
      NewAccount: AugmentedEvent<ApiType, [account: AccountId32], { account: AccountId32 }>;
      /**
       * On on-chain remark happened.
       **/
      Remarked: AugmentedEvent<ApiType, [sender: AccountId32, hash_: H256], { sender: AccountId32, hash_: H256 }>;
    };
    transactionPayment: {
      /**
       * A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,
       * has been paid by `who`.
       **/
      TransactionFeePaid: AugmentedEvent<ApiType, [who: AccountId32, actualFee: u128, tip: u128], { who: AccountId32, actualFee: u128, tip: u128 }>;
    };
    txPause: {
      /**
       * This pallet, or a specific call is now paused.
       **/
      CallPaused: AugmentedEvent<ApiType, [fullName: ITuple<[Bytes, Bytes]>], { fullName: ITuple<[Bytes, Bytes]> }>;
      /**
       * This pallet, or a specific call is now unpaused.
       **/
      CallUnpaused: AugmentedEvent<ApiType, [fullName: ITuple<[Bytes, Bytes]>], { fullName: ITuple<[Bytes, Bytes]> }>;
    };
    ulixeeBalances: {
      /**
       * A balance was set by root.
       **/
      BalanceSet: AugmentedEvent<ApiType, [who: AccountId32, free: u128], { who: AccountId32, free: u128 }>;
      /**
       * Some amount was burned from an account.
       **/
      Burned: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was deposited (e.g. for transaction fees).
       **/
      Deposit: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * An account was removed whose balance was non-zero but below ExistentialDeposit,
       * resulting in an outright loss.
       **/
      DustLost: AugmentedEvent<ApiType, [account: AccountId32, amount: u128], { account: AccountId32, amount: u128 }>;
      /**
       * An account was created with some free balance.
       **/
      Endowed: AugmentedEvent<ApiType, [account: AccountId32, freeBalance: u128], { account: AccountId32, freeBalance: u128 }>;
      /**
       * Some balance was frozen.
       **/
      Frozen: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Total issuance was increased by `amount`, creating a credit to be balanced.
       **/
      Issued: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was locked.
       **/
      Locked: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was minted into an account.
       **/
      Minted: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Total issuance was decreased by `amount`, creating a debt to be balanced.
       **/
      Rescinded: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was reserved (moved from free to reserved).
       **/
      Reserved: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some balance was moved from the reserve of the first account to the second account.
       * Final argument indicates the destination balance type.
       **/
      ReserveRepatriated: AugmentedEvent<ApiType, [from: AccountId32, to: AccountId32, amount: u128, destinationStatus: FrameSupportTokensMiscBalanceStatus], { from: AccountId32, to: AccountId32, amount: u128, destinationStatus: FrameSupportTokensMiscBalanceStatus }>;
      /**
       * Some amount was restored into an account.
       **/
      Restored: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was removed from the account (e.g. for misbehavior).
       **/
      Slashed: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some amount was suspended from an account (it can be restored later).
       **/
      Suspended: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some balance was thawed.
       **/
      Thawed: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Transfer succeeded.
       **/
      Transfer: AugmentedEvent<ApiType, [from: AccountId32, to: AccountId32, amount: u128], { from: AccountId32, to: AccountId32, amount: u128 }>;
      /**
       * Some balance was unlocked.
       **/
      Unlocked: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * Some balance was unreserved (moved from reserved to free).
       **/
      Unreserved: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
      /**
       * An account was upgraded.
       **/
      Upgraded: AugmentedEvent<ApiType, [who: AccountId32], { who: AccountId32 }>;
      /**
       * Some amount was withdrawn from the account (e.g. for transaction fees).
       **/
      Withdraw: AugmentedEvent<ApiType, [who: AccountId32, amount: u128], { who: AccountId32, amount: u128 }>;
    };
  } // AugmentedEvents
} // declare module
