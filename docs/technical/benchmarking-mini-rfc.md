# Mini RFC: Benchmarking Strategy for Argon Runtime

Last updated: 2026-02-27

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
`primitives/src/providers.rs`.

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

`Complete` in the table below means:

- runtime benchmark target is dispatched in both runtimes (where applicable),
- generated runtime weight modules are present in both runtimes (where applicable),
- the local pallet benchmark suite is implemented (not placeholder-only).

### Benchmark Completion Matrix

Local pallets in this repo (`pallets/*`):

| Complete | Pallet | Local Benchmark Suite | Dispatched (Argon/Canary) | Weights Present (Argon/Canary) | Notes |
| --- | --- | --- | --- | --- | --- |
| [ ] | `pallet_bitcoin_locks` | missing | no / no | no / no | no `pallets/bitcoin_locks/src/benchmarking.rs` |
| [ ] | `pallet_bitcoin_utxos` | missing | no / no | no / no | no `pallets/bitcoin_utxos/src/benchmarking.rs` |
| [ ] | `pallet_block_rewards` | placeholder-only | no / no | no / no | `pallets/block_rewards/src/benchmarking.rs` placeholder |
| [x] | `pallet_block_seal` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [x] | `pallet_block_seal_spec` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [x] | `pallet_chain_transfer` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [ ] | `pallet_digests` | missing | no / no | no / no | no `pallets/digests/src/benchmarking.rs` |
| [ ] | `pallet_domains` | missing | no / no | no / no | no `pallets/domains/src/benchmarking.rs` |
| [ ] | `pallet_fee_control` | missing | no / no | no / no | no `pallets/fee_control/src/benchmarking.rs` |
| [x] | `pallet_inbound_transfer_log` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [x] | `pallet_mining_slot` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [ ] | `pallet_mint` | missing | no / no | no / no | no `pallets/mint/src/benchmarking.rs` |
| [ ] | `pallet_notaries` | missing | no / no | no / no | no `pallets/notaries/src/benchmarking.rs` |
| [x] | `pallet_notebook` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [x] | `pallet_operational_accounts` | implemented | yes / yes | yes / yes | full suite + generated weights |
| [ ] | `pallet_price_index` | missing | no / no | no / no | no `pallets/price_index/src/benchmarking.rs` |
| [ ] | `pallet_ticks` | missing | no / no | no / no | no `pallets/ticks/src/benchmarking.rs` |
| [ ] | `pallet_treasury` | placeholder-only | no / no | no / no | `pallets/treasury/src/benchmarking.rs` placeholder |
| [x] | `pallet_vaults` | implemented | yes / yes | yes / yes | full suite + generated weights |

Runtime-dispatched upstream benchmarks:

| Complete | Benchmark Target | Suite Source | Dispatched (Argon/Canary) | Weights Present (Argon/Canary) | Notes |
| --- | --- | --- | --- | --- | --- |
| [x] | `frame_benchmarking::BaselineBench` | upstream | yes / yes | n/a | baseline harness benchmark |
| [x] | `frame_system` | upstream | yes / yes | yes / yes | `frame_system.rs` |
| [x] | `pallet_balances::Balances` | upstream | yes / yes | yes / yes | `pallet_balances_balances.rs` |
| [x] | `pallet_balances::Ownership` | upstream | yes / yes | yes / yes | `pallet_balances_ownership.rs` |
| [x] | `pallet_timestamp` | upstream | yes / yes | yes / yes | `pallet_timestamp.rs` |

### Provider-weight pattern adoption

- Shared benchmark providers are centralized in `pallet_prelude::benchmarking` and exported via
  `runtime/common/src/benchmarking.rs`.
- Provider-weight adapters/wrappers are in pallet `weights.rs` (not `lib.rs`) for:
  - `pallet_block_seal`,
  - `pallet_block_seal_spec`,
  - `pallet_chain_transfer`,
  - `pallet_notebook`,
  - `pallet_mining_slot`,
  - `pallet_block_rewards` (wrapper shape present).

### Validation status

- targeted benchmark regeneration has been run for `pallet_block_seal`,
  `pallet_block_seal_spec`, `pallet_chain_transfer`, and `pallet_notebook` on argon/canary,
- `cargo check -p argon-runtime -p argon-canary-runtime --features runtime-benchmarks`,
- `cargo make fmt`,
- `cargo make lint`,
- no `UNKNOWN KEY` / `:argon:bench:` artifacts in targeted generated weight files.

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

## Risks

- benchmark providers can drift from production behavior if benchmark setup assumptions are not kept
  explicit in benchmark code,
- provider call counts in benchmark setup can regress if benchmark preconditions change silently.
