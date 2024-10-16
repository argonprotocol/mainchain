# Changelog

## [v0.0.14](https://github.com/argonprotocol/mainchain/compare/v0.0.13...71839151e4f9969f2b9c0e2e32d417a79941f2e1) (2024-10-11)

### [v0.0.13](https://github.com/argonprotocol/mainchain/compare/v0.0.12...v0.0.13) (2024-10-09)

#### Fixes

* **localchain:** holding mainchain locks too long
([7d0eb9d](https://github.com/argonprotocol/mainchain/commit/7d0eb9d07e3489fc694933c567bd780c6e08d1b0))
* **localchain:** updating wrong mainchain transfer
([4d9bef3](https://github.com/argonprotocol/mainchain/commit/4d9bef34bc847ccc0739ad854ecc1eeebd4f86b6))

### [v0.0.12](https://github.com/argonprotocol/mainchain/compare/v0.0.11...v0.0.12) (2024-10-08)

### [v0.0.11](https://github.com/argonprotocol/mainchain/compare/v0.0.10...v0.0.11) (2024-10-08)

### [v0.0.10](https://github.com/argonprotocol/mainchain/compare/v0.0.9...v0.0.10) (2024-10-07)

#### Features

* **localchain:** retry votes
([393fd80](https://github.com/argonprotocol/mainchain/commit/393fd804d2d033a212251f1111bf6bdf1d2dde1d))

#### Fixes

* **localchain:** require a vote to include a tick
([996d153](https://github.com/argonprotocol/mainchain/commit/996d153e147ffa50ec151c79fe1ffd3ed6451b2e))
* **node:** fix stalling of notebook auditing
([95e3f37](https://github.com/argonprotocol/mainchain/commit/95e3f3778de256ef79d4eb652b6b3b0265f4f0d2))
* **localchain:** simplify balance_sync
([8b337ab](https://github.com/argonprotocol/mainchain/commit/8b337ab1ed774a970936bcc17e1a6b54e9dd15c4))

### [v0.0.9](https://github.com/argonprotocol/mainchain/compare/v0.0.8...v0.0.9) (2024-10-01)

#### Features

* integrate keys into mining slots
([662bdd6](https://github.com/argonprotocol/mainchain/commit/662bdd61963c87147ec6f1de6dc3d8662c980dd7))

#### Fixes

* **localchain:** always create block votes now
([c746a55](https://github.com/argonprotocol/mainchain/commit/c746a55be8a0db6d3132ec48d0642cf59e62e457))

### [v0.0.8](https://github.com/argonprotocol/mainchain/compare/v0.0.7...v0.0.8) (2024-09-23)

#### Fixes

* **notary:** sqlx error
([02848ff](https://github.com/argonprotocol/mainchain/commit/02848ff4f088345cb0d46b349ee1fdeff9be6399))

### [v0.0.7](https://github.com/argonprotocol/mainchain/compare/v0.0.6...v0.0.7) (2024-09-23)

#### Features

* lock localchain and mainchain to chainId/gen
([2647511](https://github.com/argonprotocol/mainchain/commit/2647511598583e17d6c61b1ad5d515341d017caa))

### [v0.0.6](https://github.com/argonprotocol/mainchain/compare/v0.0.5...v0.0.6) (2024-09-22)

#### Fixes

* broken transaction order from refactor
([c05160f](https://github.com/argonprotocol/mainchain/commit/c05160f3b2f4e07348d789750050183f4cee33be))

### [v0.0.5](https://github.com/argonprotocol/mainchain/compare/v0.0.4...v0.0.5) (2024-09-21)

#### Fixes

* don’t require data domain for votes
([714e3b0](https://github.com/argonprotocol/mainchain/commit/714e3b045c3e2bbe448f88d0ceaa976a54016094))

### [v0.0.4](https://github.com/argonprotocol/mainchain/compare/v0.0.3...v0.0.4) (2024-09-06)

#### Features

* **bitcoin:** cli commands to store vault xpriv
([fd1694b](https://github.com/argonprotocol/mainchain/commit/fd1694b3db0e5ecbbb7c427054b6e964bca8ea17))

### [v0.0.3](https://github.com/argonprotocol/mainchain/compare/v0.0.2...v0.0.3) (2024-08-29)

### [v0.0.2](https://github.com/argonprotocol/mainchain/compare/v0.0.1...v0.0.2) (2024-08-28)

#### Fixes

* npm publish for 0.0.1 broke
([d14caf1](https://github.com/argonprotocol/mainchain/commit/d14caf1970f323dec5a4c835ad49201f43fb6a31))

### v0.0.1 (2024-08-27)

#### Features

* **localchain:** add a cli for transactions
([2e4360c](https://github.com/argonprotocol/mainchain/commit/2e4360cf5b347b31eb55f05a8b27cceb1d2afa30))
* **notary:** allow notaries to have names
([06e5abd](https://github.com/argonprotocol/mainchain/commit/06e5abd59b1932bce1735429fbbe5a6c7b40e60d))
* **localchain:** add delegated escrow signing
([7602274](https://github.com/argonprotocol/mainchain/commit/7602274555708cfca10ee839a5690677a66ab4f3))
* mining and bitcoin bonds
([9a2e67b](https://github.com/argonprotocol/mainchain/commit/9a2e67bb2416761f6fe1b867c78e027b81b9ecf6))
* bitcoin minting
([8d7bee7](https://github.com/argonprotocol/mainchain/commit/8d7bee7f95a2a0da69635169eab97c409b3a80da))
* **github:** add sccache and mold
([b9b97ba](https://github.com/argonprotocol/mainchain/commit/b9b97ba1df413f380f85f9e819551ca19d85bd77))
* **localchain:** current buying power - 2020 cpi
([0ebd5c2](https://github.com/argonprotocol/mainchain/commit/0ebd5c2f94b33b638f4454ad82ce97c4905a8168))
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
* remove signature from votes
([945b00e](https://github.com/argonprotocol/mainchain/commit/945b00e67ea80009666c41558bd86583be121c40))
* data domains as strings + parsing
([2da520c](https://github.com/argonprotocol/mainchain/commit/2da520c4e02184c0d5e9e85dccf7dc56658f0660))
* add preferred notary id to zone record
([1d0a483](https://github.com/argonprotocol/mainchain/commit/1d0a483d51fdfefbd6d0d5f8ecadb3e31586928c))
* localchain
([3793d5c](https://github.com/argonprotocol/mainchain/commit/3793d5c8d80fe1cc5535e0d55d52615e3b19d71e))

#### Fixes

* **napi:** do anything to fix napi build
([ca3b16f](https://github.com/argonprotocol/mainchain/commit/ca3b16fd2989412ea782374dc4299158c8c73c85))
* **notary:** add typed errors
([e0ce8e7](https://github.com/argonprotocol/mainchain/commit/e0ce8e7761ffb202de71a9b07d5497c297809fd0))
* docker builds
([b49faf3](https://github.com/argonprotocol/mainchain/commit/b49faf36e0a8b969a469552d83333c7a5368792e))
* github builds
([ea6e6d8](https://github.com/argonprotocol/mainchain/commit/ea6e6d829a369d81f6d9997d68e778aeef81a603))
* **e2e:** end to end test fixes
([e013a2d](https://github.com/argonprotocol/mainchain/commit/e013a2ddfb94cbd16733607f683d7c4dbb830d53))
* js test ports
([c28c242](https://github.com/argonprotocol/mainchain/commit/c28c242e40a3a686e49f600f751c8897b1cc0fd6))
* **notebook:** do not halt if bad notebook data
([633b503](https://github.com/argonprotocol/mainchain/commit/633b503a36a4a613758f5ee460b711431ce3c40a))
* build napi actions
([97ff17a](https://github.com/argonprotocol/mainchain/commit/97ff17a1fdcd944553b09eb781f35324394b11b3))
* use transfer ids for tx -> localchain
([6982aaf](https://github.com/argonprotocol/mainchain/commit/6982aaf9934c9a40c607ba3f1bfbb38d627a9873))
* allow js tests to run in docker
([653761e](https://github.com/argonprotocol/mainchain/commit/653761e96770087d1d3a86c91afdae89130a5e45))
* revert napi 3
([2320cf0](https://github.com/argonprotocol/mainchain/commit/2320cf097945fe126512169757f797d6ba173415))
* build docker
([0d7bdcb](https://github.com/argonprotocol/mainchain/commit/0d7bdcb66c16a5b8bd6aa95a2653b2968edf4ed1))
* github actions
([f674c6b](https://github.com/argonprotocol/mainchain/commit/f674c6b464f6abf621d2841e2cf6d2478fae4549))
* **node:** retrieve missing notebooks
([c8a094e](https://github.com/argonprotocol/mainchain/commit/c8a094e953df3896df5151d605eb0b0f66b10d95))
* **localchain:** js test broken
([fc5d979](https://github.com/argonprotocol/mainchain/commit/fc5d979f7694f9d9a57ca2321b429a5a5adf4217))
* convert data domain to hash in network
([94417a5](https://github.com/argonprotocol/mainchain/commit/94417a5df5cabcefda1a1e8e2d55afc9f89f5984))
