# Crosschain Transfer V1: Ethereum Inbound

This note covers the inbound Ethereum burn-proof flow implemented by `pallet_crosschain_transfer`,
`pallet_ethereum_verifier`, and the current client proof-building path.

## Scope

The current `crosschain_transfer` pallet is responsible for:

- accepting finalized Ethereum `BurnForTransfer` events
- verifying those events through `pallet_ethereum_verifier`
- settling Argon or Argonot on Argon mainchain from the Ethereum burn-accounting balance
- tracking source-account nonces so a burn proof cannot be replayed
- marking recent inbound Argon transfers for operational-account eligibility
- migrating legacy Hyperbridge balances and known refund cases into the new burn-accounting model

The current slice is Ethereum-only and inbound-only.

## High-Level Flow

1. A user burns `ArgonToken` or `ArgonotToken` through the Ethereum `MintingGateway`, including the
   destination Argon account in the event payload.
2. The Ethereum beacon chain finalizes the execution block containing that burn.
3. The Argon client syncs finalized beacon state into `pallet_ethereum_verifier`.
4. A client builds an Ethereum event proof for the `BurnForTransfer` log.
5. The user submits `crosschainTransfer.proveTransfer`.
6. The verifier proves the log exists in a finalized Ethereum receipt under a known execution
   header.
7. `pallet_crosschain_transfer` validates the configured gateway, accepted token, and next nonce for
   the Ethereum source account.
8. The pallet pays the local recipient from the configured Ethereum burn-accounting account and
   emits `BurnNoticeAccepted`.
9. For inbound Argon, the pallet also records recent-transfer evidence used by
   `pallet_operational_accounts`.

## Runtime Model

- Activation is off until root sets an Ethereum `ChainConfig`.
- The settlement source is a runtime constant, `EthereumBurnAccount`, not a user-provided address.
- `ChainConfig` controls the accepted gateway address and token addresses, plus a bounded previous
  gateway cutover window.
- The verifier boundary stays generic: it proves a raw Ethereum log, and
  `pallet_crosschain_transfer` owns the `BurnForTransfer` event semantics.

## Related Code

- `pallets/crosschain_transfer`
- `pallets/ethereum_verifier`
- `client/nodejs/src/EthereumProof.ts`
- `client/nodejs/src/__test__/ethereum.e2e.test.ts`
- `chains/ethereum/contracts`
