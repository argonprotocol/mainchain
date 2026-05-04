# Ethereum Contracts

This package holds the Ethereum-side contracts for the replacement Argon and Argonot tokens.

At a high level, the shape is:

- the tokens are plain ERC-20s
- the tokens trust one gateway address forever
- that gateway address is a proxy, so the gateway logic can change without changing the address the
  tokens trust
- the gateway handles the protocol behavior: burns for transfer and the current admin batch mint
  path
- a guardian can pause immediately
- the admin Safe owns gateway actions and upgrades today

## Visual Overview

```text
Users ----------------------> MintingGateway proxy ----------------------> ArgonToken / ArgonotToken
Guardian Safe -------------> pause()
Admin Safe ---------------> proxy upgrade / adminMintBatch / unpause
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

- every gateway amount is expressed in 6-decimal Argon runtime base units
- `burnForTransfer(...)` is the primary user path: the user first grants an exact ERC-20 allowance
  to the gateway after the app scales the selected 6-decimal amount into 18-decimal ERC-20 units,
  then signs the gateway transaction that burns that approved balance and emits the event Argon
  needs for the outbound proof flow
- `adminMintBatch(...)` is the current admin-only mint path for restoration work, and it accepts
  the same 6-decimal runtime amounts as the burn path
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
- admin Safe: `adminMintBatch(...)`, `unpause()`, and the one-time bootstrap upgrade into the final
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

Admin minting also goes through the gateway for the same reason. It is a temporary capability, but
it is still part of the same authority boundary. That keeps the token surface smaller and keeps the
current mint path in the same place as the long-term burn behavior.

## Deployment Shape

The deployment order still matters:

1. Deploy a bootstrap `MintingGateway` implementation with zero canonical-token immutables.
2. Deploy the gateway proxy with the admin Safe as its current `ProxyAdmin` owner.
3. Deploy `ArgonToken` and `ArgonotToken` with the proxy address in their constructors.
4. Deploy the final `MintingGateway` implementation with the Argon and Argonot token addresses baked
   in as immutables.
5. Have the admin Safe call `ProxyAdmin.upgradeAndCall(...)` once to move the proxy onto that final
   implementation.

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
yarn workspace @argonprotocol/ethereum-contracts bootstrap:deploy --admin-safe 0x... --guardian-safe 0x...
```

Mainnet bootstrap generation uses the env-driven Hardhat 3 network config:

```sh
ETHEREUM_RPC_URL=https://...
ETHEREUM_DEPLOYER_PRIVATE_KEY=0x...
yarn workspace @argonprotocol/ethereum-contracts bootstrap:deploy --network mainnet --admin-safe 0x... --guardian-safe 0x...
```

## Current Scope

This is still the bootstrap slice, not the full long-term mint-authority system.

Today that means:

- the gateway contract shape is in place
- the stable proxy address is in place
- the guardian pause path is in place
- the current Safe-owned admin / upgrade path is in place
- the admin batch mint path is in place
- the restoration batch generator now lives in the standalone recovery repo
- the admin mint path is still temporary
- the restoration forensics work is still the real blocker before a mainnet mint run
