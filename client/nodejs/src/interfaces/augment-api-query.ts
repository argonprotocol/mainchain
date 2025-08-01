// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/storage';

import type { ApiTypes, AugmentedQuery, QueryableStorageEntry } from '@polkadot/api-base/types';
import type {
  BTreeMap,
  BTreeSet,
  Bytes,
  Null,
  Option,
  U256,
  U8aFixed,
  Vec,
  bool,
  u128,
  u16,
  u32,
  u64,
  u8,
} from '@polkadot/types-codec';
import type { AnyNumber, ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, H256 } from '@polkadot/types/interfaces/runtime';
import type {
  ArgonNotaryAuditErrorVerifyError,
  ArgonPrimitivesBalanceChangeAccountOrigin,
  ArgonPrimitivesBitcoinBitcoinBlock,
  ArgonPrimitivesBitcoinBitcoinNetwork,
  ArgonPrimitivesBitcoinBitcoinXPub,
  ArgonPrimitivesBitcoinUtxoRef,
  ArgonPrimitivesBitcoinUtxoValue,
  ArgonPrimitivesBlockSealBlockPayout,
  ArgonPrimitivesBlockSealMiningBidStats,
  ArgonPrimitivesBlockSealMiningRegistration,
  ArgonPrimitivesBlockSealMiningSlotConfig,
  ArgonPrimitivesDigestsBlockVoteDigest,
  ArgonPrimitivesDigestsDigestset,
  ArgonPrimitivesDigestsNotebookDigest,
  ArgonPrimitivesDomainZoneRecord,
  ArgonPrimitivesForkPower,
  ArgonPrimitivesInherentsBlockSealInherent,
  ArgonPrimitivesNotaryNotaryMeta,
  ArgonPrimitivesNotaryNotaryNotebookKeyDetails,
  ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails,
  ArgonPrimitivesNotaryNotaryRecord,
  ArgonPrimitivesProvidersBlockSealerInfo,
  ArgonPrimitivesTickTicker,
  ArgonPrimitivesVault,
  FrameSupportDispatchPerDispatchClassWeight,
  FrameSupportTokensMiscIdAmountRuntimeFreezeReason,
  FrameSupportTokensMiscIdAmountRuntimeHoldReason,
  FrameSystemAccountInfo,
  FrameSystemCodeUpgradeAuthorization,
  FrameSystemEventRecord,
  FrameSystemLastRuntimeUpgradeInfo,
  FrameSystemPhase,
  IsmpConsensusStateCommitment,
  IsmpConsensusStateMachineHeight,
  IsmpConsensusStateMachineId,
  IsmpHostStateMachine,
  PalletBalancesAccountData,
  PalletBalancesBalanceLock,
  PalletBalancesReserveData,
  PalletBitcoinLocksLockReleaseRequest,
  PalletBitcoinLocksLockedBitcoin,
  PalletChainTransferQueuedTransferOut,
  PalletDomainsDomainRegistration,
  PalletGrandpaStoredPendingChange,
  PalletGrandpaStoredState,
  PalletHyperbridgeVersionedHostParams,
  PalletLiquidityPoolsLiquidityPool,
  PalletLiquidityPoolsLiquidityPoolCapital,
  PalletLiquidityPoolsPrebondedArgons,
  PalletMintMintAction,
  PalletMultisigMultisig,
  PalletPriceIndexPriceIndex,
  PalletProxyAnnouncement,
  PalletProxyProxyDefinition,
  PalletTransactionPaymentReleases,
  PalletVaultsVaultFrameRevenue,
  SpConsensusGrandpaAppPublic,
  SpRuntimeDigest,
  SpWeightsWeightV2Weight,
} from '@polkadot/types/lookup';
import type { Observable } from '@polkadot/types/types';

export type __AugmentedQuery<ApiType extends ApiTypes> = AugmentedQuery<ApiType, () => unknown>;
export type __QueryableStorageEntry<ApiType extends ApiTypes> = QueryableStorageEntry<ApiType>;

declare module '@polkadot/api-base/types/storage' {
  interface AugmentedQueries<ApiType extends ApiTypes> {
    authorship: {
      /**
       * Author of current block.
       **/
      author: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []>;
    };
    balances: {
      /**
       * The Balances pallet example of storing the balance of an account.
       *
       * # Example
       *
       * ```nocompile
       * impl pallet_balances::Config for Runtime {
       * type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
       * }
       * ```
       *
       * You can also store the balance of an account in the `System` pallet.
       *
       * # Example
       *
       * ```nocompile
       * impl pallet_balances::Config for Runtime {
       * type AccountStore = System
       * }
       * ```
       *
       * But this comes with tradeoffs, storing account balances in the system pallet stores
       * `frame_system` data alongside the account data contrary to storing account balances in the
       * `Balances` pallet, which uses a `StorageMap` to store balances data only.
       * NOTE: This is only used in the case that this pallet is used to store balances.
       **/
      account: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<PalletBalancesAccountData>,
        [AccountId32]
      >;
      /**
       * Freeze locks on account balances.
       **/
      freezes: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<Vec<FrameSupportTokensMiscIdAmountRuntimeFreezeReason>>,
        [AccountId32]
      >;
      /**
       * Holds on account balances.
       **/
      holds: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<Vec<FrameSupportTokensMiscIdAmountRuntimeHoldReason>>,
        [AccountId32]
      >;
      /**
       * The total units of outstanding deactivated balance in the system.
       **/
      inactiveIssuance: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      /**
       * Any liquidity locks on some account balances.
       * NOTE: Should only be accessed when setting, changing and freeing a lock.
       *
       * Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      locks: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<PalletBalancesBalanceLock>>,
        [AccountId32]
      >;
      /**
       * Named reserves on some account balances.
       *
       * Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      reserves: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<PalletBalancesReserveData>>,
        [AccountId32]
      >;
      /**
       * The total units issued in the system.
       **/
      totalIssuance: AugmentedQuery<ApiType, () => Observable<u128>, []>;
    };
    bitcoinLocks: {
      /**
       * Utxos that have been requested to be cosigned for releasing
       **/
      lockCosignDueByFrame: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<BTreeSet<u64>>,
        [u64]
      >;
      /**
       * Expiration of bitcoin locks by bitcoin height. Funds are burned since the user did not
       * unlock it
       **/
      lockExpirationsByBitcoinHeight: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<BTreeSet<u64>>,
        [u64]
      >;
      /**
       * Stores the block number where the lock was released
       **/
      lockReleaseCosignHeightById: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Option<u32>>,
        [u64]
      >;
      lockReleaseRequestsByUtxoId: AugmentedQuery<
        ApiType,
        (
          arg: u64 | AnyNumber | Uint8Array,
        ) => Observable<Option<PalletBitcoinLocksLockReleaseRequest>>,
        [u64]
      >;
      /**
       * Stores bitcoin utxos that have requested to be released
       **/
      locksByUtxoId: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Option<PalletBitcoinLocksLockedBitcoin>>,
        [u64]
      >;
      /**
       * The minimum number of satoshis that can be locked
       **/
      minimumSatoshis: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      nextUtxoId: AugmentedQuery<ApiType, () => Observable<Option<u64>>, []>;
    };
    bitcoinUtxos: {
      /**
       * The genesis set bitcoin network that this chain is tied to
       **/
      bitcoinNetwork: AugmentedQuery<
        ApiType,
        () => Observable<ArgonPrimitivesBitcoinBitcoinNetwork>,
        []
      >;
      /**
       * An oracle-provided confirmed bitcoin block (eg, 6 blocks back)
       **/
      confirmedBitcoinBlockTip: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesBitcoinBitcoinBlock>>,
        []
      >;
      /**
       * Check if the inherent was included
       **/
      inherentIncluded: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * Expiration date as a day since unix timestamp mapped to Bitcoin UTXOs
       **/
      lockedUtxoExpirationsByBlock: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Vec<ArgonPrimitivesBitcoinUtxoRef>>,
        [u64]
      >;
      /**
       * Locked Bitcoin UTXOs that have had ownership confirmed. If a Bitcoin UTXO is moved before
       * the expiration block, the funds are burned and the UTXO is unlocked.
       **/
      lockedUtxos: AugmentedQuery<
        ApiType,
        (
          arg:
            | ArgonPrimitivesBitcoinUtxoRef
            | { txid?: any; outputIndex?: any }
            | string
            | Uint8Array,
        ) => Observable<Option<ArgonPrimitivesBitcoinUtxoValue>>,
        [ArgonPrimitivesBitcoinUtxoRef]
      >;
      /**
       * Bitcoin Oracle Operator Account
       **/
      oracleOperatorAccount: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []>;
      previousBitcoinBlockTip: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesBitcoinBitcoinBlock>>,
        []
      >;
      /**
       * The last synched bitcoin block
       **/
      synchedBitcoinBlock: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesBitcoinBitcoinBlock>>,
        []
      >;
      /**
       * Stores if parent block had a confirmed bitcoin block
       **/
      tempParentHasSyncState: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      utxoIdToRef: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Option<ArgonPrimitivesBitcoinUtxoRef>>,
        [u64]
      >;
      /**
       * Bitcoin UTXOs that have been submitted for ownership confirmation
       **/
      utxosPendingConfirmation: AugmentedQuery<
        ApiType,
        () => Observable<BTreeMap<u64, ArgonPrimitivesBitcoinUtxoValue>>,
        []
      >;
    };
    blockRewards: {
      /**
       * The current scaled block rewards. It will adjust based on the argon movement away from price
       * target
       **/
      argonsPerBlock: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      /**
       * The cohort block rewards by mining cohort (ie, with the same starting frame id)
       **/
      blockRewardsByCohort: AugmentedQuery<ApiType, () => Observable<Vec<ITuple<[u64, u128]>>>, []>;
      /**
       * Bool if block rewards are paused
       **/
      blockRewardsPaused: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * Historical payouts by block number
       **/
      payoutsByBlock: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<ArgonPrimitivesBlockSealBlockPayout>>,
        [u32]
      >;
    };
    blockSeal: {
      /**
       * The calculated strength in the runtime so that it can be
       * upgraded, but is used by the node to determine which fork to follow
       **/
      blockForkPower: AugmentedQuery<ApiType, () => Observable<ArgonPrimitivesForkPower>, []>;
      /**
       * Is the block from a vote seal?
       **/
      isBlockFromVoteSeal: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      lastBlockSealerInfo: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesProvidersBlockSealerInfo>>,
        []
      >;
      lastTickWithVoteSeal: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      /**
       * The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed
       * Secret + VotesMerkleRoot of the parent block notebooks.
       **/
      parentVotingKey: AugmentedQuery<ApiType, () => Observable<Option<H256>>, []>;
      /**
       * Ensures only a single inherent is applied
       **/
      tempSealInherent: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesInherentsBlockSealInherent>>,
        []
      >;
      /**
       * The count of votes in the last 3 ticks
       **/
      votesInPast3Ticks: AugmentedQuery<ApiType, () => Observable<Vec<ITuple<[u64, u32]>>>, []>;
    };
    blockSealSpec: {
      /**
       * The current vote minimum of the chain. Block votes use this minimum to determine the
       * minimum amount of tax or compute needed to create a vote. It is adjusted up or down to
       * target a max number of votes
       **/
      currentComputeDifficulty: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      /**
       * The key K is selected to be the hash of a block in the blockchain - this block is called
       * the 'key block'. For optimal mining and verification performance, the key should
       * change every day
       **/
      currentComputeKeyBlock: AugmentedQuery<ApiType, () => Observable<Option<H256>>, []>;
      /**
       * The current vote minimum of the chain. Block votes use this minimum to determine the
       * minimum amount of tax or compute needed to create a vote. It is adjusted up or down to
       * target a max number of votes
       **/
      currentVoteMinimum: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      pastBlockVotes: AugmentedQuery<ApiType, () => Observable<Vec<ITuple<[u64, u32, u128]>>>, []>;
      pastComputeBlockTimes: AugmentedQuery<ApiType, () => Observable<Vec<u64>>, []>;
      /**
       * The timestamp from the previous block
       **/
      previousBlockTimestamp: AugmentedQuery<ApiType, () => Observable<Option<u64>>, []>;
      tempBlockTimestamp: AugmentedQuery<ApiType, () => Observable<Option<u64>>, []>;
      /**
       * Temporary store the vote digest
       **/
      tempBlockVoteDigest: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesDigestsBlockVoteDigest>>,
        []
      >;
      /**
       * Temporary store of any current tick notebooks included in this block (vs tick)
       **/
      tempCurrentTickNotebooksInBlock: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ArgonPrimitivesNotaryNotaryNotebookVoteDigestDetails>>,
        []
      >;
      /**
       * Keeps the last 3 vote minimums. The first one applies to the current block.
       **/
      voteMinimumHistory: AugmentedQuery<ApiType, () => Observable<Vec<u128>>, []>;
    };
    chainTransfer: {
      expiringTransfersOutByNotary: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u64 | AnyNumber | Uint8Array,
        ) => Observable<Vec<u32>>,
        [u32, u64]
      >;
      /**
       * The admin of the hyperbridge token gateway
       **/
      hyperbridgeTokenAdmin: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []>;
      nextTransferId: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []>;
      pendingTransfersOut: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array,
        ) => Observable<Option<PalletChainTransferQueuedTransferOut>>,
        [u32]
      >;
      transfersUsedInBlockNotebooks: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<ITuple<[AccountId32, u32]>>>,
        [u32]
      >;
    };
    digests: {
      tempDigests: AugmentedQuery<
        ApiType,
        () => Observable<Option<ArgonPrimitivesDigestsDigestset>>,
        []
      >;
    };
    domains: {
      expiringDomainsByBlock: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Vec<H256>>,
        [u64]
      >;
      registeredDomains: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Option<PalletDomainsDomainRegistration>>,
        [H256]
      >;
      zoneRecordsByDomain: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Option<ArgonPrimitivesDomainZoneRecord>>,
        [H256]
      >;
    };
    grandpa: {
      /**
       * The current list of authorities.
       **/
      authorities: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>>,
        []
      >;
      /**
       * The number of changes (both in terms of keys and underlying economic responsibilities)
       * in the "set" of Grandpa validators from genesis.
       **/
      currentSetId: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      /**
       * next block number where we can force a change.
       **/
      nextForced: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []>;
      /**
       * Pending change: (signaled at, scheduled change).
       **/
      pendingChange: AugmentedQuery<
        ApiType,
        () => Observable<Option<PalletGrandpaStoredPendingChange>>,
        []
      >;
      /**
       * A mapping from grandpa set ID to the index of the *most recent* session for which its
       * members were responsible.
       *
       * This is only used for validating equivocation proofs. An equivocation proof must
       * contains a key-ownership proof for a given session, therefore we need a way to tie
       * together sessions and GRANDPA set ids, i.e. we need to validate that a validator
       * was the owner of a given key on a given session, and what the active set ID was
       * during that session.
       *
       * TWOX-NOTE: `SetId` is not under user control.
       **/
      setIdSession: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Option<u32>>,
        [u64]
      >;
      /**
       * `true` if we are currently stalled.
       **/
      stalled: AugmentedQuery<ApiType, () => Observable<Option<ITuple<[u32, u32]>>>, []>;
      /**
       * State of the current authority set.
       **/
      state: AugmentedQuery<ApiType, () => Observable<PalletGrandpaStoredState>, []>;
    };
    hyperbridge: {
      /**
       * The host parameters of the pallet-hyperbridge.
       **/
      hostParams: AugmentedQuery<
        ApiType,
        () => Observable<PalletHyperbridgeVersionedHostParams>,
        []
      >;
    };
    ismp: {
      /**
       * A mapping of state machine Ids to their challenge periods
       **/
      challengePeriod: AugmentedQuery<
        ApiType,
        (
          arg:
            | IsmpConsensusStateMachineId
            | { stateId?: any; consensusStateId?: any }
            | string
            | Uint8Array,
        ) => Observable<Option<u64>>,
        [IsmpConsensusStateMachineId]
      >;
      /**
       * The child trie root of messages
       **/
      childTrieRoot: AugmentedQuery<ApiType, () => Observable<H256>, []>;
      /**
       * Holds the timestamp at which a consensus client was recently updated.
       * Used in ensuring that the configured challenge period elapses.
       **/
      consensusClientUpdateTime: AugmentedQuery<
        ApiType,
        (arg: U8aFixed | string | Uint8Array) => Observable<Option<u64>>,
        [U8aFixed]
      >;
      /**
       * A mapping of consensus state identifier to it's associated consensus client identifier
       **/
      consensusStateClient: AugmentedQuery<
        ApiType,
        (arg: U8aFixed | string | Uint8Array) => Observable<Option<U8aFixed>>,
        [U8aFixed]
      >;
      /**
       * Holds a map of consensus state identifiers to their consensus state.
       **/
      consensusStates: AugmentedQuery<
        ApiType,
        (arg: U8aFixed | string | Uint8Array) => Observable<Option<Bytes>>,
        [U8aFixed]
      >;
      /**
       * Holds a map of consensus clients frozen due to byzantine
       * behaviour
       **/
      frozenConsensusClients: AugmentedQuery<
        ApiType,
        (arg: U8aFixed | string | Uint8Array) => Observable<bool>,
        [U8aFixed]
      >;
      /**
       * The latest verified height for a state machine
       **/
      latestStateMachineHeight: AugmentedQuery<
        ApiType,
        (
          arg:
            | IsmpConsensusStateMachineId
            | { stateId?: any; consensusStateId?: any }
            | string
            | Uint8Array,
        ) => Observable<Option<u64>>,
        [IsmpConsensusStateMachineId]
      >;
      /**
       * Latest nonce for messages sent from this chain
       **/
      nonce: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      /**
       * The previous verified height for a state machine
       **/
      previousStateMachineHeight: AugmentedQuery<
        ApiType,
        (
          arg:
            | IsmpConsensusStateMachineId
            | { stateId?: any; consensusStateId?: any }
            | string
            | Uint8Array,
        ) => Observable<Option<u64>>,
        [IsmpConsensusStateMachineId]
      >;
      /**
       * Tracks requests that have been responded to
       * The key is the request commitment
       **/
      responded: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<bool>,
        [H256]
      >;
      /**
       * Holds a map of state machine heights to their verified state commitments. These state
       * commitments end up here after they are successfully verified by a `ConsensusClient`
       **/
      stateCommitments: AugmentedQuery<
        ApiType,
        (
          arg: IsmpConsensusStateMachineHeight | { id?: any; height?: any } | string | Uint8Array,
        ) => Observable<Option<IsmpConsensusStateCommitment>>,
        [IsmpConsensusStateMachineHeight]
      >;
      /**
       * Holds the timestamp at which a state machine height was updated.
       * Used in ensuring that the configured challenge period elapses.
       **/
      stateMachineUpdateTime: AugmentedQuery<
        ApiType,
        (
          arg: IsmpConsensusStateMachineHeight | { id?: any; height?: any } | string | Uint8Array,
        ) => Observable<Option<u64>>,
        [IsmpConsensusStateMachineHeight]
      >;
      /**
       * A mapping of consensus state identifiers to their unbonding periods
       **/
      unbondingPeriod: AugmentedQuery<
        ApiType,
        (arg: U8aFixed | string | Uint8Array) => Observable<Option<u64>>,
        [U8aFixed]
      >;
    };
    ismpGrandpa: {
      /**
       * Registered state machines for the grandpa consensus client
       **/
      supportedStateMachines: AugmentedQuery<
        ApiType,
        (
          arg:
            | IsmpHostStateMachine
            | { Evm: any }
            | { Polkadot: any }
            | { Kusama: any }
            | { Substrate: any }
            | { Tendermint: any }
            | { Relay: any }
            | string
            | Uint8Array,
        ) => Observable<Option<u64>>,
        [IsmpHostStateMachine]
      >;
    };
    liquidityPools: {
      /**
       * The liquidity pool for the current frame. This correlates with the bids coming in for the
       * current frame. Sorted with the biggest share last. (current frame + 1)
       **/
      capitalActive: AugmentedQuery<
        ApiType,
        () => Observable<Vec<PalletLiquidityPoolsLiquidityPoolCapital>>,
        []
      >;
      /**
       * The liquidity pool still raising capital. (current frame + 2)
       **/
      capitalRaising: AugmentedQuery<
        ApiType,
        () => Observable<Vec<PalletLiquidityPoolsLiquidityPoolCapital>>,
        []
      >;
      /**
       * Any vaults that have been pre-registered for bonding argons. This is used by the vault
       * operator to allocate argons to be bonded once bitcoins are securitized in their vault.
       **/
      prebondedByVaultId: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array,
        ) => Observable<Option<PalletLiquidityPoolsPrebondedArgons>>,
        [u32]
      >;
      /**
       * The currently earning contributors for the current epoch's bond funds. Sorted by highest
       * bids first
       **/
      vaultPoolsByFrame: AugmentedQuery<
        ApiType,
        (
          arg: u64 | AnyNumber | Uint8Array,
        ) => Observable<BTreeMap<u32, PalletLiquidityPoolsLiquidityPool>>,
        [u64]
      >;
    };
    miningSlot: {
      /**
       * Lookup by account id to the corresponding index in MinersByCohort and MinerXorKeysByCohort
       **/
      accountIndexLookup: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Option<ITuple<[u64, u32]>>>,
        [AccountId32]
      >;
      activeMinersCount: AugmentedQuery<ApiType, () => Observable<u16>, []>;
      /**
       * Argonots that must be locked to take a Miner role
       **/
      argonotsPerMiningSeat: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      /**
       * The average price per seat for the last 10 frames (newest first)
       **/
      averagePricePerSeat: AugmentedQuery<ApiType, () => Observable<Vec<u128>>, []>;
      /**
       * The cohort set to go into effect in the next slot. The Vec has all
       * registrants with their bid amount
       **/
      bidsForNextSlotCohort: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ArgonPrimitivesBlockSealMiningRegistration>>,
        []
      >;
      /**
       * Did this block activate a new frame
       **/
      didStartNewCohort: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * The previous 10 frame start block numbers
       **/
      frameStartBlockNumbers: AugmentedQuery<ApiType, () => Observable<Vec<u32>>, []>;
      hasAddedGrandpaRotation: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * The number of bids per slot for the last 10 slots (newest first)
       **/
      historicalBidsPerSlot: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ArgonPrimitivesBlockSealMiningBidStats>>,
        []
      >;
      /**
       * Is the next slot still open for bids
       **/
      isNextSlotBiddingOpen: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * Miners that are active in the current block (post initialize) by their starting frame
       **/
      minersByCohort: AugmentedQuery<
        ApiType,
        (
          arg: u64 | AnyNumber | Uint8Array,
        ) => Observable<Vec<ArgonPrimitivesBlockSealMiningRegistration>>,
        [u64]
      >;
      /**
       * This is a lookup of each miner's XOR key to use. It's a blake2 256 hash of the miner account
       * id and the block hash at time of activation.
       **/
      minerXorKeysByCohort: AugmentedQuery<ApiType, () => Observable<BTreeMap<u64, Vec<U256>>>, []>;
      /**
       * The mining slot configuration set in genesis
       **/
      miningConfig: AugmentedQuery<
        ApiType,
        () => Observable<ArgonPrimitivesBlockSealMiningSlotConfig>,
        []
      >;
      /**
       * The number of allow miners to bid for the next mining cohort
       **/
      nextCohortSize: AugmentedQuery<ApiType, () => Observable<u32>, []>;
      /**
       * The next frameId. A frame in argon is the 24 hours between the start of two different mining
       * cohorts.
       **/
      nextFrameId: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      /**
       * The miners released in the last block (only kept for a single block)
       **/
      releasedMinersByAccountId: AugmentedQuery<
        ApiType,
        () => Observable<BTreeMap<AccountId32, ArgonPrimitivesBlockSealMiningRegistration>>,
        []
      >;
    };
    mint: {
      blockMintAction: AugmentedQuery<
        ApiType,
        () => Observable<ITuple<[u32, PalletMintMintAction]>>,
        []
      >;
      /**
       * The amount of argons minted per mining cohort (ie, grouped by starting frame id)
       **/
      miningMintPerCohort: AugmentedQuery<ApiType, () => Observable<BTreeMap<u64, u128>>, []>;
      /**
       * The total amount of Bitcoin argons minted. Cannot exceed `MintedMiningArgons`.
       **/
      mintedBitcoinArgons: AugmentedQuery<ApiType, () => Observable<U256>, []>;
      /**
       * The total amount of argons minted for mining
       **/
      mintedMiningArgons: AugmentedQuery<ApiType, () => Observable<U256>, []>;
      /**
       * Bitcoin UTXOs that have been submitted for minting. This list is FIFO for minting whenever
       * a) CPI >= 0 and
       * b) the aggregate minted Bitcoins <= the aggregate minted Argons from mining
       **/
      pendingMintUtxos: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ITuple<[u64, AccountId32, u128]>>>,
        []
      >;
    };
    multisig: {
      /**
       * The set of open multisig operations.
       **/
      multisigs: AugmentedQuery<
        ApiType,
        (
          arg1: AccountId32 | string | Uint8Array,
          arg2: U8aFixed | string | Uint8Array,
        ) => Observable<Option<PalletMultisigMultisig>>,
        [AccountId32, U8aFixed]
      >;
    };
    notaries: {
      activeNotaries: AugmentedQuery<
        ApiType,
        () => Observable<Vec<ArgonPrimitivesNotaryNotaryRecord>>,
        []
      >;
      expiringProposals: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<AccountId32>>,
        [u32]
      >;
      nextNotaryId: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []>;
      notaryKeyHistory: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<ITuple<[u64, U8aFixed]>>>,
        [u32]
      >;
      proposedNotaries: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<Option<ITuple<[ArgonPrimitivesNotaryNotaryMeta, u32]>>>,
        [AccountId32]
      >;
      /**
       * Metadata changes to be activated at the given tick
       **/
      queuedNotaryMetaChanges: AugmentedQuery<
        ApiType,
        (
          arg: u64 | AnyNumber | Uint8Array,
        ) => Observable<BTreeMap<u32, ArgonPrimitivesNotaryNotaryMeta>>,
        [u64]
      >;
    };
    notebook: {
      /**
       * Storage map of account origin (notary_id, notebook, account_uid) to the last
       * notebook containing this account in the changed accounts merkle root
       * (NotebookChangedAccountsRootByNotary)
       **/
      accountOriginLastChangedNotebookByNotary: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2:
            | ArgonPrimitivesBalanceChangeAccountOrigin
            | { notebookNumber?: any; accountUid?: any }
            | string
            | Uint8Array,
        ) => Observable<Option<u32>>,
        [u32, ArgonPrimitivesBalanceChangeAccountOrigin]
      >;
      /**
       * The notebooks included in this block
       **/
      blockNotebooks: AugmentedQuery<
        ApiType,
        () => Observable<ArgonPrimitivesDigestsNotebookDigest>,
        []
      >;
      /**
       * Check if the inherent was included
       **/
      inherentIncluded: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * List of last few notebook details by notary. The bool is whether the notebook is eligible
       * for votes (received at correct tick and audit passed)
       **/
      lastNotebookDetailsByNotary: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array,
        ) => Observable<Vec<ITuple<[ArgonPrimitivesNotaryNotaryNotebookKeyDetails, bool]>>>,
        [u32]
      >;
      /**
       * Notaries ready to start reprocessing at a given notebook number
       **/
      lockedNotaryReadyForReprocess: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<u32>>,
        [u32]
      >;
      /**
       * Notaries locked for failing audits
       **/
      notariesLockedForFailedAudit: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array,
        ) => Observable<Option<ITuple<[u32, u64, ArgonNotaryAuditErrorVerifyError]>>>,
        [u32]
      >;
      /**
       * Double storage map of notary id + notebook # to the change root
       **/
      notebookChangedAccountsRootByNotary: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2: u32 | AnyNumber | Uint8Array,
        ) => Observable<Option<H256>>,
        [u32, u32]
      >;
    };
    ownership: {
      /**
       * The Balances pallet example of storing the balance of an account.
       *
       * # Example
       *
       * ```nocompile
       * impl pallet_balances::Config for Runtime {
       * type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
       * }
       * ```
       *
       * You can also store the balance of an account in the `System` pallet.
       *
       * # Example
       *
       * ```nocompile
       * impl pallet_balances::Config for Runtime {
       * type AccountStore = System
       * }
       * ```
       *
       * But this comes with tradeoffs, storing account balances in the system pallet stores
       * `frame_system` data alongside the account data contrary to storing account balances in the
       * `Balances` pallet, which uses a `StorageMap` to store balances data only.
       * NOTE: This is only used in the case that this pallet is used to store balances.
       **/
      account: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<PalletBalancesAccountData>,
        [AccountId32]
      >;
      /**
       * Freeze locks on account balances.
       **/
      freezes: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<Vec<FrameSupportTokensMiscIdAmountRuntimeFreezeReason>>,
        [AccountId32]
      >;
      /**
       * Holds on account balances.
       **/
      holds: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<Vec<FrameSupportTokensMiscIdAmountRuntimeHoldReason>>,
        [AccountId32]
      >;
      /**
       * The total units of outstanding deactivated balance in the system.
       **/
      inactiveIssuance: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      /**
       * Any liquidity locks on some account balances.
       * NOTE: Should only be accessed when setting, changing and freeing a lock.
       *
       * Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      locks: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<PalletBalancesBalanceLock>>,
        [AccountId32]
      >;
      /**
       * Named reserves on some account balances.
       *
       * Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      reserves: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<Vec<PalletBalancesReserveData>>,
        [AccountId32]
      >;
      /**
       * The total units issued in the system.
       **/
      totalIssuance: AugmentedQuery<ApiType, () => Observable<u128>, []>;
    };
    priceIndex: {
      /**
       * Stores the active price index
       **/
      current: AugmentedQuery<ApiType, () => Observable<Option<PalletPriceIndexPriceIndex>>, []>;
      /**
       * The price index operator account
       **/
      operator: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []>;
    };
    proxy: {
      /**
       * The announcements made by the proxy (key).
       **/
      announcements: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<ITuple<[Vec<PalletProxyAnnouncement>, u128]>>,
        [AccountId32]
      >;
      /**
       * The set of account proxies. Maps the account which has delegated to the accounts
       * which are being delegated to, together with the amount held on deposit.
       **/
      proxies: AugmentedQuery<
        ApiType,
        (
          arg: AccountId32 | string | Uint8Array,
        ) => Observable<ITuple<[Vec<PalletProxyProxyDefinition>, u128]>>,
        [AccountId32]
      >;
    };
    sudo: {
      /**
       * The `AccountId` of the sudo key.
       **/
      key: AugmentedQuery<ApiType, () => Observable<Option<AccountId32>>, []>;
    };
    system: {
      /**
       * The full account information for a particular account ID.
       **/
      account: AugmentedQuery<
        ApiType,
        (arg: AccountId32 | string | Uint8Array) => Observable<FrameSystemAccountInfo>,
        [AccountId32]
      >;
      /**
       * Total length (in bytes) for all extrinsics put together, for the current block.
       **/
      allExtrinsicsLen: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []>;
      /**
       * `Some` if a code upgrade has been authorized.
       **/
      authorizedUpgrade: AugmentedQuery<
        ApiType,
        () => Observable<Option<FrameSystemCodeUpgradeAuthorization>>,
        []
      >;
      /**
       * Map of block numbers to block hashes.
       **/
      blockHash: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<H256>,
        [u32]
      >;
      /**
       * The current weight for the block.
       **/
      blockWeight: AugmentedQuery<
        ApiType,
        () => Observable<FrameSupportDispatchPerDispatchClassWeight>,
        []
      >;
      /**
       * Digest of the current block, also part of the block header.
       **/
      digest: AugmentedQuery<ApiType, () => Observable<SpRuntimeDigest>, []>;
      /**
       * The number of events in the `Events<T>` list.
       **/
      eventCount: AugmentedQuery<ApiType, () => Observable<u32>, []>;
      /**
       * Events deposited for the current block.
       *
       * NOTE: The item is unbound and should therefore never be read on chain.
       * It could otherwise inflate the PoV size of a block.
       *
       * Events have a large in-memory size. Box the events to not go out-of-memory
       * just in case someone still reads them from within the runtime.
       **/
      events: AugmentedQuery<ApiType, () => Observable<Vec<FrameSystemEventRecord>>, []>;
      /**
       * Mapping between a topic (represented by T::Hash) and a vector of indexes
       * of events in the `<Events<T>>` list.
       *
       * All topic vectors have deterministic storage locations depending on the topic. This
       * allows light-clients to leverage the changes trie storage tracking mechanism and
       * in case of changes fetch the list of events of interest.
       *
       * The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just
       * the `EventIndex` then in case if the topic has the same contents on the next block
       * no notification will be triggered thus the event might be lost.
       **/
      eventTopics: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Vec<ITuple<[u32, u32]>>>,
        [H256]
      >;
      /**
       * The execution phase of the block.
       **/
      executionPhase: AugmentedQuery<ApiType, () => Observable<Option<FrameSystemPhase>>, []>;
      /**
       * Total extrinsics count for the current block.
       **/
      extrinsicCount: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []>;
      /**
       * Extrinsics data for the current block (maps an extrinsic's index to its data).
       **/
      extrinsicData: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Bytes>,
        [u32]
      >;
      /**
       * The weight reclaimed for the extrinsic.
       *
       * This information is available until the end of the extrinsic execution.
       * More precisely this information is removed in `note_applied_extrinsic`.
       *
       * Logic doing some post dispatch weight reduction must update this storage to avoid duplicate
       * reduction.
       **/
      extrinsicWeightReclaimed: AugmentedQuery<
        ApiType,
        () => Observable<SpWeightsWeightV2Weight>,
        []
      >;
      /**
       * Whether all inherents have been applied.
       **/
      inherentsApplied: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened.
       **/
      lastRuntimeUpgrade: AugmentedQuery<
        ApiType,
        () => Observable<Option<FrameSystemLastRuntimeUpgradeInfo>>,
        []
      >;
      /**
       * The current block number being processed. Set by `execute_block`.
       **/
      number: AugmentedQuery<ApiType, () => Observable<u32>, []>;
      /**
       * Hash of the previous block.
       **/
      parentHash: AugmentedQuery<ApiType, () => Observable<H256>, []>;
      /**
       * True if we have upgraded so that AccountInfo contains three types of `RefCount`. False
       * (default) if not.
       **/
      upgradedToTripleRefCount: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * True if we have upgraded so that `type RefCount` is `u32`. False (default) if not.
       **/
      upgradedToU32RefCount: AugmentedQuery<ApiType, () => Observable<bool>, []>;
    };
    ticks: {
      currentTick: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      genesisTick: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      genesisTicker: AugmentedQuery<ApiType, () => Observable<ArgonPrimitivesTickTicker>, []>;
      previousTick: AugmentedQuery<ApiType, () => Observable<u64>, []>;
      /**
       * Blocks from the last 100 ticks. Trimmed in on_initialize.
       * NOTE: cannot include the current block hash until next block
       **/
      recentBlocksAtTicks: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Vec<H256>>,
        [u64]
      >;
    };
    timestamp: {
      /**
       * Whether the timestamp has been updated in this block.
       *
       * This value is updated to `true` upon successful submission of a timestamp by a node.
       * It is then checked at the end of each block execution in the `on_finalize` hook.
       **/
      didUpdate: AugmentedQuery<ApiType, () => Observable<bool>, []>;
      /**
       * The current time for the current block.
       **/
      now: AugmentedQuery<ApiType, () => Observable<u64>, []>;
    };
    tokenGateway: {
      /**
       * Assets supported by this instance of token gateway
       * A map of the token gateway asset id to the local asset id
       **/
      localAssets: AugmentedQuery<
        ApiType,
        (arg: H256 | string | Uint8Array) => Observable<Option<u32>>,
        [H256]
      >;
      /**
       * Assets that originate from this chain
       **/
      nativeAssets: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<bool>,
        [u32]
      >;
      /**
       * The decimals used by the EVM counterpart of this asset
       **/
      precisions: AugmentedQuery<
        ApiType,
        (
          arg1: u32 | AnyNumber | Uint8Array,
          arg2:
            | IsmpHostStateMachine
            | { Evm: any }
            | { Polkadot: any }
            | { Kusama: any }
            | { Substrate: any }
            | { Tendermint: any }
            | { Relay: any }
            | string
            | Uint8Array,
        ) => Observable<Option<u8>>,
        [u32, IsmpHostStateMachine]
      >;
      /**
       * Assets supported by this instance of token gateway
       * A map of the local asset id to the token gateway asset id
       **/
      supportedAssets: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<H256>>,
        [u32]
      >;
      /**
       * The token gateway adresses on different chains
       **/
      tokenGatewayAddresses: AugmentedQuery<
        ApiType,
        (
          arg:
            | IsmpHostStateMachine
            | { Evm: any }
            | { Polkadot: any }
            | { Kusama: any }
            | { Substrate: any }
            | { Tendermint: any }
            | { Relay: any }
            | string
            | Uint8Array,
        ) => Observable<Option<Bytes>>,
        [IsmpHostStateMachine]
      >;
    };
    transactionPayment: {
      nextFeeMultiplier: AugmentedQuery<ApiType, () => Observable<u128>, []>;
      storageVersion: AugmentedQuery<
        ApiType,
        () => Observable<PalletTransactionPaymentReleases>,
        []
      >;
    };
    txPause: {
      /**
       * The set of calls that are explicitly paused.
       **/
      pausedCalls: AugmentedQuery<
        ApiType,
        (
          arg: ITuple<[Bytes, Bytes]> | [Bytes | string | Uint8Array, Bytes | string | Uint8Array],
        ) => Observable<Option<Null>>,
        [ITuple<[Bytes, Bytes]>]
      >;
    };
    vaults: {
      /**
       * The last collect frame of each vault
       **/
      lastCollectFrameByVaultId: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<u64>>,
        [u32]
      >;
      nextVaultId: AugmentedQuery<ApiType, () => Observable<Option<u32>>, []>;
      /**
       * Bitcoin Locks pending cosign by vault id
       **/
      pendingCosignByVaultId: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<BTreeSet<u64>>,
        [u32]
      >;
      /**
       * Pending terms that will be committed at the given block number (must be a minimum of 1 slot
       * change away)
       **/
      pendingTermsModificationsByTick: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<Vec<u32>>,
        [u64]
      >;
      /**
       * Tracks revenue from Bitcoin Locks and Liquidity Pools for the trailing frames for each vault
       * (a frame is a "mining day" in Argon). Newest frames are first. Frames are removed after the
       * collect expiration window (`RevenueCollectionExpirationFrames`).
       **/
      revenuePerFrameByVault: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Vec<PalletVaultsVaultFrameRevenue>>,
        [u32]
      >;
      /**
       * The vaults that have funds releasing at a given bitcoin height
       **/
      vaultFundsReleasingByHeight: AugmentedQuery<
        ApiType,
        (arg: u64 | AnyNumber | Uint8Array) => Observable<BTreeSet<u32>>,
        [u64]
      >;
      /**
       * Vaults by id
       **/
      vaultsById: AugmentedQuery<
        ApiType,
        (arg: u32 | AnyNumber | Uint8Array) => Observable<Option<ArgonPrimitivesVault>>,
        [u32]
      >;
      /**
       * Vault Bitcoin Xpub and current child counter by VaultId
       **/
      vaultXPubById: AugmentedQuery<
        ApiType,
        (
          arg: u32 | AnyNumber | Uint8Array,
        ) => Observable<Option<ITuple<[ArgonPrimitivesBitcoinBitcoinXPub, u32]>>>,
        [u32]
      >;
    };
  } // AugmentedQueries
} // declare module
