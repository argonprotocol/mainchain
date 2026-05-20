This document describes the options to move Argon tokens across chains.

## Mainchain <-> Localchain

Each account in Argon exists as both a Mainchain and a Localchain account, even if you have not
transacted on one side yet. Argon allows native transfers between the Mainchain and each account's
corresponding Localchain.

Instructions to perform these steps can be found [here](./localchain.md#creating-a-localchain).

## Argon <-> Ethereum

Argon currently supports an Ethereum inbound transfer flow.

The current implemented flow is:

1. Call `startTransferToArgon(...)` on the Ethereum `MintingGateway`.
2. Include the destination Argon account in the gateway activity payload.
3. Wait for the Ethereum execution block containing that activity to finalize through the beacon
   chain.
4. Build an Ethereum gateway-activity proof batch for the ordered `TransferToArgonStarted` log range
   you want to settle.
5. Submit `crosschainTransfer.proveGatewayActivity` on Argon mainchain with that proof batch.
6. If the proof, gateway, token, and ordered gateway activity nonce are all valid, Argon mainchain
   settles the matching funds to the destination account.

What exists today:

- Ethereum to Argon mainchain inbound settlement
- support for both Argon and Argonot gateway activity proofs
- replay protection through ordered gateway activity nonces
- inbound-only Ethereum support in this flow

For the implementation-oriented overview, see
[Crosschain Transfer V1: Ethereum Inbound](./technical/crosschain-transfer-v1-ethereum-inbound-design.md).

## Argon <-> EVM Chains

Cross-chain EVM transfers will be documented here once the new bridging mechanism is available.
