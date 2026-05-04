# Ethereum Gateway TODO

- [x] Ability to burn for transfer, with an event that has the data Argon needs for the outbound
      proof flow.
- [x] Put `MintingGateway` behind a stable proxy address and deploy canonical tokens against that
      proxy address from the start.
- [x] Split emergency pause from governance so a guardian can `pause()` immediately while only the
      admin Safe can `unpause()`.
- [ ] MintingAuthority and Council to approve new authority
- [ ] Ability to submit mints
- [ ] Move proxy administration from the Foundation Safe to the long-term `TimelockController` setup
      when the governance flow is ready.
- [ ] Monitor timelock and proxy admin for unexpected changes, and set up a process to respond to
      them if they happen.
