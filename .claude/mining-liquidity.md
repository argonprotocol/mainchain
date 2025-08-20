# Mining and Liquidity Pool Economics

This document explains how the Argon protocol's mining consensus mechanism generates revenue for
vaults through liquidity pools, creating the primary economic incentive for vault operators.

## Overview: Mining Revenue Distribution

The Argon protocol uses a unique consensus mechanism where **miners bid for slots**, and those bid
payments are distributed to **vault liquidity pools** as the primary revenue source for the network.

### Key Economic Flow

```
Miners pay bids → Mining bid pool → 20% burned → 80% distributed to vault liquidity pools → Vaults earn based on participation
```

## Mining Slot Mechanism (`pallets/mining_slot`)

### How Miners Participate

1. **Slot Bidding Process**:
   - Miners submit Argon bids for future mining slots in "cohorts" (groups of miners per frame)
   - **Mining terms**: Winners mine for 10 frames, creating overlapping cohorts
   - **Overlapping operation**: 10 different cohorts operate simultaneously, each at different
     stages of their term
   - **Benefits**: Gradual miner turnover (vs 100% replacement), attack resistance, continuous price
     discovery
   - Bids are denominated in Argons and submitted in 10 milligon increments
   - Higher bids get priority for mining slots
   - Miners must hold argonots (ownership tokens) as collateral
   - **Current requirement**: Minimum 0.01 argonots per mining slot
   - **Adjusts dynamically**: Target of 20 bids per slot; requirements increase/decrease to maintain
     competition (max 20% change per frame)
   - **Found in**: `ArgonotSeatCost` storage item, configured in runtime

2. **Bid Competition:**
   - Miners submit bids in 10 milligon increments
   - Bids sorted by amount (highest first)
   - Top N bidders win slots for that cohort
   - Winning bids go to central bid pool
   - Outbid miners get immediate refunds
   - **Implementation**: `bid()` in `pallets/mining_slot/src/lib.rs`

3. **Mining Revenue Source**:
   - **Miners earn**: 75% of block rewards (currently 500k argons per block, growing to 5M argons)
   - **Block rewards grow**: +1,000 argons every 118 blocks until reaching 5M argon cap
   - **Transaction fees**: Additional earnings on top of block rewards
   - **Cost**: Must pay upfront bids for the right to mine
   - **Found in**: Block rewards pallet configuration, `MinerPayoutPercent = 75%`

## Liquidity Pool Mechanics (`pallets/liquidity_pools`)

### Vault Participation Requirements

Each vault can participate in liquidity pools based on their **activated securitization**:

**Activated securitization:** A vault can only participate in mining revenue based on how much
Bitcoin backing they actually have verified and working. A securitization ratio can be up to 2x,
which allows a multiplier on the activated securitization. If a vault has 1M Argons of
securitization and a 2x ratio, but only 200k Argons worth of verified Bitcoin locks, they can only
use up to 400k Argons (2x the Bitcoin) for mining revenue eligibility, not their full 1M.

**Liquidity pool capacity per frame:** Each frame, a vault can only allocate up to 10% of their
total activated securitization into the liquidity pool. This amount must exist in the form of
liquidity pool contributed funds. So if they have 400k activated securitization, up to 40k can be
contributed to compete for that frame's mining bid revenue.

**Implementation**: `Vault::get_activated_securitization()` in `primitives/src/vault.rs`

### Revenue Distribution Model

**How mining revenue gets distributed:**

1. **Pool-level:** When miners pay bids, 20% gets burned and 80% gets split among all vault
   liquidity pools based on how much each vault contributed to the total pool.

2. **Within each vault:** The vault operator sets a profit sharing percentage that determines what
   portion goes to liquidity providers (with the operator keeping the remainder). Liquidity
   providers then split their portion based on how much each person contributed.

**Implementation**: `distribute_bid_pool()` in `pallets/liquidity_pools/src/lib.rs`

## Economic Incentive Structure

### For Vault Operators

**Revenue Sources:**

1. **Mining bid revenue** (primary) - Share of 80% of mining bids via liquidity pool participation
2. **Bitcoin lock fees** - Direct fees from users locking Bitcoin
3. **Annual percentage rates** - Ongoing fees on locked Bitcoin

**Revenue Amplification:**

- More Bitcoin locks → Higher activated securitization → Higher liquidity pool capacity → More
  mining revenue
- This creates a **virtuous cycle** where bitcoin lock success drives mining revenue capacity

### For Liquidity Providers

**Incentives:**

- Earn pro-rata share of 80% of mining bid pool (distributed each frame start)
- Participate in vault success without operating vault infrastructure
- Capital is backed by vault securitization (risk mitigation)

**Constraints:**

- Can only contribute to vaults with available capacity (based on bitcoin locks)
- Share revenue with vault operator based on profit sharing terms

### For Miners

**Economics:**

- Pay bids upfront for mining rights
- Earn block rewards + transaction fees
- Competitive market ensures bids reflect expected mining profitability

## Connection to Bitcoin Locks

### Why Bitcoin Locks Matter for Vault Revenue

1. **Securitization Capacity**: More bitcoin locks = higher activated securitization
2. **Liquidity Pool Eligibility**: Activated securitization determines maximum liquidity pool
   participation
3. **Mining Revenue Access**: Liquidity pool participation = share of mining bid revenue

### Economic Balance

**Vault Bitcoin Risk is Minimal:**

- If user defaults: Vault loses Argons but gains Bitcoin claim
- Bitcoin value ≈ Argon value (by design)
- **Real business**: Mining revenue via liquidity pools, not Bitcoin speculation

## Consensus Security Model

### System Design

**Mining Competition**: Competitive bidding ensures market-rate pricing and economic security
**Revenue Alignment**: Vaults provide infrastructure, miners provide security, revenue sharing
aligns incentives **Network Effects**: More vault success → More bitcoin locks → Higher mining
revenue capacity → Self-reinforcing growth

## Technical Implementation

### Key Pallets Integration

1. **`mining_slot`** - Manages bidding, slot allocation, bid pool accumulation
2. **`liquidity_pools`** - Manages vault participation, revenue distribution
3. **`vaults`** - Tracks securitization, activated capital calculations
4. **`bitcoin_locks`** - Drives securitization activation through lock activity

### Cross-Pallet Dependencies

**Data Flow**: `mining_slot` accumulates bids → `liquidity_pools` distributes 80% → `vaults` track
capacity → `bitcoin_locks` drive activation

**Key Calculation**: `vault_revenue = (vault_pool_capital / total_pools) * mining_bid_pool_80%`

This creates a sustainable revenue model where vault operators earn primarily from mining bid
distributions rather than Bitcoin speculation, while providing essential securitization
infrastructure.

---

# Technical Implementation Deep Dive

> **Note**: This section provides detailed technical analysis of the mining system implementation,
> including cross-pallet operations, algorithms, and benchmarking implications.

## Mining Bid Lifecycle: Complete Technical Flow

### Bid Submission Process (`mining_slot::bid()`)

**Cross-Pallet Operations:**

1. **Argon Transfer**: `send_argons_to_pool()` transfers bids to liquidity pools account
2. **Argonot Hold**: `hold_argonots()` places ownership tokens on hold as collateral
3. **Tick Query**: Current blockchain tick retrieved for bid timestamping

**Validation Steps:**

- Bidding window must be open
- Bid amount must be multiple of 10 milligon increments (not 1 milligon)
- Session overlap validation (prevents consecutive mining terms)
- Sufficient balance verification for both Argons and Argonots

**Storage Operations per Bid:**

- **Reads**: 7 storage items (bidding state, cohort, config, tick, registration, balances)
- **Writes**: 3-4 storage items (cohort update, statistics, holds, provider counts)
- **Computational**: O(log n) binary search insertion + O(n) eviction if cohort full

### Bid Pool Management

**Data Structure**: `BidsForNextSlotCohort` - Bounded vector sorted by bid amount (highest first)

**Insertion Algorithm**:

**Bid insertion process:** Bids are inserted in order from highest to lowest amount. When bids are
equal, earlier submissions get priority. If the cohort is full, the lowest bidder gets evicted and
refunded.

**Implementation**: `bid()` in `pallets/mining_slot/src/lib.rs`

**Economic Security**: Two-currency system (Argons for bids, Argonots for collateral) prevents Sybil
attacks and ensures long-term commitment.

## Frame Transition Mechanics: Cross-Pallet Orchestration

### Frame Transition Trigger

**Trigger Mechanism**: Time-based in `mining_slot::on_finalize()`

**Frame timing:** New frames start automatically based on blockchain ticks. Each frame lasts a fixed
duration (24 hours on mainnet).

**Implementation**: `on_finalize()` in `pallets/mining_slot/src/lib.rs`

### Complete Frame Transition Sequence

**Phase 1: Miner Retirement**

1. Remove miners whose 10-frame term expired (those who started at frame_id - 10)
2. Release argonot holdings for non-renewing miners
3. Clear account index lookups
4. Collect removed miner information
5. **Note**: Only the cohort that started 10 frames ago retires each frame

**Phase 2: Miner Activation**

1. Extract winning cohort from `BidsForNextSlotCohort`
2. Generate XOR keys for localchain matching: `blake2_256(account_id + parent_block_hash)`
3. Insert new cohort into `MinersByCohort` storage
4. Update `AccountIndexLookup` for new miners

**Phase 3: Cross-Pallet Notifications**

**Cross-pallet notifications:** Frame transitions notify other pallets to update consensus
authorities and distribute mining revenue.

**Implementation**: `SlotEvents` trait calls in `pallets/mining_slot/src/lib.rs`

### Cross-Pallet Event Cascade

**SlotEvents Trait**: Tuple-based notification system
`(GrandpaSlotRotation, BlockRewards, LiquidityPools)`

**Liquidity Pools Frame Operations** (triggered by `on_frame_start`):

1. **`release_rolling_contributors()`**: Release holds for contributors who opted out
2. **`distribute_bid_pool()`**: Distribute 80% of mining bids to vault liquidity pools
3. **`end_pool_capital_raise()`**: Lock in capital for next frame, refund excess
4. **`rollover_contributors()`**: Automatically renew profitable contributors

## Dynamic Economic Adjustment Algorithms

### Argonot Seat Cost Adjustment

**How it works**: The system targets 20 bids per mining slot to ensure healthy competition. If
there's too much bidding competition, argonot requirements increase to cool things down. If too
little, requirements decrease to encourage more participation.

**Parameters**:

- **Target**: 20 bids per slot
- **Damping**: 20% maximum change per frame
- **Bounds**: 0.01 argonots minimum to 80% of total network argonots
- **Found in**: Runtime config `TargetBidsPerSlot = 20`, `MinimumArgonotsPerSeat = 10,000`
  (micro-units, = 0.01 argonots)

### Cohort Size Adjustment

**Algorithm**: Adjusts number of miners per frame based on bid pricing

**How it works**: If average bid prices are higher than the target (1,000 argons per seat), the
system creates more mining slots for the next frame. If prices are lower, it reduces slots. This
balances mining profitability with network security needs.

**Current Parameters**:

- **Target price per seat**: 1,000 argons
- **Cohort size range**: 10 to 1,000 miners per frame
- **Adjustment damping**: Maximum 20% change per frame
- **Found in**: Mining slot pallet, `TargetPricePerSeat` and cohort size adjustment logic

### VRF Bidding Close Mechanism ("Candle Auction")

**How it works**: In the final 30 ticks (30 minutes) before frame end, each block has a chance to
randomly close bidding based on the block's VRF proof.

**Implementation**: `vote_seal_proof < (U256::MAX / 30)` checked each block during final window

**Purpose**: Prevents last-minute bid sniping. With ~3.3% chance per block in the final 30 minutes,
bidding likely closes before the very end, encouraging earlier bid submission.

## Liquidity Pool Three-Phase Capital Management

### Phase System Overview

**Raising Phase (Frame N+2)**:

- Contributors bond Argons for future liquidity pools (minimum 100 Argons mainnet)
- Storage: `CapitalRaising<T>`
- **Automatic refunds**: If vault capacity exceeded, smallest contributors refunded
- Sorts contributors by amount (largest first), max 100 per vault

**Active Phase (Frame N+1)**:

- Capital transitions from raising to active state
- Storage: `CapitalActive<T>`
- **Capital amounts finalized**: Based on vault's current activated securitization ÷ 10
- **Excess contributions**: Smallest contributors automatically refunded if over limit

**Distributed Phase (Frame N)**:

- Mining bid profits distributed to contributors
- Storage: `VaultPoolsByFrame<T>`
- Pro-rata distribution with vault operator profit sharing
- Automatic profit compounding for next cycle

### Bonding Process

**Validation Chain**:

1. Vault must be open and accepting bonds
2. **Minimum threshold**: 100 Argons (mainnet) or 1 Argon (canary)
3. **Contributor displacement**: Max 100 contributors; new higher contributions displace smallest
   existing ones

**Hold System**: Uses `HoldReason::ContributedToLiquidityPool` for capital security **Sorting**:
Contributors ranked by amount (highest first) using binary search insertion

### Automatic Rollover System

**10-Frame Cycle**: Contributors automatically renewed every 10 frames with earnings compounded

**Rollover Exclusions**:

- Contributors below minimum threshold (varies by network)
- Manual opt-outs via `unbond_argons`
- Cases where vault reduced profit sharing below contributor's original terms

**Profit Compounding**: Previous earnings automatically added to principal for next cycle

**Related Systems:**

- [Bitcoin Locks](./bitcoin-locks.md) - For vault economics integration
- [Notebook](./notebook.md) - For cross-pallet event patterns
