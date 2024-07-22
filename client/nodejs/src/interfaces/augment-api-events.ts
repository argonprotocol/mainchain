// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/events';

import type { ApiTypes, AugmentedEvent } from '@polkadot/api-base/types';
import type { Bytes, Null, Option, Result, U8aFixed, Vec, bool, u128, u16, u32, u64 } from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, H256 } from '@polkadot/types/interfaces/runtime';
import type { FrameSupportDispatchDispatchInfo, FrameSupportTokensMiscBalanceStatus, PalletDataDomainDataDomainRegistration, PalletMintMintType, PalletMultisigTimepoint, SpConsensusGrandpaAppPublic, SpRuntimeDispatchError, UlxNodeRuntimeProxyType, UlxNotaryAuditErrorVerifyError, UlxPrimitivesBitcoinBitcoinRejectedReason, UlxPrimitivesBitcoinCompressedBitcoinPubkey, UlxPrimitivesBitcoinUtxoRef, UlxPrimitivesBlockSealBlockPayout, UlxPrimitivesBlockSealMiningRegistration, UlxPrimitivesBondBondExpiration, UlxPrimitivesBondBondType, UlxPrimitivesDataDomainZoneRecord, UlxPrimitivesNotaryNotaryMeta, UlxPrimitivesNotaryNotaryRecord } from '@polkadot/types/lookup';

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
       * The `TotalIssuance` was forcefully changed.
       **/
      TotalIssuanceForced: AugmentedEvent<ApiType, [old: u128, new_: u128], { old: u128, new_: u128 }>;
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
    bitcoinUtxos: {
      UtxoExpiredError: AugmentedEvent<ApiType, [utxoRef: UlxPrimitivesBitcoinUtxoRef, error: SpRuntimeDispatchError], { utxoRef: UlxPrimitivesBitcoinUtxoRef, error: SpRuntimeDispatchError }>;
      UtxoRejected: AugmentedEvent<ApiType, [utxoId: u64, rejectedReason: UlxPrimitivesBitcoinBitcoinRejectedReason], { utxoId: u64, rejectedReason: UlxPrimitivesBitcoinBitcoinRejectedReason }>;
      UtxoRejectedError: AugmentedEvent<ApiType, [utxoId: u64, error: SpRuntimeDispatchError], { utxoId: u64, error: SpRuntimeDispatchError }>;
      UtxoSpent: AugmentedEvent<ApiType, [utxoId: u64, blockHeight: u64], { utxoId: u64, blockHeight: u64 }>;
      UtxoSpentError: AugmentedEvent<ApiType, [utxoId: u64, error: SpRuntimeDispatchError], { utxoId: u64, error: SpRuntimeDispatchError }>;
      UtxoUnwatched: AugmentedEvent<ApiType, [utxoId: u64], { utxoId: u64 }>;
      UtxoVerified: AugmentedEvent<ApiType, [utxoId: u64], { utxoId: u64 }>;
      UtxoVerifiedError: AugmentedEvent<ApiType, [utxoId: u64, error: SpRuntimeDispatchError], { utxoId: u64, error: SpRuntimeDispatchError }>;
    };
    blockRewards: {
      RewardCreated: AugmentedEvent<ApiType, [maturationBlock: u32, rewards: Vec<UlxPrimitivesBlockSealBlockPayout>], { maturationBlock: u32, rewards: Vec<UlxPrimitivesBlockSealBlockPayout> }>;
      RewardCreateError: AugmentedEvent<ApiType, [accountId: AccountId32, argons: Option<u128>, ulixees: Option<u128>, error: SpRuntimeDispatchError], { accountId: AccountId32, argons: Option<u128>, ulixees: Option<u128>, error: SpRuntimeDispatchError }>;
      RewardUnlocked: AugmentedEvent<ApiType, [rewards: Vec<UlxPrimitivesBlockSealBlockPayout>], { rewards: Vec<UlxPrimitivesBlockSealBlockPayout> }>;
      RewardUnlockError: AugmentedEvent<ApiType, [accountId: AccountId32, argons: Option<u128>, ulixees: Option<u128>, error: SpRuntimeDispatchError], { accountId: AccountId32, argons: Option<u128>, ulixees: Option<u128>, error: SpRuntimeDispatchError }>;
    };
    blockSealSpec: {
      ComputeDifficultyAdjusted: AugmentedEvent<ApiType, [expectedBlockTime: u64, actualBlockTime: u64, startDifficulty: u128, newDifficulty: u128], { expectedBlockTime: u64, actualBlockTime: u64, startDifficulty: u128, newDifficulty: u128 }>;
      VoteMinimumAdjusted: AugmentedEvent<ApiType, [expectedBlockVotes: u128, actualBlockVotes: u128, startVoteMinimum: u128, newVoteMinimum: u128], { expectedBlockVotes: u128, actualBlockVotes: u128, startVoteMinimum: u128, newVoteMinimum: u128 }>;
    };
    bonds: {
      BitcoinBondBurned: AugmentedEvent<ApiType, [vaultId: u32, bondId: u64, utxoId: u64, amountBurned: u128, amountHeld: u128, wasUtxoSpent: bool], { vaultId: u32, bondId: u64, utxoId: u64, amountBurned: u128, amountHeld: u128, wasUtxoSpent: bool }>;
      BitcoinCosignPastDue: AugmentedEvent<ApiType, [bondId: u64, vaultId: u32, utxoId: u64, compensationAmount: u128, compensationStillOwed: u128, compensatedAccountId: AccountId32], { bondId: u64, vaultId: u32, utxoId: u64, compensationAmount: u128, compensationStillOwed: u128, compensatedAccountId: AccountId32 }>;
      BitcoinUtxoCosigned: AugmentedEvent<ApiType, [bondId: u64, vaultId: u32, utxoId: u64, pubkey: UlxPrimitivesBitcoinCompressedBitcoinPubkey, signature: Bytes], { bondId: u64, vaultId: u32, utxoId: u64, pubkey: UlxPrimitivesBitcoinCompressedBitcoinPubkey, signature: Bytes }>;
      BitcoinUtxoCosignRequested: AugmentedEvent<ApiType, [bondId: u64, vaultId: u32, utxoId: u64], { bondId: u64, vaultId: u32, utxoId: u64 }>;
      BondCanceled: AugmentedEvent<ApiType, [vaultId: u32, bondId: u64, bondedAccountId: AccountId32, bondType: UlxPrimitivesBondBondType, returnedFee: u128], { vaultId: u32, bondId: u64, bondedAccountId: AccountId32, bondType: UlxPrimitivesBondBondType, returnedFee: u128 }>;
      BondCompleted: AugmentedEvent<ApiType, [vaultId: u32, bondId: u64], { vaultId: u32, bondId: u64 }>;
      /**
       * An error occurred while completing a bond
       **/
      BondCompletionError: AugmentedEvent<ApiType, [bondId: u64, error: SpRuntimeDispatchError], { bondId: u64, error: SpRuntimeDispatchError }>;
      BondCreated: AugmentedEvent<ApiType, [vaultId: u32, bondId: u64, bondType: UlxPrimitivesBondBondType, bondedAccountId: AccountId32, utxoId: Option<u64>, amount: u128, expiration: UlxPrimitivesBondBondExpiration], { vaultId: u32, bondId: u64, bondType: UlxPrimitivesBondBondType, bondedAccountId: AccountId32, utxoId: Option<u64>, amount: u128, expiration: UlxPrimitivesBondBondExpiration }>;
      /**
       * An error occurred while refunding an overdue cosigned bitcoin bond
       **/
      CosignOverdueError: AugmentedEvent<ApiType, [utxoId: u64, error: SpRuntimeDispatchError], { utxoId: u64, error: SpRuntimeDispatchError }>;
    };
    chainTransfer: {
      /**
       * A localchain transfer could not be cleaned up properly. Possible invalid transfer
       * needing investigation.
       **/
      PossibleInvalidTransferAllowed: AugmentedEvent<ApiType, [transferId: u32, notaryId: u32, notebookNumber: u32], { transferId: u32, notaryId: u32, notebookNumber: u32 }>;
      /**
       * Taxation failed
       **/
      TaxationError: AugmentedEvent<ApiType, [notaryId: u32, notebookNumber: u32, tax: u128, error: SpRuntimeDispatchError], { notaryId: u32, notebookNumber: u32, tax: u128, error: SpRuntimeDispatchError }>;
      TransferIn: AugmentedEvent<ApiType, [accountId: AccountId32, amount: u128, notaryId: u32], { accountId: AccountId32, amount: u128, notaryId: u32 }>;
      /**
       * A transfer into the mainchain failed
       **/
      TransferInError: AugmentedEvent<ApiType, [accountId: AccountId32, amount: u128, notaryId: u32, notebookNumber: u32, error: SpRuntimeDispatchError], { accountId: AccountId32, amount: u128, notaryId: u32, notebookNumber: u32, error: SpRuntimeDispatchError }>;
      TransferToLocalchain: AugmentedEvent<ApiType, [accountId: AccountId32, amount: u128, transferId: u32, notaryId: u32, expirationTick: u32], { accountId: AccountId32, amount: u128, transferId: u32, notaryId: u32, expirationTick: u32 }>;
      TransferToLocalchainExpired: AugmentedEvent<ApiType, [accountId: AccountId32, transferId: u32, notaryId: u32], { accountId: AccountId32, transferId: u32, notaryId: u32 }>;
      /**
       * An expired transfer to localchain failed to be refunded
       **/
      TransferToLocalchainRefundError: AugmentedEvent<ApiType, [accountId: AccountId32, transferId: u32, notaryId: u32, notebookNumber: u32, error: SpRuntimeDispatchError], { accountId: AccountId32, transferId: u32, notaryId: u32, notebookNumber: u32, error: SpRuntimeDispatchError }>;
    };
    dataDomain: {
      /**
       * A data domain was expired
       **/
      DataDomainExpired: AugmentedEvent<ApiType, [domainHash: H256], { domainHash: H256 }>;
      /**
       * A data domain was registered
       **/
      DataDomainRegistered: AugmentedEvent<ApiType, [domainHash: H256, registration: PalletDataDomainDataDomainRegistration], { domainHash: H256, registration: PalletDataDomainDataDomainRegistration }>;
      /**
       * A data domain registration was canceled due to a conflicting registration in the same
       * tick
       **/
      DataDomainRegistrationCanceled: AugmentedEvent<ApiType, [domainHash: H256, registration: PalletDataDomainDataDomainRegistration], { domainHash: H256, registration: PalletDataDomainDataDomainRegistration }>;
      /**
       * A data domain registration failed due to an error
       **/
      DataDomainRegistrationError: AugmentedEvent<ApiType, [domainHash: H256, accountId: AccountId32, error: SpRuntimeDispatchError], { domainHash: H256, accountId: AccountId32, error: SpRuntimeDispatchError }>;
      /**
       * A data domain was registered
       **/
      DataDomainRenewed: AugmentedEvent<ApiType, [domainHash: H256], { domainHash: H256 }>;
      /**
       * A data domain zone record was updated
       **/
      ZoneRecordUpdated: AugmentedEvent<ApiType, [domainHash: H256, zoneRecord: UlxPrimitivesDataDomainZoneRecord], { domainHash: H256, zoneRecord: UlxPrimitivesDataDomainZoneRecord }>;
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
      UnbondMinerError: AugmentedEvent<ApiType, [accountId: AccountId32, bondId: Option<u64>, error: SpRuntimeDispatchError], { accountId: AccountId32, bondId: Option<u64>, error: SpRuntimeDispatchError }>;
    };
    mint: {
      ArgonsMinted: AugmentedEvent<ApiType, [mintType: PalletMintMintType, accountId: AccountId32, utxoId: Option<u64>, amount: u128], { mintType: PalletMintMintType, accountId: AccountId32, utxoId: Option<u64>, amount: u128 }>;
      MintError: AugmentedEvent<ApiType, [mintType: PalletMintMintType, accountId: AccountId32, utxoId: Option<u64>, amount: u128, error: SpRuntimeDispatchError], { mintType: PalletMintMintType, accountId: AccountId32, utxoId: Option<u64>, amount: u128, error: SpRuntimeDispatchError }>;
    };
    multisig: {
      /**
       * A multisig operation has been approved by someone.
       **/
      MultisigApproval: AugmentedEvent<ApiType, [approving: AccountId32, timepoint: PalletMultisigTimepoint, multisig: AccountId32, callHash: U8aFixed], { approving: AccountId32, timepoint: PalletMultisigTimepoint, multisig: AccountId32, callHash: U8aFixed }>;
      /**
       * A multisig operation has been cancelled.
       **/
      MultisigCancelled: AugmentedEvent<ApiType, [cancelling: AccountId32, timepoint: PalletMultisigTimepoint, multisig: AccountId32, callHash: U8aFixed], { cancelling: AccountId32, timepoint: PalletMultisigTimepoint, multisig: AccountId32, callHash: U8aFixed }>;
      /**
       * A multisig operation has been executed.
       **/
      MultisigExecuted: AugmentedEvent<ApiType, [approving: AccountId32, timepoint: PalletMultisigTimepoint, multisig: AccountId32, callHash: U8aFixed, result: Result<Null, SpRuntimeDispatchError>], { approving: AccountId32, timepoint: PalletMultisigTimepoint, multisig: AccountId32, callHash: U8aFixed, result: Result<Null, SpRuntimeDispatchError> }>;
      /**
       * A new multisig operation has begun.
       **/
      NewMultisig: AugmentedEvent<ApiType, [approving: AccountId32, multisig: AccountId32, callHash: U8aFixed], { approving: AccountId32, multisig: AccountId32, callHash: U8aFixed }>;
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
       * Error updating queued notary info
       **/
      NotaryMetaUpdateError: AugmentedEvent<ApiType, [notaryId: u32, error: SpRuntimeDispatchError, meta: UlxPrimitivesNotaryNotaryMeta], { notaryId: u32, error: SpRuntimeDispatchError, meta: UlxPrimitivesNotaryNotaryMeta }>;
      /**
       * Notary metadata queued for update
       **/
      NotaryMetaUpdateQueued: AugmentedEvent<ApiType, [notaryId: u32, meta: UlxPrimitivesNotaryNotaryMeta, effectiveTick: u32], { notaryId: u32, meta: UlxPrimitivesNotaryNotaryMeta, effectiveTick: u32 }>;
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
    priceIndex: {
      /**
       * Event emitted when a new price index is submitted
       **/
      NewIndex: AugmentedEvent<ApiType, []>;
      OperatorChanged: AugmentedEvent<ApiType, [operatorId: AccountId32], { operatorId: AccountId32 }>;
    };
    proxy: {
      /**
       * An announcement was placed to make a call in the future.
       **/
      Announced: AugmentedEvent<ApiType, [real: AccountId32, proxy: AccountId32, callHash: H256], { real: AccountId32, proxy: AccountId32, callHash: H256 }>;
      /**
       * A proxy was added.
       **/
      ProxyAdded: AugmentedEvent<ApiType, [delegator: AccountId32, delegatee: AccountId32, proxyType: UlxNodeRuntimeProxyType, delay: u32], { delegator: AccountId32, delegatee: AccountId32, proxyType: UlxNodeRuntimeProxyType, delay: u32 }>;
      /**
       * A proxy was executed correctly, with the given.
       **/
      ProxyExecuted: AugmentedEvent<ApiType, [result: Result<Null, SpRuntimeDispatchError>], { result: Result<Null, SpRuntimeDispatchError> }>;
      /**
       * A proxy was removed.
       **/
      ProxyRemoved: AugmentedEvent<ApiType, [delegator: AccountId32, delegatee: AccountId32, proxyType: UlxNodeRuntimeProxyType, delay: u32], { delegator: AccountId32, delegatee: AccountId32, proxyType: UlxNodeRuntimeProxyType, delay: u32 }>;
      /**
       * A pure account has been created by new proxy with given
       * disambiguation index and proxy type.
       **/
      PureCreated: AugmentedEvent<ApiType, [pure: AccountId32, who: AccountId32, proxyType: UlxNodeRuntimeProxyType, disambiguationIndex: u16], { pure: AccountId32, who: AccountId32, proxyType: UlxNodeRuntimeProxyType, disambiguationIndex: u16 }>;
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
      /**
       * An upgrade was authorized.
       **/
      UpgradeAuthorized: AugmentedEvent<ApiType, [codeHash: H256, checkVersion: bool], { codeHash: H256, checkVersion: bool }>;
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
       * The `TotalIssuance` was forcefully changed.
       **/
      TotalIssuanceForced: AugmentedEvent<ApiType, [old: u128, new_: u128], { old: u128, new_: u128 }>;
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
    vaults: {
      VaultClosed: AugmentedEvent<ApiType, [vaultId: u32, bitcoinAmountStillBonded: u128, miningAmountStillBonded: u128, securitizationStillBonded: u128], { vaultId: u32, bitcoinAmountStillBonded: u128, miningAmountStillBonded: u128, securitizationStillBonded: u128 }>;
      VaultCreated: AugmentedEvent<ApiType, [vaultId: u32, bitcoinArgons: u128, miningArgons: u128, securitizationPercent: u128, operatorAccountId: AccountId32], { vaultId: u32, bitcoinArgons: u128, miningArgons: u128, securitizationPercent: u128, operatorAccountId: AccountId32 }>;
      VaultModified: AugmentedEvent<ApiType, [vaultId: u32, bitcoinArgons: u128, miningArgons: u128, securitizationPercent: u128], { vaultId: u32, bitcoinArgons: u128, miningArgons: u128, securitizationPercent: u128 }>;
      VaultTermsChanged: AugmentedEvent<ApiType, [vaultId: u32], { vaultId: u32 }>;
      VaultTermsChangeScheduled: AugmentedEvent<ApiType, [vaultId: u32, changeBlock: u32], { vaultId: u32, changeBlock: u32 }>;
    };
  } // AugmentedEvents
} // declare module
