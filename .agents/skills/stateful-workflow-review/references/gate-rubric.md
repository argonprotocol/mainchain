# Stateful Workflow Gate Rubric

## Origin-Universe Proof

Before tracing handlers, prove what may arrive in production.

- Establish the declared compatibility window first. A version is supported because the repository contract, deployed/current pairing, or observed producer says so—not merely because a fallback parser accepts its number.
- Classify each origin set as closed, versioned, or open-ended.
- For a closed set, enumerate every authoritative event, enum, status, transaction outcome, and storage variant. Record why excluded variants cannot enter this workflow.
- For a versioned set, include every release, runtime, and schema version the application claims to restore or upgrade.
- For an unknown variant inside the supported window, verify a path that preserves progress, evidence, and an actionable disposition.
- Do not invent handlers for unspecified future versions outside the declared window. It is sufficient to preserve valid state, stop only the affected workflow, and require an app upgrade.
- Triangulate from definitions outside the proposed implementation: runtime/client metadata, generated registries, database schema and migrations, supported upgrade history, and analogous established consumers.

Handlers and tests can prove a modeled case works. They cannot prove the case list is complete when they supplied that list themselves.

## Provenance Matrix

Construct rows from actual producers, not branches in the consumer being reviewed.

| Production origin | Versions or temporal variants | Normalization | Durable owner | Loaded owner | Derived observers | Terminal outcome |
| ----------------- | ----------------------------- | ------------- | ------------- | ------------ | ----------------- | ---------------- |

Include current events, historical event versions, persisted schema versions, import and migration state, partial prior attempts, restart state, and live observations that can represent the same domain fact.

Add excluded authoritative variants as explicit rows. A missing row is not equivalent to an impossible case.

An unknown origin within the supported window needs an explicit disposition: unsupported with actionable error, quarantined for operator action, or safely ignored with evidence. It must not fall into unbounded polling.

## Authority Questions

- Which source wins when historical replay conflicts with newer finalized or live state?
- Is the relevant fact authoritative, derived, cached, or merely presentation state?
- At which exact boundary does staged work become observable?
- Can a restored flag contradict the underlying durable or external fact?
- Does a shared helper's name hide rollback, cancellation, or timeout semantics that differ from this workflow's needs?

## Transition Review

For every transition, identify:

1. Preconditions and authoritative input.
2. Durable writes.
3. External side effects.
4. Published in-memory state.
5. Derived observer changes.
6. Failure after each completed step.
7. Retry and idempotency behavior.
8. Restart recovery.
9. Concurrency with a newer live transition.
10. Success, quarantine, or actionable terminal failure.

Do not infer atomicity from sequential code. Database transactions, external calls, queue timeouts, and in-memory publication have different durability boundaries.

## Observer Convergence

Audit every mechanism selected from reconstructed state:

- subscriptions and event filters;
- queues and pending work indexes;
- caches and canonical-object identity;
- database indexes and lookup tables;
- alerts, timers, badges, and readiness gates;
- UI stores and already-mounted consumers.

Data reconciliation does not prove observer reconciliation. Verify an explicit rebuild, refresh, canonical-object merge, or other convergence mechanism.

## User-Visible Transition Map

For every durable or asynchronous transition, record:

| Transition | Valid information before | Visible information during | What is withheld or removed | Exit event | Visible terminal failure |
| ---------- | ------------------------ | -------------------------- | --------------------------- | ---------- | ------------------------ |

Check these invariants:

- Last-known-valid data remains visible while replacement or historical state is incomplete, unless an authoritative fact makes that data invalid or unsafe to show.
- A partial domain failure removes only the actions or values that depend on that domain; it does not blank unrelated valid state.
- Replayed or staged data does not incrementally replace newer visible state before the commit boundary.
- Every spinner, pending badge, disabled action, and recovery notice has a concrete success or terminal transition.
- Already-mounted consumers receive the transition; app reload is not the convergence mechanism.
- When information must be removed, the user receives a stable reason and an actionable or explicitly terminal outcome.

## Architectural Warning Signals

- new flags interpreting combinations of other flags;
- event-specific branches added to a broad recovery helper;
- replay that calls ordinary live mutation paths incrementally;
- retries keyed only to elapsed time rather than canonical state;
- multiple representations of the same workflow status;
- a test fixture constructed through the consumer's assumptions instead of a producer history;
- recovery success asserted only after a full app reload, masking missing live publication.
- valid loaded data cleared at the start of refresh, replay, migration, or recovery;
- a global loading flag used for one incomplete history domain;
- a spinner whose exit depends on an observer selected before reconstructed state was published.

These signals require investigation. They are not findings without a concrete violated invariant or uncovered production origin.
