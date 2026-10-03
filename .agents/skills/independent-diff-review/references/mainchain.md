# Mainchain boundary probes

Select only the rows relevant to changed behavior. Establish the pinned framework and production configuration before alleging a defect.

| Changed boundary | Source trace |
| --- | --- |
| Runtime writes and cross-pallet calls | Enumerate the state writes and side effects in their actual order. Check which entry point owns rollback: dispatchable, hook, migration, inherent, or ordinary trait call. Inspect transaction attributes and the pinned framework implementation; an error return alone does not prove either rollback or partial commit. Follow every supported caller. |
| Economic quantities and lifecycle accounting | Name the units and the stock/flow identity being preserved. Reconcile issuance, balances, holds, pending obligations, and backing across the transition, including independently arriving funding/spend/expiry/release facts. Check partial settlement, zero, rounding, and terminal leftovers only where supported. Names such as minted or released do not establish when an obligation becomes spendable or disappears. |
| Migrations and runtime/client compatibility | Start from the actual previous storage layout and version. Trace transformed rows, aggregate/index repair, hooks, events, and generated client consumers together. Check both configured runtimes and real provider-trait wiring where changed. A version bump or compiling candidate client is not migration or deployed-runtime evidence. |
| Hooks, bounded collections, and block work | Follow the actual insert/remove/iterate path, full-capacity behavior, and caller's response to an error. Distinguish a bound on one collection from a bound on total work per block. Establish reachable cardinality, frequency, weights, and retry/retention behavior; do not propose a persistent aggregate without evidence that the real workload needs it. |
| Node state, observers, and external chains | Trace proposal/import/verification and background callers through shared locks and caches. Check identity and finality of the snapshot they combine, durable checkpoint/publication order, and supported fork, duplicate, restart, or missing-data paths. An outer mutex or RPC method name is not proof of serialization or consensus authority. |

Use existing test launchers and fixtures for a necessary reproduction. Do not run formatters, broad service launchers, mutating lint tasks, or the release task as part of source review. A bounded source-only assignment runs none of these commands.
