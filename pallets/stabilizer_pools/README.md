# Stabilizer pool checkpoints

Users retain their Uniswap v4 NFTs and subscribe them to `StabilizationPoolSubscriber`, passing
`abi.encode(argonAccountId)`. The subscriber stores only that beneficiary and a permanent revision.
Enrollment, nonzero liquidity changes, and burn increment it. Unsubscribe notification is best
effort, so it makes no write: every checkpoint separately proves PositionManager's canonical
`subscriber(tokenId)` address. Re-enrollment increments the revision even if the beneficiary and
liquidity return to their earlier values.

There is no event relay, indexer requirement, global activity cursor, or ongoing position sync.

## Calls

- `register_pool(pool_manager, config)` is root-only. It pins the canonical pool key,
  PositionManager, and subscriber. Existing configurations cannot be replaced.
- `checkpoint_position(pool_manager, pool_id, token_id, proof)` is signed by the proven Argon
  beneficiary. Its first proof registers a baseline. An unchanged revision extends the verified
  maintained interval and can submit newly proved full frames within the last ten closed frames
  to the claim provider. An increased revision starts a new baseline and does not verify the gap.
  Older proofs, repeated proofs, regressed revisions, and changed position fields without a revision
  increase are rejected. The position watermark survives beneficiary changes.
- `report_position_change(position, proof)` lets anyone prove that the enrolled revision increased
  or the NFT left its registered subscription. It removes the position from future rewards.
  A report during the next frame also removes it from the closed frame's total if its owner
  had yet to prove continuity through that frame.

Each checkpoint contains actual account/storage witnesses for the subscriber (beneficiary and
revision), PositionManager (subscription and packed position metadata), and PoolManager (NFT
liquidity). All witnesses must use the identical execution-block proof. The existing Ethereum
verifier authenticates the state root and header ancestry; block number, hash, and timestamp come
from that verified header. No caller-provided timestamp or `eth_call` result is trusted. Native v4
PoolIds are scoped by PoolManager; NFT checkpoints are scoped by PositionManager and token ID.

Prefer a recently retained Ethereum anchor to avoid header ancestry and old-state RPC limits. A
short ancestor witness also works. Rewards would end at the proven block, not submission time; there
is no one-block freshness rule. Before changing liquidity, checkpoint the old revision if its
unproven interval should be preserved. After the change, prove the new baseline.

## Supported deployments

Storage paths are pinned to subscriber layout v1, the purchase integration's `@uniswap/v4-periphery`
revision `545a5d2a87228167edde48f3b9eda122d1e3c4d6` PositionManager, and `@uniswap/v4-core` 1.0.2
PoolManager. The deployment approval must verify these contract implementations, their immutable
PositionManager/PoolManager bindings, and the configured pool key. A valid trie witness alone cannot
establish the semantics of arbitrary contract bytecode. The Solidity integration tests compile that
PositionManager revision and check the offsets against its deployed storage and the published
PoolManager bytecode.

## Scope

Each registration contributes to the total liquidity for its exact Uniswap range starting in the
next mining frame. A proof of a new revision removes the old position from the current total and
schedules its new liquidity for the next frame. Existing checkpoints enter the totals on their next
accepted proof.

At frame close, the pallet saves the total liquidity in each qualifying range using that frame's
sampled price. Reports can correct those totals during the next frame. After review, the pallet
converts the corrected totals into ARGON and USDC principal using the saved price. A frame without a
usable price has zero eligible principal.

Each frame records the Unix millisecond timestamp of its opening block. The next frame's opening
also closes the previous frame. These timestamps are recorded even when price sampling falls short.

The claim provider receives the proven beneficiary and eligible ARGON and USDC principal
for each newly proved frame. On a replacement or removal, any claim from the previous checkpoint
goes to its original beneficiary. The claim cursor advances only after the provider accepts the
eligible frames. Claim submission starts when the runtime connects the settlement provider.
