// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/submittable';

import type {
  ApiTypes,
  AugmentedSubmittable,
  SubmittableExtrinsic,
  SubmittableExtrinsicFunction,
} from '@polkadot/api-base/types';
import type {
  BTreeMap,
  Bytes,
  Compact,
  Option,
  U8aFixed,
  Vec,
  bool,
  u128,
  u16,
  u32,
  u64,
} from '@polkadot/types-codec';
import type { AnyNumber, IMethod, ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, Call, H256, MultiAddress } from '@polkadot/types/interfaces/runtime';
import type {
  ArgonPrimitivesBitcoinCompressedBitcoinPubkey,
  ArgonPrimitivesBitcoinH256Le,
  ArgonPrimitivesBitcoinOpaqueBitcoinXpub,
  ArgonPrimitivesDomainZoneRecord,
  ArgonPrimitivesInherentsBitcoinUtxoSync,
  ArgonPrimitivesInherentsBlockSealInherent,
  ArgonPrimitivesNotaryNotaryMeta,
  ArgonPrimitivesNotebookSignedNotebookHeader,
  ArgonPrimitivesVaultVaultTerms,
  ArgonRuntimeOriginCaller,
  ArgonRuntimeProxyType,
  ArgonRuntimeSessionKeys,
  IsmpGrandpaAddStateMachine,
  IsmpHostStateMachine,
  IsmpMessagingCreateConsensusState,
  IsmpMessagingMessage,
  PalletBalancesAdjustmentDirection,
  PalletIsmpUtilsFundMessageParams,
  PalletIsmpUtilsUpdateConsensusState,
  PalletMultisigTimepoint,
  PalletPriceIndexPriceIndex,
  PalletTokenGatewayAssetRegistration,
  PalletTokenGatewayPrecisionUpdate,
  PalletTokenGatewayTeleportParams,
  PalletVaultsVaultConfig,
  SpConsensusGrandpaEquivocationProof,
  SpCoreVoid,
  SpWeightsWeightV2Weight,
  TokenGatewayPrimitivesGatewayAssetUpdate,
} from '@polkadot/types/lookup';

export type __AugmentedSubmittable = AugmentedSubmittable<() => unknown>;
export type __SubmittableExtrinsic<ApiType extends ApiTypes> = SubmittableExtrinsic<ApiType>;
export type __SubmittableExtrinsicFunction<ApiType extends ApiTypes> =
  SubmittableExtrinsicFunction<ApiType>;

declare module '@polkadot/api-base/types/submittable' {
  interface AugmentedSubmittables<ApiType extends ApiTypes> {
    balances: {
      /**
       * Burn the specified liquid free balance from the origin account.
       *
       * If the origin's account ends up below the existential deposit as a result
       * of the burn and `keep_alive` is false, the account will be reaped.
       *
       * Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,
       * this `burn` operation will reduce total issuance by the amount _burned_.
       **/
      burn: AugmentedSubmittable<
        (
          value: Compact<u128> | AnyNumber | Uint8Array,
          keepAlive: bool | boolean | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Compact<u128>, bool]
      >;
      /**
       * Adjust the total issuance in a saturating way.
       *
       * Can only be called by root and always needs a positive `delta`.
       *
       * # Example
       **/
      forceAdjustTotalIssuance: AugmentedSubmittable<
        (
          direction:
            | PalletBalancesAdjustmentDirection
            | 'Increase'
            | 'Decrease'
            | number
            | Uint8Array,
          delta: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletBalancesAdjustmentDirection, Compact<u128>]
      >;
      /**
       * Set the regular balance of a given account.
       *
       * The dispatch origin for this call is `root`.
       **/
      forceSetBalance: AugmentedSubmittable<
        (
          who:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          newFree: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Compact<u128>]
      >;
      /**
       * Exactly as `transfer_allow_death`, except the origin must be root and the source account
       * may be specified.
       **/
      forceTransfer: AugmentedSubmittable<
        (
          source:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          value: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, MultiAddress, Compact<u128>]
      >;
      /**
       * Unreserve some balance from a user by force.
       *
       * Can only be called by ROOT.
       **/
      forceUnreserve: AugmentedSubmittable<
        (
          who:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          amount: u128 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, u128]
      >;
      /**
       * Transfer the entire transferable balance from the caller account.
       *
       * NOTE: This function only attempts to transfer _transferable_ balances. This means that
       * any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be
       * transferred by this function. To ensure that this function results in a killed account,
       * you might need to prepare the account by removing any reference counters, storage
       * deposits, etc...
       *
       * The dispatch origin of this call must be Signed.
       *
       * - `dest`: The recipient of the transfer.
       * - `keep_alive`: A boolean to determine if the `transfer_all` operation should send all
       * of the funds the account has, causing the sender account to be killed (false), or
       * transfer everything except at least the existential deposit, which will guarantee to
       * keep the sender account alive (true).
       **/
      transferAll: AugmentedSubmittable<
        (
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          keepAlive: bool | boolean | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, bool]
      >;
      /**
       * Transfer some liquid free balance to another account.
       *
       * `transfer_allow_death` will set the `FreeBalance` of the sender and receiver.
       * If the sender's account is below the existential deposit as a result
       * of the transfer, the account will be reaped.
       *
       * The dispatch origin for this call must be `Signed` by the transactor.
       **/
      transferAllowDeath: AugmentedSubmittable<
        (
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          value: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Compact<u128>]
      >;
      /**
       * Same as the [`transfer_allow_death`] call, but with a check that the transfer will not
       * kill the origin account.
       *
       * 99% of the time you want [`transfer_allow_death`] instead.
       *
       * [`transfer_allow_death`]: struct.Pallet.html#method.transfer
       **/
      transferKeepAlive: AugmentedSubmittable<
        (
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          value: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Compact<u128>]
      >;
      /**
       * Upgrade a specified account.
       *
       * - `origin`: Must be `Signed`.
       * - `who`: The account to be upgraded.
       *
       * This will waive the transaction fee if at least all but 10% of the accounts needed to
       * be upgraded. (We let some not have to be upgraded just in order to allow for the
       * possibility of churn).
       **/
      upgradeAccounts: AugmentedSubmittable<
        (
          who: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<AccountId32>]
      >;
    };
    bitcoinLocks: {
      adminModifyMinimumLockedSats: AugmentedSubmittable<
        (satoshis: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [u64]
      >;
      /**
       * Submitted by a Vault operator to cosign the release of a bitcoin utxo. The Bitcoin owner
       * release fee will be burned, and the lock will be allowed to expire without a penalty.
       *
       * This is submitted as a no-fee transaction off chain to allow keys to remain in cold
       * wallets.
       **/
      cosignRelease: AugmentedSubmittable<
        (
          utxoId: u64 | AnyNumber | Uint8Array,
          signature: Bytes | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u64, Bytes]
      >;
      /**
       * Initialize a bitcoin lock. This will create a LockedBitcoin for the submitting account
       * and log the Bitcoin Script hash to Events.
       *
       * The pubkey submitted here will be used to create a script pubkey that will be used in a
       * timelock multisig script to lock the bitcoin.
       *
       * NOTE: A "lock-er" must send btc to the cosigner UTXO address to "complete" the
       * LockedBitcoin and be added to the Bitcoin Mint line.
       **/
      initialize: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          satoshis: Compact<u64> | AnyNumber | Uint8Array,
          bitcoinPubkey: ArgonPrimitivesBitcoinCompressedBitcoinPubkey | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, Compact<u64>, ArgonPrimitivesBitcoinCompressedBitcoinPubkey]
      >;
      /**
       * Ratcheting allows a user to change the lock price of their bitcoin lock. This is
       * functionally the same as releasing and re-initializing, but it allows a user to skip
       * sending transactions through bitcoin and any associated fees. It also allows you to stay
       * on your original lock expiration without having to pay the full year of fees again.
       *
       * Ratcheting "down" - when the price of bitcoin is lower than your lock price, you pay the
       * full release price and get added back to the mint queue at the current market rate. You
       * pocket the difference between the already minted "lock price" and the new market value
       * (which you just had burned). Your new lock price is set to the market low, so you can
       * take advantage of ratchets "up" in the future.
       *
       * Ratcheting "up" - when the price of bitcoin is higher than your lock price, you pay a
       * prorated fee for the remainder of your existing lock duration. You are added to the mint
       * queue for the difference in your new lock price vs the previous lock price.
       **/
      ratchet: AugmentedSubmittable<
        (utxoId: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [u64]
      >;
      /**
       * Submitted by a Bitcoin holder to trigger the release of their Utxo out of the cosign
       * script. A transaction spending the UTXO should be pre-created so that the sighash
       * can be submitted here. The vault operator will have 10 days to counter-sign the
       * transaction. It will be published with the public key as a BitcoinUtxoCosigned Event.
       *
       * Owner must submit a script pubkey and also a fee to pay to the bitcoin network.
       **/
      requestRelease: AugmentedSubmittable<
        (
          utxoId: u64 | AnyNumber | Uint8Array,
          toScriptPubkey: Bytes | string | Uint8Array,
          bitcoinNetworkFee: u64 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u64, Bytes, u64]
      >;
    };
    bitcoinUtxos: {
      /**
       * Sets the most recent confirmed bitcoin block height (only executable by the Oracle
       * Operator account)
       *
       * # Arguments
       * * `bitcoin_height` - the latest bitcoin block height to be confirmed
       **/
      setConfirmedBlock: AugmentedSubmittable<
        (
          bitcoinHeight: u64 | AnyNumber | Uint8Array,
          bitcoinBlockHash: ArgonPrimitivesBitcoinH256Le | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u64, ArgonPrimitivesBitcoinH256Le]
      >;
      /**
       * Sets the oracle operator account id (only executable by the Root account)
       *
       * # Arguments
       * * `account_id` - the account id of the operator
       **/
      setOperator: AugmentedSubmittable<
        (accountId: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [AccountId32]
      >;
      /**
       * Submitted when a bitcoin UTXO has been moved or confirmed
       **/
      sync: AugmentedSubmittable<
        (
          utxoSync:
            | ArgonPrimitivesInherentsBitcoinUtxoSync
            | { spent?: any; verified?: any; invalid?: any; syncToBlock?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [ArgonPrimitivesInherentsBitcoinUtxoSync]
      >;
    };
    blockRewards: {
      setBlockRewardsPaused: AugmentedSubmittable<
        (paused: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [bool]
      >;
    };
    blockSeal: {
      apply: AugmentedSubmittable<
        (
          seal:
            | ArgonPrimitivesInherentsBlockSealInherent
            | { Vote: any }
            | { Compute: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [ArgonPrimitivesInherentsBlockSealInherent]
      >;
    };
    blockSealSpec: {
      configure: AugmentedSubmittable<
        (
          voteMinimum: Option<u128> | null | Uint8Array | u128 | AnyNumber,
          computeDifficulty: Option<u128> | null | Uint8Array | u128 | AnyNumber,
        ) => SubmittableExtrinsic<ApiType>,
        [Option<u128>, Option<u128>]
      >;
    };
    chainTransfer: {
      sendToLocalchain: AugmentedSubmittable<
        (
          amount: Compact<u128> | AnyNumber | Uint8Array,
          notaryId: u32 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Compact<u128>, u32]
      >;
    };
    domains: {
      setZoneRecord: AugmentedSubmittable<
        (
          domainHash: H256 | string | Uint8Array,
          zoneRecord:
            | ArgonPrimitivesDomainZoneRecord
            | { paymentAccount?: any; notaryId?: any; versions?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [H256, ArgonPrimitivesDomainZoneRecord]
      >;
    };
    grandpa: {
      /**
       * Note that the current authority set of the GRANDPA finality gadget has stalled.
       *
       * This will trigger a forced authority set change at the beginning of the next session, to
       * be enacted `delay` blocks after that. The `delay` should be high enough to safely assume
       * that the block signalling the forced change will not be re-orged e.g. 1000 blocks.
       * The block production rate (which may be slowed down because of finality lagging) should
       * be taken into account when choosing the `delay`. The GRANDPA voters based on the new
       * authority will start voting on top of `best_finalized_block_number` for new finalized
       * blocks. `best_finalized_block_number` should be the highest of the latest finalized
       * block of all validators of the new authority set.
       *
       * Only callable by root.
       **/
      noteStalled: AugmentedSubmittable<
        (
          delay: u32 | AnyNumber | Uint8Array,
          bestFinalizedBlockNumber: u32 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, u32]
      >;
      /**
       * Report voter equivocation/misbehavior. This method will verify the
       * equivocation proof and validate the given key ownership proof
       * against the extracted offender. If both are valid, the offence
       * will be reported.
       **/
      reportEquivocation: AugmentedSubmittable<
        (
          equivocationProof:
            | SpConsensusGrandpaEquivocationProof
            | { setId?: any; equivocation?: any }
            | string
            | Uint8Array,
          keyOwnerProof: SpCoreVoid | null,
        ) => SubmittableExtrinsic<ApiType>,
        [SpConsensusGrandpaEquivocationProof, SpCoreVoid]
      >;
      /**
       * Report voter equivocation/misbehavior. This method will verify the
       * equivocation proof and validate the given key ownership proof
       * against the extracted offender. If both are valid, the offence
       * will be reported.
       *
       * This extrinsic must be called unsigned and it is expected that only
       * block authors will call it (validated in `ValidateUnsigned`), as such
       * if the block author is defined it will be defined as the equivocation
       * reporter.
       **/
      reportEquivocationUnsigned: AugmentedSubmittable<
        (
          equivocationProof:
            | SpConsensusGrandpaEquivocationProof
            | { setId?: any; equivocation?: any }
            | string
            | Uint8Array,
          keyOwnerProof: SpCoreVoid | null,
        ) => SubmittableExtrinsic<ApiType>,
        [SpConsensusGrandpaEquivocationProof, SpCoreVoid]
      >;
    };
    ismp: {
      /**
       * Create a consensus client, using a subjectively chosen consensus state. This can also
       * be used to overwrite an existing consensus state. The dispatch origin for this
       * call must be `T::AdminOrigin`.
       *
       * - `message`: [`CreateConsensusState`] struct.
       *
       * Emits [`Event::ConsensusClientCreated`] if successful.
       **/
      createConsensusClient: AugmentedSubmittable<
        (
          message:
            | IsmpMessagingCreateConsensusState
            | {
                consensusState?: any;
                consensusClientId?: any;
                consensusStateId?: any;
                unbondingPeriod?: any;
                challengePeriods?: any;
                stateMachineCommitments?: any;
              }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [IsmpMessagingCreateConsensusState]
      >;
      /**
       * Add more funds to a message (request or response) to be used for delivery and execution.
       *
       * Should not be called on a message that has been completed (delivered or timed-out) as
       * those funds will be lost forever.
       **/
      fundMessage: AugmentedSubmittable<
        (
          message:
            | PalletIsmpUtilsFundMessageParams
            | { commitment?: any; amount?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletIsmpUtilsFundMessageParams]
      >;
      /**
       * Execute the provided batch of ISMP messages, this will short-circuit and revert if any
       * of the provided messages are invalid. This is an unsigned extrinsic that permits anyone
       * execute ISMP messages for free, provided they have valid proofs and the messages have
       * not been previously processed.
       *
       * The dispatch origin for this call must be an unsigned one.
       *
       * - `messages`: the messages to handle or process.
       *
       * Emits different message events based on the Message received if successful.
       **/
      handleUnsigned: AugmentedSubmittable<
        (
          messages:
            | Vec<IsmpMessagingMessage>
            | (
                | IsmpMessagingMessage
                | { Consensus: any }
                | { FraudProof: any }
                | { Request: any }
                | { Response: any }
                | { Timeout: any }
                | string
                | Uint8Array
              )[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<IsmpMessagingMessage>]
      >;
      /**
       * Modify the unbonding period and challenge period for a consensus state.
       * The dispatch origin for this call must be `T::AdminOrigin`.
       *
       * - `message`: `UpdateConsensusState` struct.
       **/
      updateConsensusState: AugmentedSubmittable<
        (
          message:
            | PalletIsmpUtilsUpdateConsensusState
            | { consensusStateId?: any; unbondingPeriod?: any; challengePeriods?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletIsmpUtilsUpdateConsensusState]
      >;
    };
    ismpGrandpa: {
      /**
       * Add some a state machine to the list of supported state machines
       **/
      addStateMachines: AugmentedSubmittable<
        (
          newStateMachines:
            | Vec<IsmpGrandpaAddStateMachine>
            | (
                | IsmpGrandpaAddStateMachine
                | { stateMachine?: any; slotDuration?: any }
                | string
                | Uint8Array
              )[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<IsmpGrandpaAddStateMachine>]
      >;
      /**
       * Remove a state machine from the list of supported state machines
       **/
      removeStateMachines: AugmentedSubmittable<
        (
          stateMachines:
            | Vec<IsmpHostStateMachine>
            | (
                | IsmpHostStateMachine
                | { Evm: any }
                | { Polkadot: any }
                | { Kusama: any }
                | { Substrate: any }
                | { Tendermint: any }
                | { Relay: any }
                | string
                | Uint8Array
              )[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<IsmpHostStateMachine>]
      >;
    };
    liquidityPools: {
      /**
       * Bond argons to a Vault's next liquidity pool, tied to the next frame (aka,
       * tomorrow noon EDT to day after tomorrow noon). The amount bonded to the pool cannot
       * exceed 1/10th of the activated securitization for the vault.
       *
       * The bonded argons and profits will be automatically rolled over to the next fund up to
       * the max securitization activated.
       *
       * - `origin`: The account that is joining the fund
       * - `vault_id`: The vault id that the account would like to join a fund for
       * - `amount`: The amount of argons to contribute to the fund. If you change this amount,
       * it will just add the incremental amount
       **/
      bondArgons: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          amount: u128 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, u128]
      >;
      /**
       * Allows a user to remove their bonded argons from the fund after the hold is released
       * (once epoch starting at bonded frame is complete).
       **/
      unbondArgons: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          frameId: u64 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, u64]
      >;
      /**
       * Set the prebonded argons for a vault. This is used by the vault operator to
       * pre-register funding for each frame. The allocation can be capped per frame using the
       * `max_amount_per_frame` parameter. This can be desirable to get an even spread across all
       * frames. This amount cannot be less than the total amount / 10 or it will never be
       * depleted.
       *
       * NOTE: a second call is additive
       **/
      vaultOperatorPrebond: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          amount: u128 | AnyNumber | Uint8Array,
          maxAmountPerFrame: u128 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, u128, u128]
      >;
    };
    miningSlot: {
      /**
       * Submit a bid for a mining slot in the next cohort. Once all spots are filled in the next
       * cohort, a bidder can be supplanted by supplying a higher bid.
       *
       * Each slot has `MaxCohortSize` spots available.
       *
       * To be eligible for a slot, you must have the required ownership tokens (argonots) in
       * this account. The required amount is calculated as a percentage of the total ownership
       * tokens in the network. This percentage is adjusted before the beginning of each slot.
       *
       * If your bid is no longer winning, a `SlotBidderDropped` event will be emitted. By
       * monitoring for this event, you will be able to ensure your bid is accepted.
       *
       * NOTE: bidding for each slot will be closed at a random block within
       * `mining_config.ticks_before_bid_end_for_vrf_close` blocks of the slot end time.
       *
       * The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`.
       *
       * Parameters:
       * - `bid`: The amount of argons to bid
       * - `keys`: The session "hot" keys for the slot (BlockSealAuthorityId and GrandpaId).
       * - `mining_account_id`: This account_id allows you to operate as this miner account id,
       * but use funding (argonots and bid) from the submitting account
       **/
      bid: AugmentedSubmittable<
        (
          bid: u128 | AnyNumber | Uint8Array,
          keys:
            | ArgonRuntimeSessionKeys
            | { grandpa?: any; blockSealAuthority?: any }
            | string
            | Uint8Array,
          miningAccountId: Option<AccountId32> | null | Uint8Array | AccountId32 | string,
        ) => SubmittableExtrinsic<ApiType>,
        [u128, ArgonRuntimeSessionKeys, Option<AccountId32>]
      >;
      /**
       * Admin function to update the mining slot delay.
       **/
      configureMiningSlotDelay: AugmentedSubmittable<
        (
          miningSlotDelay: Option<u64> | null | Uint8Array | u64 | AnyNumber,
          ticksBeforeBidEndForVrfClose: Option<u64> | null | Uint8Array | u64 | AnyNumber,
        ) => SubmittableExtrinsic<ApiType>,
        [Option<u64>, Option<u64>]
      >;
    };
    mint: {};
    multisig: {
      /**
       * Register approval for a dispatch to be made from a deterministic composite account if
       * approved by a total of `threshold - 1` of `other_signatories`.
       *
       * Payment: `DepositBase` will be reserved if this is the first approval, plus
       * `threshold` times `DepositFactor`. It is returned once this dispatch happens or
       * is cancelled.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * - `threshold`: The total number of approvals for this dispatch before it is executed.
       * - `other_signatories`: The accounts (other than the sender) who can approve this
       * dispatch. May not be empty.
       * - `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is
       * not the first approval, then it must be `Some`, with the timepoint (block number and
       * transaction index) of the first approval transaction.
       * - `call_hash`: The hash of the call to be executed.
       *
       * NOTE: If this is the final approval, you will want to use `as_multi` instead.
       *
       * ## Complexity
       * - `O(S)`.
       * - Up to one balance-reserve or unreserve operation.
       * - One passthrough operation, one insert, both `O(S)` where `S` is the number of
       * signatories. `S` is capped by `MaxSignatories`, with weight being proportional.
       * - One encode & hash, both of complexity `O(S)`.
       * - Up to one binary search and insert (`O(logS + S)`).
       * - I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove.
       * - One event.
       * - Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit
       * taken for its lifetime of `DepositBase + threshold * DepositFactor`.
       **/
      approveAsMulti: AugmentedSubmittable<
        (
          threshold: u16 | AnyNumber | Uint8Array,
          otherSignatories: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
          maybeTimepoint:
            | Option<PalletMultisigTimepoint>
            | null
            | Uint8Array
            | PalletMultisigTimepoint
            | { height?: any; index?: any }
            | string,
          callHash: U8aFixed | string | Uint8Array,
          maxWeight:
            | SpWeightsWeightV2Weight
            | { refTime?: any; proofSize?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u16, Vec<AccountId32>, Option<PalletMultisigTimepoint>, U8aFixed, SpWeightsWeightV2Weight]
      >;
      /**
       * Register approval for a dispatch to be made from a deterministic composite account if
       * approved by a total of `threshold - 1` of `other_signatories`.
       *
       * If there are enough, then dispatch the call.
       *
       * Payment: `DepositBase` will be reserved if this is the first approval, plus
       * `threshold` times `DepositFactor`. It is returned once this dispatch happens or
       * is cancelled.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * - `threshold`: The total number of approvals for this dispatch before it is executed.
       * - `other_signatories`: The accounts (other than the sender) who can approve this
       * dispatch. May not be empty.
       * - `maybe_timepoint`: If this is the first approval, then this must be `None`. If it is
       * not the first approval, then it must be `Some`, with the timepoint (block number and
       * transaction index) of the first approval transaction.
       * - `call`: The call to be executed.
       *
       * NOTE: Unless this is the final approval, you will generally want to use
       * `approve_as_multi` instead, since it only requires a hash of the call.
       *
       * Result is equivalent to the dispatched result if `threshold` is exactly `1`. Otherwise
       * on success, result is `Ok` and the result from the interior call, if it was executed,
       * may be found in the deposited `MultisigExecuted` event.
       *
       * ## Complexity
       * - `O(S + Z + Call)`.
       * - Up to one balance-reserve or unreserve operation.
       * - One passthrough operation, one insert, both `O(S)` where `S` is the number of
       * signatories. `S` is capped by `MaxSignatories`, with weight being proportional.
       * - One call encode & hash, both of complexity `O(Z)` where `Z` is tx-len.
       * - One encode & hash, both of complexity `O(S)`.
       * - Up to one binary search and insert (`O(logS + S)`).
       * - I/O: 1 read `O(S)`, up to 1 mutate `O(S)`. Up to one remove.
       * - One event.
       * - The weight of the `call`.
       * - Storage: inserts one item, value size bounded by `MaxSignatories`, with a deposit
       * taken for its lifetime of `DepositBase + threshold * DepositFactor`.
       **/
      asMulti: AugmentedSubmittable<
        (
          threshold: u16 | AnyNumber | Uint8Array,
          otherSignatories: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
          maybeTimepoint:
            | Option<PalletMultisigTimepoint>
            | null
            | Uint8Array
            | PalletMultisigTimepoint
            | { height?: any; index?: any }
            | string,
          call: Call | IMethod | string | Uint8Array,
          maxWeight:
            | SpWeightsWeightV2Weight
            | { refTime?: any; proofSize?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u16, Vec<AccountId32>, Option<PalletMultisigTimepoint>, Call, SpWeightsWeightV2Weight]
      >;
      /**
       * Immediately dispatch a multi-signature call using a single approval from the caller.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * - `other_signatories`: The accounts (other than the sender) who are part of the
       * multi-signature, but do not participate in the approval process.
       * - `call`: The call to be executed.
       *
       * Result is equivalent to the dispatched result.
       *
       * ## Complexity
       * O(Z + C) where Z is the length of the call and C its execution weight.
       **/
      asMultiThreshold1: AugmentedSubmittable<
        (
          otherSignatories: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<AccountId32>, Call]
      >;
      /**
       * Cancel a pre-existing, on-going multisig transaction. Any deposit reserved previously
       * for this operation will be unreserved on success.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * - `threshold`: The total number of approvals for this dispatch before it is executed.
       * - `other_signatories`: The accounts (other than the sender) who can approve this
       * dispatch. May not be empty.
       * - `timepoint`: The timepoint (block number and transaction index) of the first approval
       * transaction for this dispatch.
       * - `call_hash`: The hash of the call to be executed.
       *
       * ## Complexity
       * - `O(S)`.
       * - Up to one balance-reserve or unreserve operation.
       * - One passthrough operation, one insert, both `O(S)` where `S` is the number of
       * signatories. `S` is capped by `MaxSignatories`, with weight being proportional.
       * - One encode & hash, both of complexity `O(S)`.
       * - One event.
       * - I/O: 1 read `O(S)`, one remove.
       * - Storage: removes one item.
       **/
      cancelAsMulti: AugmentedSubmittable<
        (
          threshold: u16 | AnyNumber | Uint8Array,
          otherSignatories: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
          timepoint: PalletMultisigTimepoint | { height?: any; index?: any } | string | Uint8Array,
          callHash: U8aFixed | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u16, Vec<AccountId32>, PalletMultisigTimepoint, U8aFixed]
      >;
      /**
       * Poke the deposit reserved for an existing multisig operation.
       *
       * The dispatch origin for this call must be _Signed_ and must be the original depositor of
       * the multisig operation.
       *
       * The transaction fee is waived if the deposit amount has changed.
       *
       * - `threshold`: The total number of approvals needed for this multisig.
       * - `other_signatories`: The accounts (other than the sender) who are part of the
       * multisig.
       * - `call_hash`: The hash of the call this deposit is reserved for.
       *
       * Emits `DepositPoked` if successful.
       **/
      pokeDeposit: AugmentedSubmittable<
        (
          threshold: u16 | AnyNumber | Uint8Array,
          otherSignatories: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
          callHash: U8aFixed | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u16, Vec<AccountId32>, U8aFixed]
      >;
    };
    notaries: {
      activate: AugmentedSubmittable<
        (operatorAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [AccountId32]
      >;
      propose: AugmentedSubmittable<
        (
          meta:
            | ArgonPrimitivesNotaryNotaryMeta
            | { name?: any; public?: any; hosts?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [ArgonPrimitivesNotaryNotaryMeta]
      >;
      /**
       * Update the metadata of a notary, to be effective at the given tick height, which must be
       * >= MetaChangesTickDelay ticks in the future.
       **/
      update: AugmentedSubmittable<
        (
          notaryId: Compact<u32> | AnyNumber | Uint8Array,
          meta:
            | ArgonPrimitivesNotaryNotaryMeta
            | { name?: any; public?: any; hosts?: any }
            | string
            | Uint8Array,
          effectiveTick: Compact<u64> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Compact<u32>, ArgonPrimitivesNotaryNotaryMeta, Compact<u64>]
      >;
    };
    notebook: {
      submit: AugmentedSubmittable<
        (
          notebooks:
            | Vec<ArgonPrimitivesNotebookSignedNotebookHeader>
            | (
                | ArgonPrimitivesNotebookSignedNotebookHeader
                | { header?: any; signature?: any }
                | string
                | Uint8Array
              )[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<ArgonPrimitivesNotebookSignedNotebookHeader>]
      >;
      unlock: AugmentedSubmittable<
        (notaryId: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [u32]
      >;
    };
    ownership: {
      /**
       * Burn the specified liquid free balance from the origin account.
       *
       * If the origin's account ends up below the existential deposit as a result
       * of the burn and `keep_alive` is false, the account will be reaped.
       *
       * Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,
       * this `burn` operation will reduce total issuance by the amount _burned_.
       **/
      burn: AugmentedSubmittable<
        (
          value: Compact<u128> | AnyNumber | Uint8Array,
          keepAlive: bool | boolean | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Compact<u128>, bool]
      >;
      /**
       * Adjust the total issuance in a saturating way.
       *
       * Can only be called by root and always needs a positive `delta`.
       *
       * # Example
       **/
      forceAdjustTotalIssuance: AugmentedSubmittable<
        (
          direction:
            | PalletBalancesAdjustmentDirection
            | 'Increase'
            | 'Decrease'
            | number
            | Uint8Array,
          delta: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletBalancesAdjustmentDirection, Compact<u128>]
      >;
      /**
       * Set the regular balance of a given account.
       *
       * The dispatch origin for this call is `root`.
       **/
      forceSetBalance: AugmentedSubmittable<
        (
          who:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          newFree: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Compact<u128>]
      >;
      /**
       * Exactly as `transfer_allow_death`, except the origin must be root and the source account
       * may be specified.
       **/
      forceTransfer: AugmentedSubmittable<
        (
          source:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          value: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, MultiAddress, Compact<u128>]
      >;
      /**
       * Unreserve some balance from a user by force.
       *
       * Can only be called by ROOT.
       **/
      forceUnreserve: AugmentedSubmittable<
        (
          who:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          amount: u128 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, u128]
      >;
      /**
       * Transfer the entire transferable balance from the caller account.
       *
       * NOTE: This function only attempts to transfer _transferable_ balances. This means that
       * any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be
       * transferred by this function. To ensure that this function results in a killed account,
       * you might need to prepare the account by removing any reference counters, storage
       * deposits, etc...
       *
       * The dispatch origin of this call must be Signed.
       *
       * - `dest`: The recipient of the transfer.
       * - `keep_alive`: A boolean to determine if the `transfer_all` operation should send all
       * of the funds the account has, causing the sender account to be killed (false), or
       * transfer everything except at least the existential deposit, which will guarantee to
       * keep the sender account alive (true).
       **/
      transferAll: AugmentedSubmittable<
        (
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          keepAlive: bool | boolean | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, bool]
      >;
      /**
       * Transfer some liquid free balance to another account.
       *
       * `transfer_allow_death` will set the `FreeBalance` of the sender and receiver.
       * If the sender's account is below the existential deposit as a result
       * of the transfer, the account will be reaped.
       *
       * The dispatch origin for this call must be `Signed` by the transactor.
       **/
      transferAllowDeath: AugmentedSubmittable<
        (
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          value: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Compact<u128>]
      >;
      /**
       * Same as the [`transfer_allow_death`] call, but with a check that the transfer will not
       * kill the origin account.
       *
       * 99% of the time you want [`transfer_allow_death`] instead.
       *
       * [`transfer_allow_death`]: struct.Pallet.html#method.transfer
       **/
      transferKeepAlive: AugmentedSubmittable<
        (
          dest:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          value: Compact<u128> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Compact<u128>]
      >;
      /**
       * Upgrade a specified account.
       *
       * - `origin`: Must be `Signed`.
       * - `who`: The account to be upgraded.
       *
       * This will waive the transaction fee if at least all but 10% of the accounts needed to
       * be upgraded. (We let some not have to be upgraded just in order to allow for the
       * possibility of churn).
       **/
      upgradeAccounts: AugmentedSubmittable<
        (
          who: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<AccountId32>]
      >;
    };
    priceIndex: {
      /**
       * Sets the operator account id (only executable by the Root account)
       *
       * # Arguments
       * * `account_id` - the account id of the operator
       **/
      setOperator: AugmentedSubmittable<
        (accountId: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [AccountId32]
      >;
      /**
       * Submit the latest price index. Only valid for the configured operator account
       **/
      submit: AugmentedSubmittable<
        (
          index:
            | PalletPriceIndexPriceIndex
            | {
                btcUsdPrice?: any;
                argonotUsdPrice?: any;
                argonUsdPrice?: any;
                argonUsdTargetPrice?: any;
                argonTimeWeightedAverageLiquidity?: any;
                tick?: any;
              }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletPriceIndexPriceIndex]
      >;
    };
    proxy: {
      /**
       * Register a proxy account for the sender that is able to make calls on its behalf.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * Parameters:
       * - `proxy`: The account that the `caller` would like to make a proxy.
       * - `proxy_type`: The permissions allowed for this proxy account.
       * - `delay`: The announcement period required of the initial proxy. Will generally be
       * zero.
       **/
      addProxy: AugmentedSubmittable<
        (
          delegate:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          proxyType:
            | ArgonRuntimeProxyType
            | 'Any'
            | 'NonTransfer'
            | 'PriceIndex'
            | 'MiningBid'
            | 'BitcoinCosign'
            | 'VaultAdmin'
            | number
            | Uint8Array,
          delay: u32 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, ArgonRuntimeProxyType, u32]
      >;
      /**
       * Publish the hash of a proxy-call that will be made in the future.
       *
       * This must be called some number of blocks before the corresponding `proxy` is attempted
       * if the delay associated with the proxy relationship is greater than zero.
       *
       * No more than `MaxPending` announcements may be made at any one time.
       *
       * This will take a deposit of `AnnouncementDepositFactor` as well as
       * `AnnouncementDepositBase` if there are no other pending announcements.
       *
       * The dispatch origin for this call must be _Signed_ and a proxy of `real`.
       *
       * Parameters:
       * - `real`: The account that the proxy will make a call on behalf of.
       * - `call_hash`: The hash of the call to be made by the `real` account.
       **/
      announce: AugmentedSubmittable<
        (
          real:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          callHash: H256 | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, H256]
      >;
      /**
       * Spawn a fresh new account that is guaranteed to be otherwise inaccessible, and
       * initialize it with a proxy of `proxy_type` for `origin` sender.
       *
       * Requires a `Signed` origin.
       *
       * - `proxy_type`: The type of the proxy that the sender will be registered as over the
       * new account. This will almost always be the most permissive `ProxyType` possible to
       * allow for maximum flexibility.
       * - `index`: A disambiguation index, in case this is called multiple times in the same
       * transaction (e.g. with `utility::batch`). Unless you're using `batch` you probably just
       * want to use `0`.
       * - `delay`: The announcement period required of the initial proxy. Will generally be
       * zero.
       *
       * Fails with `Duplicate` if this has already been called in this transaction, from the
       * same sender, with the same parameters.
       *
       * Fails if there are insufficient funds to pay for deposit.
       **/
      createPure: AugmentedSubmittable<
        (
          proxyType:
            | ArgonRuntimeProxyType
            | 'Any'
            | 'NonTransfer'
            | 'PriceIndex'
            | 'MiningBid'
            | 'BitcoinCosign'
            | 'VaultAdmin'
            | number
            | Uint8Array,
          delay: u32 | AnyNumber | Uint8Array,
          index: u16 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [ArgonRuntimeProxyType, u32, u16]
      >;
      /**
       * Removes a previously spawned pure proxy.
       *
       * WARNING: **All access to this account will be lost.** Any funds held in it will be
       * inaccessible.
       *
       * Requires a `Signed` origin, and the sender account must have been created by a call to
       * `pure` with corresponding parameters.
       *
       * - `spawner`: The account that originally called `pure` to create this account.
       * - `index`: The disambiguation index originally passed to `pure`. Probably `0`.
       * - `proxy_type`: The proxy type originally passed to `pure`.
       * - `height`: The height of the chain when the call to `pure` was processed.
       * - `ext_index`: The extrinsic index in which the call to `pure` was processed.
       *
       * Fails with `NoPermission` in case the caller is not a previously created pure
       * account whose `pure` call has corresponding parameters.
       **/
      killPure: AugmentedSubmittable<
        (
          spawner:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          proxyType:
            | ArgonRuntimeProxyType
            | 'Any'
            | 'NonTransfer'
            | 'PriceIndex'
            | 'MiningBid'
            | 'BitcoinCosign'
            | 'VaultAdmin'
            | number
            | Uint8Array,
          index: u16 | AnyNumber | Uint8Array,
          height: Compact<u32> | AnyNumber | Uint8Array,
          extIndex: Compact<u32> | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, ArgonRuntimeProxyType, u16, Compact<u32>, Compact<u32>]
      >;
      /**
       * Poke / Adjust deposits made for proxies and announcements based on current values.
       * This can be used by accounts to possibly lower their locked amount.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * The transaction fee is waived if the deposit amount has changed.
       *
       * Emits `DepositPoked` if successful.
       **/
      pokeDeposit: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
      /**
       * Dispatch the given `call` from an account that the sender is authorised for through
       * `add_proxy`.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * Parameters:
       * - `real`: The account that the proxy will make a call on behalf of.
       * - `force_proxy_type`: Specify the exact proxy type to be used and checked for this call.
       * - `call`: The call to be made by the `real` account.
       **/
      proxy: AugmentedSubmittable<
        (
          real:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          forceProxyType:
            | Option<ArgonRuntimeProxyType>
            | null
            | Uint8Array
            | ArgonRuntimeProxyType
            | 'Any'
            | 'NonTransfer'
            | 'PriceIndex'
            | 'MiningBid'
            | 'BitcoinCosign'
            | 'VaultAdmin'
            | number,
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Option<ArgonRuntimeProxyType>, Call]
      >;
      /**
       * Dispatch the given `call` from an account that the sender is authorized for through
       * `add_proxy`.
       *
       * Removes any corresponding announcement(s).
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * Parameters:
       * - `real`: The account that the proxy will make a call on behalf of.
       * - `force_proxy_type`: Specify the exact proxy type to be used and checked for this call.
       * - `call`: The call to be made by the `real` account.
       **/
      proxyAnnounced: AugmentedSubmittable<
        (
          delegate:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          real:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          forceProxyType:
            | Option<ArgonRuntimeProxyType>
            | null
            | Uint8Array
            | ArgonRuntimeProxyType
            | 'Any'
            | 'NonTransfer'
            | 'PriceIndex'
            | 'MiningBid'
            | 'BitcoinCosign'
            | 'VaultAdmin'
            | number,
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, MultiAddress, Option<ArgonRuntimeProxyType>, Call]
      >;
      /**
       * Remove the given announcement of a delegate.
       *
       * May be called by a target (proxied) account to remove a call that one of their delegates
       * (`delegate`) has announced they want to execute. The deposit is returned.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * Parameters:
       * - `delegate`: The account that previously announced the call.
       * - `call_hash`: The hash of the call to be made.
       **/
      rejectAnnouncement: AugmentedSubmittable<
        (
          delegate:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          callHash: H256 | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, H256]
      >;
      /**
       * Remove a given announcement.
       *
       * May be called by a proxy account to remove a call they previously announced and return
       * the deposit.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * Parameters:
       * - `real`: The account that the proxy will make a call on behalf of.
       * - `call_hash`: The hash of the call to be made by the `real` account.
       **/
      removeAnnouncement: AugmentedSubmittable<
        (
          real:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          callHash: H256 | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, H256]
      >;
      /**
       * Unregister all proxy accounts for the sender.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * WARNING: This may be called on accounts created by `pure`, however if done, then
       * the unreserved fees will be inaccessible. **All access to this account will be lost.**
       **/
      removeProxies: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
      /**
       * Unregister a proxy account for the sender.
       *
       * The dispatch origin for this call must be _Signed_.
       *
       * Parameters:
       * - `proxy`: The account that the `caller` would like to remove as a proxy.
       * - `proxy_type`: The permissions currently enabled for the removed proxy account.
       **/
      removeProxy: AugmentedSubmittable<
        (
          delegate:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          proxyType:
            | ArgonRuntimeProxyType
            | 'Any'
            | 'NonTransfer'
            | 'PriceIndex'
            | 'MiningBid'
            | 'BitcoinCosign'
            | 'VaultAdmin'
            | number
            | Uint8Array,
          delay: u32 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, ArgonRuntimeProxyType, u32]
      >;
    };
    sudo: {
      /**
       * Permanently removes the sudo key.
       *
       * **This cannot be un-done.**
       **/
      removeKey: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
      /**
       * Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo
       * key.
       **/
      setKey: AugmentedSubmittable<
        (
          updated:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress]
      >;
      /**
       * Authenticates the sudo key and dispatches a function call with `Root` origin.
       **/
      sudo: AugmentedSubmittable<
        (call: Call | IMethod | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Call]
      >;
      /**
       * Authenticates the sudo key and dispatches a function call with `Signed` origin from
       * a given account.
       *
       * The dispatch origin for this call must be _Signed_.
       **/
      sudoAs: AugmentedSubmittable<
        (
          who:
            | MultiAddress
            | { Id: any }
            | { Index: any }
            | { Raw: any }
            | { Address32: any }
            | { Address20: any }
            | string
            | Uint8Array,
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [MultiAddress, Call]
      >;
      /**
       * Authenticates the sudo key and dispatches a function call with `Root` origin.
       * This function does not check the weight of the call, and instead allows the
       * Sudo user to specify the weight of the call.
       *
       * The dispatch origin for this call must be _Signed_.
       **/
      sudoUncheckedWeight: AugmentedSubmittable<
        (
          call: Call | IMethod | string | Uint8Array,
          weight:
            | SpWeightsWeightV2Weight
            | { refTime?: any; proofSize?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Call, SpWeightsWeightV2Weight]
      >;
    };
    system: {
      /**
       * Provide the preimage (runtime binary) `code` for an upgrade that has been authorized.
       *
       * If the authorization required a version check, this call will ensure the spec name
       * remains unchanged and that the spec version has increased.
       *
       * Depending on the runtime's `OnSetCode` configuration, this function may directly apply
       * the new `code` in the same block or attempt to schedule the upgrade.
       *
       * All origins are allowed.
       **/
      applyAuthorizedUpgrade: AugmentedSubmittable<
        (code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Bytes]
      >;
      /**
       * Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied
       * later.
       *
       * This call requires Root origin.
       **/
      authorizeUpgrade: AugmentedSubmittable<
        (codeHash: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [H256]
      >;
      /**
       * Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied
       * later.
       *
       * WARNING: This authorizes an upgrade that will take place without any safety checks, for
       * example that the spec name remains the same and that the version number increases. Not
       * recommended for normal use. Use `authorize_upgrade` instead.
       *
       * This call requires Root origin.
       **/
      authorizeUpgradeWithoutChecks: AugmentedSubmittable<
        (codeHash: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [H256]
      >;
      /**
       * Kill all storage items with a key that starts with the given prefix.
       *
       * **NOTE:** We rely on the Root origin to provide us the number of subkeys under
       * the prefix we are removing to accurately calculate the weight of this function.
       **/
      killPrefix: AugmentedSubmittable<
        (
          prefix: Bytes | string | Uint8Array,
          subkeys: u32 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Bytes, u32]
      >;
      /**
       * Kill some items from storage.
       **/
      killStorage: AugmentedSubmittable<
        (keys: Vec<Bytes> | (Bytes | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>,
        [Vec<Bytes>]
      >;
      /**
       * Make some on-chain remark.
       *
       * Can be executed by every `origin`.
       **/
      remark: AugmentedSubmittable<
        (remark: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Bytes]
      >;
      /**
       * Make some on-chain remark and emit event.
       **/
      remarkWithEvent: AugmentedSubmittable<
        (remark: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Bytes]
      >;
      /**
       * Set the new runtime code.
       **/
      setCode: AugmentedSubmittable<
        (code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Bytes]
      >;
      /**
       * Set the new runtime code without doing any checks of the given `code`.
       *
       * Note that runtime upgrades will not run if this is called with a not-increasing spec
       * version!
       **/
      setCodeWithoutChecks: AugmentedSubmittable<
        (code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Bytes]
      >;
      /**
       * Set the number of pages in the WebAssembly environment's heap.
       **/
      setHeapPages: AugmentedSubmittable<
        (pages: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [u64]
      >;
      /**
       * Set some items of storage.
       **/
      setStorage: AugmentedSubmittable<
        (
          items:
            | Vec<ITuple<[Bytes, Bytes]>>
            | [Bytes | string | Uint8Array, Bytes | string | Uint8Array][],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<ITuple<[Bytes, Bytes]>>]
      >;
    };
    ticks: {};
    timestamp: {
      /**
       * Set the current time.
       *
       * This call should be invoked exactly once per block. It will panic at the finalization
       * phase, if this call hasn't been invoked by that time.
       *
       * The timestamp should be greater than the previous one by the amount specified by
       * [`Config::MinimumPeriod`].
       *
       * The dispatch origin for this call must be _None_.
       *
       * This dispatch class is _Mandatory_ to ensure it gets executed in the block. Be aware
       * that changing the complexity of this call could result exhausting the resources in a
       * block to execute any other calls.
       *
       * ## Complexity
       * - `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)
       * - 1 storage read and 1 storage mutation (codec `O(1)` because of `DidUpdate::take` in
       * `on_finalize`)
       * - 1 event handler `on_timestamp_set`. Must be `O(1)`.
       **/
      set: AugmentedSubmittable<
        (now: Compact<u64> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [Compact<u64>]
      >;
    };
    tokenGateway: {
      /**
       * Registers a multi-chain ERC6160 asset. The asset should not already exist.
       *
       * This works by dispatching a request to the TokenGateway module on each requested chain
       * to create the asset.
       * `native` should be true if this asset originates from this chain
       **/
      createErc6160Asset: AugmentedSubmittable<
        (
          asset:
            | PalletTokenGatewayAssetRegistration
            | { localId?: any; reg?: any; native?: any; precision?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletTokenGatewayAssetRegistration]
      >;
      /**
       * Set the token gateway address for specified chains
       **/
      setTokenGatewayAddresses: AugmentedSubmittable<
        (addresses: BTreeMap<IsmpHostStateMachine, Bytes>) => SubmittableExtrinsic<ApiType>,
        [BTreeMap<IsmpHostStateMachine, Bytes>]
      >;
      /**
       * Teleports a registered asset
       * locks the asset and dispatches a request to token gateway on the destination
       **/
      teleport: AugmentedSubmittable<
        (
          params:
            | PalletTokenGatewayTeleportParams
            | {
                assetId?: any;
                destination?: any;
                recepient?: any;
                amount?: any;
                timeout?: any;
                tokenGateway?: any;
                relayerFee?: any;
                callData?: any;
                redeem?: any;
              }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletTokenGatewayTeleportParams]
      >;
      /**
       * Update the precision for an existing asset
       **/
      updateAssetPrecision: AugmentedSubmittable<
        (
          update:
            | PalletTokenGatewayPrecisionUpdate
            | { assetId?: any; precisions?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletTokenGatewayPrecisionUpdate]
      >;
      /**
       * Registers a multi-chain ERC6160 asset. The asset should not already exist.
       *
       * This works by dispatching a request to the TokenGateway module on each requested chain
       * to create the asset.
       **/
      updateErc6160Asset: AugmentedSubmittable<
        (
          asset:
            | TokenGatewayPrimitivesGatewayAssetUpdate
            | { assetId?: any; addChains?: any; removeChains?: any; newAdmins?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [TokenGatewayPrimitivesGatewayAssetUpdate]
      >;
    };
    txPause: {
      /**
       * Pause a call.
       *
       * Can only be called by [`Config::PauseOrigin`].
       * Emits an [`Event::CallPaused`] event on success.
       **/
      pause: AugmentedSubmittable<
        (
          fullName:
            | ITuple<[Bytes, Bytes]>
            | [Bytes | string | Uint8Array, Bytes | string | Uint8Array],
        ) => SubmittableExtrinsic<ApiType>,
        [ITuple<[Bytes, Bytes]>]
      >;
      /**
       * Un-pause a call.
       *
       * Can only be called by [`Config::UnpauseOrigin`].
       * Emits an [`Event::CallUnpaused`] event on success.
       **/
      unpause: AugmentedSubmittable<
        (
          ident:
            | ITuple<[Bytes, Bytes]>
            | [Bytes | string | Uint8Array, Bytes | string | Uint8Array],
        ) => SubmittableExtrinsic<ApiType>,
        [ITuple<[Bytes, Bytes]>]
      >;
    };
    utility: {
      /**
       * Send a call through an indexed pseudonym of the sender.
       *
       * Filter from origin are passed along. The call will be dispatched with an origin which
       * use the same filter as the origin of this call.
       *
       * NOTE: If you need to ensure that any account-based filtering is not honored (i.e.
       * because you expect `proxy` to have been used prior in the call stack and you do not want
       * the call restrictions to apply to any sub-accounts), then use `as_multi_threshold_1`
       * in the Multisig pallet instead.
       *
       * NOTE: Prior to version *12, this was called `as_limited_sub`.
       *
       * The dispatch origin for this call must be _Signed_.
       **/
      asDerivative: AugmentedSubmittable<
        (
          index: u16 | AnyNumber | Uint8Array,
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u16, Call]
      >;
      /**
       * Send a batch of dispatch calls.
       *
       * May be called from any origin except `None`.
       *
       * - `calls`: The calls to be dispatched from the same origin. The number of call must not
       * exceed the constant: `batched_calls_limit` (available in constant metadata).
       *
       * If origin is root then the calls are dispatched without checking origin filter. (This
       * includes bypassing `frame_system::Config::BaseCallFilter`).
       *
       * ## Complexity
       * - O(C) where C is the number of calls to be batched.
       *
       * This will return `Ok` in all circumstances. To determine the success of the batch, an
       * event is deposited. If a call failed and the batch was interrupted, then the
       * `BatchInterrupted` event is deposited, along with the number of successful calls made
       * and the error of the failed call. If all were successful, then the `BatchCompleted`
       * event is deposited.
       **/
      batch: AugmentedSubmittable<
        (
          calls: Vec<Call> | (Call | IMethod | string | Uint8Array)[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<Call>]
      >;
      /**
       * Send a batch of dispatch calls and atomically execute them.
       * The whole transaction will rollback and fail if any of the calls failed.
       *
       * May be called from any origin except `None`.
       *
       * - `calls`: The calls to be dispatched from the same origin. The number of call must not
       * exceed the constant: `batched_calls_limit` (available in constant metadata).
       *
       * If origin is root then the calls are dispatched without checking origin filter. (This
       * includes bypassing `frame_system::Config::BaseCallFilter`).
       *
       * ## Complexity
       * - O(C) where C is the number of calls to be batched.
       **/
      batchAll: AugmentedSubmittable<
        (
          calls: Vec<Call> | (Call | IMethod | string | Uint8Array)[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<Call>]
      >;
      /**
       * Dispatches a function call with a provided origin.
       *
       * The dispatch origin for this call must be _Root_.
       *
       * ## Complexity
       * - O(1).
       **/
      dispatchAs: AugmentedSubmittable<
        (
          asOrigin: ArgonRuntimeOriginCaller | { system: any } | string | Uint8Array,
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [ArgonRuntimeOriginCaller, Call]
      >;
      /**
       * Dispatches a function call with a provided origin.
       *
       * Almost the same as [`Pallet::dispatch_as`] but forwards any error of the inner call.
       *
       * The dispatch origin for this call must be _Root_.
       **/
      dispatchAsFallible: AugmentedSubmittable<
        (
          asOrigin: ArgonRuntimeOriginCaller | { system: any } | string | Uint8Array,
          call: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [ArgonRuntimeOriginCaller, Call]
      >;
      /**
       * Send a batch of dispatch calls.
       * Unlike `batch`, it allows errors and won't interrupt.
       *
       * May be called from any origin except `None`.
       *
       * - `calls`: The calls to be dispatched from the same origin. The number of call must not
       * exceed the constant: `batched_calls_limit` (available in constant metadata).
       *
       * If origin is root then the calls are dispatch without checking origin filter. (This
       * includes bypassing `frame_system::Config::BaseCallFilter`).
       *
       * ## Complexity
       * - O(C) where C is the number of calls to be batched.
       **/
      forceBatch: AugmentedSubmittable<
        (
          calls: Vec<Call> | (Call | IMethod | string | Uint8Array)[],
        ) => SubmittableExtrinsic<ApiType>,
        [Vec<Call>]
      >;
      /**
       * Dispatch a fallback call in the event the main call fails to execute.
       * May be called from any origin except `None`.
       *
       * This function first attempts to dispatch the `main` call.
       * If the `main` call fails, the `fallback` is attemted.
       * if the fallback is successfully dispatched, the weights of both calls
       * are accumulated and an event containing the main call error is deposited.
       *
       * In the event of a fallback failure the whole call fails
       * with the weights returned.
       *
       * - `main`: The main call to be dispatched. This is the primary action to execute.
       * - `fallback`: The fallback call to be dispatched in case the `main` call fails.
       *
       * ## Dispatch Logic
       * - If the origin is `root`, both the main and fallback calls are executed without
       * applying any origin filters.
       * - If the origin is not `root`, the origin filter is applied to both the `main` and
       * `fallback` calls.
       *
       * ## Use Case
       * - Some use cases might involve submitting a `batch` type call in either main, fallback
       * or both.
       **/
      ifElse: AugmentedSubmittable<
        (
          main: Call | IMethod | string | Uint8Array,
          fallback: Call | IMethod | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Call, Call]
      >;
      /**
       * Dispatch a function call with a specified weight.
       *
       * This function does not check the weight of the call, and instead allows the
       * Root origin to specify the weight of the call.
       *
       * The dispatch origin for this call must be _Root_.
       **/
      withWeight: AugmentedSubmittable<
        (
          call: Call | IMethod | string | Uint8Array,
          weight:
            | SpWeightsWeightV2Weight
            | { refTime?: any; proofSize?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [Call, SpWeightsWeightV2Weight]
      >;
    };
    vaults: {
      /**
       * Stop offering additional bitcoin locks from this vault. Will not affect existing
       * locks. As funds are returned, they will be released to the vault owner.
       **/
      close: AugmentedSubmittable<
        (vaultId: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>,
        [u32]
      >;
      create: AugmentedSubmittable<
        (
          vaultConfig:
            | PalletVaultsVaultConfig
            | { terms?: any; securitization?: any; bitcoinXpubkey?: any; securitizationRatio?: any }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [PalletVaultsVaultConfig]
      >;
      /**
       * Modify funds allocated by the vault. This will not affect issued bitcoin locks, but will
       * affect the amount of funds available for new ones.
       *
       * The securitization percent must be maintained or increased.
       *
       * The amount allocated may not go below the existing reserved amounts, but you can release
       * funds in this vault as bitcoin locks are released. To stop issuing any more bitcoin
       * locks, use the `close` api.
       **/
      modifyFunding: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          securitization: u128 | AnyNumber | Uint8Array,
          securitizationRatio: u128 | AnyNumber | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, u128, u128]
      >;
      /**
       * Change the terms of this vault. The change will be applied at the next mining slot
       * change that is at least `MinTermsModificationBlockDelay` blocks away.
       **/
      modifyTerms: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          terms:
            | ArgonPrimitivesVaultVaultTerms
            | {
                bitcoinAnnualPercentRate?: any;
                bitcoinBaseFee?: any;
                liquidityPoolProfitSharing?: any;
              }
            | string
            | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, ArgonPrimitivesVaultVaultTerms]
      >;
      /**
       * Replace the bitcoin xpubkey for this vault. This will not affect existing bitcoin locks,
       * but will be used for any locks after this point. Will be rejected if already
       * used.
       **/
      replaceBitcoinXpub: AugmentedSubmittable<
        (
          vaultId: u32 | AnyNumber | Uint8Array,
          bitcoinXpub: ArgonPrimitivesBitcoinOpaqueBitcoinXpub | string | Uint8Array,
        ) => SubmittableExtrinsic<ApiType>,
        [u32, ArgonPrimitivesBitcoinOpaqueBitcoinXpub]
      >;
    };
  } // AugmentedSubmittables
} // declare module
