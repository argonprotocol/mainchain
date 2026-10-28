# Test Evidence Rubric

Do not use test count, line coverage, or mock call coverage as evidence of stateful correctness.

## Keep Coverage Proportional

Start with relevant existing tests. Reuse coverage that already detects incorrect behavior; extend
an existing scenario when it lacks a meaningful assertion. Add a new scenario only for a concrete
uncovered risk in the changed behavior. No new test is required merely because a PR or review
exists.

Inspect supported production paths to understand risks, but do not require a separate test for every
producer, branch, flag, or lifecycle combination. Add failure, retry, restart, or mounted-consumer
coverage only when the change puts that behavior at risk and existing tests do not catch it.
Recommend the smallest test change that closes the gap; use the existing test infrastructure when it
can exercise the behavior.

## Strong Evidence

A strong test identifies:

- the production history that creates the state;
- the durable, temporal, or external boundary crossed;
- the domain invariant asserted;
- the resulting state or observable outcome relevant to the changed behavior;
- for a bug fix, why it fails on the defective revision.

When the risk is user-facing recovery, start with valid visible or actionable state. Cross the
failure or reconstruction boundary, then prove both information preservation and the already-mounted
consumer's exit from pending state.

Use FRAME `TestExternalities` and the real configured pallets for runtime storage boundaries. Keep
the Currency, Mint, Vault, Lock, Fission, and Treasury implementations real when their interaction
is the invariant under review. Test repeated hooks or callbacks over the same storage when failure
or retry is at risk. A storage container alone is not integration evidence: inspect which production
components the fixture replaces and assert the resulting durable state. Pure calculations do not
need a runtime fixture.

Prefer a focused integration scenario when the risk lies in components working together. Use the
smallest setup that exercises the production flow. Fake only the external chain, indexer, transport,
clock, or process boundary needed to make the history deterministic; keep the boundary under review
real. Full application or network setup is appropriate when the behavior depends on it, not as a
default requirement.

## Weak or Misleading Evidence

Treat these as little or no gate evidence:

- inputs assembled from the implementation and asserted unchanged at the output;
- mocks that return the exact state the consumer expects;
- assertions that one helper called another helper;
- one test per branch or flag combination without deriving the input universe from producers;
- recovery proven only by reloading the entire app when live consumers should converge immediately;
- persistence asserted without subscriptions, queues, caches, alerts, or mounted consumers;
- recovery that starts empty and therefore cannot detect loss of previously valid visible data;
- tests that cannot fail on the defective commit.

## Required Test Inspection

Inspect added or changed tests and any existing scenarios claimed as evidence. This procedure also
applies to behavioral changes that do not require a stateful review; do not expand their scope
merely to satisfy this rubric.

1. Follow each relevant test into the production code it actually executes. Identify the real state
   owner, calculation, transition, or boundary being verified.
2. Read the mocks, spies, fixtures, and setup writes. Check whether they replace that behavior or
   prepopulate the outcome later asserted. External inputs may be mocked; the claimed production
   result must not be supplied by a mock.
3. Read the outcome assertions. Establish their expectation from the production invariant, protocol
   contract, or original incident. Explain a plausible incorrect production behavior that would make
   those assertions fail. Mock calls are useful only when they establish the actual external
   contract or side effect under review; calls alone do not prove a domain transition.
4. For a bug fix, inspect the failing-before and passing-after results. The old behavior must fail
   on the defective outcome, not merely on incompatible imports, compilation, or fixture setup.
   Missing or unrun evidence remains a gap.
5. Replace, strengthen, or remove newly added tests that only echo mocked results or prove wiring.
   Do not count them toward readiness because another meaningful test passes.

Return `Test evidence: PASS` or `BLOCKED` and cite the decisive scenario, real production path,
mocked boundaries, and outcome assertion. Group related scenarios and report exceptions; do not add
a per-test dossier. A bare assurance that tests are real, or that no mock tests were added, is not a
completed inspection.

## Valuable Stateful Scenarios

These are candidate scenarios, not a checklist. Select only those needed to address a concrete
uncovered risk in the changed behavior:

- supported origins of the same terminal domain state that exercise distinct behavior;
- failure immediately after each durable or external side effect;
- retry after temporary failure;
- restart over the same durable data;
- newer live state arriving during older replay;
- duplicate, delayed, or reordered external facts where supported;
- migration followed by ordinary startup and observation;
- bounded unsupported-state behavior.

Fold a regression into an existing scenario when it is the same lifecycle. Add a new test only when
a distinct production history, boundary, or invariant at risk in this change is not already covered.
