# Changelog

## [v0.0.15](https://github.com/argonprotocol/mainchain/compare/v0.0.14...5f22086ffa71902ec243d030597a9c6377a91f3f) (2024-10-24)

### Features

* **node:** check for blocks when importing
([0a4e567](https://github.com/argonprotocol/mainchain/commit/0a4e567bc8800b564660862fc513356f27f7aaf1))
* **notary:** stop apis if audit fails
([5787461](https://github.com/argonprotocol/mainchain/commit/57874612671dc6f4f178438a86653592d6fa0bee))
* **node:** async notebook processing
([b0482e1](https://github.com/argonprotocol/mainchain/commit/b0482e10bff8dea01e3257d7f686095f036a977e))
* **notebook:** allow a notary to recover
([e1246d5](https://github.com/argonprotocol/mainchain/commit/e1246d5c61fd4e1397bd65b4b5e173455098acd0))
* **localchain:** improve backtrace logging
([9f60056](https://github.com/argonprotocol/mainchain/commit/9f60056de03823a9a88ccb542021cbd883e55b77))

### Fixes

* llvm 19.1.2 broke build on mac
([5f22086](https://github.com/argonprotocol/mainchain/commit/5f22086ffa71902ec243d030597a9c6377a91f3f))
* **ticks:** only allow a single block per tick
([cdf295a](https://github.com/argonprotocol/mainchain/commit/cdf295aae082adae7f72deb4ddc9517b48e9ccbd))

### [v0.0.14](https://github.com/argonprotocol/mainchain/compare/v0.0.13...v0.0.14) (2024-10-11)

#### Fixes

* missing unwrap
([7183915](https://github.com/argonprotocol/mainchain/commit/71839151e4f9969f2b9c0e2e32d417a79941f2e1))
* **localchain:** holding mainchain locks too long
([7d0eb9d](https://github.com/argonprotocol/mainchain/commit/7d0eb9d07e3489fc694933c567bd780c6e08d1b0))
* **localchain:** updating wrong mainchain transfer
([4d9bef3](https://github.com/argonprotocol/mainchain/commit/4d9bef34bc847ccc0739ad854ecc1eeebd4f86b6))

### [v0.0.13](https://github.com/argonprotocol/mainchain/compare/v0.0.12...v0.0.13) (2024-10-09)

#### Fixes

* **localchain:** holding mainchain locks too long
([c07cbbb](https://github.com/argonprotocol/mainchain/commit/c07cbbb5a9c0f7774370cfa6e0adc5fbf29e7e28))
* **localchain:** updating wrong mainchain transfer
([7a7e31c](https://github.com/argonprotocol/mainchain/commit/7a7e31c6726aaecbc8a9dee459efd219dee648de))

### [v0.0.12](https://github.com/argonprotocol/mainchain/compare/v0.0.11...v0.0.12) (2024-10-08)

#### Fixes

* **node:** import missing notebooks on verify
([ba7f86f](https://github.com/argonprotocol/mainchain/commit/ba7f86fdeb646c6db350c60cd57e6a3bc0ff3cb9))
* publish task
([67d60f6](https://github.com/argonprotocol/mainchain/commit/67d60f6ff1787a982c475d7d815657b8bce3312f))

### [v0.0.11](https://github.com/argonprotocol/mainchain/compare/v0.0.10...v0.0.11) (2024-10-08)

#### Fixes

* change format of releasing
([0e0bdcd](https://github.com/argonprotocol/mainchain/commit/0e0bdcdd2528985ce65f3ea69c212368f72e4021))

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
* don’t log 0 argon rewards
([3a4bfab](https://github.com/argonprotocol/mainchain/commit/3a4bfab8cb296b85da0543d577a2a33e85b83b54))

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
* generate changelogs
([5139ca0](https://github.com/argonprotocol/mainchain/commit/5139ca0c62f2f18772c78c39d24a828d4052c827))

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
* check in a testnet.json for stable genesis
([a1fb7b9](https://github.com/argonprotocol/mainchain/commit/a1fb7b9969c01f2fcc50ed59211791708ae7bcc7))
* force node to correct bitcoin network
([82c9c26](https://github.com/argonprotocol/mainchain/commit/82c9c2690ebf7308fb8bc4fc50ecacefc4ab99cb))
* **bitcoin:** allow anyone to pay fee
([7d81e9a](https://github.com/argonprotocol/mainchain/commit/7d81e9a741d23983325aef36a99fac36aac6b337))
* **localchain:** add a cli for transactions
([2e4360c](https://github.com/argonprotocol/mainchain/commit/2e4360cf5b347b31eb55f05a8b27cceb1d2afa30))
* **bitcoin/cli:** cli for managing bitcoin
([a582fae](https://github.com/argonprotocol/mainchain/commit/a582fae78e3b2f7a4df1cb21cb51048d8233d358))
* **bitcoin:** restrict addresses to network
([fa5f2ac](https://github.com/argonprotocol/mainchain/commit/fa5f2ac53fe1909eef7dbe6b31bc6710731c7475))
* **vault:** convert to xpub keys
([5e7c06c](https://github.com/argonprotocol/mainchain/commit/5e7c06cb62fe5296af64bcbe7bba11aafe2969ac))
* **vaults:** check bitcoin cosign sig
([508b517](https://github.com/argonprotocol/mainchain/commit/508b517ee97510477f17d43bffd1c6307b6c180a))
* **vaults:** allow changing vault terms
([ad42e55](https://github.com/argonprotocol/mainchain/commit/ad42e55f8e43b7910bd750e17e52f1e32bfeec5e))
* **oracle:** add ability to run oracles
([8b6dab8](https://github.com/argonprotocol/mainchain/commit/8b6dab81cbcaaf0909aa224c97f3317573fe6325))
* **vaults:** allow vault to issue profit sharing
([6905a7f](https://github.com/argonprotocol/mainchain/commit/6905a7f02968cbae9889f278b026919f4c4c7b9f))
* **mining_slot:** adjust shares by target bids
([9df3acb](https://github.com/argonprotocol/mainchain/commit/9df3acb6139abc784531c86dc5c895670911a2bf))
* **mining_slot:** allow bidding to start at block
([c352e62](https://github.com/argonprotocol/mainchain/commit/c352e62e35e5f1445796bbea249aeaca3e2487c6))
* **mining_slot:** close bidding with seal as vrf
([54adbea](https://github.com/argonprotocol/mainchain/commit/54adbea308d71d2ecfea3bc7c72a6348aba37557))
* **notary:** allow notaries to have names
([06e5abd](https://github.com/argonprotocol/mainchain/commit/06e5abd59b1932bce1735429fbbe5a6c7b40e60d))
* **localchain:** add delegated escrow signing
([7602274](https://github.com/argonprotocol/mainchain/commit/7602274555708cfca10ee839a5690677a66ab4f3))
* add multisig pallet
([bb29ded](https://github.com/argonprotocol/mainchain/commit/bb29ded5d4ce51c2e33894debd36b972e5df0bdd))
* asic resistent mining using randomx
([945a48b](https://github.com/argonprotocol/mainchain/commit/945a48b824405fafe366e39695186d4029e4bcbc))
* mining and bitcoin bonds
([9a2e67b](https://github.com/argonprotocol/mainchain/commit/9a2e67bb2416761f6fe1b867c78e027b81b9ecf6))
* bitcoin minting
([8444d04](https://github.com/argonprotocol/mainchain/commit/8444d046b2527b61dfbf0dfa0f41d3499ceceac8))
* bitcoin minting
([8d7bee7](https://github.com/argonprotocol/mainchain/commit/8d7bee7f95a2a0da69635169eab97c409b3a80da))
* **github:** add sccache and mold
([b9b97ba](https://github.com/argonprotocol/mainchain/commit/b9b97ba1df413f380f85f9e819551ca19d85bd77))
* **localchain:** current buying power - 2020 cpi
([0ebd5c2](https://github.com/argonprotocol/mainchain/commit/0ebd5c2f94b33b638f4454ad82ce97c4905a8168))
* **localchain:** add uniffi bindings for ios
([cd156ec](https://github.com/argonprotocol/mainchain/commit/cd156ecd746e06bcefcd54033992a058fa8d59fd))
* ability to set genesis notaries
([f3279a8](https://github.com/argonprotocol/mainchain/commit/f3279a8232d32d97610c8492e9ca212d3ca4a26f))
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
* add tracking for ulixee transfers
([699b2df](https://github.com/argonprotocol/mainchain/commit/699b2dfdaea219ab0e15710ac016b7143b585235))
* data domains
([ae806de](https://github.com/argonprotocol/mainchain/commit/ae806de7b1ce3a4af847ebb65cbe879a9b528cca))
* merge primitives libraries
([03d3152](https://github.com/argonprotocol/mainchain/commit/03d3152076a089294f537d4842bbed32b4d70677))
* **node:** change how votes are collected
([354a796](https://github.com/argonprotocol/mainchain/commit/354a796d86e30d8be4b088a48570b4aac19fc9f7))
* **node:** version 2 of compute
([12d80fc](https://github.com/argonprotocol/mainchain/commit/12d80fc371262d0d6ceef4f1d0c99f09e2f79721))
* **notary:** notebook best nonces flow v2
([101e3a6](https://github.com/argonprotocol/mainchain/commit/101e3a6b85dcc652c986c9df74b6633f5a694e53))
* **localchain:** burn tax amount from mainchain
([2cb5e17](https://github.com/argonprotocol/mainchain/commit/2cb5e178eecf826a0d4b8e0e2e11e077eb747651))
* **notary:** add channel locks and tax
([fdeee4f](https://github.com/argonprotocol/mainchain/commit/fdeee4fdb436c76bf84195d80c11a42b955965d2))
* integration test for notary
([1430df9](https://github.com/argonprotocol/mainchain/commit/1430df90e06fbdbc1f89d62713d2e50f3a25a002))
* **notary:** add mainchain audit
([ec020bd](https://github.com/argonprotocol/mainchain/commit/ec020bd19eed2435a00db36f7df03dbe88b08854))
* **localchain_relay:** store notary hash+changes
([e6157a7](https://github.com/argonprotocol/mainchain/commit/e6157a7bed7561f80cb13bb726c97136e36a0e78))
* **notary:** transfers between chains
([c9cb14b](https://github.com/argonprotocol/mainchain/commit/c9cb14b086a281051a19139fdf86e249e099ee6c))
* **runtime:** add tx-pause feature
([acecec8](https://github.com/argonprotocol/mainchain/commit/acecec830f6d7907e8830aed51093684eeb8ac5d))
* **cohorts:** add a zero miner for fallback
([e2bce57](https://github.com/argonprotocol/mainchain/commit/e2bce57c6e80a0265c436d27637ed072580bf323))
* **block_seal:** allow sudo config of block seal
([7b38c59](https://github.com/argonprotocol/mainchain/commit/7b38c5929b4a405babf9b080b24669f7251f93de))
* **runtime:** add fast block closing option
([11a11d3](https://github.com/argonprotocol/mainchain/commit/11a11d3568313ba364f9801573555ae5b2809c3e))
* allow multiple rpc and notary hosts
([a6b49a5](https://github.com/argonprotocol/mainchain/commit/a6b49a5a2b5399d412d91b0b29f082761c0e164b))
* **node:** configurable miner threads
([c561480](https://github.com/argonprotocol/mainchain/commit/c5614809a8149f717e81edbb389544f3f7282517))
* **pallet:** localchain relay
([a6e5572](https://github.com/argonprotocol/mainchain/commit/a6e5572a328d80e87663d2572d9c7dcf998a1371))
* bond pallet
([d31e621](https://github.com/argonprotocol/mainchain/commit/d31e6215d777ccc597ad0a6c744a050c4b8c1e4f))
* add tax block seal
([6e29972](https://github.com/argonprotocol/mainchain/commit/6e29972602b386902c6a411b2425baf135537e43))
* add block author and give fees to author
([4977c93](https://github.com/argonprotocol/mainchain/commit/4977c931156a0e1e701da77a30606fd58f8c45ab))
* ulixee balances separate from argons
([c19fe03](https://github.com/argonprotocol/mainchain/commit/c19fe0334c02aa5403bde98909e719cf7ca36c47))
* initial replacement of consesnsus with pow
([25b2abd](https://github.com/argonprotocol/mainchain/commit/25b2abda82529e2c9728c7f3fd31e18f6425299f))

#### Fixes

* save cpi state for reboots
([d87307d](https://github.com/argonprotocol/mainchain/commit/d87307db00df9ae48f1b9c45161c0fb78119be11))
* adjust testnet difficulty down
([6654749](https://github.com/argonprotocol/mainchain/commit/6654749b3dd9d53eb777ce4ba7f993a878f25c79))
* **ghactions:** clear space on check linux
([7d318a9](https://github.com/argonprotocol/mainchain/commit/7d318a9aff9dc2d19749333002114dfd6b052c4b))
* **napi:** do anything to fix napi build
([ca3b16f](https://github.com/argonprotocol/mainchain/commit/ca3b16fd2989412ea782374dc4299158c8c73c85))
* **notary:** add typed errors
([e0ce8e7](https://github.com/argonprotocol/mainchain/commit/e0ce8e7761ffb202de71a9b07d5497c297809fd0))
* docker builds
([b49faf3](https://github.com/argonprotocol/mainchain/commit/b49faf36e0a8b969a469552d83333c7a5368792e))
* github actions build
([c8b2b88](https://github.com/argonprotocol/mainchain/commit/c8b2b884f9dfa90c1171dd3325d0cb60d4ea0eb8))
* github builds
([ea6e6d8](https://github.com/argonprotocol/mainchain/commit/ea6e6d829a369d81f6d9997d68e778aeef81a603))
* **vault:** require hardened xpub
([52d11ad](https://github.com/argonprotocol/mainchain/commit/52d11ad98f3a1c318aa59b2c6fc9822155271d73))
* **oracle:** nonces overlapping
([d0ddfdd](https://github.com/argonprotocol/mainchain/commit/d0ddfddd791cdffde77814a4d0dc9df87fee3018))
* **napi:** strip linux bindings
([33d824f](https://github.com/argonprotocol/mainchain/commit/33d824f8a125217c250162211b179a979e04df21))
* **e2e:** end to end test fixes
([e013a2d](https://github.com/argonprotocol/mainchain/commit/e013a2ddfb94cbd16733607f683d7c4dbb830d53))
* **github:** make run id configurable
([1bf35ae](https://github.com/argonprotocol/mainchain/commit/1bf35aec8ff0ee757f87aca7bd8eeea46cf612ae))
* **github:** test adding a new job for e2e
([28514a0](https://github.com/argonprotocol/mainchain/commit/28514a0a9e3c53fb3ee8fd51a8bddced38f95242))
* **github:** try to run tests in band
([5c208c2](https://github.com/argonprotocol/mainchain/commit/5c208c2865b23429c4250e0c79541fec2080b344))
* **ci:** try running js tests in band
([f8edca6](https://github.com/argonprotocol/mainchain/commit/f8edca68984bd46bea11b794f9515e8884ad98b4))
* js test ports
([c28c242](https://github.com/argonprotocol/mainchain/commit/c28c242e40a3a686e49f600f751c8897b1cc0fd6))
* **docker:** docker image needs python + security
([3931bea](https://github.com/argonprotocol/mainchain/commit/3931bead00cbb896ac749f957154638f823bea7f))
* **notebook:** do not halt if bad notebook data
([633b503](https://github.com/argonprotocol/mainchain/commit/633b503a36a4a613758f5ee460b711431ce3c40a))
* **vaults:** convert min fees to base + prorata
([a77dc87](https://github.com/argonprotocol/mainchain/commit/a77dc8717a589201d4ada599f66c24bbaf781b59))
* **node:** submit any stronger seals
([9459bd2](https://github.com/argonprotocol/mainchain/commit/9459bd28230c39fdf28ecb9c0c9045da23b6d750))
* **notary:** add signature message prefixes
([87a84d5](https://github.com/argonprotocol/mainchain/commit/87a84d5522b9cc6a2af0f4a7455f01758a0a0e38))
* use fixed u128 for prices and rates
([4708dbe](https://github.com/argonprotocol/mainchain/commit/4708dbe2e370788314e1c630cdceabe942958bea))
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
* **localchain_relay:** convert to shared notary pr
([c37b549](https://github.com/argonprotocol/mainchain/commit/c37b54951d2f11105ee7377a48bb589cd55d993d))
* **notary:** preserve old notary keys
([4ba062c](https://github.com/argonprotocol/mainchain/commit/4ba062c6d3fa19009bb650ae47991512c10e2332))
