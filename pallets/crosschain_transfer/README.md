# `pallet_crosschain_transfer`

This pallet is the Argon-side control plane for external gateways.

The shortest useful mental model is:

- the gateway is where canonical cross-chain activity happens
- Argon is where we queue shared updates, track local obligations, hold or release funds, and prove
  gateway activity back home

This README is meant to answer two practical questions:

- what are the operating rules?
- which storage items and events should I watch so I do not break those rules?

## Terms

### Gateway

A gateway is the destination-side contract system Argon coordinates with.

In this branch, Ethereum's `MintingGateway` is the first live gateway, but the pallet is written in
gateway terms, not Ethereum-only terms.

### `GlobalIssuanceCouncil`

The council that approves shared gateway updates.

It does not sign user transfer-outs. It signs queue entries such as:

- council rotation
- minting-authority activation
- minting-authority deactivation

### `MintingAuthority`

A minting authority is a destination-chain operator that:

- puts up backing
- owns a destination-chain signing key
- authorizes outbound transfers with that key
- can later deactivate and recover unused backing

### Queue entry

A queue entry is one council-approved gateway update Argon wants applied remotely.

It has a nonce, an approving council, a target, and a `due_frame_id`.

In the current runtime config, new queue entries are given a due window of 10 frames. In code that
is `current_frame + CouncilRotationFrames`.

### Transfer-out

A transfer-out is a user request to move `Argon` or `Argonot` from Argon to a gateway.

It starts on Argon, collects minting-authority backing, becomes `Ready` once fully covered, and ends
only when the gateway result is proven back.

### Proof-back

Proof-back is `prove_gateway_activity(...)`.

It replays ordered gateway activity back onto Argon so local state can follow what the gateway has
already done.

## The Two Flows

This pallet really has two live flows layered together.

### Shared gateway upkeep

The council queue is how Argon tells a gateway about shared state changes:

- council rotation
- minting-authority activation
- minting-authority deactivation

Those updates are queued on Argon, approved by the `GlobalIssuanceCouncil`, relayed to the gateway,
and only considered final on Argon once the matching gateway activity is proven back.

### User transfer-out

A transfer-out moves value out of Argon and onto a gateway.

The flow is:

1. A user opens `transfer_out(...)`.
2. The principal is burned to the Argon-side burn account and the minting authority tip is held.
3. Active minting authorities attach signed collateral rows.
4. Once attached collateral covers the amount, the transfer becomes `Ready`.
5. Anyone can finalize it on the destination chain gateway.
6. Proof-back reconciles the result on Argon: release minting authority collateral, settle backing
   consumption, and pay the minting authority tip.

## Core Rules

### 1. Proven gateway activity is canonical

Once gateway activity is successfully proven to the Argon chain, Argon follows it.

That means:

- queue progress follows the proven gateway nonces
- minting-authority activation and deactivation finalize from proof-back
- transfer finalization or cancellation settles from proof-back
- whatever comes from the destination chain is considered canonical, even if it does not match local
  expectations

Every gateway activity must get the system back in balance: argon and argonot circulation must match
pending requests on Argon side (where funds are burned) and on destination chain (where funds are
minted). This equation means in-flight requests are not yet part of the gateway circulation.

If local cleanup cannot decipher a proven gateway action, the pallet pauses instead of pretending
state is still balanced.

### 2. The council queue is synced in order

Council queue entries are the shared updates Argon wants the gateway to apply. They are processed in
nonce order and linked via hashes.

Important rules:

- council signers sign queue entries, not user transfers
- being in the active council means you have a signing obligation on the queue entries approved by
  that council
- if you do not sign the next owed queue item before its due frame, your vault stops being able to
  `collect` earnings
- today that due window is 10 frames from when the queue item is created
- `collect` blocking comes from the next overdue queue item a signer still owes
- deactivation queue entries are skipped when deciding the next fresh collect-blocking obligation
- a queue entry missing its `due_frame_id` is late, not expired

### 3. Minting-authority activation is paid for up front and settled on proof-back

Registering a minting authority does three things:

- records the requested backing and destination signing key
- queues an activation for council approval
- holds an Argon-side activation reimbursement quote

The authority is not usable until `MintingAuthorityActivated` is proven back.

When that proof arrives:

- if the operator relayed their own activation, the held amount is released back to them
- if someone else relayed it, the relayer is paid the realized amount and any excess hold is
  refunded to the operator

The realized split comes from the gateway event's activation cohort, not from Argon guessing later.

### 4. Transfer-out funds move immediately, but rewards pay only after proof-back

When a user opens `transfer_out(...)`:

- the principal is burned into the source-chain burn account immediately
- the minting authority tip is held immediately

While the request is open:

- each minting authority gets at most one collateral row per transfer
- attached collateral becomes locally reserved
- the transfer becomes `Ready` only once attached collateral covers the amount

When finalization is later proven:

- attached reservations are released
- gateway collateral consumption is reconciled
- the minting authority tip is split by finalized `collateral_share`

For `Argon` transfers, the pallet expects an authority to exhaust available microgons before leaning
on micronot-backed value.

### 5. Unknown finalization is accepted as canonical consumption

If the gateway proves a `TransferOutOfArgonFinalized` that Argon does not have locally:

- the participating minting-authority backing is still burned locally
- the same principal amount is minted into Argon's burn account
- no local user transfer receives a minting authority tip

That keeps Argon's source-side circulation aligned with the fact that the gateway already minted the
asset.

## Storage Worth Watching

These are the main storage items that tell you whether the pallet is healthy.

### Gateway sync and pause state

- `GatewayStateBySourceChain`
  - last proven gateway nonces and circulation snapshot for each source chain
- `GatewaySyncPauseBySourceChain`
  - if this is set, the pallet saw a proven-gateway condition it would not continue through

### Council and queue state

- `ActiveGlobalIssuanceCouncilByDestinationChain`
  - the currently active council Argon expects for that destination chain
- `GlobalIssuanceCouncilByHash`
  - historical council snapshots still referenced by queue entries or transfers
- `NextCouncilApprovalQueueNonceByDestinationChain`
  - the latest queued approval nonce on Argon
- `CouncilApprovalQueueByDestinationChainAndNonce`
  - the actual ordered queue entries
- `CouncilApprovalCursorByDestinationChainAndAccountId`
  - the last nonce each council signer has signed
- `CouncilSignerByDestinationChainAndAccountId`
  - active registered council signer proofs
- `PendingCouncilSignerByDestinationChainAndAccountId`
  - council signer registrations waiting for the next council installation

If you are debugging queue or collect behavior, these are the first storages to inspect.

### Minting-authority state

- `MintingAuthoritiesBySigner`
  - the main authority record: state, remaining collateral, pending reservations, activation and
    deactivation queue nonces
- `MinimumMintingAuthorityValueByDestinationChain`
  - minimum required total authority value for that destination chain
- `MintingAuthorityActivationRepaymentPricingByDestinationChain`
  - the pricing inputs used to quote activation repayment holds

### Transfer-out state

- `TransferOutById`
  - the canonical local transfer record, including attached rows and current state
- `PendingCollateralizationRequestsByChain`
  - transfer-outs still looking for more backing
- `PendingTransferOutCirculationByDestinationChain`
  - local transfer principal currently parked in the burn account but not yet terminal
- `NonTerminalTransferOutCountByDestinationChain`
  - count of still-open transfer-outs
- `NextTransferOutNonceBySendingAccountId`
  - per-account transfer nonce source

If something looks off in transfer readiness, collateral reservations, or circulation, these are the
storages to compare first.

## Events Worth Watching

The most useful events are the ones that announce new work, terminal state, or a safety stop.

### New work or opportunity

- `QueueEntryApprovalReady`
  - a queue entry has enough council approval and is ready to relay
- `TransferOutReady`
  - a transfer-out is fully collateralized and can be finalized on the gateway
- `TransferCollateralized`
  - a minting authority attached a new row to a transfer
- `MintingAuthorityRegistered`
  - a new activation entered the queue

### Terminal state

- `MintingAuthorityActivationFinalized`
- `MintingAuthorityDeactivationFinalized`
- `TransferOutFinalized`
- `TransferOutCanceled`

These tell you the gateway result has made it all the way back to Argon.

### Safety or drift signals

- `GatewaySyncPaused`
  - the pallet refused to continue applying proven gateway activity
- `TransferCollateralInvalidated`
  - a previously attached reservation had to be removed during reconciliation

## Current Implementation Notes

The pallet is written in gateway terms, but this branch only has one live gateway path today:
Ethereum.

That is why a few surfaces still name Ethereum directly:

- the only live `SourceChain` path here is `Ethereum`
- the `collect` blocker currently looks only at the Ethereum queue, because that is the only live
  queue path in this branch
- proof-back in this branch currently comes from the Ethereum verifier
