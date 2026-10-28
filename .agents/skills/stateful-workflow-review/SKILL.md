---
name: stateful-workflow-review
description:
  Independently audits state provenance, authority, transitions, recovery, replay, retry, migration,
  backfill, and observer convergence. Use for implementation or review when a change touches
  persistent state, event ingestion, historical data, asynchronous retries, queues, subscriptions,
  caches, alerts, startup restoration, process restart, or any workflow reconstructed from stored or
  external state. Also use retrospectively to determine which missing production history or system
  boundary caused a defect.
---

# Stateful Workflow Review

Audit the system model independently of the proposed implementation. The implementation and PR
description are evidence, not the definition of all possible production state.

## Operating Boundary

- Default to report-only. Do not edit, stage, commit, push, or change branches unless the user
  separately asks for fixes.
- Review a current diff, branch, PR, commit range, or historical defect chain. Resolve the exact
  base and head before analysis.
- Use concrete repository and history evidence. Mark unsupported assumptions and unknown production
  variants explicitly.
- Do not call multiple reviewers with the same implementation-led brief and treat their agreement as
  independent completeness evidence.

## Workflow

1. Resolve the change and its stated intent.
2. Run `scripts/detect_stateful_diff.py` against the resolved base and head. Treat its signals as
   routing hints, never proof of correctness or completeness.
3. Read `references/gate-rubric.md` and `references/mainchain.md` completely.
4. Establish the production-origin universe independently of the implementation:
   - establish the repository's declared compatibility window before classifying versions as
     supported;
   - decide whether the set of origins is closed, versioned, or open-ended;
   - enumerate lifecycle enums, event variants, storage states, transaction outcomes, and supported
     historical schema/runtime versions from their authoritative definitions;
   - compare generated event registries, runtime/client types, migrations, previous release
     behavior, and analogous consumers when no single definition is complete;
   - require an explicit safe disposition for unknown variants inside the supported window. A list
     inferred from handlers, tests, or files changed by the PR is not completeness evidence. Do not
     require semantic handling for unspecified future versions outside the declared window. An
     actionable unsupported-version outcome that preserves valid state is sufficient.
5. Reconstruct authoritative producers before judging consumers:
   - durable tables and migrations;
   - current and historical chain or service events;
   - live subscriptions, indexers, polling, and external observations;
   - import, restart, recovery, replay, replacement, and backfill paths.
6. Build a provenance matrix. Each supported production origin must map to normalization, durable
   state, loaded state, observers, and a terminal outcome. Include an exclusions row for
   authoritative variants that do not apply, with the domain reason.
7. Build a transition and side-effect map. For every durable or external boundary, trace success,
   failure after the boundary, retry, restart, and concurrency with newer live work.
8. Build a user-visible transition map. For each step, record what valid information remains
   visible, what disappears or becomes unavailable, the pending or error state shown, and the event
   that makes an already-mounted consumer leave that state. Treat loss of valid visible information
   and pending states without a reachable exit as defects unless an authoritative invalidation
   requires them.
9. Inventory mechanisms derived from state: subscriptions, queues, caches, indexes, alerts, timers,
   and UI readiness. Verify they are rebuilt or resynchronized after reconstructed state changes
   their predicates.
10. Read `references/test-evidence.md` completely and grade only tests relevant to the claimed
    invariants. Do not reward test counts.
11. For retrospective review, verify that each claimed gap is absent on the original head and
    addressed by the corrective change. Distinguish the introducing assumption from the proximate
    defect.
12. Render the report using `references/report-format.md`.

## Decision Rules

Return `BLOCKED` when any of these is supported by evidence:

- a supported production origin has no disposition;
- the origin universe is derived only from the proposed handlers or tests, with no independent
  enumeration or safe fallback;
- a closed event, status, or schema variant is silently absent from the provenance matrix;
- historical and live authority are ambiguous;
- staged or replayed state can become observable before its declared commit boundary;
- failure after a durable side effect has no idempotent retry or restart behavior;
- reconstructed state changes an observer predicate without rebuilding that observer;
- a transition hides or discards valid user-visible information before replacement state is
  authoritative, without a domain requirement to do so;
- a visible loading, pending, blocked, or recovery state has no evidenced event that makes an
  already-mounted consumer leave it;
- a retry or recovery loop lacks success, explicit quarantine, or actionable terminal failure;
- the only tests derive their scenario universe from the implementation or assert wiring instead of
  domain outcomes.

An open-ended fallback in code does not by itself make every hypothetical future variant a supported
origin. Return `NEEDS DECISION` when the repository has not declared whether later versions are
supported, or when accepting them is a product choice rather than a defect in the current
compatibility window. Return `PASS` when out-of-window versions stop the affected workflow with an
actionable upgrade requirement while preserving valid state. Return `PASS` only when no blocking
exception remains; do not require prose for cells already proved by code or behavior tests.

## Review Economy

- Report exceptions and decisive invariants, not a tutorial on the whole subsystem.
- Prefer one counterexample with exact provenance over a long list of speculative risks.
- Recommend the smallest evidence that closes a gap: an invariant, exhaustive normalization, a
  focused boundary test, or a demonstrated architecture correction.
- Do not prescribe broad E2E expansion when a real-database service test can cross the relevant
  boundary more deterministically.
