---
name: independent-diff-review
description: Find actionable bugs in a Mainchain PR, branch, commit, or working diff using a fresh reviewer, producer-to-consumer tracing, and explicit attempts to disprove findings. Use for an independent correctness review or a Copilot review replacement. Covers runtime, economic accounting, migration, hook, and node boundaries where changed.
---

# Independent Diff Review

Find a concrete supported input or production history that makes the change produce the wrong result. Review the code independently of the implementation author's explanation. A clean report is valid; do not manufacture findings to meet a quota.

## Freeze the review scope

- Record the checkout, base, head, and changed paths. Resolve what the user wants reviewed before choosing a diff; a PR's base may be earlier than the head's immediate parent. For working changes, also record the staged/unstaged/untracked partition and a diff/content fingerprint, then check it again before reporting.
- Verify the requested paths actually changed. If the scope is empty, report the mismatch and obtain the correct range instead of choosing a likely replacement. For a partially empty scope, identify the unchanged paths and continue on the actual changes.
- Compare against the actual baseline. Read dependencies, callers, configuration, and tests at the reviewed revision; do not silently mix current checkout files with an older head.
- Capture the intended outcome and explicit supported deployment/upgrade constraints from the user and repository. Keep these separate from the author's claims that an implementation is safe. A task-specific user decision takes precedence over a generic repository convention; an undocumented assumption is an open question.
- Review only by default. This activity does not authorize fixes, staging, commits, pushes, GitHub comments, or resolving threads.

## Use one fresh reviewer

If coordinating work you implemented or have already discussed, use one fresh independent reviewer. Honor the provider the user selects. Choose one available provider for this pass: use the [Claude Code reviewer procedure](references/claude-code.md) when selected and authorized, or delegate to an independent subagent with `fork_turns: "none"`. Provider diversity is an option, not evidence of higher quality. Supply the packet below. For independent discovery, withhold the implementation conversation, suspected defects, proposed fixes, prior reviewer findings, and claims that tests prove correctness. For feedback validation, supply the comments being assessed as candidates, with their original and requested current revisions. If you are already that independent reviewer, perform discovery directly; do not recursively delegate.

```text
Review this exact change for actionable introduced bugs and incomplete fixes
within the stated outcome. Label pre-existing gaps separately.
Review mode: <discovery, existing-feedback validation, or behavioral readiness>
Required verdicts: <findings only, or applicable boundary reviews and test evidence>
Repository: <absolute path>
Scope: <base and head, or immutable patch/snapshot and fingerprint>
Intended outcome: <brief user-visible/domain goal>
Constraints: <explicit supported versions, deployment scope, product decisions>
Effort cap: <agreed time or turns, when bounded; disclose remaining coverage>
Use <absolute path to this SKILL.md>; you are the independent reviewer.
First confirm access to this procedure and the exact reviewed source objects.
Read source, callers, producers, and tests at the reviewed revision.
For discovery, do not read prior review comments or later fixes.
For feedback validation, inspect the supplied comments and original/current source.
In either mode, keep task transcripts, memory, and benchmark answers out of context.
Do not modify repository files or external state.
For readiness, the same reviewer inspects the applicable repository gates and
returns their required verdicts with evidence; skipped gates remain blocked.
Return candidates with trigger, causal trace, expected versus actual outcome,
baseline comparison, exact locations, disconfirming evidence checked, and gaps.
```

Default to one discovery reviewer plus coordinator validation. Split only a change with genuinely separate workflows that one reviewer cannot cover coherently; partition by workflow and give each the same neutral constraints. For a large migration, plan that partition before discovery. Unexamined changed paths remain a review gap even when sampled paths yield good findings. If effort is capped, record elapsed effort and whether the cap was reached or work stopped earlier; spend remaining discovery effort on unexamined relevant workflows rather than polishing an already adequate report.

Do not run several reviewers with the same author-led brief. Use the selected provider's configured model unless the user requested another. Record the actual provider/model and completion evidence; never label a review as Claude or another provider merely because the prompt requested it.

If independent delegation is unavailable, disclose that limitation; do not present the author's self-review as independent.

## Discover from contracts

First read the diff, including deletions, and identify the changed behaviors. Before reading test expectations or the PR's correctness narrative, follow each important changed value or transition to its authoritative producer and its actual consumer. Use a short scratch ledger: changed assumption, source contract, affected consumer, plausible counterexample. Inspect analogous established flows where they clarify the contract.

Choose the probes that match the change. For runtime or node work, also read [the Mainchain boundary probes](references/mainchain.md) and select only the relevant rows:

| Changed surface | Concrete probe |
| --- | --- |
| Values, filters, lookups, totals | Can the producer supply zero, absent, unknown, pending, a different identity/kind, or multiple records? Does the consumer treat those as the same thing? Check units, cardinality, fallback values, and membership from real definitions. For accounting, reconcile intended allocations with completed side effects and downstream notifications; identical final balances can hide different domain accounting. |
| Removed code or changed ownership | Which supported caller, durable record, platform, configuration, or previous release still depends on the removed behavior? Trace its replacement end to end. |
| Async work, recovery, caches | Start with valid state, cross the changed boundary, and follow failure/retry/restart or newer live work only where at risk. Inspect the actual shared queue/promise/cache behavior and the already-mounted observer. |
| Repeated events or polling | Follow the emitted event into its listeners. Derive work per event and total work across independently growing collections from the actual loops; distinguish constructing a client from performing I/O. |
| Build, process, service or runtime boundary | Inspect the invoked command/configuration and the real implementation/type at both ends. Establish which platform/version pairings are supported before alleging incompatibility. |

Follow a promising path beyond the diff until its outcome is established or contradicted. A helper name, type name, comment, or generic library rule is a search lead, not proof. Do not broaden into an unrelated repository audit.

Before closing discovery, make a short second sweep of changed guards, early exits, fallback values, loops, and error/await paths. Check whether each condition measures the state its caller actually needs. Account for each changed path as reviewed, outside the requested scope, or still unexamined. This sweep is source inspection, not a demand for a test per branch; it prevents one deep investigation from consuming the whole review.

For each candidate, write: **supported trigger → changed path → incorrect observable/durable result → violated contract**, plus how the baseline behaves. Distinguish a new regression, an existing defect made newly reachable, and a pre-existing issue. For a bug-fix PR, check the named outcome across its supported entry points; an incomplete correction can be a relevant scope gap without being newly introduced. Group locations with the same root cause into one finding.

## Try to disprove the candidates

The coordinator checks each candidate against source at the frozen revision. An independent reviewer working alone performs this pass after discovery, without replacing the need for independence when reviewing their own implementation.

1. Verify reachability: find the real producer of the disputed value/history and the actual caller that reaches the outcome. A test fixture that invents an impossible state is insufficient.
2. Search for protections outside the hunk: normalization, guards, idempotent consumers, transaction rollback, retry ownership, framework behavior, and deployment guarantees. Read their implementations.
3. Challenge external claims using the pinned dependency source or authoritative documentation. Verify the actual version, representation, overload, and side effects that determine the contract.
4. Validate the defect independently of the suggested fix. A real bug can come with an incorrect explanation or an overbroad remedy. Report the smallest necessary correction direction, without prescribing an unverified implementation.
5. Use the smallest decisive evidence: an exact deterministic source trace, an existing behavior test, or a focused reproduction in temporary space. Do not require new test infrastructure for an obvious trace. Do not edit production code during review.

If evidence contradicts the reviewer, return that concrete evidence once for reconsideration. Do not dismiss a finding merely because the author intended the behavior or tests passed. Conversely, do not retain it merely because the reviewer insists. Classify each candidate as confirmed, disproved, or unresolved; keep product/support decisions and verification gaps separate from confirmed bugs.

## Apply repository requirements

Read the review and test requirements that apply in the reviewed repository and revision. Read `CLAUDE.md` and relevant `.claude/` guidance at that revision when present. Resolve repository-local procedures from the reviewed repository; do not assume skills or commands from another checkout exist. Carry the frozen scope, explicit user constraints, and source restrictions into every procedure. Discovery alone does not complete an implementation task or satisfy a requested readiness verdict; repository-required reviews and checks still apply.

If a required contract document is absent at the reviewed historical revision, report that limitation instead of substituting a newer contract. A source-confirmed bug does not require a newly written test to be reported. Declaring a behavioral correction complete requires the repository's checks and independent evidence at the real changed boundary. Missing verification is a gap, not proof of a production bug.

For explicitly bounded discovery or historical calibration, use applicable provenance, transition, and ownership probes without claiming full readiness. Later corrections and historical comments belong to coordinator calibration after the discovery report is fixed. Required readiness gates remain open until completed.

## Assess existing GitHub feedback

When asked to assess existing comments, use the same source validation and disproof procedure with the comments as supplied candidates. Label this as feedback validation, not blind discovery. Record each comment's reviewed commit, path, and root claim; compare that original revision with the requested current revision. Distinguish a valid original defect that is now fixed, a still-present defect, a duplicate, a pre-existing issue, a disproved claim, and an unresolved support or product assumption. A resolved thread, accepted reply, or later patch is corroboration rather than proof. Validate the consequence separately from the proposed remedy. Posting replies or resolving threads requires separate authorization.

## Report and stop

Lead with confirmed findings, ordered by impact. Each finding needs a concise title, precise file/line or symbol, trigger, causal explanation, user/domain consequence, and evidence. Cite the changed location plus the producer/consumer that establishes the contract when needed. Avoid style requests and speculative hardening without a supported failure.

Close with the reviewed revision/fingerprint, meaningful coverage limits, and unresolved assumptions or missing checks. Preserve a concise disposition for each finding submitted by the independent reviewer: confirmed, disproved with the protecting source, or unresolved with the missing fact. A linked review record can hold these dispositions; exploratory leads need not become findings.

Use `No confirmed findings` when appropriate; this does not imply the change is merge-ready. Include test-evidence and boundary verdicts only when the repository or requested readiness review requires them and those reviews were performed, with their decisive evidence. Do not certify skipped gates.

Stop after the relevant workflows and candidate validation are covered. Repeat review only for changed behavior or a concrete unresolved concern. If the source changed during review, identify the stale evidence and recheck the affected scope before a final verdict.

Keep historical evaluation records outside the installed skill. Only the coordinator maintaining or benchmarking it reads those records, after discovery reports are fixed; never supply known answers to independent discovery reviewers.
