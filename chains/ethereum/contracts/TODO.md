# Ethereum Gateway TODO

- [ ] Replace the Foundation Safe admin path with council-controlled Ethereum governance. Use one
      council-owned executor / timelock contract as the owner of both `MintingGateway` and
      `ProxyAdmin`, so the current Argon council can approve and execute: - gateway admin actions
      such as `unpause()` and `setGuardian(...)` - proxy upgrades through
      `ProxyAdmin.upgradeAndCall(...)` Keep upgrade authority outside `MintingGateway` itself so
      gateway upgrades do not depend on the old implementation executing correctly. The tokens can
      stay unchanged because they already trust the stable gateway proxy address, not a specific
      implementation.
- [ ] Monitor timelock and proxy admin for unexpected changes, and set up a process to respond to
      them if they happen.
