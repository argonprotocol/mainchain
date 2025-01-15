# Changelog

## [v0.0.27](https://github.com/argonprotocol/mainchain/compare/v0.0.26...00c9e63576313d98dacc0f0ea5ce2eabb608efaa) (2025-01-14)

### Fixes

* **seal_spec:** trim to 80th pctl of block times
([4c3458d](https://github.com/argonprotocol/mainchain/commit/4c3458da6ab4b402892507639be246206e6f5d8b))

### [v0.0.26](https://github.com/argonprotocol/mainchain/compare/v0.0.25...v0.0.26) (2025-01-13)

#### Fixes

* update metadata
([c5273ad](https://github.com/argonprotocol/mainchain/commit/c5273ad9bf1c623c8be770774d68186a0dac7fbf))
* **mining_slot:** cap ownership max amount at 80%
([15387e1](https://github.com/argonprotocol/mainchain/commit/15387e1e20e3ce2c42caacffcf32d7a3cabd2045))

### [v0.0.25](https://github.com/argonprotocol/mainchain/compare/v0.0.24...v0.0.25) (2025-01-06)

#### Features

* **oracle:** move cpi to env var
([68828c6](https://github.com/argonprotocol/mainchain/commit/68828c66f71bd5d873272109c893a63f8d306680))
* **node:** add earnings metrics
([7f7dc1e](https://github.com/argonprotocol/mainchain/commit/7f7dc1e4f3faab0b3ef7881bc912a4628b14b3f5))

#### Fixes

* **ticks:** max 5 blocks per tick
([0e43dbb](https://github.com/argonprotocol/mainchain/commit/0e43dbbed467d1978f4aba969c4b859b60377aae))
* **mining:** starts slots after ticks vs blocks
([ff4428f](https://github.com/argonprotocol/mainchain/commit/ff4428f53acdf0735121492cd2a6a810d75db8e6))

### [v0.0.24](https://github.com/argonprotocol/mainchain/compare/v0.0.23...v0.0.24) (2024-12-20)

### [v0.0.23](https://github.com/argonprotocol/mainchain/compare/v0.0.22...v0.0.23) (2024-12-19)

### [v0.0.22](https://github.com/argonprotocol/mainchain/compare/v0.0.21...v0.0.22) (2024-12-17)

### [v0.0.21](https://github.com/argonprotocol/mainchain/compare/v0.0.20...v0.0.21) (2024-12-08)

### [v0.0.20](https://github.com/argonprotocol/mainchain/compare/v0.0.19...v0.0.20) (2024-12-07)

### [v0.0.19](https://github.com/argonprotocol/mainchain/compare/v0.0.18...v0.0.19) (2024-12-06)

### [v0.0.18](https://github.com/argonprotocol/mainchain/compare/v0.0.17...v0.0.18) (2024-12-05)

#### Features

* **client:** add a wage protector
([c2bba70](https://github.com/argonprotocol/mainchain/commit/c2bba7038005251280a15f21829577359853d955))
* **node:** remove compute notebook block sort
([e087392](https://github.com/argonprotocol/mainchain/commit/e08739228cad43b071b1d2181de0cb3197ae12c5))
* **chain_transfer:** bridge scripts
([de5f351](https://github.com/argonprotocol/mainchain/commit/de5f351c9253de09c5be939f5ca6d830089d72a1))
* **chain_transfer:** add ability to pause bridge
([3cfd210](https://github.com/argonprotocol/mainchain/commit/3cfd21014038a476fc2b610d187445cd6e643252))
* **runtime:** add a canary runtime
([1eb7a61](https://github.com/argonprotocol/mainchain/commit/1eb7a61e25183d29bef294d3fab99c8d842ff66c))
* convert ticks to use unix epoch
([36d230e](https://github.com/argonprotocol/mainchain/commit/36d230e0f18e631a92da0e9b1b466028f02cde13))
* **runtime:** integrate hyperbridge to evm
([e5b8d35](https://github.com/argonprotocol/mainchain/commit/e5b8d3587b5ba285c96470a628f16fc1b1fde5f5))
* **runtime:** lower minimum vote start
([d7bfbab](https://github.com/argonprotocol/mainchain/commit/d7bfbab847742bf55db866fca01b2329f3e8c1f0))

#### Fixes

* build
([7628e02](https://github.com/argonprotocol/mainchain/commit/7628e02d9566eb03e019bd23d897fe7fdd1d5a31))
* **block_rewards:** start with smaller rewards
([237971a](https://github.com/argonprotocol/mainchain/commit/237971a211fac9e770a7e11b1d1cabb4ad789554))
* **node:** default block votes
([4c5f52d](https://github.com/argonprotocol/mainchain/commit/4c5f52d9a73d5de4d3b53a93b9d5d672c1933582))
* **mining_slot:** remove miner zero
([52f33f1](https://github.com/argonprotocol/mainchain/commit/52f33f10b04b2314e49257e749aebf4ac2096de5))
* **block_seal:** sign full block
([e73cfc9](https://github.com/argonprotocol/mainchain/commit/e73cfc965b91a161bdf67b79e872294bafdb5d00))

### [v0.0.17](https://github.com/argonprotocol/mainchain/compare/v0.0.16...v0.0.17) (2024-10-25)

### [v0.0.16](https://github.com/argonprotocol/mainchain/compare/v0.0.15...v0.0.16) (2024-10-25)

### [v0.0.15](https://github.com/argonprotocol/mainchain/compare/v0.0.14...v0.0.15) (2024-10-25)

#### Features

* **notary:** stop apis if audit fails
([5787461](https://github.com/argonprotocol/mainchain/commit/57874612671dc6f4f178438a86653592d6fa0bee))
* **notebook:** allow a notary to recover
([e1246d5](https://github.com/argonprotocol/mainchain/commit/e1246d5c61fd4e1397bd65b4b5e173455098acd0))

#### Fixes

* **ticks:** only allow a single block per tick
([cdf295a](https://github.com/argonprotocol/mainchain/commit/cdf295aae082adae7f72deb4ddc9517b48e9ccbd))

### [v0.0.14](https://github.com/argonprotocol/mainchain/compare/v0.0.13...v0.0.14) (2024-10-11)

### [v0.0.13](https://github.com/argonprotocol/mainchain/compare/v0.0.12...v0.0.13) (2024-10-09)

### [v0.0.12](https://github.com/argonprotocol/mainchain/compare/v0.0.11...v0.0.12) (2024-10-08)

### [v0.0.11](https://github.com/argonprotocol/mainchain/compare/v0.0.10...v0.0.11) (2024-10-08)

### [v0.0.10](https://github.com/argonprotocol/mainchain/compare/v0.0.9...v0.0.10) (2024-10-07)

#### Fixes

* **localchain:** require a vote to include a tick
([996d153](https://github.com/argonprotocol/mainchain/commit/996d153e147ffa50ec151c79fe1ffd3ed6451b2e))
* **localchain:** simplify balance_sync
([8b337ab](https://github.com/argonprotocol/mainchain/commit/8b337ab1ed774a970936bcc17e1a6b54e9dd15c4))

### [v0.0.9](https://github.com/argonprotocol/mainchain/compare/v0.0.8...v0.0.9) (2024-10-01)

#### Features

* integrate keys into mining slots
([662bdd6](https://github.com/argonprotocol/mainchain/commit/662bdd61963c87147ec6f1de6dc3d8662c980dd7))

### [v0.0.8](https://github.com/argonprotocol/mainchain/compare/v0.0.7...v0.0.8) (2024-09-23)

### [v0.0.7](https://github.com/argonprotocol/mainchain/compare/v0.0.6...v0.0.7) (2024-09-23)

### [v0.0.6](https://github.com/argonprotocol/mainchain/compare/v0.0.5...v0.0.6) (2024-09-22)

#### Fixes

* broken transaction order from refactor
([c05160f](https://github.com/argonprotocol/mainchain/commit/c05160f3b2f4e07348d789750050183f4cee33be))

### [v0.0.5](https://github.com/argonprotocol/mainchain/compare/v0.0.4...v0.0.5) (2024-09-21)

#### Fixes

* don’t require data domain for votes
([714e3b0](https://github.com/argonprotocol/mainchain/commit/714e3b045c3e2bbe448f88d0ceaa976a54016094))

### [v0.0.4](https://github.com/argonprotocol/mainchain/compare/v0.0.3...v0.0.4) (2024-09-06)

### [v0.0.3](https://github.com/argonprotocol/mainchain/compare/v0.0.2...v0.0.3) (2024-08-29)

### [v0.0.2](https://github.com/argonprotocol/mainchain/compare/v0.0.1...v0.0.2) (2024-08-28)

#### Fixes

* npm publish for 0.0.1 broke
([d14caf1](https://github.com/argonprotocol/mainchain/commit/d14caf1970f323dec5a4c835ad49201f43fb6a31))

### v0.0.1 (2024-08-27)

#### Features

* reduce testnet minimum bitcoins to 5000 sats
([9a5289c](https://github.com/argonprotocol/mainchain/commit/9a5289c7e08bdd780e0fa5075e916f2f81c4eee6))
* don't require old bitcoin block sync
([8e242c2](https://github.com/argonprotocol/mainchain/commit/8e242c2beebd22cd42af141bda210ed4c8a9b6e0))
* **localchain:** add a cli for transactions
([2e4360c](https://github.com/argonprotocol/mainchain/commit/2e4360cf5b347b31eb55f05a8b27cceb1d2afa30))
* **bitcoin/cli:** cli for managing bitcoin
([a582fae](https://github.com/argonprotocol/mainchain/commit/a582fae78e3b2f7a4df1cb21cb51048d8233d358))
* **bitcoin:** restrict addresses to network
([fa5f2ac](https://github.com/argonprotocol/mainchain/commit/fa5f2ac53fe1909eef7dbe6b31bc6710731c7475))
* **vault:** convert to xpub keys
([5e7c06c](https://github.com/argonprotocol/mainchain/commit/5e7c06cb62fe5296af64bcbe7bba11aafe2969ac))
* **vaults:** allow changing vault terms
([ad42e55](https://github.com/argonprotocol/mainchain/commit/ad42e55f8e43b7910bd750e17e52f1e32bfeec5e))
* **oracle:** add ability to run oracles
([8b6dab8](https://github.com/argonprotocol/mainchain/commit/8b6dab81cbcaaf0909aa224c97f3317573fe6325))
* **vaults:** allow vault to issue profit sharing
([6905a7f](https://github.com/argonprotocol/mainchain/commit/6905a7f02968cbae9889f278b026919f4c4c7b9f))
* **mining_slot:** adjust shares by target bids
([9df3acb](https://github.com/argonprotocol/mainchain/commit/9df3acb6139abc784531c86dc5c895670911a2bf))
* **mining_slot:** close bidding with seal as vrf
([54adbea](https://github.com/argonprotocol/mainchain/commit/54adbea308d71d2ecfea3bc7c72a6348aba37557))
* **notary:** allow notaries to have names
([06e5abd](https://github.com/argonprotocol/mainchain/commit/06e5abd59b1932bce1735429fbbe5a6c7b40e60d))
* **localchain:** add delegated escrow signing
([7602274](https://github.com/argonprotocol/mainchain/commit/7602274555708cfca10ee839a5690677a66ab4f3))
* add multisig pallet
([bb29ded](https://github.com/argonprotocol/mainchain/commit/bb29ded5d4ce51c2e33894debd36b972e5df0bdd))
* mining and bitcoin bonds
([9a2e67b](https://github.com/argonprotocol/mainchain/commit/9a2e67bb2416761f6fe1b867c78e027b81b9ecf6))
* bitcoin minting
([8d7bee7](https://github.com/argonprotocol/mainchain/commit/8d7bee7f95a2a0da69635169eab97c409b3a80da))
* **localchain:** add uniffi bindings for ios
([cd156ec](https://github.com/argonprotocol/mainchain/commit/cd156ecd746e06bcefcd54033992a058fa8d59fd))
* **localchain:** transaction log
([069ffa8](https://github.com/argonprotocol/mainchain/commit/069ffa825e4f61a99c0465a3e7a813c722c4750c))
* **localchain:** transactions
([925b8cc](https://github.com/argonprotocol/mainchain/commit/925b8cc4b5c3032d3fff886da9de44975d781b1f))
* embedded keystore
([0e5db86](https://github.com/argonprotocol/mainchain/commit/0e5db862b541b6f130fbb24434d00bf44a896293))
* argon file type
([44e3f90](https://github.com/argonprotocol/mainchain/commit/44e3f909bee671e17e66bb29c8a0c7efd08df11d))
* **localchain:** add cli
([b5ef73a](https://github.com/argonprotocol/mainchain/commit/b5ef73a4e5e51e6ffae2b29ef0c1bca5e9621e06))
* data domains as strings + parsing
([2da520c](https://github.com/argonprotocol/mainchain/commit/2da520c4e02184c0d5e9e85dccf7dc56658f0660))
* add preferred notary id to zone record
([1d0a483](https://github.com/argonprotocol/mainchain/commit/1d0a483d51fdfefbd6d0d5f8ecadb3e31586928c))
* localchain
([3793d5c](https://github.com/argonprotocol/mainchain/commit/3793d5c8d80fe1cc5535e0d55d52615e3b19d71e))

#### Fixes

* github builds
([ea6e6d8](https://github.com/argonprotocol/mainchain/commit/ea6e6d829a369d81f6d9997d68e778aeef81a603))
* **vault:** require hardened xpub
([52d11ad](https://github.com/argonprotocol/mainchain/commit/52d11ad98f3a1c318aa59b2c6fc9822155271d73))
* **notebook:** do not halt if bad notebook data
([633b503](https://github.com/argonprotocol/mainchain/commit/633b503a36a4a613758f5ee460b711431ce3c40a))
* **vaults:** convert min fees to base + prorata
([a77dc87](https://github.com/argonprotocol/mainchain/commit/a77dc8717a589201d4ada599f66c24bbaf781b59))
* use fixed u128 for prices and rates
([4708dbe](https://github.com/argonprotocol/mainchain/commit/4708dbe2e370788314e1c630cdceabe942958bea))
* use transfer ids for tx -> localchain
([6982aaf](https://github.com/argonprotocol/mainchain/commit/6982aaf9934c9a40c607ba3f1bfbb38d627a9873))
* convert data domain to hash in network
([94417a5](https://github.com/argonprotocol/mainchain/commit/94417a5df5cabcefda1a1e8e2d55afc9f89f5984))
