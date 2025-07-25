# Argon Node Consensus Architecture

## Executive Summary

This document explains how Argon's consensus layer operates in practice, covering the dual consensus
mechanism (vote blocks and compute blocks), mining authority selection, and the complex block import
handling that ensures network stability.

## Architecture Overview

### Core Components

1. **Block Builder Task** (`lib.rs`) - Main orchestration loop
2. **Block Creator** (`block_creator.rs`) - Block proposal and submission
3. **Import Queue** (`import_queue.rs`) - Block verification and import
4. **Aux Client** (`aux_client.rs`) - Auxiliary consensus data management
5. **Notebook Sealer** (`notebook_sealer.rs`) - Vote block creation
6. **Compute Worker** (`compute_worker.rs`) - Proof-of-work mining
7. **Notary Client** (`notary_client.rs`) - Notebook verification

### Consensus Flow Types

Argon uses a dual consensus mechanism:

1. **Vote Blocks** - Created by registered mining authorities using notebook voting power and XOR
   distance selection
2. **Compute Blocks** - Created through proof-of-work when voting fails or no authorities are
   available

## Mining Authority Registration and Selection

### XOR Distance Algorithm and Block Submission

When notebooks create voting opportunities, the system uses an **XOR distance algorithm** to
determine mining priority:

1. **Seal Strength Generation**: Each notebook's votes are processed through the voting system
   (detailed in [notebook.md](./notebook.md#vote-seal-calculation)) to produce a "seal strength" - a
   U256 number derived from vote contents and voting power. This cannot be pre-gamed since it
   depends on actual committed vote data and unpredictable voting keys.

2. **XOR Key Assignment**: Each mining authority has a unique XOR key generated by hashing their
   account ID with the parent block hash at the time of frame rotation. This ensures keys are
   deterministic but unpredictable.

3. **Distance Calculation**: The system calculates XOR distance between the seal strength and each
   authority's XOR key to determine priority using simple bitwise XOR:
   `distance = seal_strength ^ miner_xor_key`.

**Current Consensus Model (Post-Update):**

- **Any Miner Can Submit**: While the XOR-closest miner has priority, any registered mining
  authority can now submit the block vote
- **Delayed Submission**: Non-closest miners submit on a delay based on their XOR distance
  percentile and remaining tick time
- **Network Resilience**: This prevents chain stalls if the closest miner is offline or unresponsive

**Why This Works:**

- **Unpredictable Selection**: Seal strength depends on notebook content that can't be known in
  advance
- **Fair Over Time**: Statistical distribution ensures all authorities get opportunities
- **Prevents Gaming**: XOR keys tied to specific parent blocks at frame rotation
- **Resilient Consensus**: Chain continues even if preferred miners fail to respond

### Mining Authority Lifecycle

#### 1. **Mining Authority Registration**

To become a mining authority, operators must submit competitive bids through the mining slot system:

**Requirements:**

- **Economic Stake**: Hold minimum argonots (ownership tokens) as collateral
- **Competitive Bid**: Submit Argon payment to compete for mining slots
- **Cryptographic Keys**: Provide Ed25519 session keys for block signing
- **Account Binding**: Link the mining authority to a funding account

**Auction Process**: Mining slots are awarded to the highest bidders each frame (roughly 24-hour
periods). The number of available slots adjusts dynamically based on network demand.

#### 2. **Cohort Formation and Frame Transitions**

**Frame System**: The network operates in ~24-hour "frames" where mining authority cohorts rotate:

1. **Bidding Period**: During each frame, operators can submit bids for the next mining cohort
2. **Selection**: At frame transition, the highest bidders become the new mining authorities
3. **XOR Key Generation**: Each new authority gets a unique cryptographic key for the selection
   algorithm
4. **Overlapping Operation**: Multiple cohorts operate simultaneously - as one cohort starts,
   another is finishing their 10-frame mining term

**Cohort Lifecycle**: Mining authorities serve for 10 frames (roughly 10 days), creating overlapping
generations that provide stability and gradual turnover rather than sudden replacement of all
miners.

#### 3. **Authority Verification and Selection**

The system maintains a registry of active mining authorities across multiple overlapping cohorts:

**Authority Lookup**: When a block is submitted, the system verifies the submitter is a registered
mining authority with active status.

**Distance Calculation**: For vote blocks, the system calculates XOR distances between the seal
strength and all active authorities to determine:

- The closest authority (who has immediate submission rights)
- Distance rankings for all other authorities (who must wait based on their percentile)

**Overlapping Cohorts**: With 10 active cohorts at any time, the system must check authorities
across all active frames when validating block submissions.

### Block Vote Verification Flow

#### 1. **Vote Creation and Submission Timing**

When notebooks arrive, multiple mining authorities may attempt to create blocks:

**Vote Timing and Security Model**:

The XOR selection security relies on a sophisticated commit-reveal scheme that separates vote
creation from mining key calculation (detailed in
[notebook.md](./notebook.md#the-commit-reveal-security-mechanism)).

**Key Security Properties:**

- **Unpredictable Mining Keys**: Voting keys are assembled from secrets committed before votes exist
- **Temporal Separation**: Multi-tick process prevents vote manipulation
- **Economic Incentives**: Fork choice rules favor inclusion over exclusion

**Why Gaming Attempts Fail:**

- **Vote Manipulation**: Voting keys use secrets committed before votes are created
- **Notebook Exclusion**: Fork choice prioritizes blocks with more notebooks and voting power
- **Late Attacks**: Parent secrets were committed in previous ticks before attack conditions existed
- **Economic Competition**: Others can build more inclusive blocks using the same votes

**Submission Timing**:

- **Closest XOR miner**: Can submit immediately upon notebook receipt
- **Other miners**: Wait based on their XOR distance percentile and remaining tick time
- **Delay Formula**: Further miners wait longer, preventing network flood while ensuring backup
  options

#### 2. **Mining Authority Verification**

The verification process has evolved to balance security with resilience:

**Previous Model (V1)**:

- Only the XOR-closest authority could submit the vote
- Chain could stall if that specific miner was offline

**Current Model (V2)**:

- Any registered mining authority can submit the vote
- XOR distance still tracked for priority and rewards
- Network prefers closest miner's block but accepts others

**Verification Steps**:

1. Confirm submitter is a registered mining authority
2. Calculate and record XOR distance for priority
3. Verify block vote signatures and validity
4. Apply preference based on XOR distance when multiple valid blocks compete

#### 3. **Cross-Pallet Integration Points**

- **Storage Access**: `pallet_block_seal` queries mining authorities via `AuthorityProvider` trait
- **Economic Security**: Mining bids fund system security, authorities lose stake for misbehavior
- **Temporal Coordination**: Frame transitions coordinate between mining_slot registration and
  block_seal verification
- **Event Handling**: Mining events trigger block seal processing and reward distribution

## Notebook Integration with Consensus

### How Notebooks Enable Block Creation

**Notebook Structure and Content**: Notebooks are cryptographically-signed data structures created
by notaries that batch localchain transactions for mainchain settlement. Each notebook contains
votes, account changes, transfers, and domain operations (complete structure detailed in
[notebook.md](./notebook.md#notebook-lifecycle-complete-technical-flow)).

**Vote Processing Flow**:

1. **Localchain Voting**: Users vote on grandparent blocks through localchain transactions
2. **Notebook Aggregation**: Notaries collect votes and create merkle proofs of vote inclusion
3. **Voting Key Assembly**: Mainchain combines revealed secrets from all notebooks to create
   unpredictable voting keys
4. **Seal Strength Calculation**: Vote contents are hashed with voting keys to produce seal strength
   values
5. **Miner Selection**: XOR distance between seal strength and miner keys determines block creation
   priority

**Critical Timing Relationship**: The commit-reveal mechanism (explained in
[notebook.md](./notebook.md#the-commit-reveal-security-mechanism)) ensures that:

- Vote content is committed before voting keys exist
- Mining authority selection cannot be gamed
- Fork choice favors economic inclusion over exclusion

**Emergency Fallback**: When notebooks don't arrive, the system falls back to compute
(proof-of-work) blocks to maintain chain liveness.

## Block Creation Flow

### 1. Main Block Builder Loop

The node runs a continuous loop that monitors several event sources:

**Event Sources**:

- **Notebook Arrivals**: New notebooks from notaries trigger vote block opportunities
- **Network Blocks**: Imported blocks from peers may change the best chain
- **Finalization Updates**: GRANDPA finalization affects fork choice
- **Time-based Triggers**: Compute mining windows based on elapsed time

**Decision Process**:

1. When notebooks arrive, the node checks if it can create a better block than the current best
2. If no vote blocks are possible (or during emergencies), compute mining may activate
3. The node always builds on what it considers the "best" block based on fork choice rules

**Emergency Compute Mining Triggers**:

- No eligible notebooks arrive within expected time window
- All mining authorities offline or unresponsive to notebook arrivals

**Key Consideration**: The best block determination has been refined to only check if a new vote can
beat the best block at the notebook's inclusion tick, preventing excessive block creation.

### 2. Vote Block Creation Flow

1. **Trigger**: New notebooks available at tick
2. **Process**: `NotebookSealer::check_for_new_blocks()`
3. **Guard**: Duplicate author detection per voting key (`aux_client.rs:193-206`)
4. **Output**: `CreateTaxVoteBlock` sent to block creator

### 3. Compute Block Creation Flow

1. **Trigger**: No eligible votes + mining time elapsed OR emergency tick reached
2. **Process**: `BlockCreator::propose()` creates block proposal
3. **Guard**: Parent hash validation (`compute_worker.rs:131-135`)
4. **Output**: `SolvingBlock` for compute mining

### 4. Block Submission and Fork Choice

**Block Submission Process**:

1. Block creator adds author information to the block header
2. Block submitted to local import queue for validation
3. If valid, block is gossiped to the network

**Fork Choice Algorithm**: When multiple competing blocks exist, the network chooses based on
ForkPower comparison:

1. **Notebook inclusion count** (higher wins)
2. **Total vote-created blocks** (higher wins)
3. **Total voting power spent** (higher wins)
4. **Seal strength** (lower wins - for vote blocks only)
5. **XOR distance to miner** (lower wins - for vote blocks only)
6. **Compute difficulty** (higher wins - for compute blocks)

This prioritization ensures inclusive blocks beat exclusive ones, preventing manipulation attempts.

## Block Import Flow

### 1. Argon Import Pipeline (`import_queue.rs:80-100`)

Argon's block import uses a serialized pipeline that handles complex scenarios:

**Import Pipeline**:
`network → BasicQueue.check_block → Verifier.verify → ArgonBlockImport.import_block`

**Serialization**: The `import_lock` ensures atomic operations so `client.info()` and auxiliary data
writes remain consistent during concurrent imports.

**Early Exit Conditions**:

- **Missing Parent State + ExecuteIfPossible**: Returns `ImportResult::MissingState`, triggering
  state sync
- **Header Already in DB** with no new body/state: Returns `ImportResult::AlreadyInChain`

**Fork Choice Integration**: When multiple valid blocks compete, Argon uses ForkPower comparison:

- Higher fork power → `set_best = true`
- Equal fork power → lowest block hash wins → `set_best = true`
- Losers marked with `import_existing = true` and checked for duplicates via
  `aux.check_duplicate_block_at_tick`

**State Management**: The system queues "pending children" when parent state is missing, then
processes them once state becomes available.

### 2. Duplicate Block Prevention

**Vote Block Deduplication** (`aux_client.rs:193-206`): The system prevents mining authorities from
submitting multiple vote blocks per voting key by tracking `(author, voting_key)` pairs in auxiliary
storage. This ensures each authority can only vote once per eligible notebook set.

**Compute Block Validation** (`compute_worker.rs:131-135`): Compute mining workers validate that
they're building on the current best block hash before starting proof-of-work, preventing wasted
work on stale parent blocks.

**Import-Time Checks**: During import, tie-loser blocks are marked with `import_existing = true` and
checked against `aux.check_duplicate_block_at_tick` to prevent the same tick from having multiple
imported blocks from the same author.

## Block Import Handling

### Complex Import Scenarios

Substrate's import queue handles multiple scenarios where the same block hash appears:

- **Header first**, then later with state
- **Network gossip** from multiple peers
- **Retry after** "parent state missing" errors
- **GRANDPA finalization** adding justification to existing blocks
- **Peer re-broadcasting** blocks they consider best

The node must handle these scenarios correctly to avoid getting stuck in import loops or creating
excessive duplicate blocks.

### Import Flow Challenges

**State Dependency**: Blocks may arrive before their parent's state is available, requiring careful
queuing and replay logic.

**Duplicate Prevention**: The same block content arriving multiple times through different paths
must be handled efficiently without redundant processing.

**Fork Choice Stability**: During network partitions or consensus issues, deterministic tie-breaking
ensures all nodes converge on the same best block.

## Substrate Integration

### GRANDPA Finalization Layer

Argon runs GRANDPA finalization on top of its vote/compute consensus with specific voting rules
(`node/src/service.rs:390-397`):

**Finalization Delay**: GRANDPA waits 4 blocks behind the best block (`BeforeBestBlockBy(4u32)`) to
ensure notebooks and votes are settled:

- **Best Block** (tick 5)
- **Notebooks for tick** (4)
- **Eligible Votes** (tick 3)
- **Votes for blocks at tick** (2)

**Conservative Finalization**: Combined with `ThreeQuartersOfTheUnfinalizedChain`, this ensures
GRANDPA only finalizes blocks with stable vote-based consensus.

### Component Integration (`node/src/service.rs:166-176`)

**Layered Import**: `ArgonBlockImport` wraps `GrandpaBlockImport` which wraps Substrate's base
import, creating a pipeline where:

1. Argon validates vote/compute consensus rules
2. GRANDPA handles finalization logic
3. Substrate manages basic block validation

**Service Coordination**: The node integrates multiple services:

- **Notary Sync**: Downloads and validates notebooks from notaries
- **UTXO Tracker**: Monitors Bitcoin network for vault transactions
- **Block Builder**: Creates new blocks based on notebook arrivals
- **Best Block State Service**: Ensures block state remains available for mining

**Task Management**: Essential services run as spawned tasks with proper error handling and graceful
shutdown coordination.

## Conclusion

Argon's consensus architecture balances several competing requirements:

### Key Design Principles

1. **Economic Security**: Mining authority registration requires significant economic stake
2. **Fair Selection**: XOR distance algorithm ensures unpredictable but deterministic authority
   selection
3. **Robust Fallback**: Compute mining provides emergency consensus when vote system fails
4. **Network Resilience**: Complex import handling prevents nodes from getting stuck during network
   issues

### Critical Success Factors

1. **Authority Registration**: Proper mining slot bidding and cohort formation
2. **Vote Verification**: Accurate XOR distance calculation and signature validation
3. **Import Handling**: Robust duplicate detection and state dependency resolution
4. **Fork Choice**: Deterministic tie-breaking for network convergence

### Operational Considerations

The dual consensus mechanism requires careful monitoring of:

- Mining authority registration and rotation
- Vote block vs compute block ratios
- Import success rates and retry patterns
- Network partition detection and recovery

This architecture enables Argon to maintain consensus even during challenging network conditions
while providing strong economic security guarantees.
