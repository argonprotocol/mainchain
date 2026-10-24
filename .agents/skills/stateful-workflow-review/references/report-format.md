# Stateful Workflow Review Report

Keep the report compact and exception-focused.

## Required Shape

```text
Stateful workflow gate: PASS | BLOCKED | NEEDS DECISION
Scope: <base> -> <head>
Intent: <one or two sentences>

Decisive invariants
1. <authority, commit, convergence, or termination rule>

Blocking exceptions
- <production origin or boundary> -> <missing disposition or violated invariant>
  Evidence: <file:line, event/schema source, or historical commit>
  Required response: <smallest architecture, normalization, or test evidence>

Unresolved decisions
- <choice and its consequences>

Test evidence: PASS | BLOCKED
- Inspected: <decisive scenario, real production path, mocked boundaries, and outcome assertion>
- Strong: <scenario and boundary>
- Rejected as evidence: <wiring or input/output test and why>

Coverage
- Origin universe: <closed, versioned, or open; authoritative definitions inspected>
- Explicit exclusions: <variants and domain reason>
- Visible transitions: <valid information preserved, withheld information, and exit events inspected>
- Producers inspected: ...
- Durable owners inspected: ...
- Observers inspected: ...
- Unknowns: ...
```

Omit empty `Blocking exceptions` and `Unresolved decisions` sections. Do not include general best practices unless they directly close a reported exception.

## Finding Standard

Every blocking exception must name:

1. A supported production origin, transition, or observer.
2. The exact violated invariant or missing disposition.
3. Concrete repository or history evidence.
4. The smallest response that would make the gate pass.

Do not block on speculative states with no evidenced producer.
