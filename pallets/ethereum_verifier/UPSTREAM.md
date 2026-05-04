# Upstream Provenance

This pallet starts from Snowbridge's Ethereum beacon client pallet.

Upstream source:

- crate: `snowbridge-pallet-ethereum-client`
- version: `0.18.2`
- repository: `https://github.com/paritytech/polkadot-sdk`
- path: `bridges/snowbridge/pallets/ethereum-client`
- local source used for import: Cargo registry crate cache

## Imported Paths

- `src/`
- `tests/fixtures/`
- `README.md`

## Imported Verification Helpers

The `src/upstream/` module contains local imports from `snowbridge-verification-primitives` so the
pallet can avoid a direct dependency on that crate while keeping the imported verifier mechanics
reviewable.

These files need their own check-in point when this work is committed:

1. import the upstream helper files with their source versions recorded in the file headers
2. commit that import before wiring Argon calls to the helpers
3. keep later Argon edits separate so `git diff` can show exactly what changed from upstream

Current imported helper sources:

- `src/upstream/verification.rs`: `snowbridge-verification-primitives 0.8.1`, `src/lib.rs`
- `src/upstream/receipt.rs`: `snowbridge-verification-primitives 0.8.1`, `src/receipt.rs`

The indexed receipt helper is a security boundary. Runtime receipt verification should use
`src/upstream/receipt.rs` so the proof is bound to the receipt trie key derived from the transaction
index. Do not route runtime proof verification back through the older non-indexed
`snowbridge_beacon_primitives::receipt::verify_receipt_proof` helper.

## Argon Delta Policy

Keep the beacon verification path recognizable against upstream Snowbridge. The main reason for
forking this pallet is to avoid reimplementing the consensus verifier while still letting Argon own
the public runtime surface.

Argon-specific changes should stay focused on:

- runtime metadata and pallet naming
- fee-control and tx-pool key integration
- retained execution header anchors needed by Argon crosschain transfers
- governance or emergency controls that differ from upstream
- crosschain-transfer verifier traits and asset binding
- small local utility extraction when it avoids importing broad Snowbridge bridge code that is not
  part of Ethereum verification

Avoid casual rewrites inside the sync committee, finality branch, ancestry, or receipt-proof
verification flow. When upstream Snowbridge changes those paths, compare against this fork before
making local edits.

## Replaceable Local Verification Code

`src/execution_proof.rs` is an Argon extension, not part of the imported Snowbridge beacon client
surface. It should remain the only place that connects Argon-owned proof models to execution proof
checks.

The execution proof path has two different jobs:

- execution header chain verification from a user burn block to a retained Argon anchor; this is
  Argon-specific, lives in `src/execution_proof.rs`, and is expected to stay local
- indexed receipt proof verification against the target execution header receipts root; this is a
  temporary imported helper in `src/upstream/receipt.rs` that should be replaced once a compatible
  upstream Snowbridge helper is available in the dependency train Argon can use

Keep the temporary receipt proof boundary narrow:

- keep `alloy_trie::proof::verify_proof` usage inside `src/upstream/receipt.rs`
- do not spread receipt trie decoding or value-extraction logic into the pallet call path
- require receipt proofs to carry the transaction index needed to derive the receipt trie key
- prefer delegating to an upstream helper that accepts
  `(receipts_root, transaction_index, proof_nodes)` and returns a decoded receipt or equivalent
  verified value
- when replacing the local helper, record the upstream crate version and function here
