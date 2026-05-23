# `pallet_crosschain_transfer` TODO

This file is the checked-in follow-on list for the live crosschain transfer pallet and adjacent
gateway contract behavior. Remove items as they are actually completed.

Already implemented and intentionally omitted from the open list:

- activation relay reimbursement is held at authority registration time, then settled on
  `MintingAuthorityActivated` proof-back
- self-relayed activation releases that hold back to the operator directly
- third-party relayed activation pays the realized shared-tranche amount to the relayer and refunds
  any held excess
- deactivation now only relies on the normal `PendingActivation` / `Active` / `Deactivating` state
  machine rather than on a separate post-proof activation-completion step

## 1. Add steady-state council rotation queueing

- materialize the next council from eligible registered vaults at frame turn
- queue a real `GlobalIssuanceCouncil(...)` approval target instead of relying on
  `force_set_global_issuance_council(...)` for normal operation
- make the steady-state path work without root recovery seams

## 2. Revisit outbound cancel semantics

- decide whether to remove the public Ethereum-side `cancelTransferOutOfArgon(...)` entrypoint
  entirely
- decide whether expired outbound transfers should auto-cancel locally once the proven
  gateway-activity frontier has moved past `valid_until_ethereum_block`
- define the exact frontier required so Argon only auto-cancels after a valid pre-expiry
  finalization has become impossible

## 3. Add transfer-out prioritization and queue management

- revisit the current `MaxPendingTransferOutsPerDestinationChain` queue model so ready transfers do
  not swamp the scarce open-collateralization slots
- if transfer-outs become priority-queued by user-paid tip, allow a higher-tip still-unfunded
  request to evict the current lowest-ranked still-unfunded request and refund that displaced
  request locally. This might need the cancel semantics to be reworked to argon-side driven.

## 3. Handle more than one council rotation during a live transfer

- transfer-out requests currently lock in a single `council_hash`
- normal steady-state cadence is already close to safe because transfer lifetime and council
  rotation cadence are both about 10 days
- the real open question is the exception path: force / accelerated council replacement can move
  past a still-live request's `council_hash`
- decide whether that emergency path is allowed to strand live transfer-outs until expiry / cancel
- if not, add only a small bounded fallback for older `councilHash -> microgonsPerArgonot` windows
  instead of retaining full historical council snapshots indefinitely

## 4. Require multiple authority signatures for transfer finalization

- today one active authority signing key can authorize its own collateral slice directly
- add a follow-on control that requires multiple signatures per transfer or per finalized authority
  set
- goal: reduce direct minting risk if one authority key is stolen

## 5. Add issuance caps

- add per-`MintingAuthority` outstanding issuance caps for both `Argon` and `Argonot`
- add per-destination-chain mint caps for each asset
- add per-destination-chain epoch caps for each asset
- add per-destination-chain rolling daily circulation-growth caps for each asset
- enforce the same cap table on both Ethereum finalization and Argon-side approval / hydration

## 6. Add automatic pause triggers

- add pause thresholds for circulation jumps beyond configured per-activity, per-epoch, or per-day
  growth bounds
- add pause thresholds for authority-backing loss beyond configured per-authority or global bounds
- add pause thresholds for repeated proven finalizations with no valid local request match
- keep cancellation, disable, and recovery paths available while paused

## 7. Add large transfer tiers

- keep the normal path for transfers below a chain-configured large transfer threshold
- once a transfer crosses that threshold, require several distinct active `MintingAuthority`
  authorizations rather than letting one authority satisfy the whole transfer alone
- above a higher threshold, require an additional delay or explicit emergency operator enable before
  the transfer becomes mint-eligible on Ethereum

## 8. Add independent monitoring

- run at least one monitor that is not the same process as the relayer or authority operator
- continuously compare proven `gatewayActivityNonce` / `argonApprovalsNonce` progress
- compare local transfer terminal state and hydrated inbound results against proven gateway activity
- compare local authority remaining collateral against proven mint consumption
- compare proven circulation checkpoints and authority disable / burn / backing-loss events against
  local authority state
- allow the monitor to alarm and trip the same pause surface when it sees material mismatches or
  abnormal growth

## 9. Add authority disable / suspend and replacement authorization flow

- add a fast `Disabled` or `Suspended` state so a suspicious authority can be stopped immediately on
  both Argon and Ethereum
- block new transfer authorizations from a disabled authority without blocking proof-back, punitive
  burns, or close-out of already-terminal transfers
- require any non-terminal transfer that still depends on a disabled authority to gather replacement
  authorizations from still-active authorities or else finish by cancellation
- add hard per-authority issuance caps and rolling daily-growth caps alongside the remaining
  collateral checks

## 10. Add a mint-release timelock

- instead of making proven outbound finalizations immediately spendable on Ethereum, queue the
  minted funds into a pending-release state for about one day
- keep queued releases cancellable / freezable while paused so the council or guardian can respond
  if a critical bug, stolen key, or other minting hole is discovered before release
- decide whether the delay applies to every finalized transfer or only to higher-risk tiers, but
  keep the control explicit instead of relying only on manual monitoring
