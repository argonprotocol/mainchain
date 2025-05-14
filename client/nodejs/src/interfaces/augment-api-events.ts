// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/events';

import type { ApiTypes, AugmentedEvent } from '@polkadot/api-base/types';
import type {
  Bytes,
  Null,
  Option,
  Result,
  U256,
  U8aFixed,
  Vec,
  bool,
  i128,
  u128,
  u16,
  u32,
  u64,
} from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, H256 } from '@polkadot/types/interfaces/runtime';
import type {
  ArgonNotaryAuditErrorVerifyError,
  ArgonPrimitivesBitcoinBitcoinRejectedReason,
  ArgonPrimitivesBitcoinUtxoRef,
  ArgonPrimitivesBlockSealBlockPayout,
  ArgonPrimitivesBlockSealMiningRegistration,
  ArgonPrimitivesDomainZoneRecord,
  ArgonPrimitivesNotaryNotaryMeta,
  ArgonPrimitivesNotaryNotaryRecord,
  ArgonPrimitivesVaultFundType,
  ArgonPrimitivesVaultObligationExpiration,
  ArgonRuntimeOriginCaller,
  ArgonRuntimeProxyType,
  FrameSupportTokensMiscBalanceStatus,
  FrameSystemDispatchEventInfo,
  IsmpConsensusStateMachineHeight,
  IsmpConsensusStateMachineId,
  IsmpEventsRequestResponseHandled,
  IsmpEventsTimeoutHandled,
  IsmpHostStateMachine,
  PalletDomainsDomainRegistration,
  PalletHyperbridgeVersionedHostParams,
  PalletIsmpErrorsHandlingError,
  PalletMintMintType,
  PalletMultisigTimepoint,
  SpConsensusGrandpaAppPublic,
  SpRuntimeDispatchError,
} from '@polkadot/types/lookup';

export type __AugmentedEvent<ApiType extends ApiTypes> =
  AugmentedEvent<ApiType>;

declare module '@polkadot/api-base/types/events' {
  interface AugmentedEvents<ApiType extends ApiTypes> {
    balances: {
      /**
       * A balance was set by root.
       **/
      BalanceSet: AugmentedEvent<
        ApiType,
        [who: AccountId32, free: u128],
        { who: AccountId32; free: u128 }
      >;
      /**
       * Some amount was burned from an account.
       **/
      Burned: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was deposited (e.g. for transaction fees).
       **/
      Deposit: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * An account was removed whose balance was non-zero but below ExistentialDeposit,
       * resulting in an outright loss.
       **/
      DustLost: AugmentedEvent<
        ApiType,
        [account: AccountId32, amount: u128],
        { account: AccountId32; amount: u128 }
      >;
      /**
       * An account was created with some free balance.
       **/
      Endowed: AugmentedEvent<
        ApiType,
        [account: AccountId32, freeBalance: u128],
        { account: AccountId32; freeBalance: u128 }
      >;
      /**
       * Some balance was frozen.
       **/
      Frozen: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Total issuance was increased by `amount`, creating a credit to be balanced.
       **/
      Issued: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was locked.
       **/
      Locked: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was minted into an account.
       **/
      Minted: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Total issuance was decreased by `amount`, creating a debt to be balanced.
       **/
      Rescinded: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was reserved (moved from free to reserved).
       **/
      Reserved: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was moved from the reserve of the first account to the second account.
       * Final argument indicates the destination balance type.
       **/
      ReserveRepatriated: AugmentedEvent<
        ApiType,
        [
          from: AccountId32,
          to: AccountId32,
          amount: u128,
          destinationStatus: FrameSupportTokensMiscBalanceStatus,
        ],
        {
          from: AccountId32;
          to: AccountId32;
          amount: u128;
          destinationStatus: FrameSupportTokensMiscBalanceStatus;
        }
      >;
      /**
       * Some amount was restored into an account.
       **/
      Restored: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was removed from the account (e.g. for misbehavior).
       **/
      Slashed: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was suspended from an account (it can be restored later).
       **/
      Suspended: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was thawed.
       **/
      Thawed: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * The `TotalIssuance` was forcefully changed.
       **/
      TotalIssuanceForced: AugmentedEvent<
        ApiType,
        [old: u128, new_: u128],
        { old: u128; new_: u128 }
      >;
      /**
       * Transfer succeeded.
       **/
      Transfer: AugmentedEvent<
        ApiType,
        [from: AccountId32, to: AccountId32, amount: u128],
        { from: AccountId32; to: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was unlocked.
       **/
      Unlocked: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was unreserved (moved from reserved to free).
       **/
      Unreserved: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * An account was upgraded.
       **/
      Upgraded: AugmentedEvent<
        ApiType,
        [who: AccountId32],
        { who: AccountId32 }
      >;
      /**
       * Some amount was withdrawn from the account (e.g. for transaction fees).
       **/
      Withdraw: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
    };
    bitcoinLocks: {
      BitcoinCosignPastDue: AugmentedEvent<
        ApiType,
        [
          utxoId: u64,
          obligationId: u64,
          vaultId: u32,
          compensationAmount: u128,
          compensationStillOwed: u128,
          compensatedAccountId: AccountId32,
        ],
        {
          utxoId: u64;
          obligationId: u64;
          vaultId: u32;
          compensationAmount: u128;
          compensationStillOwed: u128;
          compensatedAccountId: AccountId32;
        }
      >;
      BitcoinLockBurned: AugmentedEvent<
        ApiType,
        [
          utxoId: u64,
          vaultId: u32,
          obligationId: u64,
          amountBurned: u128,
          amountHeld: u128,
          wasUtxoSpent: bool,
        ],
        {
          utxoId: u64;
          vaultId: u32;
          obligationId: u64;
          amountBurned: u128;
          amountHeld: u128;
          wasUtxoSpent: bool;
        }
      >;
      BitcoinLockCreated: AugmentedEvent<
        ApiType,
        [
          utxoId: u64,
          vaultId: u32,
          obligationId: u64,
          lockPrice: u128,
          accountId: AccountId32,
        ],
        {
          utxoId: u64;
          vaultId: u32;
          obligationId: u64;
          lockPrice: u128;
          accountId: AccountId32;
        }
      >;
      BitcoinUtxoCosigned: AugmentedEvent<
        ApiType,
        [utxoId: u64, obligationId: u64, vaultId: u32, signature: Bytes],
        { utxoId: u64; obligationId: u64; vaultId: u32; signature: Bytes }
      >;
      BitcoinUtxoCosignRequested: AugmentedEvent<
        ApiType,
        [utxoId: u64, obligationId: u64, vaultId: u32],
        { utxoId: u64; obligationId: u64; vaultId: u32 }
      >;
      /**
       * An error occurred while refunding an overdue cosigned bitcoin lock
       **/
      CosignOverdueError: AugmentedEvent<
        ApiType,
        [utxoId: u64, error: SpRuntimeDispatchError],
        { utxoId: u64; error: SpRuntimeDispatchError }
      >;
    };
    bitcoinUtxos: {
      UtxoExpiredError: AugmentedEvent<
        ApiType,
        [utxoRef: ArgonPrimitivesBitcoinUtxoRef, error: SpRuntimeDispatchError],
        {
          utxoRef: ArgonPrimitivesBitcoinUtxoRef;
          error: SpRuntimeDispatchError;
        }
      >;
      UtxoRejected: AugmentedEvent<
        ApiType,
        [
          utxoId: u64,
          rejectedReason: ArgonPrimitivesBitcoinBitcoinRejectedReason,
        ],
        {
          utxoId: u64;
          rejectedReason: ArgonPrimitivesBitcoinBitcoinRejectedReason;
        }
      >;
      UtxoRejectedError: AugmentedEvent<
        ApiType,
        [utxoId: u64, error: SpRuntimeDispatchError],
        { utxoId: u64; error: SpRuntimeDispatchError }
      >;
      UtxoSpent: AugmentedEvent<
        ApiType,
        [utxoId: u64, blockHeight: u64],
        { utxoId: u64; blockHeight: u64 }
      >;
      UtxoSpentError: AugmentedEvent<
        ApiType,
        [utxoId: u64, error: SpRuntimeDispatchError],
        { utxoId: u64; error: SpRuntimeDispatchError }
      >;
      UtxoUnwatched: AugmentedEvent<ApiType, [utxoId: u64], { utxoId: u64 }>;
      UtxoVerified: AugmentedEvent<ApiType, [utxoId: u64], { utxoId: u64 }>;
      UtxoVerifiedError: AugmentedEvent<
        ApiType,
        [utxoId: u64, error: SpRuntimeDispatchError],
        { utxoId: u64; error: SpRuntimeDispatchError }
      >;
    };
    blockRewards: {
      RewardCreated: AugmentedEvent<
        ApiType,
        [rewards: Vec<ArgonPrimitivesBlockSealBlockPayout>],
        { rewards: Vec<ArgonPrimitivesBlockSealBlockPayout> }
      >;
      RewardCreateError: AugmentedEvent<
        ApiType,
        [
          accountId: AccountId32,
          argons: Option<u128>,
          ownership: Option<u128>,
          error: SpRuntimeDispatchError,
        ],
        {
          accountId: AccountId32;
          argons: Option<u128>;
          ownership: Option<u128>;
          error: SpRuntimeDispatchError;
        }
      >;
    };
    blockSealSpec: {
      ComputeDifficultyAdjusted: AugmentedEvent<
        ApiType,
        [
          expectedBlockTime: u64,
          actualBlockTime: u64,
          startDifficulty: u128,
          newDifficulty: u128,
        ],
        {
          expectedBlockTime: u64;
          actualBlockTime: u64;
          startDifficulty: u128;
          newDifficulty: u128;
        }
      >;
      VoteMinimumAdjusted: AugmentedEvent<
        ApiType,
        [
          expectedBlockVotes: u128,
          actualBlockVotes: u128,
          startVoteMinimum: u128,
          newVoteMinimum: u128,
        ],
        {
          expectedBlockVotes: u128;
          actualBlockVotes: u128;
          startVoteMinimum: u128;
          newVoteMinimum: u128;
        }
      >;
    };
    chainTransfer: {
      /**
       * A localchain transfer could not be cleaned up properly. Possible invalid transfer
       * needing investigation.
       **/
      PossibleInvalidLocalchainTransferAllowed: AugmentedEvent<
        ApiType,
        [transferId: u32, notaryId: u32, notebookNumber: u32],
        { transferId: u32; notaryId: u32; notebookNumber: u32 }
      >;
      /**
       * Taxation failed
       **/
      TaxationError: AugmentedEvent<
        ApiType,
        [
          notaryId: u32,
          notebookNumber: u32,
          tax: u128,
          error: SpRuntimeDispatchError,
        ],
        {
          notaryId: u32;
          notebookNumber: u32;
          tax: u128;
          error: SpRuntimeDispatchError;
        }
      >;
      /**
       * Transfer from Localchain to Mainchain
       **/
      TransferFromLocalchain: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, amount: u128, notaryId: u32],
        { accountId: AccountId32; amount: u128; notaryId: u32 }
      >;
      /**
       * A transfer into the mainchain failed
       **/
      TransferFromLocalchainError: AugmentedEvent<
        ApiType,
        [
          accountId: AccountId32,
          amount: u128,
          notaryId: u32,
          notebookNumber: u32,
          error: SpRuntimeDispatchError,
        ],
        {
          accountId: AccountId32;
          amount: u128;
          notaryId: u32;
          notebookNumber: u32;
          error: SpRuntimeDispatchError;
        }
      >;
      /**
       * Funds sent to a localchain
       **/
      TransferToLocalchain: AugmentedEvent<
        ApiType,
        [
          accountId: AccountId32,
          amount: u128,
          transferId: u32,
          notaryId: u32,
          expirationTick: u64,
        ],
        {
          accountId: AccountId32;
          amount: u128;
          transferId: u32;
          notaryId: u32;
          expirationTick: u64;
        }
      >;
      /**
       * Transfer to localchain expired and rolled back
       **/
      TransferToLocalchainExpired: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, transferId: u32, notaryId: u32],
        { accountId: AccountId32; transferId: u32; notaryId: u32 }
      >;
      /**
       * An expired transfer to localchain failed to be refunded
       **/
      TransferToLocalchainRefundError: AugmentedEvent<
        ApiType,
        [
          accountId: AccountId32,
          transferId: u32,
          notaryId: u32,
          notebookNumber: u32,
          error: SpRuntimeDispatchError,
        ],
        {
          accountId: AccountId32;
          transferId: u32;
          notaryId: u32;
          notebookNumber: u32;
          error: SpRuntimeDispatchError;
        }
      >;
    };
    digests: {};
    domains: {
      /**
       * A domain was expired
       **/
      DomainExpired: AugmentedEvent<
        ApiType,
        [domainHash: H256],
        { domainHash: H256 }
      >;
      /**
       * A domain was registered
       **/
      DomainRegistered: AugmentedEvent<
        ApiType,
        [domainHash: H256, registration: PalletDomainsDomainRegistration],
        { domainHash: H256; registration: PalletDomainsDomainRegistration }
      >;
      /**
       * A domain registration was canceled due to a conflicting registration in the same
       * tick
       **/
      DomainRegistrationCanceled: AugmentedEvent<
        ApiType,
        [domainHash: H256, registration: PalletDomainsDomainRegistration],
        { domainHash: H256; registration: PalletDomainsDomainRegistration }
      >;
      /**
       * A domain registration failed due to an error
       **/
      DomainRegistrationError: AugmentedEvent<
        ApiType,
        [
          domainHash: H256,
          accountId: AccountId32,
          error: SpRuntimeDispatchError,
        ],
        {
          domainHash: H256;
          accountId: AccountId32;
          error: SpRuntimeDispatchError;
        }
      >;
      /**
       * A domain was registered
       **/
      DomainRenewed: AugmentedEvent<
        ApiType,
        [domainHash: H256],
        { domainHash: H256 }
      >;
      /**
       * A domain zone record was updated
       **/
      ZoneRecordUpdated: AugmentedEvent<
        ApiType,
        [domainHash: H256, zoneRecord: ArgonPrimitivesDomainZoneRecord],
        { domainHash: H256; zoneRecord: ArgonPrimitivesDomainZoneRecord }
      >;
    };
    feelessTransaction: {
      /**
       * A transaction fee was skipped.
       **/
      FeeSkipped: AugmentedEvent<
        ApiType,
        [origin: ArgonRuntimeOriginCaller],
        { origin: ArgonRuntimeOriginCaller }
      >;
    };
    grandpa: {
      /**
       * New authority set has been applied.
       **/
      NewAuthorities: AugmentedEvent<
        ApiType,
        [authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>],
        { authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>> }
      >;
      /**
       * Current authority set has been paused.
       **/
      Paused: AugmentedEvent<ApiType, []>;
      /**
       * Current authority set has been resumed.
       **/
      Resumed: AugmentedEvent<ApiType, []>;
    };
    hyperbridge: {
      /**
       * Hyperbridge governance has now updated it's host params on this chain.
       **/
      HostParamsUpdated: AugmentedEvent<
        ApiType,
        [
          old: PalletHyperbridgeVersionedHostParams,
          new_: PalletHyperbridgeVersionedHostParams,
        ],
        {
          old: PalletHyperbridgeVersionedHostParams;
          new_: PalletHyperbridgeVersionedHostParams;
        }
      >;
      /**
       * Hyperbridge has withdrawn it's protocol revenue
       **/
      ProtocolRevenueWithdrawn: AugmentedEvent<
        ApiType,
        [amount: u128, account: AccountId32],
        { amount: u128; account: AccountId32 }
      >;
      /**
       * A relayer has withdrawn some fees
       **/
      RelayerFeeWithdrawn: AugmentedEvent<
        ApiType,
        [amount: u128, account: AccountId32],
        { amount: u128; account: AccountId32 }
      >;
    };
    ismp: {
      /**
       * Indicates that a consensus client has been created
       **/
      ConsensusClientCreated: AugmentedEvent<
        ApiType,
        [consensusClientId: U8aFixed],
        { consensusClientId: U8aFixed }
      >;
      /**
       * Indicates that a consensus client has been created
       **/
      ConsensusClientFrozen: AugmentedEvent<
        ApiType,
        [consensusClientId: U8aFixed],
        { consensusClientId: U8aFixed }
      >;
      /**
       * Some errors handling some ismp messages
       **/
      Errors: AugmentedEvent<
        ApiType,
        [errors: Vec<PalletIsmpErrorsHandlingError>],
        { errors: Vec<PalletIsmpErrorsHandlingError> }
      >;
      /**
       * Get Response Handled
       **/
      GetRequestHandled: AugmentedEvent<
        ApiType,
        [IsmpEventsRequestResponseHandled]
      >;
      /**
       * Get request timeout handled
       **/
      GetRequestTimeoutHandled: AugmentedEvent<
        ApiType,
        [IsmpEventsTimeoutHandled]
      >;
      /**
       * Post Request Handled
       **/
      PostRequestHandled: AugmentedEvent<
        ApiType,
        [IsmpEventsRequestResponseHandled]
      >;
      /**
       * Post request timeout handled
       **/
      PostRequestTimeoutHandled: AugmentedEvent<
        ApiType,
        [IsmpEventsTimeoutHandled]
      >;
      /**
       * Post Response Handled
       **/
      PostResponseHandled: AugmentedEvent<
        ApiType,
        [IsmpEventsRequestResponseHandled]
      >;
      /**
       * Post response timeout handled
       **/
      PostResponseTimeoutHandled: AugmentedEvent<
        ApiType,
        [IsmpEventsTimeoutHandled]
      >;
      /**
       * An Outgoing Request has been deposited
       **/
      Request: AugmentedEvent<
        ApiType,
        [
          destChain: IsmpHostStateMachine,
          sourceChain: IsmpHostStateMachine,
          requestNonce: u64,
          commitment: H256,
        ],
        {
          destChain: IsmpHostStateMachine;
          sourceChain: IsmpHostStateMachine;
          requestNonce: u64;
          commitment: H256;
        }
      >;
      /**
       * An Outgoing Response has been deposited
       **/
      Response: AugmentedEvent<
        ApiType,
        [
          destChain: IsmpHostStateMachine,
          sourceChain: IsmpHostStateMachine,
          requestNonce: u64,
          commitment: H256,
          reqCommitment: H256,
        ],
        {
          destChain: IsmpHostStateMachine;
          sourceChain: IsmpHostStateMachine;
          requestNonce: u64;
          commitment: H256;
          reqCommitment: H256;
        }
      >;
      /**
       * Emitted when a state commitment is vetoed by a fisherman
       **/
      StateCommitmentVetoed: AugmentedEvent<
        ApiType,
        [height: IsmpConsensusStateMachineHeight, fisherman: Bytes],
        { height: IsmpConsensusStateMachineHeight; fisherman: Bytes }
      >;
      /**
       * Emitted when a state machine is successfully updated to a new height
       **/
      StateMachineUpdated: AugmentedEvent<
        ApiType,
        [stateMachineId: IsmpConsensusStateMachineId, latestHeight: u64],
        { stateMachineId: IsmpConsensusStateMachineId; latestHeight: u64 }
      >;
    };
    ismpGrandpa: {
      /**
       * State machines have been added to whitelist
       **/
      StateMachineAdded: AugmentedEvent<
        ApiType,
        [stateMachines: Vec<IsmpHostStateMachine>],
        { stateMachines: Vec<IsmpHostStateMachine> }
      >;
      /**
       * State machines have been removed from the whitelist
       **/
      StateMachineRemoved: AugmentedEvent<
        ApiType,
        [stateMachines: Vec<IsmpHostStateMachine>],
        { stateMachines: Vec<IsmpHostStateMachine> }
      >;
    };
    liquidityPools: {
      /**
       * Funds from the active bid pool have been distributed
       **/
      BidPoolDistributed: AugmentedEvent<
        ApiType,
        [
          frameId: u64,
          bidPoolDistributed: u128,
          bidPoolBurned: u128,
          bidPoolShares: u32,
        ],
        {
          frameId: u64;
          bidPoolDistributed: u128;
          bidPoolBurned: u128;
          bidPoolShares: u32;
        }
      >;
      /**
       * An error occurred burning from the bid pool
       **/
      CouldNotBurnBidPool: AugmentedEvent<
        ApiType,
        [frameId: u64, amount: u128, dispatchError: SpRuntimeDispatchError],
        { frameId: u64; amount: u128; dispatchError: SpRuntimeDispatchError }
      >;
      /**
       * An error occurred distributing a bid pool
       **/
      CouldNotDistributeBidPool: AugmentedEvent<
        ApiType,
        [
          accountId: AccountId32,
          frameId: u64,
          vaultId: u32,
          amount: u128,
          dispatchError: SpRuntimeDispatchError,
          isForVault: bool,
        ],
        {
          accountId: AccountId32;
          frameId: u64;
          vaultId: u32;
          amount: u128;
          dispatchError: SpRuntimeDispatchError;
          isForVault: bool;
        }
      >;
      /**
       * An error occurred releasing a contributor hold
       **/
      ErrorRefundingLiquidityPoolCapital: AugmentedEvent<
        ApiType,
        [
          frameId: u64,
          vaultId: u32,
          amount: u128,
          accountId: AccountId32,
          dispatchError: SpRuntimeDispatchError,
        ],
        {
          frameId: u64;
          vaultId: u32;
          amount: u128;
          accountId: AccountId32;
          dispatchError: SpRuntimeDispatchError;
        }
      >;
      /**
       * The next bid pool has been locked in
       **/
      NextBidPoolCapitalLocked: AugmentedEvent<
        ApiType,
        [frameId: u64, totalActivatedCapital: u128, participatingVaults: u32],
        { frameId: u64; totalActivatedCapital: u128; participatingVaults: u32 }
      >;
      /**
       * Some mining bond capital was refunded due to less activated vault funds than bond
       * capital
       **/
      RefundedLiquidityPoolCapital: AugmentedEvent<
        ApiType,
        [frameId: u64, vaultId: u32, amount: u128, accountId: AccountId32],
        { frameId: u64; vaultId: u32; amount: u128; accountId: AccountId32 }
      >;
    };
    miningSlot: {
      /**
       * Bids are closed due to the VRF randomized function triggering
       **/
      MiningBidsClosed: AugmentedEvent<
        ApiType,
        [cohortFrameId: u64],
        { cohortFrameId: u64 }
      >;
      MiningConfigurationUpdated: AugmentedEvent<
        ApiType,
        [
          ticksBeforeBidEndForVrfClose: u64,
          ticksBetweenSlots: u64,
          slotBiddingStartAfterTicks: u64,
        ],
        {
          ticksBeforeBidEndForVrfClose: u64;
          ticksBetweenSlots: u64;
          slotBiddingStartAfterTicks: u64;
        }
      >;
      NewMiners: AugmentedEvent<
        ApiType,
        [
          startIndex: u32,
          newMiners: Vec<ArgonPrimitivesBlockSealMiningRegistration>,
          releasedMiners: u32,
          cohortFrameId: u64,
        ],
        {
          startIndex: u32;
          newMiners: Vec<ArgonPrimitivesBlockSealMiningRegistration>;
          releasedMiners: u32;
          cohortFrameId: u64;
        }
      >;
      ReleaseBidError: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, error: SpRuntimeDispatchError],
        { accountId: AccountId32; error: SpRuntimeDispatchError }
      >;
      ReleaseMinerSeatError: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, error: SpRuntimeDispatchError],
        { accountId: AccountId32; error: SpRuntimeDispatchError }
      >;
      SlotBidderAdded: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, bidAmount: u128, index: u32],
        { accountId: AccountId32; bidAmount: u128; index: u32 }
      >;
      SlotBidderDropped: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, preservedArgonotHold: bool],
        { accountId: AccountId32; preservedArgonotHold: bool }
      >;
    };
    mint: {
      /**
       * Any bitcoins minted
       **/
      BitcoinMint: AugmentedEvent<
        ApiType,
        [accountId: AccountId32, utxoId: Option<u64>, amount: u128],
        { accountId: AccountId32; utxoId: Option<u64>; amount: u128 }
      >;
      /**
       * The amount of argons minted for mining. NOTE: accounts below Existential Deposit will
       * not be able to mint
       **/
      MiningMint: AugmentedEvent<
        ApiType,
        [amount: U256, perMiner: u128, argonCpi: i128, liquidity: u128],
        { amount: U256; perMiner: u128; argonCpi: i128; liquidity: u128 }
      >;
      /**
       * Errors encountered while minting. Most often due to mint amount still below Existential
       * Deposit
       **/
      MintError: AugmentedEvent<
        ApiType,
        [
          mintType: PalletMintMintType,
          accountId: AccountId32,
          utxoId: Option<u64>,
          amount: u128,
          error: SpRuntimeDispatchError,
        ],
        {
          mintType: PalletMintMintType;
          accountId: AccountId32;
          utxoId: Option<u64>;
          amount: u128;
          error: SpRuntimeDispatchError;
        }
      >;
    };
    multisig: {
      /**
       * A multisig operation has been approved by someone.
       **/
      MultisigApproval: AugmentedEvent<
        ApiType,
        [
          approving: AccountId32,
          timepoint: PalletMultisigTimepoint,
          multisig: AccountId32,
          callHash: U8aFixed,
        ],
        {
          approving: AccountId32;
          timepoint: PalletMultisigTimepoint;
          multisig: AccountId32;
          callHash: U8aFixed;
        }
      >;
      /**
       * A multisig operation has been cancelled.
       **/
      MultisigCancelled: AugmentedEvent<
        ApiType,
        [
          cancelling: AccountId32,
          timepoint: PalletMultisigTimepoint,
          multisig: AccountId32,
          callHash: U8aFixed,
        ],
        {
          cancelling: AccountId32;
          timepoint: PalletMultisigTimepoint;
          multisig: AccountId32;
          callHash: U8aFixed;
        }
      >;
      /**
       * A multisig operation has been executed.
       **/
      MultisigExecuted: AugmentedEvent<
        ApiType,
        [
          approving: AccountId32,
          timepoint: PalletMultisigTimepoint,
          multisig: AccountId32,
          callHash: U8aFixed,
          result: Result<Null, SpRuntimeDispatchError>,
        ],
        {
          approving: AccountId32;
          timepoint: PalletMultisigTimepoint;
          multisig: AccountId32;
          callHash: U8aFixed;
          result: Result<Null, SpRuntimeDispatchError>;
        }
      >;
      /**
       * A new multisig operation has begun.
       **/
      NewMultisig: AugmentedEvent<
        ApiType,
        [approving: AccountId32, multisig: AccountId32, callHash: U8aFixed],
        { approving: AccountId32; multisig: AccountId32; callHash: U8aFixed }
      >;
    };
    notaries: {
      /**
       * A notary proposal has been accepted
       **/
      NotaryActivated: AugmentedEvent<
        ApiType,
        [notary: ArgonPrimitivesNotaryNotaryRecord],
        { notary: ArgonPrimitivesNotaryNotaryRecord }
      >;
      /**
       * Notary metadata updated
       **/
      NotaryMetaUpdated: AugmentedEvent<
        ApiType,
        [notaryId: u32, meta: ArgonPrimitivesNotaryNotaryMeta],
        { notaryId: u32; meta: ArgonPrimitivesNotaryNotaryMeta }
      >;
      /**
       * Error updating queued notary info
       **/
      NotaryMetaUpdateError: AugmentedEvent<
        ApiType,
        [
          notaryId: u32,
          error: SpRuntimeDispatchError,
          meta: ArgonPrimitivesNotaryNotaryMeta,
        ],
        {
          notaryId: u32;
          error: SpRuntimeDispatchError;
          meta: ArgonPrimitivesNotaryNotaryMeta;
        }
      >;
      /**
       * Notary metadata queued for update
       **/
      NotaryMetaUpdateQueued: AugmentedEvent<
        ApiType,
        [
          notaryId: u32,
          meta: ArgonPrimitivesNotaryNotaryMeta,
          effectiveTick: u64,
        ],
        {
          notaryId: u32;
          meta: ArgonPrimitivesNotaryNotaryMeta;
          effectiveTick: u64;
        }
      >;
      /**
       * A user has proposed operating as a notary
       **/
      NotaryProposed: AugmentedEvent<
        ApiType,
        [
          operatorAccount: AccountId32,
          meta: ArgonPrimitivesNotaryNotaryMeta,
          expires: u32,
        ],
        {
          operatorAccount: AccountId32;
          meta: ArgonPrimitivesNotaryNotaryMeta;
          expires: u32;
        }
      >;
    };
    notebook: {
      NotebookAuditFailure: AugmentedEvent<
        ApiType,
        [
          notaryId: u32,
          notebookNumber: u32,
          notebookHash: H256,
          firstFailureReason: ArgonNotaryAuditErrorVerifyError,
        ],
        {
          notaryId: u32;
          notebookNumber: u32;
          notebookHash: H256;
          firstFailureReason: ArgonNotaryAuditErrorVerifyError;
        }
      >;
      NotebookReadyForReprocess: AugmentedEvent<
        ApiType,
        [notaryId: u32, notebookNumber: u32],
        { notaryId: u32; notebookNumber: u32 }
      >;
      NotebookSubmitted: AugmentedEvent<
        ApiType,
        [notaryId: u32, notebookNumber: u32],
        { notaryId: u32; notebookNumber: u32 }
      >;
    };
    ownership: {
      /**
       * A balance was set by root.
       **/
      BalanceSet: AugmentedEvent<
        ApiType,
        [who: AccountId32, free: u128],
        { who: AccountId32; free: u128 }
      >;
      /**
       * Some amount was burned from an account.
       **/
      Burned: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was deposited (e.g. for transaction fees).
       **/
      Deposit: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * An account was removed whose balance was non-zero but below ExistentialDeposit,
       * resulting in an outright loss.
       **/
      DustLost: AugmentedEvent<
        ApiType,
        [account: AccountId32, amount: u128],
        { account: AccountId32; amount: u128 }
      >;
      /**
       * An account was created with some free balance.
       **/
      Endowed: AugmentedEvent<
        ApiType,
        [account: AccountId32, freeBalance: u128],
        { account: AccountId32; freeBalance: u128 }
      >;
      /**
       * Some balance was frozen.
       **/
      Frozen: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Total issuance was increased by `amount`, creating a credit to be balanced.
       **/
      Issued: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was locked.
       **/
      Locked: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was minted into an account.
       **/
      Minted: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Total issuance was decreased by `amount`, creating a debt to be balanced.
       **/
      Rescinded: AugmentedEvent<ApiType, [amount: u128], { amount: u128 }>;
      /**
       * Some balance was reserved (moved from free to reserved).
       **/
      Reserved: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was moved from the reserve of the first account to the second account.
       * Final argument indicates the destination balance type.
       **/
      ReserveRepatriated: AugmentedEvent<
        ApiType,
        [
          from: AccountId32,
          to: AccountId32,
          amount: u128,
          destinationStatus: FrameSupportTokensMiscBalanceStatus,
        ],
        {
          from: AccountId32;
          to: AccountId32;
          amount: u128;
          destinationStatus: FrameSupportTokensMiscBalanceStatus;
        }
      >;
      /**
       * Some amount was restored into an account.
       **/
      Restored: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was removed from the account (e.g. for misbehavior).
       **/
      Slashed: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some amount was suspended from an account (it can be restored later).
       **/
      Suspended: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was thawed.
       **/
      Thawed: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * The `TotalIssuance` was forcefully changed.
       **/
      TotalIssuanceForced: AugmentedEvent<
        ApiType,
        [old: u128, new_: u128],
        { old: u128; new_: u128 }
      >;
      /**
       * Transfer succeeded.
       **/
      Transfer: AugmentedEvent<
        ApiType,
        [from: AccountId32, to: AccountId32, amount: u128],
        { from: AccountId32; to: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was unlocked.
       **/
      Unlocked: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * Some balance was unreserved (moved from reserved to free).
       **/
      Unreserved: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
      /**
       * An account was upgraded.
       **/
      Upgraded: AugmentedEvent<
        ApiType,
        [who: AccountId32],
        { who: AccountId32 }
      >;
      /**
       * Some amount was withdrawn from the account (e.g. for transaction fees).
       **/
      Withdraw: AugmentedEvent<
        ApiType,
        [who: AccountId32, amount: u128],
        { who: AccountId32; amount: u128 }
      >;
    };
    priceIndex: {
      /**
       * Event emitted when a new price index is submitted
       **/
      NewIndex: AugmentedEvent<ApiType, []>;
      OperatorChanged: AugmentedEvent<
        ApiType,
        [operatorId: AccountId32],
        { operatorId: AccountId32 }
      >;
    };
    proxy: {
      /**
       * An announcement was placed to make a call in the future.
       **/
      Announced: AugmentedEvent<
        ApiType,
        [real: AccountId32, proxy: AccountId32, callHash: H256],
        { real: AccountId32; proxy: AccountId32; callHash: H256 }
      >;
      /**
       * A proxy was added.
       **/
      ProxyAdded: AugmentedEvent<
        ApiType,
        [
          delegator: AccountId32,
          delegatee: AccountId32,
          proxyType: ArgonRuntimeProxyType,
          delay: u32,
        ],
        {
          delegator: AccountId32;
          delegatee: AccountId32;
          proxyType: ArgonRuntimeProxyType;
          delay: u32;
        }
      >;
      /**
       * A proxy was executed correctly, with the given.
       **/
      ProxyExecuted: AugmentedEvent<
        ApiType,
        [result: Result<Null, SpRuntimeDispatchError>],
        { result: Result<Null, SpRuntimeDispatchError> }
      >;
      /**
       * A proxy was removed.
       **/
      ProxyRemoved: AugmentedEvent<
        ApiType,
        [
          delegator: AccountId32,
          delegatee: AccountId32,
          proxyType: ArgonRuntimeProxyType,
          delay: u32,
        ],
        {
          delegator: AccountId32;
          delegatee: AccountId32;
          proxyType: ArgonRuntimeProxyType;
          delay: u32;
        }
      >;
      /**
       * A pure account has been created by new proxy with given
       * disambiguation index and proxy type.
       **/
      PureCreated: AugmentedEvent<
        ApiType,
        [
          pure: AccountId32,
          who: AccountId32,
          proxyType: ArgonRuntimeProxyType,
          disambiguationIndex: u16,
        ],
        {
          pure: AccountId32;
          who: AccountId32;
          proxyType: ArgonRuntimeProxyType;
          disambiguationIndex: u16;
        }
      >;
    };
    sudo: {
      /**
       * The sudo key has been updated.
       **/
      KeyChanged: AugmentedEvent<
        ApiType,
        [old: Option<AccountId32>, new_: AccountId32],
        { old: Option<AccountId32>; new_: AccountId32 }
      >;
      /**
       * The key was permanently removed.
       **/
      KeyRemoved: AugmentedEvent<ApiType, []>;
      /**
       * A sudo call just took place.
       **/
      Sudid: AugmentedEvent<
        ApiType,
        [sudoResult: Result<Null, SpRuntimeDispatchError>],
        { sudoResult: Result<Null, SpRuntimeDispatchError> }
      >;
      /**
       * A [sudo_as](Pallet::sudo_as) call just took place.
       **/
      SudoAsDone: AugmentedEvent<
        ApiType,
        [sudoResult: Result<Null, SpRuntimeDispatchError>],
        { sudoResult: Result<Null, SpRuntimeDispatchError> }
      >;
    };
    system: {
      /**
       * `:code` was updated.
       **/
      CodeUpdated: AugmentedEvent<ApiType, []>;
      /**
       * An extrinsic failed.
       **/
      ExtrinsicFailed: AugmentedEvent<
        ApiType,
        [
          dispatchError: SpRuntimeDispatchError,
          dispatchInfo: FrameSystemDispatchEventInfo,
        ],
        {
          dispatchError: SpRuntimeDispatchError;
          dispatchInfo: FrameSystemDispatchEventInfo;
        }
      >;
      /**
       * An extrinsic completed successfully.
       **/
      ExtrinsicSuccess: AugmentedEvent<
        ApiType,
        [dispatchInfo: FrameSystemDispatchEventInfo],
        { dispatchInfo: FrameSystemDispatchEventInfo }
      >;
      /**
       * An account was reaped.
       **/
      KilledAccount: AugmentedEvent<
        ApiType,
        [account: AccountId32],
        { account: AccountId32 }
      >;
      /**
       * A new account was created.
       **/
      NewAccount: AugmentedEvent<
        ApiType,
        [account: AccountId32],
        { account: AccountId32 }
      >;
      /**
       * On on-chain remark happened.
       **/
      Remarked: AugmentedEvent<
        ApiType,
        [sender: AccountId32, hash_: H256],
        { sender: AccountId32; hash_: H256 }
      >;
      /**
       * An upgrade was authorized.
       **/
      UpgradeAuthorized: AugmentedEvent<
        ApiType,
        [codeHash: H256, checkVersion: bool],
        { codeHash: H256; checkVersion: bool }
      >;
    };
    tokenGateway: {
      /**
       * An asset has been received and transferred to the beneficiary's account
       **/
      AssetReceived: AugmentedEvent<
        ApiType,
        [beneficiary: AccountId32, amount: u128, source: IsmpHostStateMachine],
        { beneficiary: AccountId32; amount: u128; source: IsmpHostStateMachine }
      >;
      /**
       * An asset has been refunded and transferred to the beneficiary's account
       **/
      AssetRefunded: AugmentedEvent<
        ApiType,
        [beneficiary: AccountId32, amount: u128, source: IsmpHostStateMachine],
        { beneficiary: AccountId32; amount: u128; source: IsmpHostStateMachine }
      >;
      /**
       * An asset has been teleported
       **/
      AssetTeleported: AugmentedEvent<
        ApiType,
        [
          from: AccountId32,
          to: H256,
          amount: u128,
          dest: IsmpHostStateMachine,
          commitment: H256,
        ],
        {
          from: AccountId32;
          to: H256;
          amount: u128;
          dest: IsmpHostStateMachine;
          commitment: H256;
        }
      >;
      /**
       * ERC6160 asset creation request dispatched to hyperbridge
       **/
      ERC6160AssetRegistrationDispatched: AugmentedEvent<
        ApiType,
        [commitment: H256],
        { commitment: H256 }
      >;
    };
    transactionPayment: {
      /**
       * A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,
       * has been paid by `who`.
       **/
      TransactionFeePaid: AugmentedEvent<
        ApiType,
        [who: AccountId32, actualFee: u128, tip: u128],
        { who: AccountId32; actualFee: u128; tip: u128 }
      >;
    };
    txPause: {
      /**
       * This pallet, or a specific call is now paused.
       **/
      CallPaused: AugmentedEvent<
        ApiType,
        [fullName: ITuple<[Bytes, Bytes]>],
        { fullName: ITuple<[Bytes, Bytes]> }
      >;
      /**
       * This pallet, or a specific call is now unpaused.
       **/
      CallUnpaused: AugmentedEvent<
        ApiType,
        [fullName: ITuple<[Bytes, Bytes]>],
        { fullName: ITuple<[Bytes, Bytes]> }
      >;
    };
    utility: {
      /**
       * Batch of dispatches completed fully with no error.
       **/
      BatchCompleted: AugmentedEvent<ApiType, []>;
      /**
       * Batch of dispatches completed but has errors.
       **/
      BatchCompletedWithErrors: AugmentedEvent<ApiType, []>;
      /**
       * Batch of dispatches did not complete fully. Index of first failing dispatch given, as
       * well as the error.
       **/
      BatchInterrupted: AugmentedEvent<
        ApiType,
        [index: u32, error: SpRuntimeDispatchError],
        { index: u32; error: SpRuntimeDispatchError }
      >;
      /**
       * A call was dispatched.
       **/
      DispatchedAs: AugmentedEvent<
        ApiType,
        [result: Result<Null, SpRuntimeDispatchError>],
        { result: Result<Null, SpRuntimeDispatchError> }
      >;
      /**
       * A single item within a Batch of dispatches has completed with no error.
       **/
      ItemCompleted: AugmentedEvent<ApiType, []>;
      /**
       * A single item within a Batch of dispatches has completed with error.
       **/
      ItemFailed: AugmentedEvent<
        ApiType,
        [error: SpRuntimeDispatchError],
        { error: SpRuntimeDispatchError }
      >;
    };
    vaults: {
      ObligationCompleted: AugmentedEvent<
        ApiType,
        [vaultId: u32, obligationId: u64, wasCanceled: bool],
        { vaultId: u32; obligationId: u64; wasCanceled: bool }
      >;
      /**
       * An error occurred while completing an obligation
       **/
      ObligationCompletionError: AugmentedEvent<
        ApiType,
        [obligationId: u64, error: SpRuntimeDispatchError],
        { obligationId: u64; error: SpRuntimeDispatchError }
      >;
      ObligationCreated: AugmentedEvent<
        ApiType,
        [
          vaultId: u32,
          obligationId: u64,
          fundType: ArgonPrimitivesVaultFundType,
          beneficiary: AccountId32,
          amount: u128,
          expiration: ArgonPrimitivesVaultObligationExpiration,
        ],
        {
          vaultId: u32;
          obligationId: u64;
          fundType: ArgonPrimitivesVaultFundType;
          beneficiary: AccountId32;
          amount: u128;
          expiration: ArgonPrimitivesVaultObligationExpiration;
        }
      >;
      ObligationModified: AugmentedEvent<
        ApiType,
        [vaultId: u32, obligationId: u64, amount: u128],
        { vaultId: u32; obligationId: u64; amount: u128 }
      >;
      VaultBitcoinXpubChange: AugmentedEvent<
        ApiType,
        [vaultId: u32],
        { vaultId: u32 }
      >;
      VaultClosed: AugmentedEvent<
        ApiType,
        [vaultId: u32, remainingSecuritization: u128, released: u128],
        { vaultId: u32; remainingSecuritization: u128; released: u128 }
      >;
      VaultCreated: AugmentedEvent<
        ApiType,
        [
          vaultId: u32,
          securitization: u128,
          securitizationRatio: u128,
          operatorAccountId: AccountId32,
          openedTick: u64,
        ],
        {
          vaultId: u32;
          securitization: u128;
          securitizationRatio: u128;
          operatorAccountId: AccountId32;
          openedTick: u64;
        }
      >;
      VaultModified: AugmentedEvent<
        ApiType,
        [vaultId: u32, securitization: u128, securitizationRatio: u128],
        { vaultId: u32; securitization: u128; securitizationRatio: u128 }
      >;
      VaultTermsChanged: AugmentedEvent<
        ApiType,
        [vaultId: u32],
        { vaultId: u32 }
      >;
      VaultTermsChangeScheduled: AugmentedEvent<
        ApiType,
        [vaultId: u32, changeTick: u64],
        { vaultId: u32; changeTick: u64 }
      >;
    };
  } // AugmentedEvents
} // declare module
