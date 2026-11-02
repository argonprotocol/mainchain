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
  maintained interval; an increased revision starts a new baseline and does not verify the gap.
  Older proofs, repeated proofs, regressed revisions, and changed position fields without a revision
  increase are rejected. The position watermark survives beneficiary changes.

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

This version tracks checkpoints and emits registered/maintained/reset outcomes. It creates no reward
entitlements, funding obligations, maturity buckets, or collection calls. Reward calculation must
settle the old verified interval before replacing a checkpoint when those features are added;
`maintained_since` tracks only the current continuous revision. Future accounting must use
historical pool prices and reward policy, and retain already-earned credits across resets.
