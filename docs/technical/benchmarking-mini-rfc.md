# Mini RFC: Benchmarking Strategy for Runtime Weights

Last updated: 2026-03-11

## Context

Running benchmarks against full production wiring caused recurring issues:

- brittle setup requirements (cross-pallet prerequisites, runtime ordering, key material),
- unclear attribution of provider costs across pallets,
- repeated drift between benchmark assumptions and runtime-benchmark execution (WASM no-std).

We still want small, reviewable, per-pallet benchmark PRs with minimal runtime churn.

## Goals

- Deterministic, low-friction benchmark execution.
- Provider costs charged where consumed, without adding per-call arithmetic in `lib.rs`.
- Runtime-benchmark overrides that are compile-time only.
- Worst-case benchmark coverage for consensus-sensitive paths.

## Non-goals

- Full e2e/perf scenario benchmarking.
- Economic parameter tuning (for example `WeightToFee`) in this effort.
- Perfect real-world crypto modeling.

## Accepted Pattern (Current)

### 1) Benchmark-aware runtime config

Under `feature = "runtime-benchmarks"`, runtime associated types use benchmark providers via
`use_unless_benchmark!(Prod, Bench)` inline in runtime `Config` impls.

Current benchmark providers are centralized in `pallet_prelude::benchmarking` and exported via
`runtime/common/src/benchmarking.rs`.

### 2) Provider weights are published by provider pallets

Provider traits expose associated weight types (for example `type Weights`) through traits in
`primitives/src/providers.rs`, including event-handler style providers such as `BitcoinUtxoEvents`.

Provider-owning pallets expose provider weight adapters in their own `weights.rs`, for example:

- `ProviderWeightAdapter<T>` maps provider-weight trait methods to pallet `WeightInfo`.
- consumer pallets compose provider costs in `WithProviderWeights<...>` wrappers.

This keeps provider weight composition in `weights.rs` and out of pallet/runtime `lib.rs`.

### 3) Benchmark provider state is in-memory, not storage side channels

Benchmark mock providers are explicitly seeded by benchmarks.

- `std` benchmark builds use `parameter_types! { static ... }` holder state.
- no-std runtime-benchmark WASM uses an internal in-memory fallback.

No `:argon:bench:*` temporary storage keys are used for provider state lookup.

### 4) Worst-case benchmark shaping is explicit

- `pallet_block_seal::apply` benchmark measures the vote path (not compute path).
- notebook-provider lookup benchmarks seed max history depth for notary notebook history.
- `BenchmarkNotaryProvider::verify_signature` executes a deterministic lightweight real
  `ed25519_verify` to include signature verification cost shape in benchmarked paths.

## Keeping runtime changes minimal

- no new provider `saturating_add(...)` logic in pallet/runtime `lib.rs`.
- provider-cost composition is done in pallet `weights.rs` wrappers (`WithProviderWeights`).
- runtime `lib.rs` changes are limited to associated type wiring for provider overrides and
  selecting wrapper weight types.

## Implementation Status (Overall)

The tables below use two checks:

- `Runtime Measured`: this repo/runtime has wired and generated measured benchmark weights for the
  pallet.
- `Weights Wired`: the runtime uses a real, non-placeholder weight implementation for the pallet.

### Local Pallets

Local pallets in this repo (`pallets/*`):

| Pallet                        | Runtime Measured | Weights Wired |
| ----------------------------- | ---------------- | ------------- |
| `pallet_bitcoin_locks`        | [x]              | [x]           |
| `pallet_bitcoin_utxos`        | [x]              | [x]           |
| `pallet_block_rewards`        | [ ]              | [ ]           |
| `pallet_block_seal`           | [x]              | [x]           |
| `pallet_block_seal_spec`      | [x]              | [x]           |
| `pallet_chain_transfer`       | [x]              | [x]           |
| `pallet_digests`              | [ ]              | [ ]           |
| `pallet_domains`              | [ ]              | [ ]           |
| `pallet_fee_control`          | [ ]              | [ ]           |
| `pallet_inbound_transfer_log` | [x]              | [x]           |
| `pallet_mining_slot`          | [x]              | [x]           |
| `pallet_mint`                 | [ ]              | [ ]           |
| `pallet_notaries`             | [ ]              | [ ]           |
| `pallet_notebook`             | [x]              | [x]           |
| `pallet_operational_accounts` | [x]              | [x]           |
| `pallet_price_index`          | [ ]              | [ ]           |
| `pallet_ticks`                | [ ]              | [ ]           |
| `pallet_treasury`             | [ ]              | [ ]           |
| `pallet_vaults`               | [x]              | [x]           |

Upstream / runtime pallets:

| Pallet                       | Runtime Measured | Weights Wired |
| ---------------------------- | ---------------- | ------------- |
| `frame_system`               | [x]              | [x]           |
| `pallet_balances::Balances`  | [x]              | [x]           |
| `pallet_balances::Ownership` | [x]              | [x]           |
| `pallet_timestamp`           | [x]              | [x]           |
| `pallet_tx_pause`            | [ ]              | [x]           |
| `pallet_multisig`            | [ ]              | [x]           |
| `pallet_proxy`               | [ ]              | [x]           |
| `pallet_sudo`                | [ ]              | [x]           |
| `pallet_utility`             | [ ]              | [x]           |
| `pallet_transaction_payment` | [ ]              | [x]           |
| `ismp_grandpa`               | [ ]              | [x]           |
| `pallet_grandpa`             | [ ]              | [ ]           |

### Provider-weight pattern adoption

- Shared benchmark providers are centralized in `pallet_prelude::benchmarking` and exported via
  `runtime/common/src/benchmarking.rs`.
- Provider-weight adapters/wrappers are in pallet `weights.rs` (not `lib.rs`) for:
  - `pallet_block_seal`,
  - `pallet_block_seal_spec`,
  - `pallet_bitcoin_utxos`,
  - `pallet_chain_transfer`,
  - `pallet_notebook`,
  - `pallet_mining_slot`,
  - `pallet_block_rewards` (wrapper shape present).

### Validation status

- targeted benchmark regeneration has been run for `pallet_block_seal`, `pallet_block_seal_spec`,
  `pallet_chain_transfer`, `pallet_notebook`, `pallet_bitcoin_utxos`, and `pallet_bitcoin_locks`,
- `cargo check -p argon-runtime --features runtime-benchmarks`,
- `cargo check -p argon-canary-runtime --features runtime-benchmarks`,
- `cargo make fmt`,
- `cargo make lint`,
- no `UNKNOWN KEY` / `:argon:bench:` artifacts in targeted generated weight files.

### Notebook inherent event handler weight composition

`NotebookEventHandler` now exposes `notebook_submitted_weight(&NotebookHeader) -> Weight` (same
pattern as `BlockSealEventHandler::block_seal_read_weight`). The notebook `submit` inherent includes
this in its weight declaration so the block builder accounts for event handler costs.

`pallet_chain_transfer` implements this as
`notebook_submitted_event_handler(chain_transfers) + process_expired_transfers(MaxPendingTransfersOutPerBlock)`.

Measured cost at worst case (1000 expired transfers across 1000 ticks): ~44ms total ref_time, which
is 0.44% of the 10-second block weight budget. Expiry processing is not overweight.

### Deferred / follow-up

- implement actual benchmark suites for `pallet_block_rewards` and `pallet_treasury` and wire their
  generated runtime weight modules when complete,
- additional provider cleanup work outside this scope (for example domains/block_rewards follow-up
  items) remain separate,
- mandatory-inherent liveness guardrails are deferred to a follow-up PR (not this benchmark PR),
  specifically:
  - authoring-side caps for mandatory inherent payload size/count (notebook + bitcoin paths),
  - worst-case inclusion/import tests to ensure mandatory inherents do not fail with
    `InvalidTransaction::BadMandatory`,
  - safe defer/drop policy for overflow inputs so block production remains live,
- if we need richer modeling for crypto-heavy providers, add dedicated provider weight trait methods
  in a follow-up instead of pushing weight arithmetic into runtime logic.

## Reading benchmark weights

Substrate weights are in **picoseconds** (10^-12 seconds). Common conversions:

| Picoseconds            | Human-readable |
| ---------------------- | -------------- |
| 1,000,000 (1M)         | 1 microsecond  |
| 1,000,000,000 (1B)     | 1 millisecond  |
| 1,000,000,000,000 (1T) | 1 second       |

The block weight budget is `WEIGHT_REF_TIME_PER_SECOND * 10` = **10 seconds** = 10 trillion
picoseconds. A weight of `44_000_000` is 44 microseconds (0.044ms), not 44 milliseconds.

## Risks

- benchmark providers can drift from production behavior if benchmark setup assumptions are not kept
  explicit in benchmark code,
- provider call counts in benchmark setup can regress if benchmark preconditions change silently.
