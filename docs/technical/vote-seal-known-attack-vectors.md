# Known Vote-Seal Attack Vectors and Mitigations

## Purpose

This document captures known attack classes against the vote-seal block production flow and its
notary/notebook dependencies.

It is intentionally theory-first:

- Focus on attacker goals and protocol properties.
- Explain what controls exist and what gaps remain.
- Avoid deep code-level detail, so this stays useful as the design evolves.

## Scope

In scope:

- Tick-based block production.
- Notary notebook commit/reveal flow.
- Vote seals and miner selection.
- Fork choice behavior under adversarial timing.

Out of scope:

- Wallet UX issues.
- Generic infrastructure hardening unrelated to consensus safety/liveness.

## System Model (High Level)

1. Notaries produce notebooks per tick, including vote roots and secret commit/reveal material.
2. Miners use notebook-derived voting inputs to produce vote-sealed blocks.
3. Fork choice prefers higher fork power (vote-related criteria dominate compute paths in comparable
   contexts).
4. Finalization provides eventual immutability, but near-tip history is reorgable.

## Security Objectives

1. Safety: honest nodes should converge on the same canonical chain.
2. Liveness: chain should continue producing valid blocks under partial faults.
3. Fairness: no coalition should gain disproportionate block-winning power through timing
   manipulation.
4. Unforgeability: invalid notebooks, votes, or seals should be rejected deterministically.

## Threat Assumptions

1. Adversaries can collude across roles (notary, miner, voter).
2. Adversaries can delay, withhold, and selectively relay messages.
3. Adversaries may control multiple identities in one role.
4. Adversaries cannot break cryptographic primitives.
5. Finality is not instant; a bounded number of recent ticks are reorgable.

## Main Attack Classes

### 1) Late-Reveal Entropy Grinding

Attacker set:

- Notary + miner (possibly with selected voters).

Goal:

- Bias current tick winner selection by revealing notebook data after observing others.

Mechanism:

- Delay notebook reveal in reorgable window.
- Build alternate branch that includes late reveal and stronger seal.
- Use fork power to displace honest branch.

Current protections:

- Commit/reveal binds revealed secret to earlier commitment.
- Vote source proofs and signatures prevent direct forgery.

Residual risk:

- If entropy for current competition is sourced from near-tip (reorgable) ticks, timing manipulation
  remains possible.

Primary mitigation:

- Entropy maturity lag.
  - Let `R = max reorg depth in ticks`.
  - Require `entropy source tick <= current_tick - (R + 1)`.
  - Practical rule: set `D = R + 1` and source entropy from `D` ticks behind.

### 2) Reveal Withholding (Selective Participation)

Attacker set:

- One or more notaries.

Goal:

- Influence outcome by choosing whether to reveal.

Mechanism:

- Commit value in prior tick, then strategically withhold reveal when unfavorable.

Current protections:

- Invalid reveal values are rejected.

Residual risk:

- Withholding can still bias participation set unless punished.

Primary mitigation:

- Reveal-or-penalty policy.
  - Temporary ineligibility, slashing, or reduced weight for non-reveal.
  - Deadline must be protocol-defined and deterministic.

### 3) Private Relay and Propagation Advantage

Attacker set:

- Notary + miner.

Goal:

- Create private lead by sharing notebooks/votes to coalition peers first.

Mechanism:

- Differential propagation lets coalition mine before public network catches up.

Current protections:

- Deterministic validity checks prevent invalid private blocks.

Residual risk:

- Valid but privately relayed blocks can still gain head-start.

Primary mitigation:

- Network-level fairness improvements:
  - Better gossip fanout and peer diversity.
  - Tight monitoring of notary publication latency.
  - Incentive penalties for repeated late publication patterns.

### 4) Fork-Power Gaming by Notebook Inclusion Timing

Attacker set:

- Notary + miner.

Goal:

- Win fork choice by combining stronger seal with notebook-count or vote-power advantage.

Mechanism:

- Include additional timely notebooks on one branch to improve comparison metrics.

Current protections:

- Duplicate authored block-key constraints.
- One vote-seal block per tick constraints in runtime context.

Residual risk:

- If weighting strongly favors near-tip inclusion effects, manipulation can remain attractive.

Primary mitigation:

- Align weighting with matured data:
  - Avoid over-rewarding same-tick notebook additions in fork comparison.
  - Prefer stable, matured entropy and finalized-equivalent inputs.

### 5) Vote Censorship by Notaries

Attacker set:

- One or more notaries.

Goal:

- Exclude selected voters or redirect economic flow.

Mechanism:

- Omit or delay vote-bearing entries from notebooks.

Current protections:

- Vote signatures and minimum thresholds protect authenticity, not inclusion fairness.

Residual risk:

- Inclusion policy remains partly discretionary per notary.

Primary mitigation:

- Add policy and observability:
  - SLA metrics per notary for inclusion latency.
  - Reputation and penalty mechanisms for systematic censorship.
  - Diversity across active notaries.

### 6) Miner Equivocation / Duplicate Production

Attacker set:

- Single miner identity or related identities.

Goal:

- Gain advantage by producing multiple competing blocks for same opportunity.

Mechanism:

- Submit duplicate vote/compute variants for same effective key space.

Current protections:

- Duplicate authored block-key checks.
- Signature and authority-score verification.

Residual risk:

- Multi-identity miner coalitions can still coordinate without violating per-identity limits.

Primary mitigation:

- Strong identity economics and anti-sybil controls.

### 7) Voter-Miner Economic Collusion

Attacker set:

- Voters + miners.

Goal:

- Concentrate winning probability and payout flow.

Mechanism:

- Coordinated vote placement to increase expected seal competitiveness.

Current protections:

- Vote minimums and signature checks.

Residual risk:

- Economic collusion remains possible when protocol permits market coordination.

Primary mitigation:

- Keep vote-minimum calibration responsive.
- Monitor concentration metrics and adjust policy parameters.

### 8) Network Eclipse and Timing Distortion

Attacker set:

- Network-level adversary.

Goal:

- Make target node adopt inferior branch or delayed state.

Mechanism:

- Isolate target peers and control message ordering.

Current protections:

- Deterministic validity and eventual finality.

Residual risk:

- Short-term head selection can still be influenced during isolation.

Primary mitigation:

- Peer diversity, anti-eclipse networking, and delayed confidence for near-tip branch switches.

## Design Principles for Robustness

1. Use chain-derived state, not local wall-clock receive times, for validity decisions.
2. Separate safety controls (validity) from policy controls (authoring preference).
3. Source decisive entropy from matured history older than reorg risk.
4. Penalize strategic withholding, not just invalid data.
5. Keep fork-choice comparisons stable under normal propagation variance.

## Recommended Parameters

Define and publish these as explicit chain parameters:

1. `R`: maximum reorg depth considered realistic.
2. `D`: entropy maturity lag; require `D >= R + 1`.
3. `reveal_deadline_ticks`: deterministic reveal deadline.
4. `non_reveal_penalty_ticks` and/or slashing policy.
5. Fork-power weighting policy for notebook-related terms.

## Operational Monitoring

Track these metrics continuously:

1. Notebook close time after tick end.
2. Notebook publication delay distribution by notary.
3. Frequency of near-tip reorgs by depth.
4. Share of winning blocks correlated with late notebook arrivals.
5. Notary inclusion fairness and censorship indicators.

## Immediate Priority Items

1. Enforce entropy maturity lag relative to reorg window.
2. Define reveal-or-penalty semantics.
3. Re-evaluate fork-power notebook weighting for near-tip competition.
4. Add explicit security invariants tests for collusion scenarios.
