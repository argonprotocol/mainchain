# Mainchain state boundaries

Use these probes with the generic gate rubric. They identify relevant evidence; they do not require
a test for every branch or hypothetical state.

## Establish supported origins

- Resolve the runtime and storage versions that the change actually supports. Read historical SCALE
  layouts and migration registration, rather than treating every older or future version as
  supported.
- Enumerate origins from authoritative extrinsics, inherent and UTXO event types, hook schedules,
  provider traits, and storage definitions. Include supported duplicate, delayed, and reordered
  Bitcoin observations.
- Identify which external facts are confirmed, which are pending requests, and which are derived
  accounting. A transaction request, a Vault signature, and an observed Bitcoin spend are different
  facts.

## Trace the commit boundary

- Inspect the pinned FRAME implementation for dispatch, hooks, inherents, migrations, and ordinary
  provider calls. Do not infer whole-workflow rollback from `try_mutate`, sequential Rust
  statements, or an error return.
- Trace mutations to Currency holds and issuance, Vault allocations and release schedules, Lock and
  Fission records, Mint queues and repayment counters, Treasury positions, and operational-account
  hooks where affected.
- For hook work removed with `take` or `drain`, establish the disposition after a later failure:
  retry scheduling, retained observation, quarantine, or an actionable terminal error. A logged
  error does not itself restore pending work.
- For Bitcoin callbacks, follow the UTXO tracker and its caller as well as the receiving pallet.
  Establish which owner retains the observation when the callback fails.

## Check derived state and consumers

- Reconcile authoritative records with owner/Vault/Fission indexes, funding and expiry buckets,
  release and cosign schedules, mint indices, and aggregate financial positions.
- Follow migration changes through those same indexes, schedules, hooks, and normal post-upgrade
  operations. Version writes belong after the corresponding conversion has completed.
- Treat committed runtime storage and finalized events as the publication boundary for clients.
  Inspect affected client, indexer, or Apps consumers when event or metadata contracts change; do
  not require unrelated Vue or SQLite tests for a runtime-only transition.
- Distinguish a source-confirmed state defect from missing live-chain qualification. Synthetic state
  cannot establish which production records exist; full-state replay cannot replace lifecycle and
  failure-path review.

## Grade evidence at the changed boundary

Use existing pallet and runtime integration fixtures first. A test with a mocked Vault cannot prove
real Vault collateral accounting, and a no-op Mint provider cannot prove repayment or pending-mint
conservation. External Bitcoin, price, clock, signature, or transport inputs may be deterministic
adapters when the state transition being reviewed remains real.

For economic state, reconcile BTC quantities, paid and activated securitization, pending
reservations, actual burns and issuance, mint promises, holds, and release maturities. Separate
expected protocol policy from implementation claims, and state deferred solvency or compatibility
decisions explicitly.
