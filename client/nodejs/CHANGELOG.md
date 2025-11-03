# Changelog

## [v1.3.14](https://github.com/argonprotocol/mainchain/compare/v1.3.13...4be866bd0b56dcf2d3bf54acd5f235306d4d25e3) (2025-11-02)

### Fixes

* update distribution of miners chosen
([7a771a6](https://github.com/argonprotocol/mainchain/commit/7a771a6fd8990033587865d3188980f42086bc47))

### [v1.3.13](https://github.com/argonprotocol/mainchain/compare/v1.3.12...v1.3.13) (2025-10-30)

#### Features

* **client/nodejs:** allow external progress
([5a99c96](https://github.com/argonprotocol/mainchain/commit/5a99c962ec04e4d35517f05711ec21bd9faf37e5))

### [v1.3.12](https://github.com/argonprotocol/mainchain/compare/v1.3.11...v1.3.12) (2025-10-27)

#### Fixes

* **client/nodejs:** add fee to btc release cost
([168fade](https://github.com/argonprotocol/mainchain/commit/168fade5c63a182cc28ff1818b884cd077591ba3))
* **treasury:** fix double burn of uncollected
([071bb24](https://github.com/argonprotocol/mainchain/commit/071bb24308c51b4d8ff2dac3322266623656b008))

### [v1.3.11](https://github.com/argonprotocol/mainchain/compare/v1.3.10...v1.3.11) (2025-10-15)

### [v1.3.10](https://github.com/argonprotocol/mainchain/compare/v1.3.9...v1.3.10) (2025-10-15)

### [v1.3.9](https://github.com/argonprotocol/mainchain/compare/v1.3.8...v1.3.9) (2025-10-12)

#### Fixes

* **migration:** fix comparison in migration
([f0c2efd](https://github.com/argonprotocol/mainchain/commit/f0c2efd7123b34db0f412c0d2d4fe470b8f67582))
* **treasury:** don’t roll vault earnings
([3f6cbb9](https://github.com/argonprotocol/mainchain/commit/3f6cbb9c5c79ff19819008f348b2ca62a25aa916))
* **treasury:** change frame for bonds
([b311295](https://github.com/argonprotocol/mainchain/commit/b31129544d50ddea503ddff19a3790dda1f9a65c))
* **client:** bitcoin locks liquidity promised
([d9b2594](https://github.com/argonprotocol/mainchain/commit/d9b25943fe9dcdbd992bf8b8ee67227364ab65c7))

### [v1.3.8](https://github.com/argonprotocol/mainchain/compare/v1.3.7...v1.3.8) (2025-10-02)

#### Features

* **bitcoin:** track locked liquidity value
([2149d7f](https://github.com/argonprotocol/mainchain/commit/2149d7fb8895754c40a9a9c42c64ce5271519665))

#### Fixes

* **vaults:** only allow a single vault per account
([b567081](https://github.com/argonprotocol/mainchain/commit/b56708170eb912983261fe260a59a2c86e8f26e8))
* **nodejs:** cohort bidder subscriptions broken
([dee6c33](https://github.com/argonprotocol/mainchain/commit/dee6c33d9a3524f77e2f31aa56b93c4a41b4587f))

### [v1.3.7](https://github.com/argonprotocol/mainchain/compare/v1.3.6...v1.3.7) (2025-08-14)

#### Features

* **client/nodejs:** add account minisecret
([d21d36a](https://github.com/argonprotocol/mainchain/commit/d21d36a82cc7139c3a5c42c0d367673691259b14))

#### Fixes

* **client/nodejs:** handle negative bigints
([1688fe4](https://github.com/argonprotocol/mainchain/commit/1688fe429e740fbc69709754af60d67c6b772324))

### [v1.3.6](https://github.com/argonprotocol/mainchain/compare/v1.3.5...v1.3.6) (2025-08-05)

#### Features

* **client/nodejs:** cohort bidder callbacks
([7d017bd](https://github.com/argonprotocol/mainchain/commit/7d017bd7f592d40277bc138c18059ac9497878df))
* **liquidity_pool:** simplify prebond amount
([f4242c4](https://github.com/argonprotocol/mainchain/commit/f4242c4c6d079dc86982cfa8b9f20b4c5db522c3))
* **vaults:** earnings must be collected
([ab688ce](https://github.com/argonprotocol/mainchain/commit/ab688cebc6354ec66b8dfe4a488ad1f1f13ad028))

#### Fixes

* **block_rewards:** grant fees to rewards account
([5bb00c5](https://github.com/argonprotocol/mainchain/commit/5bb00c5158be1340bf4107627d3d367a80832476))

### [v1.3.5](https://github.com/argonprotocol/mainchain/compare/v1.3.4...v1.3.5) (2025-07-28)

#### Fixes

* **bitcoin/nodejs:** enable signet
([75d6857](https://github.com/argonprotocol/mainchain/commit/75d6857755bd7456716260f1d8d3ac7d3ca20053))

### [v1.3.4](https://github.com/argonprotocol/mainchain/compare/v1.3.3...v1.3.4) (2025-07-27)

#### Features

* **client/nodejs:** add progress callback to transaction submission
([cdc7d12](https://github.com/argonprotocol/mainchain/commit/cdc7d12f11cb83f86f90d120dbfa685fc6bdda8a))

#### Fixes

* **bitcoin/nodejs:** signing without derive issues
([f2e3831](https://github.com/argonprotocol/mainchain/commit/f2e3831af7ac17d13d94fdc3575c6107c9a5aa43))
* **bitcoin/nodejs:** fix browser compat
([69e364e](https://github.com/argonprotocol/mainchain/commit/69e364e9502c758051b36db87b514dbbcfa7e1de))

### [v1.3.3](https://github.com/argonprotocol/mainchain/compare/v1.3.2...v1.3.3) (2025-07-23)

#### Features

* **client:** make nodejs libs browser friendly
([3d2df95](https://github.com/argonprotocol/mainchain/commit/3d2df95b5766b9a4729d3440f2307a2404c19ba8))

### [v1.3.2](https://github.com/argonprotocol/mainchain/compare/v1.3.1...v1.3.2) (2025-07-07)

### [v1.3.1](https://github.com/argonprotocol/mainchain/compare/v1.3.0...v1.3.1) (2025-07-06)

## [v1.3.0](https://github.com/argonprotocol/mainchain/compare/v1.2.0...v1.3.0) (2025-07-04)

### Features

* **liquidity_pools:** vault operator prebonding
([e6066fe](https://github.com/argonprotocol/mainchain/commit/e6066febe05897393c648afca2bce75d5cba6772))
* **node:** allow multiple miners to submit a vote
([b98e8fa](https://github.com/argonprotocol/mainchain/commit/b98e8fa81e8982f7879601c048f19514886bfaa0))
* **bitcoin:** nodejs library for psbt
([83d2288](https://github.com/argonprotocol/mainchain/commit/83d2288eced197b87087736719662513cac17753))

### Fixes

* **bitcoin:** wasm generation
([1ceaa00](https://github.com/argonprotocol/mainchain/commit/1ceaa00406b69d79ba13ac8d189de1975e5af578))

## [v1.2.0](https://github.com/argonprotocol/mainchain/compare/v1.1.0...v1.2.0) (2025-06-11)

### Features

* **bitcoin:** use xpriv for bitcoin
([0847d50](https://github.com/argonprotocol/mainchain/commit/0847d50158d1decc25b1f935242f2edfa8981b69))
* **bitcoin_lock:** ratcheting
([d47296e](https://github.com/argonprotocol/mainchain/commit/d47296e42e763ac182d496fe0a002441d70920e2))
* **mining_slot:** dynamic seats
([d874e82](https://github.com/argonprotocol/mainchain/commit/d874e82b2a2ef42614e739d7e9472318a99769c7))

### Fixes

* **client:** tests broken from xpriv
([7eddd59](https://github.com/argonprotocol/mainchain/commit/7eddd59d2058e9dfc2631954f98b05c6acc916ff))
* remove nanoevents dependency
([8009109](https://github.com/argonprotocol/mainchain/commit/8009109481f8d75043b93a2dc5682b9383b1f718))
* **client/node:** don’t make deps optional
([1328e6e](https://github.com/argonprotocol/mainchain/commit/1328e6e1834d969bba01de135ca32b391f6d3919))

## [v1.1.0](https://github.com/argonprotocol/mainchain/compare/v1.0.18...v1.1.0) (2025-05-12)

### Features

* **vaults:** track revenue per frame
([0147fce](https://github.com/argonprotocol/mainchain/commit/0147fce4098aa9a59d03e135d145a0fc826dddf2))
* **client:** add bidding history to cohortbidder
([8a69d17](https://github.com/argonprotocol/mainchain/commit/8a69d17184682baa34d70262488eb6813c6ce865))
* **client:** ability to use a password file
([b08cfdd](https://github.com/argonprotocol/mainchain/commit/b08cfdd9f9a6065a525968d6b7d540a918c82dce))
* **mining_slot:** add tick to bids + mining reg
([4e8e475](https://github.com/argonprotocol/mainchain/commit/4e8e4755af2367e42e042be76e79b880c31141e4))
* upgrade to polkadot-sdk umbrella
([06f0e09](https://github.com/argonprotocol/mainchain/commit/06f0e098f65e19204d1da64e22e6dcf096a859ad))
* hyperbridge doc
([fd8a425](https://github.com/argonprotocol/mainchain/commit/fd8a425e82dfb7adc923805dafbb34818f12a108))
* **client:** add nodejs cli + bidding
([babcfb6](https://github.com/argonprotocol/mainchain/commit/babcfb6070d7d430ba13b39a0bbbbd7d9ffb2bcd))
* **mining-bonds:** Mining Bonds
([ab75e7f](https://github.com/argonprotocol/mainchain/commit/ab75e7fa8e5804d2e9df31ee5a880f7caed0f5e7))

### Fixes

* **client:** cap budget by balance
([4286a17](https://github.com/argonprotocol/mainchain/commit/4286a172c15741fd3ba618ece025cdf959bc8456))
* **client:** rotation id still one off
([fe3d1d6](https://github.com/argonprotocol/mainchain/commit/fe3d1d6c7571b84374a85b698246179ea89ba4f7))
* **node:** additional fast sync fixes
([45adbc5](https://github.com/argonprotocol/mainchain/commit/45adbc5c5c4c581311c66206f50ac453cd59db22))
* mining rotations calculation
([58e39df](https://github.com/argonprotocol/mainchain/commit/58e39df794b965585a6ef38274a8d79054f2f9d4))
* **bitcoin_locks:** add error decode
([59771b5](https://github.com/argonprotocol/mainchain/commit/59771b5e5fc91e57843ca582634f1a70fd275201))
* **client:** codeql issue
([edb2c7d](https://github.com/argonprotocol/mainchain/commit/edb2c7dd5a3cd1bb2da8e06bc7e3b3b5b0dc73a1))
* **nodejs:** change rotation calculation
([476035c](https://github.com/argonprotocol/mainchain/commit/476035c2befa0baf17670c0f000a7b3307757bee))
* bump spec version
([ea2887b](https://github.com/argonprotocol/mainchain/commit/ea2887ba1fe2f95e007e093c9bda9c8e566d62f5))
* **client:** update nodejs testing
([1e0908c](https://github.com/argonprotocol/mainchain/commit/1e0908cbeeefadf71255657c7c286d5205f0ee3a))

### [v1.0.18](https://github.com/argonprotocol/mainchain/compare/v1.0.17...v1.0.18) (2025-03-22)

#### Features

* **mint:** track per-cohort mint amounts
([5350fdd](https://github.com/argonprotocol/mainchain/commit/5350fddf176ee9ddf184be6762a81277192b4342))
* **proxy:** add a mining bid and vault proxy
([42f69c4](https://github.com/argonprotocol/mainchain/commit/42f69c4b6b3f63af2a278ace434e7d6cf9e849b4))
* **block_rewards:** scale with target price
([23104e9](https://github.com/argonprotocol/mainchain/commit/23104e9e2f2cde69e4ea6421a0941aace93ce57c))
* bid pools for mining
([010b838](https://github.com/argonprotocol/mainchain/commit/010b838e836b5e09c2cdf3f860f0e8a9412032b1))

#### Fixes

* **vaults:** store apr for calculating prorata
([1bbcaa1](https://github.com/argonprotocol/mainchain/commit/1bbcaa1842e0e25dcef7722d34a41bd19fba6c13))
* **mining_slot:** miners losing lookup index
([c55e6b0](https://github.com/argonprotocol/mainchain/commit/c55e6b01fc4519e42bcc6f52d2d2beea2f9af601))
* **mining_slot:** allow full balance bid
([26e08ea](https://github.com/argonprotocol/mainchain/commit/26e08ea600ff6f39f7896adbea1322b264579059))

### [v1.0.17](https://github.com/argonprotocol/mainchain/compare/v1.0.16...v1.0.17) (2025-03-14)

#### Fixes

* **mint:** use twal for liquidity in mint
([b997171](https://github.com/argonprotocol/mainchain/commit/b997171ae1bb4db99c1ebedfdbc331f4c372b4fc))
* **bitcoin:** allow unlock of mismatched utxo
([6ead704](https://github.com/argonprotocol/mainchain/commit/6ead7045ce43e3f002bcf551d635b774e9d23410))

### [v1.0.16](https://github.com/argonprotocol/mainchain/compare/v1.0.15...v1.0.16) (2025-02-26)

### [v1.0.15](https://github.com/argonprotocol/mainchain/compare/v1.0.14...v1.0.15) (2025-02-26)

#### Fixes

* **mining_slot:** don’t allow vault mismatch
([5b5480b](https://github.com/argonprotocol/mainchain/commit/5b5480b4d7a00bd5979ff592dc4fb84949ff3ab6))

### [v1.0.14](https://github.com/argonprotocol/mainchain/compare/v1.0.13...v1.0.14) (2025-02-21)

### [v1.0.13](https://github.com/argonprotocol/mainchain/compare/v1.0.12...v1.0.13) (2025-02-17)

#### Fixes

* **vaults:** vault activation delay
([e04dcfc](https://github.com/argonprotocol/mainchain/commit/e04dcfcc188ea46276f8ea5024d7f14b1d6c1ff3))

### [v1.0.12](https://github.com/argonprotocol/mainchain/compare/v1.0.11...v1.0.12) (2025-02-12)

#### Features

* **block_rewards:** disable compute rewards
([fd393c7](https://github.com/argonprotocol/mainchain/commit/fd393c7c157d999651545cdba203f8ecf9d7e86f))
* **mining_slot:** disallow duplicate key reg
([4660021](https://github.com/argonprotocol/mainchain/commit/4660021aa2f77ddb0ca62cb712a9ac74fb433f82))

### [v1.0.11](https://github.com/argonprotocol/mainchain/compare/v1.0.10...v1.0.11) (2025-02-10)

#### Fixes

* separate runtimes
([5c5df56](https://github.com/argonprotocol/mainchain/commit/5c5df562356a7f2143e6e2fd0f99c35f4f00bbe4))

### [v1.0.10](https://github.com/argonprotocol/mainchain/compare/v1.0.9...v1.0.10) (2025-02-07)

#### Fixes

* **vault:** disable reward sharing for now
([768351b](https://github.com/argonprotocol/mainchain/commit/768351b19cdd30b023b05a185f6195492ea82c7c))
* convert mining slots gap to ticks
([4035bfd](https://github.com/argonprotocol/mainchain/commit/4035bfd774449087a8cf60b1c5d22efbdc23a01b))
* **runtme:** fixup grandpa set id history
([4614da2](https://github.com/argonprotocol/mainchain/commit/4614da20284fecb01488ddb0fe30c5d3538a14b9))

### [v1.0.9](https://github.com/argonprotocol/mainchain/compare/v1.0.8...v1.0.9) (2025-01-28)

#### Features

* **block_rewards:** ability to pause rewards
([a81c547](https://github.com/argonprotocol/mainchain/commit/a81c547149a2d5a13e378c5be13ac4478635ffdc))

### [v1.0.8](https://github.com/argonprotocol/mainchain/compare/v1.0.7...v1.0.8) (2025-01-27)

### [v1.0.7](https://github.com/argonprotocol/mainchain/compare/v1.0.6...v1.0.7) (2025-01-27)

### [v1.0.6](https://github.com/argonprotocol/mainchain/compare/v1.0.5...v1.0.6) (2025-01-24)

### [v1.0.5](https://github.com/argonprotocol/mainchain/compare/v1.0.4...v1.0.5) (2025-01-23)

### [v1.0.4](https://github.com/argonprotocol/mainchain/compare/v1.0.3...v1.0.4) (2025-01-21)

#### Fixes

* needed to update spec_version
([bcc326f](https://github.com/argonprotocol/mainchain/commit/bcc326f9682691a3a0d56b093a3dc1e3a272d481))

### [v1.0.3](https://github.com/argonprotocol/mainchain/compare/v1.0.2...v1.0.3) (2025-01-21)

#### Fixes

* **vaults:** account for pending bitcoin
([733071b](https://github.com/argonprotocol/mainchain/commit/733071be1a1cf4ad39c2323473b6d329838c0e64))

### [v1.0.2](https://github.com/argonprotocol/mainchain/compare/v1.0.1...v1.0.2) (2025-01-18)

### [v1.0.1](https://github.com/argonprotocol/mainchain/compare/v1.0.0...v1.0.1) (2025-01-16)

## [v1.0.0](https://github.com/argonprotocol/mainchain/compare/v0.0.27...v1.0.0) (2025-01-15)

### [v0.0.27](https://github.com/argonprotocol/mainchain/compare/v0.0.26...v0.0.27) (2025-01-14)

#### Fixes

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

### [v0.0.24](https://github.com/argonprotocol/mainchain/compare/v0.0.23...v0.0.24) (2024-12-19)

### [v0.0.23](https://github.com/argonprotocol/mainchain/compare/v0.0.22...v0.0.23) (2024-12-19)

### [v0.0.22](https://github.com/argonprotocol/mainchain/compare/v0.0.21...v0.0.22) (2024-12-16)

### [v0.0.21](https://github.com/argonprotocol/mainchain/compare/v0.0.20...v0.0.21) (2024-12-07)

### [v0.0.20](https://github.com/argonprotocol/mainchain/compare/v0.0.19...v0.0.20) (2024-12-06)

### [v0.0.19](https://github.com/argonprotocol/mainchain/compare/v0.0.18...v0.0.19) (2024-12-05)

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

### [v0.0.15](https://github.com/argonprotocol/mainchain/compare/v0.0.14...v0.0.15) (2024-10-24)

#### Features

* **notary:** stop apis if audit fails
([5787461](https://github.com/argonprotocol/mainchain/commit/57874612671dc6f4f178438a86653592d6fa0bee))
* **notebook:** allow a notary to recover
([e1246d5](https://github.com/argonprotocol/mainchain/commit/e1246d5c61fd4e1397bd65b4b5e173455098acd0))

#### Fixes

* **ticks:** only allow a single block per tick
([cdf295a](https://github.com/argonprotocol/mainchain/commit/cdf295aae082adae7f72deb4ddc9517b48e9ccbd))

### [v0.0.14](https://github.com/argonprotocol/mainchain/compare/v0.0.13...v0.0.14) (2024-10-10)

### [v0.0.13](https://github.com/argonprotocol/mainchain/compare/v0.0.12...v0.0.13) (2024-10-09)

### [v0.0.12](https://github.com/argonprotocol/mainchain/compare/v0.0.11...v0.0.12) (2024-10-08)

### [v0.0.11](https://github.com/argonprotocol/mainchain/compare/v0.0.10...v0.0.11) (2024-10-07)

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

### [v0.0.2](https://github.com/argonprotocol/mainchain/compare/v0.0.1...v0.0.2) (2024-08-27)

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
