# Stabilizer pool rewards

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
  beneficiary. The registration becomes eligible at the next frame and stays valid until a proof
  establishes a change. Proving a newer block for the same revision extends eligibility through
  that block. An increased revision replaces the registration and forfeits the old revision's
  unclaimed rewards. The position watermark survives beneficiary changes.
- `report_position_change(position, proof)` lets anyone prove that the enrolled revision increased
  or the NFT left its registered subscription. A successful report removes the registration and
  forfeits its unclaimed rewards. A report during the next frame also removes the position's
  liquidity from the closed frame's total before Treasury fixes the rate.

Each checkpoint contains actual account/storage witnesses for the subscriber (beneficiary and
revision), PositionManager (subscription and packed position metadata), and PoolManager (NFT
liquidity). All witnesses must use the identical execution-block proof. The existing Ethereum
verifier authenticates the state root and header ancestry; block number, hash, and timestamp come
from that verified header. No caller-provided timestamp or `eth_call` result is trusted. Native v4
PoolIds are scoped by PoolManager; NFT checkpoints are scoped by PositionManager and token ID.

Prefer a recently retained Ethereum anchor to avoid header ancestry and old-state RPC limits. A
short ancestor witness also works.

## Supported deployments

Storage paths are pinned to subscriber layout v1, the purchase integration's `@uniswap/v4-periphery`
revision `545a5d2a87228167edde48f3b9eda122d1e3c4d6` PositionManager, and `@uniswap/v4-core` 1.0.2
PoolManager. The deployment approval must verify these contract implementations, their immutable
PositionManager/PoolManager bindings, and the configured pool key. A valid trie witness alone cannot
establish the semantics of arbitrary contract bytecode. The Solidity integration tests compile that
PositionManager revision and check the offsets against its deployed storage and the published
PoolManager bytecode.

## How earnings work

A position earns for supplying the full 90%–100% price band around the ARGON target. The band may
shift as a whole by up to 1% while its owner adjusts it. Only the liquidity inside the current
reward band earns. The frame's sampled Uniswap price determines how much of that liquidity is
ARGON and how much is USDC. USDC converts to microgons at the frame's USDC/USD and ARGON target/USD
quotes.

Treasury allocates 10% of mining bid revenue to stabilizer rewards and applies the frame's APY
taper. It divides that budget by the greater of the target pool value (initially 5% of opening
circulation) and the total eligible registered value. Each position earns that rate on 90% of its
USDC value and 10% of its ARGON value. For example, at a frame rate of 1%, a position holding 800
ARGON worth of eligible USDC and 200 eligible ARGON earns 7.4 ARGON. Funding left after claims
stays in the reserve.

## From checkpoint to payment

An account can enroll up to five NFTs. Registration also creates its Treasury Positions account.
Each NFT starts contributing to its exact Uniswap range in the next frame. A later proof of the
same revision establishes how long the position stayed eligible.

At frame close, Stabilizer Pools snapshots the registered liquidity in the qualifying ranges.
Anyone has the following frame to report a changed or unsubscribed NFT. Treasury then fixes the
rate using the corrected total. An unreported change can still dilute other participants. Earning
a payment requires proof that the position lasted from one frame's opening timestamp through the
next. A frame with insufficient price observations earns zero.

For example, frame 20 is under review throughout frame 21. Its earnings can be collected in frames
22 through 31. One checkpoint can prove several complete frames. Collect before repositioning or
unsubscribing to keep the rewards from the old revision.

Calling `treasuryPositions.collect()` settles every eligible NFT belonging to the signer.
Stabilizer Pools advances each NFT's cursor through the finalized frames covered by its proof.
Treasury debits the combined amount from the shared reserve. Treasury Positions delivers one ARGON
payment and updates the account's collected totals. If payment fails, the account can retry with
its cursors and earnings intact.

Until Apps initiates Collect, Treasury Positions runs the same call for every account in the first
block after each frame starts. A failed account can retry independently.
