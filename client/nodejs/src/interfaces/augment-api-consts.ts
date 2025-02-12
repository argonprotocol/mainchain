// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/consts';

import type { ApiTypes, AugmentedConst } from '@polkadot/api-base/types';
import type { bool, u128, u16, u32, u64, u8 } from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { Percent } from '@polkadot/types/interfaces/runtime';
import type { FrameSupportPalletId, FrameSystemLimitsBlockLength, FrameSystemLimitsBlockWeights, SpVersionRuntimeVersion, SpWeightsRuntimeDbWeight } from '@polkadot/types/lookup';

export type __AugmentedConst<ApiType extends ApiTypes> = AugmentedConst<ApiType>;

declare module '@polkadot/api-base/types/consts' {
  interface AugmentedConsts<ApiType extends ApiTypes> {
    balances: {
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
       *
       * Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      maxLocks: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of named reserves that can exist on an account.
       *
       * Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      maxReserves: u32 & AugmentedConst<ApiType>;
    };
    bitcoinLocks: {
      /**
       * Argon blocks per day
       **/
      argonTicksPerDay: u64 & AugmentedConst<ApiType>;
      /**
       * The number of bitcoin blocks a bitcoin is locked for
       **/
      lockDurationBlocks: u64 & AugmentedConst<ApiType>;
      /**
       * The bitcoin blocks after an obligation expires which the vault will be allowed to claim
       * a bitcoin
       **/
      lockReclamationBlocks: u64 & AugmentedConst<ApiType>;
      /**
       * Number of bitcoin blocks a vault has to counter-sign a bitcoin release
       **/
      lockReleaseCosignDeadlineBlocks: u64 & AugmentedConst<ApiType>;
      /**
       * Maximum releasing utxos at a time
       **/
      maxConcurrentlyReleasingLocks: u32 & AugmentedConst<ApiType>;
    };
    bitcoinUtxos: {
      /**
       * Maximum bitcoin blocks to watch a Utxo for confirmation before canceling
       **/
      maxPendingConfirmationBlocks: u64 & AugmentedConst<ApiType>;
      /**
       * The maximum number of UTXOs that can be tracked in a block and/or expiring at same block
       **/
      maxPendingConfirmationUtxos: u32 & AugmentedConst<ApiType>;
    };
    blockRewards: {
      /**
       * The tick number at which the halving begins for ownership tokens
       **/
      halvingBeginTick: u64 & AugmentedConst<ApiType>;
      /**
       * Number of ticks for halving of ownership share rewards
       **/
      halvingTicks: u64 & AugmentedConst<ApiType>;
      /**
       * The growth path for both ownership and argons before halving
       **/
      incrementalGrowth: ITuple<[u128, u64, u128]> & AugmentedConst<ApiType>;
      /**
       * Blocks until a block reward is mature
       **/
      maturationBlocks: u32 & AugmentedConst<ApiType>;
      /**
       * Percent as a number out of 100 of the block reward that goes to the miner.
       **/
      minerPayoutPercent: u128 & AugmentedConst<ApiType>;
      /**
       * Number of argons minted per block
       **/
      startingArgonsPerBlock: u128 & AugmentedConst<ApiType>;
      /**
       * Number of ownership tokens minted per block
       **/
      startingOwnershipTokensPerBlock: u128 & AugmentedConst<ApiType>;
    };
    blockSealSpec: {
      /**
       * The number of historical compute times to use to calculate the rolling compute average
       * (for adjustment)
       **/
      historicalComputeBlocksForAverage: u32 & AugmentedConst<ApiType>;
      /**
       * The number of historical vote blocks to use to calculate the rolling vote average
       **/
      historicalVoteBlocksForAverage: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum active notaries allowed
       **/
      maxActiveNotaries: u32 & AugmentedConst<ApiType>;
      /**
       * The desired votes per block
       **/
      targetBlockVotes: u128 & AugmentedConst<ApiType>;
    };
    chainTransfer: {
      /**
       * How many transfers out can be queued per block
       **/
      maxPendingTransfersOutPerBlock: u32 & AugmentedConst<ApiType>;
      palletId: FrameSupportPalletId & AugmentedConst<ApiType>;
      /**
       * How long a transfer should remain in storage before returning. NOTE: there is a 2 tick
       * grace period where we will still allow a transfer
       **/
      transferExpirationTicks: u64 & AugmentedConst<ApiType>;
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
       * The max percent swing for the argonots per slot (from the last percent
       **/
      argonotsPercentAdjustmentDamper: u128 & AugmentedConst<ApiType>;
      /**
       * How many new miners can be in the cohort for each slot
       **/
      maxCohortSize: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum percent of argonots in the network that should be required for
       * mining seats
       **/
      maximumArgonotProrataPercent: Percent & AugmentedConst<ApiType>;
      /**
       * The maximum number of Miners that the pallet can hold.
       **/
      maxMiners: u32 & AugmentedConst<ApiType>;
      /**
       * The minimum argonots needed per seat
       **/
      minimumArgonotsPerSeat: u128 & AugmentedConst<ApiType>;
      /**
       * The target number of bids per slot. This will adjust the argonots per seat up or
       * down to ensure mining slots are filled.
       **/
      targetBidsPerSlot: u32 & AugmentedConst<ApiType>;
    };
    mint: {
      /**
       * The maximum number of UTXOs that can be waiting for minting
       **/
      maxPendingMintUtxos: u32 & AugmentedConst<ApiType>;
    };
    multisig: {
      /**
       * The base amount of currency needed to reserve for creating a multisig execution or to
       * store a dispatch call for later.
       *
       * This is held for an additional storage item whose value size is
       * `4 + sizeof((BlockNumber, Balance, AccountId))` bytes and whose key size is
       * `32 + sizeof(AccountId)` bytes.
       **/
      depositBase: u128 & AugmentedConst<ApiType>;
      /**
       * The amount of currency needed per unit threshold when creating a multisig execution.
       *
       * This is held for adding 32 bytes more into a pre-existing storage value.
       **/
      depositFactor: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of signatories allowed in the multisig.
       **/
      maxSignatories: u32 & AugmentedConst<ApiType>;
    };
    notaries: {
      /**
       * The maximum active notaries allowed
       **/
      maxActiveNotaries: u32 & AugmentedConst<ApiType>;
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
       * Number of ticks to maintain key history for each notary
       * NOTE: only pruned when new keys are added
       **/
      maxTicksForKeyHistory: u32 & AugmentedConst<ApiType>;
      /**
       * Number of ticks to delay changing a notaries' meta (this is to allow a window for
       * notaries to switch to new keys after a new key is finalized)
       **/
      metaChangesTickDelay: u64 & AugmentedConst<ApiType>;
    };
    ownership: {
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
       *
       * Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      maxLocks: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of named reserves that can exist on an account.
       *
       * Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      maxReserves: u32 & AugmentedConst<ApiType>;
    };
    priceIndex: {
      /**
       * The max price difference dropping below target or raising above target per tick. There's
       * no corresponding constant for time to recovery to target
       **/
      maxArgonChangePerTickAwayFromTarget: u128 & AugmentedConst<ApiType>;
      maxArgonTargetChangePerTick: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum number of ticks to preserve a price index
       **/
      maxDowntimeTicksBeforeReset: u64 & AugmentedConst<ApiType>;
      /**
       * The oldest history to keep
       **/
      maxPriceAgeInTicks: u64 & AugmentedConst<ApiType>;
    };
    proxy: {
      /**
       * The base amount of currency needed to reserve for creating an announcement.
       *
       * This is held when a new storage item holding a `Balance` is created (typically 16
       * bytes).
       **/
      announcementDepositBase: u128 & AugmentedConst<ApiType>;
      /**
       * The amount of currency needed per announcement made.
       *
       * This is held for adding an `AccountId`, `Hash` and `BlockNumber` (typically 68 bytes)
       * into a pre-existing storage value.
       **/
      announcementDepositFactor: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of time-delayed announcements that are allowed to be pending.
       **/
      maxPending: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of proxies allowed for a single account.
       **/
      maxProxies: u32 & AugmentedConst<ApiType>;
      /**
       * The base amount of currency needed to reserve for creating a proxy.
       *
       * This is held for an additional storage item whose value size is
       * `sizeof(Balance)` bytes and whose key size is `sizeof(AccountId)` bytes.
       **/
      proxyDepositBase: u128 & AugmentedConst<ApiType>;
      /**
       * The amount of currency needed per proxy added.
       *
       * This is held for adding 32 bytes plus an instance of `ProxyType` more into a
       * pre-existing storage value. Thus, when configuring `ProxyDepositFactor` one should take
       * into account `32 + proxy_type.encode().len()` bytes of data.
       **/
      proxyDepositFactor: u128 & AugmentedConst<ApiType>;
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
       * Get the chain's in-code version.
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
    tokenGateway: {
      /**
       * The decimals of the native currency
       **/
      decimals: u8 & AugmentedConst<ApiType>;
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
    utility: {
      /**
       * The limit on the number of batched calls.
       **/
      batchedCallsLimit: u32 & AugmentedConst<ApiType>;
    };
    vaults: {
      /**
       * Is reward sharing enabled
       **/
      enableRewardSharing: bool & AugmentedConst<ApiType>;
      /**
       * Pallet storage requires bounds, so we have to set a maximum number that can expire in a
       * single block
       **/
      maxConcurrentlyExpiringObligations: u32 & AugmentedConst<ApiType>;
      /**
       * The max pending vault term changes per block
       **/
      maxPendingTermModificationsPerTick: u32 & AugmentedConst<ApiType>;
      /**
       * Minimum amount for an obligation
       **/
      minimumObligationAmount: u128 & AugmentedConst<ApiType>;
      /**
       * The number of ticks that a funding change will be delayed before it takes effect
       **/
      miningArgonIncreaseTickDelay: u64 & AugmentedConst<ApiType>;
      /**
       * The number of ticks that a change in terms will take before applying. Terms only apply
       * on a slot changeover, so this setting is the minimum blocks that must pass, in
       * addition to the time to the next slot after that
       **/
      minTermsModificationTickDelay: u64 & AugmentedConst<ApiType>;
      /**
       * Argon blocks per day
       **/
      ticksPerDay: u64 & AugmentedConst<ApiType>;
    };
  } // AugmentedConsts
} // declare module
