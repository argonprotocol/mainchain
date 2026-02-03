# Mini RFC: Benchmarking Strategy for Argon Runtime

## Context
The previous benchmark effort ran against the real runtime configuration. That caused recurring issues:
- Benchmarks required real crypto/material (xpubs, signatures), making setup brittle.
- Cross-pallet event handlers (notebooks, slot changes, bitcoin flows) cascaded and were hard to measure without double-counting.
- Per-pallet benchmarks became tightly coupled to other pallets' runtime behavior.

We also want to split work into small, per-pallet PRs with clear review boundaries.

## Goals
- Deterministic, low-friction weight benchmarks per pallet.
- Avoid crypto and external validation in weight benchmarks.
- Avoid double-counting handler costs; make event-driven costs explicit.
- Support per-pallet PRs without huge runtime wiring diffs.

## Non-goals
- Full end-to-end scenario/perf benchmarks (keep separate).
- Economic/parameter changes (e.g., WeightToFee) as part of benchmark infra.
- Perfect modeling of real-world crypto costs inside weight benchmarks.

## Proposal

### 1) Two-tier benchmarking
- Tier A (weights): per-pallet, minimal-state benchmarks used to generate WeightInfo.
- Tier B (scenario/perf): end-to-end flows (notebooks, bitcoin lifecycle) measured separately and not used for weights.

### 2) Benchmark-aware runtime config
Under `cfg(feature = "runtime-benchmarks")`, override runtime Config types to benchmark stubs:
- BitcoinVerifier -> BenchmarkBitcoinVerifier (accepts deterministic inputs).
- NotaryProvider / NotebookProvider / AuthorityProvider -> static benchmark providers.
- EventHandler -> () or a no-op benchmark handler.

This keeps weight benchmarks deterministic and avoids real signature/validation requirements. The override is compile-time only and does not affect production runtime.

### 3) Event handler weight aggregation
Add weight reporting to event handler traits so we can charge handlers once, in a single place.

Example (conceptual):
- In primitives/src/providers.rs:
  - fn notebook_submitted_weight(header: &NotebookHeader) -> Weight;
  - Provide tuple impl that sums weights.
- In pallets/notebook:
  - Weight function includes T::EventHandler::notebook_submitted_weight(header).

This avoids double-counting and makes event-driven costs explicit and testable.

### 4) Benchmark-only setup helpers (optional)
If benchmark-only providers are sufficient, we should not need extra storage helpers. Only add
helpers if a pallet cannot be made deterministic via provider overrides.

### 5) Per-pallet PRs
Split work into small, reviewable PRs:
1. Infra PR: benchmark-only providers + event handler weight aggregation + mini RFC.
2. Per-pallet PRs: add or refine benchmarks for each pallet, using the shared pattern.
3. Runtime wiring PR: update weight modules and benchmark lists (including liquidity_pools -> treasury rename).

## Alternatives Considered
- Keep real runtime config for benchmarks: rejected (brittle, slow, double-count risk).
- Measure crypto cost inside benchmarks: possible but not required; can be added as fixed costs later.
- Charge event handler weight inside each handler: risks double counting and unclear attribution.

## Risks
- Benchmark-only providers could diverge from real runtime behavior if overused.
- Event handler weight aggregation must be kept in sync as handlers evolve.

## Keeping runtime changes minimal
To avoid littering runtime code with weight logic:
- Use `cfg(feature = \"runtime-benchmarks\")` directly on the associated types inside each runtime
  `Config` impl (so the override is visible where it is used).
- Keep event handler weight aggregation inside traits in primitives, not in runtime.
- Prefer tuple implementations with default `Weight::zero()` so runtime wiring stays unchanged.
- Limit runtime diffs to small, local blocks (no per-call additions in runtime logic).

## Open Questions
- Should crypto verification get a fixed weight constant, or be ignored in weights?
- Should event handler weights be charged by the source pallet (notebook) or the handler pallet?
- Which pallets should be prioritized for early PRs?

## Suggested Next Steps
- Implement infra PR (benchmark-only providers + event handler weight aggregation).
- Pick 1-2 pallets (e.g., bitcoin_locks, notebook) to validate the pattern.

## Agent Prompt (copy/paste)
You are in `/Users/blakebyrnes/Projects/DataLiberationFoundation/mainchain-work`.
Goal: finish benchmark/weight integration by pallet (reviewable PRs).
Use `./scripts/benchmark.sh` (template: `scripts/weight_template.hbs`) to generate weights.
Do not reintroduce `frame_benchmarking` weights; `pallet_ismp_grandpa` needs the rename fix.
Keep benchmark providers gated by `cfg(feature = "runtime-benchmarks")` using the
`use_unless_benchmark!` macro inline in runtime configs (`type Foo = use_unless_benchmark!(Prod, Bench);`).
Mining_slot benchmarks: `bid` fills cohort to capacity and bumps lowest bidder; ensure DB weights are present.
If benchmark output has duplicate “range of component” docs, de-dupe via the benchmark script.
Prefer minimal, reviewable changes; split PRs by pallet.
