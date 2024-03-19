// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/consts';

import type { ApiTypes, AugmentedConst } from '@polkadot/api-base/types';
import type { u128, u16, u32, u64, u8 } from '@polkadot/types-codec';
import type { FrameSupportPalletId, FrameSystemLimitsBlockLength, FrameSystemLimitsBlockWeights, SpVersionRuntimeVersion, SpWeightsRuntimeDbWeight } from '@polkadot/types/lookup';

export type __AugmentedConst<ApiType extends ApiTypes> = AugmentedConst<ApiType>;

declare module '@polkadot/api-base/types/consts' {
  interface AugmentedConsts<ApiType extends ApiTypes> {
    argonBalances: {
      /**
       * The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!
       * 
       * If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for
       * this pallet. However, you do so at your own risk: this will open up a major DoS vector.
       * In case you have multiple sources of provider references, you may also get unexpected
       * behaviour if you set this to zero.
       * 
       * Bottom line: Do yourself a favour and make it at least one!
       **/
      existentialDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum number of individual freeze locks that can exist on an account at any time.
       **/
      maxFreezes: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of locks that should exist on an account.
       * Not strictly enforced, but used for weight estimation.
       **/
      maxLocks: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of named reserves that can exist on an account.
       **/
      maxReserves: u32 & AugmentedConst<ApiType>;
    };
    blockRewards: {
      /**
       * Number of argons minted per block
       **/
      argonsPerBlock: u128 & AugmentedConst<ApiType>;
      /**
       * Number of blocks for halving of ulixee rewards
       **/
      halvingBlocks: u32 & AugmentedConst<ApiType>;
      /**
       * Blocks until a block reward is mature
       **/
      maturationBlocks: u32 & AugmentedConst<ApiType>;
      /**
       * Percent as a number out of 100 of the block reward that goes to the miner.
       **/
      minerPayoutPercent: u32 & AugmentedConst<ApiType>;
      /**
       * Number of ulixees minted per block
       **/
      startingUlixeesPerBlock: u128 & AugmentedConst<ApiType>;
    };
    blockSealSpec: {
      /**
       * The frequency for changing the minimum
       **/
      changePeriod: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum active notaries allowed
       **/
      maxActiveNotaries: u32 & AugmentedConst<ApiType>;
      /**
       * The desired votes per block
       **/
      targetBlockVotes: u128 & AugmentedConst<ApiType>;
    };
    bond: {
      /**
       * Blocks per year used for APR calculations
       **/
      blocksPerYear: u32 & AugmentedConst<ApiType>;
      /**
       * Pallet storage requires bounds, so we have to set a maximum number that can expire in a
       * single block
       **/
      maxConcurrentlyExpiringBondFunds: u32 & AugmentedConst<ApiType>;
      /**
       * Pallet storage requires bounds, so we have to set a maximum number that can expire in a
       * single block
       **/
      maxConcurrentlyExpiringBonds: u32 & AugmentedConst<ApiType>;
      /**
       * Minimum amount for a bond
       **/
      minimumBondAmount: u128 & AugmentedConst<ApiType>;
    };
    chainTransfer: {
      /**
       * How many transfers out can be queued per block
       **/
      maxPendingTransfersOutPerBlock: u32 & AugmentedConst<ApiType>;
      palletId: FrameSupportPalletId & AugmentedConst<ApiType>;
      /**
       * How long a transfer should remain in storage before returning.
       **/
      transferExpirationBlocks: u32 & AugmentedConst<ApiType>;
    };
    grandpa: {
      /**
       * Max Authorities in use
       **/
      maxAuthorities: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of nominators for each validator.
       **/
      maxNominators: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of entries to keep in the set id to session index mapping.
       * 
       * Since the `SetIdSession` map is only used for validating equivocations this
       * value should relate to the bonding duration of whatever staking system is
       * being used (if any). If equivocation handling is not enabled then this value
       * can be zero.
       **/
      maxSetIdSessionEntries: u64 & AugmentedConst<ApiType>;
    };
    miningSlot: {
      /**
       * How many blocks transpire between slots
       **/
      blocksBetweenSlots: u32 & AugmentedConst<ApiType>;
      /**
       * How many blocks buffer shall we use to stop accepting bids for the next period
       **/
      blocksBufferToStopAcceptingBids: u32 & AugmentedConst<ApiType>;
      /**
       * How many new miners can be in the cohort for each slot
       **/
      maxCohortSize: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of Miners that the pallet can hold.
       **/
      maxMiners: u32 & AugmentedConst<ApiType>;
      /**
       * The reduction in percent of ownership currency required to secure a slot
       **/
      ownershipPercentDamper: u32 & AugmentedConst<ApiType>;
      /**
       * How many session indexes to keep session history
       **/
      sessionIndicesToKeepInHistory: u32 & AugmentedConst<ApiType>;
    };
    notaries: {
      /**
       * The maximum active notaries allowed
       **/
      maxActiveNotaries: u32 & AugmentedConst<ApiType>;
      /**
       * Number of blocks to maintain key history for each notary
       * NOTE: only pruned when new keys are added
       **/
      maxBlocksForKeyHistory: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum hosts a notary can supply
       **/
      maxNotaryHosts: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum blocks a proposal can sit unapproved
       **/
      maxProposalHoldBlocks: u32 & AugmentedConst<ApiType>;
      maxProposalsPerBlock: u32 & AugmentedConst<ApiType>;
      /**
       * Number of blocks to delay changing a notaries' meta
       **/
      metaChangesBlockDelay: u32 & AugmentedConst<ApiType>;
    };
    system: {
      /**
       * Maximum number of block number to block hash mappings to keep (oldest pruned first).
       **/
      blockHashCount: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum length of a block (in bytes).
       **/
      blockLength: FrameSystemLimitsBlockLength & AugmentedConst<ApiType>;
      /**
       * Block & extrinsics weights: base values and limits.
       **/
      blockWeights: FrameSystemLimitsBlockWeights & AugmentedConst<ApiType>;
      /**
       * The weight of runtime database operations the runtime can invoke.
       **/
      dbWeight: SpWeightsRuntimeDbWeight & AugmentedConst<ApiType>;
      /**
       * The designated SS58 prefix of this chain.
       * 
       * This replaces the "ss58Format" property declared in the chain spec. Reason is
       * that the runtime should know about the prefix in order to make use of it as
       * an identifier of the chain.
       **/
      ss58Prefix: u16 & AugmentedConst<ApiType>;
      /**
       * Get the chain's current version.
       **/
      version: SpVersionRuntimeVersion & AugmentedConst<ApiType>;
    };
    timestamp: {
      /**
       * The minimum period between blocks.
       * 
       * Be aware that this is different to the *expected* period that the block production
       * apparatus provides. Your chosen consensus system will generally work with this to
       * determine a sensible block time. For example, in the Aura pallet it will be double this
       * period on default settings.
       **/
      minimumPeriod: u64 & AugmentedConst<ApiType>;
    };
    transactionPayment: {
      /**
       * A fee multiplier for `Operational` extrinsics to compute "virtual tip" to boost their
       * `priority`
       * 
       * This value is multiplied by the `final_fee` to obtain a "virtual tip" that is later
       * added to a tip component in regular `priority` calculations.
       * It means that a `Normal` transaction can front-run a similarly-sized `Operational`
       * extrinsic (with no tip), by including a tip value greater than the virtual tip.
       * 
       * ```rust,ignore
       * // For `Normal`
       * let priority = priority_calc(tip);
       * 
       * // For `Operational`
       * let virtual_tip = (inclusion_fee + tip) * OperationalFeeMultiplier;
       * let priority = priority_calc(tip + virtual_tip);
       * ```
       * 
       * Note that since we use `final_fee` the multiplier applies also to the regular `tip`
       * sent with the transaction. So, not only does the transaction get a priority bump based
       * on the `inclusion_fee`, but we also amplify the impact of tips applied to `Operational`
       * transactions.
       **/
      operationalFeeMultiplier: u8 & AugmentedConst<ApiType>;
    };
    txPause: {
      /**
       * Maximum length for pallet name and call name SCALE encoded string names.
       * 
       * TOO LONG NAMES WILL BE TREATED AS PAUSED.
       **/
      maxNameLen: u32 & AugmentedConst<ApiType>;
    };
    ulixeeBalances: {
      /**
       * The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!
       * 
       * If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for
       * this pallet. However, you do so at your own risk: this will open up a major DoS vector.
       * In case you have multiple sources of provider references, you may also get unexpected
       * behaviour if you set this to zero.
       * 
       * Bottom line: Do yourself a favour and make it at least one!
       **/
      existentialDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum number of individual freeze locks that can exist on an account at any time.
       **/
      maxFreezes: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of locks that should exist on an account.
       * Not strictly enforced, but used for weight estimation.
       **/
      maxLocks: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of named reserves that can exist on an account.
       **/
      maxReserves: u32 & AugmentedConst<ApiType>;
    };
  } // AugmentedConsts
} // declare module
