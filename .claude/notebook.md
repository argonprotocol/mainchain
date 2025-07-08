# Notebook System: L2 Scaling Architecture

The notebook system enables Argon's L2 scaling by batching localchain transactions into
cryptographically-verified submissions to the mainchain. Notaries validate localchain state and
submit notebooks that trigger cross-pallet processing for final settlement.

## What Problem Does This Solve?

**The Challenge**: Blockchain scalability requires handling thousands of micropayments without
overwhelming the mainchain with transaction fees and processing delays.

**The Solution**: Batch many localchain transactions into single mainchain submissions called
"notebooks" that maintain cryptographic integrity while enabling high-frequency micropayments.

## Architecture Overview

**Core Function**: Notaries collect localchain transactions, validate state changes, and submit
signed notebooks to mainchain as mandatory inherents.

**Key Flow**:
`Localchain transactions → Notary validation → Notebook creation → Mainchain submission → Cross-pallet processing`

**Why This Works**: Users get immediate localchain confirmation for micropayments, while the
mainchain only processes periodic batch settlements with full cryptographic verification.

## Document Guide

This document covers three main areas:

1. **How Notebooks Work** (Lifecycle, Structure, Processing) - The basic mechanics
2. **Consensus Security** (Voting, Commit-Reveal, Mining Selection) - How it prevents gaming
3. **Implementation Details** (Cross-pallet integration, Storage, Economics) - Technical deep dives

## Notebook Lifecycle

### Phase 1: Localchain Processing

1. **Transaction Accumulation**: Users submit high-frequency micropayments to localchain
2. **Validation**: Notary validates transactions against localchain state (SQLite database)
3. **Audit Preparation**: Balance changes, transfers, and voting data accumulated
4. **Notebook Rotation**: `NotebookCloser::try_rotate_notebook()` creates new notebooks at tick
   boundaries

### Phase 2: Notebook Creation and Signing

1. **State Collection**: Account balance changes with merkle proofs, chain transfers (localchain ↔
   mainchain), domain registrations/renewals, block votes for consensus, tax calculations
2. **Merkle Root Generation**: Generates multiple cryptographic proofs using Blake2-256 hashing:
   - `changed_accounts_root`: Merkle root of all account balance changes (ordered by account_id,
     account_type)
   - `block_votes_root`: Merkle root of all block votes created in this notebook (sorted by
     account_id and index)
3. **Notebook Header Structure**: Contains sequential `notebook_number`, blockchain `tick`, both
   merkle roots, `chain_transfers`, `domains`, `block_votes_count`, `block_voting_power`, `tax`
   amount, `secret_hash` (commitment), `parent_secret` (reveal)
4. **Digital Signature**: Ed25519 signature over notebook header hash using notary keypair

### Phase 3: Submission Pipeline

1. **Archive Storage**: Complete notebook (header + full transaction data) stored in S3/MinIO
2. **Mainchain Inherent**: Signed notebook header submitted as mandatory inherent
3. **Node Audit**: Independent validation before block inclusion (comprehensive verification in
   `notary/audit/src/lib.rs`)

### Phase 4: Mainchain Processing

1. **On-Chain Validation**: Format, sequence, signature verification in `pallet_notebook::submit()`
   (`pallets/notebook/src/lib.rs`)
2. **Cross-Pallet Events**: `T::EventHandler::notebook_submitted(&header)` triggers cascade to
   subscribed pallets
3. **State Updates**: Notebook history and account change tracking stored in pallet storage

## Cross-Pallet Operations: Event-Driven Architecture

### Primary Integration Points

**1. Chain Transfer Processing (`pallet_chain_transfer`)**

- **ToMainchain Transfers**: Move funds from notary accounts to user mainchain accounts
- **ToLocalchain Transfers**: Validate and remove pending transfers, moving funds to notary accounts
- **Transfer Expiration**: Automatic refunds for expired transfers
- **Tax Burning**: Burn tax amounts collected from localchain operations

**2. Domain Management (`pallet_domains`)**

- **Domain Registration**: Process domain lease payments from notebook taxes
- **Domain Renewal**: Extend domain leases and update expiration tracking
- **Expiration Scheduling**: Add domains to tick-based expiration queues
- **Conflict Resolution**: Handle domain registration conflicts and refunds

**3. Consensus Participation (`pallet_block_seal`)**

- **Voting Root Updates**: Provide eligible voting roots for consensus
- **Mining Integration**: Connect localchain votes to mining slot allocation
- **Block Vote Validation**: Verify voting power calculations against submitted votes

### Provider Trait Dependencies

**Critical Cross-Pallet Interfaces:**

- **NotaryProvider**: Notary validation and state management
- **ChainTransferLookup**: Transfer validation for authenticity
- **NotebookProvider**: Voting and consensus integration
- **TickProvider**: Timing and voting schedules

Implementation details in `primitives/src/providers.rs`

## Storage Architecture

**Primary Storage**: Account change tracking, account origin mapping, recent notebook history (3
notebooks), block digest for current processing (`pallets/notebook/src/lib.rs`)

**State Synchronization**: Tick-based ordering (t-1 submission), sequential numbering per notary,
merkle root verification

**Automatic Cleanup**: Transfer expiration tracking and automatic refunds, processed during notebook
submission

## Security Model and Validation

### Cryptographic Security

**Multi-Layer Validation:**

1. **Ed25519 Signatures**: Prevent unauthorized notebook submission
2. **Merkle Proofs**: Ensure state consistency across account changes
3. **Sequential Numbering**: Prevent replay and reordering attacks
4. **Tick-Based Timing**: Prevent temporal manipulation

**Node-Level Audit Process**: Complete notebook audit before mainchain inclusion includes merkle
proof verification, transfer authenticity, signature validation, tax calculations, voting power
validation, and payment channel state verification (`notary/audit/src/lib.rs`)

### Economic Security

**Notary Incentive Alignment:**

- **Slashing Risk**: Failed audits result in notary locking and penalties
- **Revenue Model**: Notaries earn fees from localchain transaction processing
- **Reputational Stakes**: Successful operation builds trust and user adoption

**Collusion Resistance:**

- **Independent Audits**: Node performs independent verification before inclusion
- **Cryptographic Proofs**: Merkle roots prevent state manipulation
- **Public Verification**: All notebook data archived and publicly auditable

## How Notebooks Enable Secure Consensus

### The Mining Selection Problem

**The Challenge**: In blockchain consensus, miners could potentially game the system if they can
predict or influence which miner gets selected to create the next block.

**Argon's Approach**: Use "voting keys" that combine data from different time periods to create
unpredictable but deterministic miner selection.

**Key Insight**: If voting keys are calculated using secrets that were committed before votes
existed, miners can't game the selection process.

### Block Voting Mechanism: Complete Technical Deep Dive

The Argon voting system prevents vote manipulation through a carefully orchestrated timing sequence
that ensures **voting keys are unpredictable when votes are created**.

**The Core Security Challenge:**

- Users must vote on blocks without knowing which miner will process their vote
- The system must prevent miners from gaming their selection probability
- Vote timing must be deterministic but unpredictable

**The Solution - Temporal Separation:**

**Concrete Example** - Block being finalized at tick 5:

**Runtime Context** (`VotingSchedule::from_runtime_current_tick(5)`):

- `notebook_tick()` = 4 (current_tick - 1)
- `eligible_votes_tick()` = 3 (notebook_tick - 1)

**Voting Key Assembly**:

1. Takes notebooks submitted at **tick 4** (each contains `parent_secret`)
2. Gets vote roots from **tick 3** notebooks via `get_eligible_tick_votes_root(notary_id, 3)`
3. Combines: `BlockVotingKey { parent_secret: from_tick_4, parent_vote_root: from_tick_3 }`
4. Final key: `Blake2(encode([all_BlockVotingKey_pairs]))`

**Why This Prevents Gaming**:

- Tick 4 notebooks contain `parent_secret` values that were committed as `secret_hash` back in tick
  3
- At tick 3, notaries had to commit these secrets before tick 3 votes were cast
- So miners can't influence the voting key at tick 5, since it depends on secrets committed before
  the relevant votes existed

**Why This Works:**

1. **Blind Voting**: Users vote without knowing the voting key
2. **Pre-committed Secrets**: Voting key uses secrets committed before votes existed
3. **No Gaming**: Miners can't influence their selection probability since the key combines
   historical commitments with actual votes

### Vote Creation and Collection

**Localchain Vote Creation**: Users create BlockVote transactions on localchain with grandparent
block hash (from tick 1), voting power from tax payment, and user-chosen index
(`primitives/src/block_vote.rs`)

**Notary Vote Collection:**

1. **Vote Aggregation**: Notary collects all BlockVote transactions from localchain
2. **Merkle Root Generation**: Creates merkle tree of all votes for inclusion proof
3. **Notebook Header**: Includes `block_votes_root` and `block_voting_power` totals
4. **Default Vote**: If no votes, creates proxy vote to maintain liveness

### The Commit/Reveal Security Mechanism

**Understanding the Attack**:

- Miners compete to create blocks by having the closest "XOR distance" to a target number
- This target number (called "seal strength") is calculated using a "voting key"
- If miners could influence the voting key, they could manipulate their chances of winning

**The Defense - Commit/Reveal Across Time**: The voting key uses data from different time periods so
miners can't game it:

**Notary Commit-Reveal Chain**:

- Each notebook has `secret_hash` (commitment to future secret) and `parent_secret` (revelation of
  previous secret)
- Notaries must commit to secrets before knowing what votes will be cast

**What Gets Combined for Voting Key**:

1. **Revealed Secrets** (`parent_secret`): From current tick's notebooks
2. **Vote Roots** (`parent_vote_root`): From earlier tick's notebooks

**Voting Key Assembly** (in `block_seal` pallet `on_finalize` at tick 5):

```rust
// Get notebooks from tick 4
let notebooks_at_tick = T::NotebookProvider::notebooks_at_tick(4);

// For each notebook, create BlockVotingKey pairs
let parent_voting_keys = notebooks_at_tick
    .filter_map(|(notary_id, _, parent_secret)| {
        // Get vote root from tick 3 for this notary
        if let Some((parent_vote_root, _)) =
            T::NotebookProvider::get_eligible_tick_votes_root(notary_id, 3) {
            Some(BlockVotingKey {
                parent_vote_root,  // From tick 3
                parent_secret      // From tick 4
            })
        }
    });

// Hash all pairs to create final voting key for tick 5
let voting_key = BlockVotingKey::create_key(parent_voting_keys);
```

**Security**: Since secrets were committed by **notaries** before votes existed, miners cannot
influence the final voting key to favor their XOR distance.

**Security Properties:**

- **Unpredictable**: Key uses secrets committed before votes existed
- **Deterministic**: Same inputs always produce same key
- **Tamper-Proof**: Changing revealed secret breaks commitment from previous tick
- **Fair**: All participants contribute to randomness

### Vote Seal Calculation

**Seal Proof Generation**: Hash of vote bytes + notary ID + voting key converted to U256
(`primitives/src/block_vote.rs`)

**Seal Strength Calculation**: Seal proof divided by voting power to balance economic weight
(`primitives/src/block_vote.rs`)

### XOR-Based Miner Selection

**Finding Closest Miner**: XOR distance between seal proof and each miner's key determines priority.
Implementation details in consensus system (see
[consensus.md](./consensus.md#xor-distance-algorithm-and-block-submission))

**Security Properties:**

- **Unpredictable Distribution**: Voting key changes each tick
- **Fair Selection**: XOR distance ensures random miner assignment
- **Sybil Resistance**: Registered miners with staked Argonots

### Vote Processing in block_seal_spec

**Vote Digest Creation**: Aggregates voting power and vote counts from all eligible notebooks at a
specific tick, excluding locked notaries (`pallets/block_seal_spec/src/lib.rs`)

### Complete Vote Verification

**Runtime Vote Validation:**

1. **Timing Verification**: Ensure vote created at correct tick
2. **Grandparent Block**: Verify vote targets valid grandparent block
3. **Merkle Proof**: Validate vote inclusion in notebook merkle tree
4. **Signature Verification**: Confirm vote signed by claimed account
5. **Minimum Power**: Check vote meets minimum voting power requirement
6. **Miner Authority**: Verify submitting miner is a registered mining authority (any can submit,
   not just XOR-closest)

### Fork Power Accumulation

**Vote-Based Fork Power**: Accumulates voting power, notebook count, vote-created blocks, and seal
strength for fork selection priority

**Fork Selection Priority:**

1. **Notebook inclusion count** (higher wins)
2. **Total vote-created blocks** (higher wins)
3. **Total voting power spent** (higher wins)
4. **Seal strength** (lower wins - for vote blocks only)
5. **XOR distance to miner** (lower wins - for vote blocks only)
6. **Compute difficulty** (higher wins - for compute blocks)

### Execution Order Within Each Block

**Critical Implementation Detail: Inherents Before Finalization**

The security model depends on precise execution order within each block:

```
Block at Tick 5:
┌─────────────────────────────────────┐
│ 1. Block Initialization             │
│ 2. Inherent Processing              │  ← Notebooks processed HERE
│    - Submit notebooks from tick 4   │
│    - Store in LastNotebookDetails   │
│    - Mark as vote eligible          │
│ 3. Extrinsic Processing            │
│ 4. Block Finalization              │  ← Voting key calculated HERE
│    - Look up notebooks from tick 4  │
│    - Calculate voting key           │
│    - Store for next block's use     │
└─────────────────────────────────────┘
```

**Why This Ordering is Critical:**

- **Step 2** processes and stores notebooks from tick 4
- **Step 4** reads those same notebooks to calculate the voting key
- If these steps were reversed, the voting key would use stale data

Both operations reference tick 4, ensuring consistency.

### Security Features

1. **Commit/Reveal Scheme**: Parent secrets prevent vote manipulation
2. **Economic Weighting**: Voting power based on tax payments
3. **Time-Based Eligibility**: Strict tick-based vote validity
4. **Cryptographic Proofs**: Merkle proofs ensure vote authenticity
5. **XOR Distribution**: Unpredictable miner selection
6. **Locked Notary Exclusion**: Failed audits exclude notary votes

### Integration with Consensus

**Hybrid PoW/PoS Properties:**

- **Economic Weight**: Tax-based voting power (PoS-like)
- **Computational Fairness**: XOR miner selection (PoW-like)
- **Scalability**: Localchain aggregation via notebooks
- **Security**: Cryptographic proofs and economic incentives

### Summary: Why This Complex Timing Works

**The Problem Solved:** In most blockchain systems, miners can see transactions before including
them, potentially gaming the selection process. Argon prevents this through temporal separation.

**The Solution:**

1. **Commitment**: Notaries commit secrets before votes exist
2. **Blind Voting**: Users vote without knowing the mining key
3. **Delayed Revelation**: Keys are calculated only after votes are submitted
4. **Fair Selection**: XOR distance ensures unpredictable but deterministic miner selection

**Security Guarantees:**

- ✅ **No Vote Manipulation**: Voting key unknown during vote creation
- ✅ **No Miner Gaming**: Key uses pre-committed randomness
- ✅ **Deterministic Outcomes**: Same inputs always produce same results
- ✅ **Economic Weighting**: Vote power based on actual economic activity (tax payments)

This creates a hybrid consensus where economic weight determines votes, but computational fairness
determines block production, all orchestrated through the notebook timing system.

## Economic Integration

### Revenue Flow for Notaries

**Primary Revenue Sources:**

1. **Transaction Fees**: Direct fees from localchain users
2. **Tax Collection**: Percentage of localchain operation value
3. **Service Fees**: Domain registration and renewal processing

**Cost Structure:**

1. **Infrastructure**: Database, storage, and compute costs for localchain validation
2. **Archive Storage**: S3/MinIO costs for notebook persistence
3. **Mainchain Fees**: Gas costs for notebook submission inherents

### Integration with Mining Economics

**Consensus Participation:**

- **Block Votes**: Notebooks include votes on mainchain blocks for consensus
- **Mining Rewards**: Voting participation enables mining reward distribution
- **Voting Power**: Based on localchain activity and stake

## Why This Enables L2 Scaling

### The Micropayment Problem

**Traditional Blockchain Issue**: Each transaction requires mainchain fees, making micropayments
economically unviable.

**Notebook Solution**: Bundle thousands of micropayments into single mainchain submissions.

### Scalability Benefits

**Efficiency**:

- **Batch Size**: Up to 100,000 localchain transactions → 1 mainchain operation
- **Cost**: Users pay micro-argon fees instead of full mainchain transaction fees
- **Speed**: Immediate localchain confirmation, periodic (every ~1 minute) mainchain settlement

**Security**:

- **Cryptographic Integrity**: Merkle proofs ensure no unauthorized state changes
- **Mainchain Settlement**: All state changes eventually finalized on secure L1
- **Economic Penalties**: Notary slashing discourages misbehavior

### Real-World Impact

**For Users**: Pay tiny fees for instant micropayments with mainchain security guarantees.

**For Network**: Handle high transaction volume without modifying core blockchain consensus.

### Economic Balance

**User Benefits:**

- **Low Fees**: Micropayment fees orders of magnitude lower than mainchain
- **Fast Confirmation**: Immediate localchain confirmation for most operations
- **Mainchain Security**: Ultimate settlement on secure L1 blockchain

**Network Benefits:**

- **Increased Throughput**: Higher transaction capacity without blockchain modification
- **Economic Activity**: More transactions generate more fees and network value
- **Decentralization**: Multiple notaries prevent single points of failure

## Event-Driven Cross-Pallet Architecture

### Multi-Pallet Event Handler Pattern

**Subscription Architecture**: Tuple-based event handler composition allows multiple pallets to
subscribe to notebook events (`pallets/chain_transfer/src/lib.rs`, `pallets/domains/src/lib.rs`,
`pallets/block_seal_spec/src/lib.rs`)

**Event Processing Sequence**:

1. **Notebook Validation**: Core notebook pallet validates and stores notebook
2. **Event Broadcast**: `T::EventHandler::notebook_submitted(&header)` triggers cascade
3. **Parallel Processing**: Each subscribed pallet processes notebook independently
4. **Cross-Pallet State Updates**: Coordinated state changes across multiple pallets

### Chain Transfer Event Processing

**Transfer Processing**:

- **ToMainchain**: Move funds from notary account to recipient
- **ToLocalchain**: Remove pending transfer and verify authenticity
- **Expired Transfer Handling**: Automatic refunds for expired transfers
- **Tax Burning**: Burn tax collected from localchain operations

**Implementation**: `pallets/chain_transfer/src/lib.rs` in `notebook_submitted()` function

### Domain Event Processing

**Domain Operations**: Registration, renewal, conflict resolution, expiration scheduling
**Implementation**: `pallets/domains/src/lib.rs` in `notebook_submitted()` function

## State Synchronization and Consistency

### Tick-Based Temporal Ordering

**Notebook Timing Requirements**: Notebooks must be submitted for previous tick with sequential
numbering per notary (validation in `pallets/notebook/src/lib.rs`)

**Cross-Pallet Synchronization**:

- **Transfer Expiration**: Based on tick timing, processed during notebook submission
- **Domain Expiration**: Scheduled by tick, processed during block finalization
- **Voting Eligibility**: Based on notebook submission tick for consensus participation

### Account Change Tracking

**Merkle Root Generation**: Off-chain notary generates `changed_accounts_root` from account balance
changes, verified on-chain against audit results

**Account Origin Tracking**: Maps each account origin to its last modification notebook for conflict
resolution

## Related Technical Documentation

**Core System Flows:**

- [Bitcoin Locks](./bitcoin-locks.md) - Vault economics and cross-pallet UTXO coordination
- [Mining Liquidity](./mining-liquidity.md) - Frame transitions and revenue distribution
- [Consensus](./consensus.md) - Voting mechanisms and mining authority selection

**Development Resources:**

- [Benchmarking](./benchmarking.md) - Complete benchmarking methodology and cross-pallet patterns
- [CLAUDE.md](../CLAUDE.md) - Project overview and development commands

**Key Integration Points:**

- **Cross-Pallet Events**: Notebook submission triggers chain transfer, domain, and voting
  operations
- **Consensus Coordination**: Block voting integrates with mining system and frame transitions
- **L2 Scaling Architecture**: Localchain transaction batching and mainchain settlement
- **Economic Incentives**: Notary revenue, tax collection, and penalty mechanisms
