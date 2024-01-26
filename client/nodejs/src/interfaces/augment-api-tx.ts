// Auto-generated via `yarn polkadot-types-from-chain`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import '@polkadot/api-base/types/submittable';

import type { ApiTypes, AugmentedSubmittable, SubmittableExtrinsic, SubmittableExtrinsicFunction } from '@polkadot/api-base/types';
import type { Bytes, Compact, Option, Vec, bool, u128, u32, u64 } from '@polkadot/types-codec';
import type { AnyNumber, IMethod, ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, Call, MultiAddress } from '@polkadot/types/interfaces/runtime';
import type { SpConsensusGrandpaEquivocationProof, SpSessionMembershipProof, SpWeightsWeightV2Weight, UlxNodeRuntimeOpaqueSessionKeys, UlxPrimitivesBlockSealRewardDestination, UlxPrimitivesDataDomain, UlxPrimitivesDataDomainZoneRecord, UlxPrimitivesInherentsBlockSealInherent, UlxPrimitivesNotaryNotaryMeta, UlxPrimitivesNotebookSignedNotebookHeader } from '@polkadot/types/lookup';

export type __AugmentedSubmittable = AugmentedSubmittable<() => unknown>;
export type __SubmittableExtrinsic<ApiType extends ApiTypes> = SubmittableExtrinsic<ApiType>;
export type __SubmittableExtrinsicFunction<ApiType extends ApiTypes> = SubmittableExtrinsicFunction<ApiType>;

declare module '@polkadot/api-base/types/submittable' {
  interface AugmentedSubmittables<ApiType extends ApiTypes> {
    argonBalances: {
      /**
       * See [`Pallet::force_set_balance`].
       **/
      forceSetBalance: AugmentedSubmittable<(who: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, newFree: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::force_transfer`].
       **/
      forceTransfer: AugmentedSubmittable<(source: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::force_unreserve`].
       **/
      forceUnreserve: AugmentedSubmittable<(who: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, amount: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, u128]>;
      /**
       * See [`Pallet::transfer_all`].
       **/
      transferAll: AugmentedSubmittable<(dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, keepAlive: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, bool]>;
      /**
       * See [`Pallet::transfer_allow_death`].
       **/
      transferAllowDeath: AugmentedSubmittable<(dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::transfer_keep_alive`].
       **/
      transferKeepAlive: AugmentedSubmittable<(dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::upgrade_accounts`].
       **/
      upgradeAccounts: AugmentedSubmittable<(who: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<AccountId32>]>;
    };
    blockRewards: {
    };
    blockSeal: {
      /**
       * See `Pallet::apply`.
       **/
      apply: AugmentedSubmittable<(seal: UlxPrimitivesInherentsBlockSealInherent | { Vote: any } | { Compute: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [UlxPrimitivesInherentsBlockSealInherent]>;
    };
    blockSealSpec: {
      /**
       * See `Pallet::configure`.
       **/
      configure: AugmentedSubmittable<(voteMinimum: Option<u128> | null | Uint8Array | u128 | AnyNumber, computeDifficulty: Option<u128> | null | Uint8Array | u128 | AnyNumber) => SubmittableExtrinsic<ApiType>, [Option<u128>, Option<u128>]>;
    };
    bond: {
      /**
       * See `Pallet::bond_self`.
       **/
      bondSelf: AugmentedSubmittable<(amount: u128 | AnyNumber | Uint8Array, bondUntilBlock: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u128, u32]>;
      /**
       * See `Pallet::end_fund`.
       **/
      endFund: AugmentedSubmittable<(bondFundId: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32]>;
      /**
       * See `Pallet::extend_bond`.
       **/
      extendBond: AugmentedSubmittable<(bondId: u64 | AnyNumber | Uint8Array, totalAmount: u128 | AnyNumber | Uint8Array, bondUntilBlock: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64, u128, u32]>;
      /**
       * See `Pallet::extend_fund`.
       **/
      extendFund: AugmentedSubmittable<(bondFundId: u32 | AnyNumber | Uint8Array, totalAmountOffered: u128 | AnyNumber | Uint8Array, expirationBlock: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u128, u32]>;
      /**
       * See `Pallet::lease`.
       **/
      lease: AugmentedSubmittable<(bondFundId: u32 | AnyNumber | Uint8Array, amount: u128 | AnyNumber | Uint8Array, leaseUntilBlock: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u128, u32]>;
      /**
       * See `Pallet::offer_fund`.
       **/
      offerFund: AugmentedSubmittable<(leaseAnnualPercentRate: Compact<u32> | AnyNumber | Uint8Array, leaseBaseFee: Compact<u128> | AnyNumber | Uint8Array, amountOffered: Compact<u128> | AnyNumber | Uint8Array, expirationBlock: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [Compact<u32>, Compact<u128>, Compact<u128>, u32]>;
      /**
       * See `Pallet::return_bond`.
       **/
      returnBond: AugmentedSubmittable<(bondId: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64]>;
    };
    chainTransfer: {
      /**
       * See `Pallet::send_to_localchain`.
       **/
      sendToLocalchain: AugmentedSubmittable<(amount: Compact<u128> | AnyNumber | Uint8Array, notaryId: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [Compact<u128>, u32]>;
    };
    dataDomain: {
      /**
       * See `Pallet::set_zone_record`.
       **/
      setZoneRecord: AugmentedSubmittable<(domain: UlxPrimitivesDataDomain | { domainName?: any; topLevelDomain?: any } | string | Uint8Array, zoneRecord: UlxPrimitivesDataDomainZoneRecord | { paymentAccount?: any; notaryId?: any; versions?: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [UlxPrimitivesDataDomain, UlxPrimitivesDataDomainZoneRecord]>;
    };
    grandpa: {
      /**
       * See [`Pallet::note_stalled`].
       **/
      noteStalled: AugmentedSubmittable<(delay: u32 | AnyNumber | Uint8Array, bestFinalizedBlockNumber: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
      /**
       * See [`Pallet::report_equivocation`].
       **/
      reportEquivocation: AugmentedSubmittable<(equivocationProof: SpConsensusGrandpaEquivocationProof | { setId?: any; equivocation?: any } | string | Uint8Array, keyOwnerProof: SpSessionMembershipProof | { session?: any; trieNodes?: any; validatorCount?: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [SpConsensusGrandpaEquivocationProof, SpSessionMembershipProof]>;
      /**
       * See [`Pallet::report_equivocation_unsigned`].
       **/
      reportEquivocationUnsigned: AugmentedSubmittable<(equivocationProof: SpConsensusGrandpaEquivocationProof | { setId?: any; equivocation?: any } | string | Uint8Array, keyOwnerProof: SpSessionMembershipProof | { session?: any; trieNodes?: any; validatorCount?: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [SpConsensusGrandpaEquivocationProof, SpSessionMembershipProof]>;
    };
    miningSlot: {
      /**
       * See `Pallet::bid`.
       **/
      bid: AugmentedSubmittable<(bondId: Option<u64> | null | Uint8Array | u64 | AnyNumber, rewardDestination: UlxPrimitivesBlockSealRewardDestination | { Owner: any } | { Account: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Option<u64>, UlxPrimitivesBlockSealRewardDestination]>;
    };
    mint: {
    };
    notaries: {
      /**
       * See `Pallet::activate`.
       **/
      activate: AugmentedSubmittable<(operatorAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [AccountId32]>;
      /**
       * See `Pallet::propose`.
       **/
      propose: AugmentedSubmittable<(meta: UlxPrimitivesNotaryNotaryMeta | { public?: any; hosts?: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [UlxPrimitivesNotaryNotaryMeta]>;
      /**
       * See `Pallet::update`.
       **/
      update: AugmentedSubmittable<(notaryId: Compact<u32> | AnyNumber | Uint8Array, meta: UlxPrimitivesNotaryNotaryMeta | { public?: any; hosts?: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Compact<u32>, UlxPrimitivesNotaryNotaryMeta]>;
    };
    notebook: {
      /**
       * See `Pallet::submit`.
       **/
      submit: AugmentedSubmittable<(notebooks: Vec<UlxPrimitivesNotebookSignedNotebookHeader> | (UlxPrimitivesNotebookSignedNotebookHeader | { header?: any; signature?: any } | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<UlxPrimitivesNotebookSignedNotebookHeader>]>;
    };
    session: {
      /**
       * See [`Pallet::purge_keys`].
       **/
      purgeKeys: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
      /**
       * See [`Pallet::set_keys`].
       **/
      setKeys: AugmentedSubmittable<(keys: UlxNodeRuntimeOpaqueSessionKeys | { grandpa?: any; blockSealAuthority?: any } | string | Uint8Array, proof: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [UlxNodeRuntimeOpaqueSessionKeys, Bytes]>;
    };
    sudo: {
      /**
       * See [`Pallet::remove_key`].
       **/
      removeKey: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
      /**
       * See [`Pallet::set_key`].
       **/
      setKey: AugmentedSubmittable<(updated: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress]>;
      /**
       * See [`Pallet::sudo`].
       **/
      sudo: AugmentedSubmittable<(call: Call | IMethod | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Call]>;
      /**
       * See [`Pallet::sudo_as`].
       **/
      sudoAs: AugmentedSubmittable<(who: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, call: Call | IMethod | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Call]>;
      /**
       * See [`Pallet::sudo_unchecked_weight`].
       **/
      sudoUncheckedWeight: AugmentedSubmittable<(call: Call | IMethod | string | Uint8Array, weight: SpWeightsWeightV2Weight | { refTime?: any; proofSize?: any } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Call, SpWeightsWeightV2Weight]>;
    };
    system: {
      /**
       * See [`Pallet::kill_prefix`].
       **/
      killPrefix: AugmentedSubmittable<(prefix: Bytes | string | Uint8Array, subkeys: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes, u32]>;
      /**
       * See [`Pallet::kill_storage`].
       **/
      killStorage: AugmentedSubmittable<(keys: Vec<Bytes> | (Bytes | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<Bytes>]>;
      /**
       * See [`Pallet::remark`].
       **/
      remark: AugmentedSubmittable<(remark: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
      /**
       * See [`Pallet::remark_with_event`].
       **/
      remarkWithEvent: AugmentedSubmittable<(remark: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
      /**
       * See [`Pallet::set_code`].
       **/
      setCode: AugmentedSubmittable<(code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
      /**
       * See [`Pallet::set_code_without_checks`].
       **/
      setCodeWithoutChecks: AugmentedSubmittable<(code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
      /**
       * See [`Pallet::set_heap_pages`].
       **/
      setHeapPages: AugmentedSubmittable<(pages: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64]>;
      /**
       * See [`Pallet::set_storage`].
       **/
      setStorage: AugmentedSubmittable<(items: Vec<ITuple<[Bytes, Bytes]>> | ([Bytes | string | Uint8Array, Bytes | string | Uint8Array])[]) => SubmittableExtrinsic<ApiType>, [Vec<ITuple<[Bytes, Bytes]>>]>;
    };
    ticks: {
    };
    timestamp: {
      /**
       * See [`Pallet::set`].
       **/
      set: AugmentedSubmittable<(now: Compact<u64> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [Compact<u64>]>;
    };
    txPause: {
      /**
       * See [`Pallet::pause`].
       **/
      pause: AugmentedSubmittable<(fullName: ITuple<[Bytes, Bytes]> | [Bytes | string | Uint8Array, Bytes | string | Uint8Array]) => SubmittableExtrinsic<ApiType>, [ITuple<[Bytes, Bytes]>]>;
      /**
       * See [`Pallet::unpause`].
       **/
      unpause: AugmentedSubmittable<(ident: ITuple<[Bytes, Bytes]> | [Bytes | string | Uint8Array, Bytes | string | Uint8Array]) => SubmittableExtrinsic<ApiType>, [ITuple<[Bytes, Bytes]>]>;
    };
    ulixeeBalances: {
      /**
       * See [`Pallet::force_set_balance`].
       **/
      forceSetBalance: AugmentedSubmittable<(who: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, newFree: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::force_transfer`].
       **/
      forceTransfer: AugmentedSubmittable<(source: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::force_unreserve`].
       **/
      forceUnreserve: AugmentedSubmittable<(who: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, amount: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, u128]>;
      /**
       * See [`Pallet::transfer_all`].
       **/
      transferAll: AugmentedSubmittable<(dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, keepAlive: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, bool]>;
      /**
       * See [`Pallet::transfer_allow_death`].
       **/
      transferAllowDeath: AugmentedSubmittable<(dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::transfer_keep_alive`].
       **/
      transferKeepAlive: AugmentedSubmittable<(dest: MultiAddress | { Id: any } | { Index: any } | { Raw: any } | { Address32: any } | { Address20: any } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
      /**
       * See [`Pallet::upgrade_accounts`].
       **/
      upgradeAccounts: AugmentedSubmittable<(who: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<AccountId32>]>;
    };
  } // AugmentedSubmittables
} // declare module
