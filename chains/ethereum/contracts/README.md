# Ethereum Contracts

This package holds the Ethereum-side contracts for the replacement Argon and Argonot tokens.

At a high level, the shape is:

- the tokens are plain ERC-20s
- the tokens trust one gateway address forever
- that gateway address is a proxy, so the gateway logic can change without changing the address the
  tokens trust
- the gateway handles the protocol behavior: burns for transfer, first-council initialization, and
  the one-time migration path
- a guardian can pause immediately
- the admin Safe owns gateway actions and upgrades today

## Visual Overview

```text
Users ----------------------> MintingGateway proxy ----------------------> ArgonToken / ArgonotToken
Guardian Safe -------------> pause()
Admin Safe ---------------> proxy upgrade / migrate / unpause
Admin Safe ---------------> ProxyAdmin --------> upgrade gateway implementation

The tokens only trust the proxy address.
The proxy runs MintingGateway implementation code behind that stable address.
```

At the basic level, that is the whole shape:

- users and admins talk to the gateway
- the gateway talks to the tokens
- the tokens trust one gateway address forever
- the gateway code can still change later because that address is a proxy
- emergency pause is separated from the normal admin path

## Contracts

- `ArgonToken.sol`
- `ArgonotToken.sol`
- `MintingGateway.sol`

The two token contracts are intentionally boring. They expose `mint(...)` and `burnFrom(...)`, but
only the gateway can call them. They do not have their own role system, and they do not expose a
public self-burn path.

The gateway is where the actual protocol behavior lives:

- steady-state transfer amounts are expressed in 6-decimal Argon runtime base units
- MintingGateway runtime-unit amount and collateral fields use `uint128` balances
- `startTransferToArgon(...)` is the primary user path: the caller supplies the runtime-unit amount
  Argon should credit plus an ERC-2612 permit signature, and the gateway scales that amount into
  exact token base units before it permits, burns, and emits the outbound proof event
- `initialize(...)` sets the owner, guardian, and first council summary when the proxy is created
- `migrate(...)` is the one-time owner migration path: it loads the exact Argon and Argonot
  migration balances from the prior contracts using base-unit calldata
- `forceUpdateActiveCouncil(...)` is the owner recovery seam: it replaces the stored active council
  summary without resetting `argonApprovalsNonce` or `argonApprovalsHash`, so later queue items can
  keep chaining from the same last applied update under the replacement council
- each council rotation carries the next `microgonsPerArgonot` floor value alongside the next
  council snapshot
- transfer-out requests bind to the current or immediately previous council number, and Ethereum
  uses the matching council-managed Argonot floor price window instead of trusting a
  request-supplied conversion rate
- the gateway stores both `microgonsPerArgonot` and `previousMicrogonsPerArgonot` so a transfer can
  resolve against the council window it was opened under
- queued council-approved updates chain each signed item to the previous queue item's signed hash,
  so Ethereum only needs to verify council signatures on council-segment tips: each council rotation
  item and the last council-approved item in the submitted batch
- `pause()` can be called by the guardian
- `unpause()` stays on the admin Safe owner path

In the app, that approval step can be hidden behind one "move" action. The user does not need to
think about allowance mechanics, but the token still enforces per-user burn authorization even if
the gateway logic changes later.

## What Can Change

There are only three things to keep in your head:

1. The token contracts are fixed. They are plain ERC-20s, and each one is deployed with one gateway
   address baked in.

2. The gateway address is stable. The tokens keep trusting that one proxy address. They do not have
   a `setGateway(...)` path.

3. The gateway code is upgradeable. The proxy points at a `MintingGateway` implementation, and that
   implementation can be replaced later.

Today the split is:

- guardian Safe: `pause()`
- admin Safe: `migrate(...)`, `unpause()`, and the one-time bootstrap upgrade into the final
  implementation
- admin Safe-owned `ProxyAdmin`: gateway upgrades

That is the main boundary today:

- fast emergency stop through the guardian
- normal admin actions and upgrades through the Safe

The timelock is still the intended later hardening step once the contract is ready to go with a
delayed governance flow.

## Why It Is Shaped This Way

The main decision here is that the token should not have a mutable trust list.

We do not want a token contract that can keep changing which gateway it trusts. That is too much
power in the wrong place, and it makes proofs, upgrades, and review harder to reason about. So the
token takes the gateway address in its constructor and never changes it.

At the same time, we probably will need to change gateway logic over time. That is why the gateway
itself sits behind a proxy. The token keeps trusting the same address, but the logic behind that
address can be upgraded later through governance.

That split is the point of this package:

- tokens stay simple and stable
- gateway owns the moving parts
- upgrades happen behind the gateway address, not by teaching the token to trust someone new

Migration also goes through the gateway for the same reason. It is a one-time setup capability, but
it is still part of the same authority boundary. That keeps the token surface smaller and keeps the
initial migration distribution in the same place as the long-term burn behavior.

## Deployment Shape

The deployment order still matters:

1. Deploy a bootstrap `MintingGateway` implementation with zero canonical-token immutables.
2. Deploy the gateway proxy with the admin Safe as its current `ProxyAdmin` owner and the bootstrap
   `initialize(...)` calldata already encoded into the proxy constructor. Token-bearing gateway
   entrypoints stay blocked in this bootstrap state until the final implementation is installed with
   canonical token addresses configured.
3. Deploy `ArgonToken` and `ArgonotToken` with the proxy address in their constructors.
4. Deploy the final `MintingGateway` implementation with the Argon and Argonot token addresses baked
   in as immutables.
5. Have the admin Safe call `ProxyAdmin.upgradeAndCall(...)` once to move the proxy onto that final
   implementation.
6. Have the admin Safe call `migrate(...)` once with the migrated Argon balances and migrated
   Argonot balances.

The reverse direction does not exist: the token contracts do not have a mutable `setGateway(...)`
path.

The generated bootstrap manifest records:

- the bootstrap gateway implementation address
- the final gateway implementation address
- the gateway proxy address
- the proxy admin address
- the token addresses
- the Safe calldata needed to upgrade the proxy to the final implementation with fixed token
  addresses

See [deployments/mainnet/bootstrap/README.md](./deployments/mainnet/bootstrap/README.md) for the
deployment handoff and [TODO.md](./TODO.md) for the remaining hardening work.

## Commands

Recovery-specific evidence and migration tooling now live in the separate
`argonprotocol/hyperbridge-recovery` repository.

Contract commands stay here:

```sh
yarn workspace @argonprotocol/ethereum-contracts test
yarn workspace @argonprotocol/ethereum-contracts typecheck
yarn workspace @argonprotocol/ethereum-contracts gas:measure
yarn workspace @argonprotocol/ethereum-contracts bootstrap:deploy --admin-safe 0x... --guardian-safe 0x... --council-signers 0x...,0x... --council-weights 40,30 --microgons-per-argonot 1000000
```

Bootstrap council inputs must use unique non-zero signer addresses and non-zero weights.

Mainnet bootstrap generation uses the env-driven Hardhat 3 network config:

```sh
ETHEREUM_RPC_URL=https://...
ETHEREUM_DEPLOYER_PRIVATE_KEY=0x...
yarn workspace @argonprotocol/ethereum-contracts bootstrap:deploy --network mainnet --admin-safe 0x... --guardian-safe 0x... --council-signers 0x...,0x... --council-weights 40,30 --microgons-per-argonot 1000000
```

## Current Costs

Refresh these sample costs with:

```sh
yarn workspace @argonprotocol/ethereum-contracts gas:measure
```

The numbers below are the current local measurements from `script/gas/measure.ts`. The wei and ETH
columns are sample gas-price math only at `10 gwei` and `20 gwei`.

### User Actions

| Action                                          |     Gas |         Wei @ 10 gwei | ETH @ 10 gwei |         Wei @ 20 gwei | ETH @ 20 gwei |
| ----------------------------------------------- | ------: | --------------------: | ------------: | --------------------: | ------------: |
| `startTransferToArgon`                          | 126,077 | 1,260,770,000,000,000 |  0.001260 ETH | 2,521,540,000,000,000 |  0.002521 ETH |
| `finalizeTransferOutOfArgon` (1 authorization)  | 162,100 | 1,621,000,000,000,000 |  0.001621 ETH | 3,242,000,000,000,000 |  0.003242 ETH |
| `finalizeTransferOutOfArgon` (3 authorizations) | 198,594 | 1,985,940,000,000,000 |  0.001985 ETH | 3,971,880,000,000,000 |  0.003971 ETH |
| `cancelTransferOutOfArgon`                      | 104,629 | 1,046,290,000,000,000 |  0.001046 ETH | 2,092,580,000,000,000 |  0.002092 ETH |

### Admin And Council Actions

| Action                                                                              |     Gas |         Wei @ 10 gwei | ETH @ 10 gwei |          Wei @ 20 gwei | ETH @ 20 gwei |
| ----------------------------------------------------------------------------------- | ------: | --------------------: | ------------: | ---------------------: | ------------: |
| Proxy deploy + `initialize`                                                         | 819,347 | 8,193,470,000,000,000 |  0.008193 ETH | 16,386,940,000,000,000 |  0.016386 ETH |
| Upgrade to final implementation                                                     |  37,834 |   378,340,000,000,000 |  0.000378 ETH |    756,680,000,000,000 |  0.000756 ETH |
| Minting authority activation (4 members, 3 signatures)                              | 186,016 | 1,860,160,000,000,000 |  0.001860 ETH |  3,720,320,000,000,000 |  0.003720 ETH |
| Minting authority activation (100 members, 90 signatures)                           | 859,145 | 8,591,450,000,000,000 |  0.008591 ETH | 17,182,900,000,000,000 |  0.017182 ETH |
| Minting authority activation batch (100 members, 3 activations, 90 signatures once) | 932,994 | 9,329,940,000,000,000 |  0.009329 ETH | 18,659,880,000,000,000 |  0.018659 ETH |
| Council rotation (4 -> 4 members, 3 signatures)                                     | 150,647 | 1,506,470,000,000,000 |  0.001506 ETH |  3,012,940,000,000,000 |  0.003012 ETH |
| Council rotation (100 -> 100 members, 90 signatures)                                | 995,505 | 9,955,050,000,000,000 |  0.009955 ETH | 19,910,100,000,000,000 |  0.019910 ETH |

The batched `100`-member activation row is where the chained queue hash pays off: three activations
land for about `932,994` gas total, or about `310,998` gas per activation, because the council
quorum is only verified once at the segment tip.

`startTransferToArgon(...)` now includes the ERC-2612 permit directly. The caller signs for the
runtime-unit amount they want Argon to credit, and the gateway scales that amount into exact token
base units before it permits and burns in one transaction.

## Current Scope

This is still the bootstrap slice, not the full long-term mint-authority system.

Today that means:

- the gateway contract shape is in place
- the stable proxy address is in place
- the guardian pause path is in place
- the owner recovery seam for the active council summary is in place
- the current Safe-owned admin / upgrade path is in place
- the one-time migration path is in place

## Deferred Controls

This package still does not include the follow-on control layer.

Not included yet:

- per-authority, per-chain, per-epoch, or rolling daily issuance caps
- delayed Ethereum-side activation for large mint paths that should allow emergency review
- automatic pause triggers on abnormal circulation growth, authority loss, or state drift
- fast authority disable / suspend and replacement-authorization handling for suspicious operators
- the restoration batch generator now lives in the standalone recovery repo
- the migration path is still temporary
- the restoration forensics work is still the real blocker before a mainnet mint run
