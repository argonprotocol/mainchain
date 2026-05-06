This document describes the options to move Argon tokens across chains.

## Mainchain <-> Localchain

Each account in Argon exists as both a Mainchain and a Localchain account, even if you have not
transacted on one side yet. Argon allows native transfers between the Mainchain and each account's
corresponding Localchain.

Instructions to perform these steps can be found [here](./localchain.md#creating-a-localchain).

## Argon <-> Ethereum

Argon currently supports an Ethereum inbound transfer flow.

The current implemented flow is:

1. Burn `ArgonToken` or `ArgonotToken` through the Ethereum `MintingGateway`.
2. Include the destination Argon account in the burn payload.
3. Wait for the Ethereum execution block containing that burn to finalize through the beacon chain.
4. Build an Ethereum event proof for the `BurnForTransfer` log.
5. Submit `crosschainTransfer.proveTransfer` on Argon mainchain.
6. If the proof, gateway, token, and nonce are all valid, Argon mainchain settles the matching
   funds to the destination account.

What exists today:

- Ethereum to Argon mainchain inbound settlement
- support for both Argon and Argonot burn proofs
- replay protection through per-source-account nonces
- inbound-only Ethereum support in this flow

For the implementation-oriented overview, see
[Crosschain Transfer V1: Ethereum Inbound](./technical/crosschain-transfer-v1-ethereum-inbound-design.md).

## Argon <-> EVM Chains

Cross-chain EVM transfers will be documented here once the new bridging mechanism is available.
