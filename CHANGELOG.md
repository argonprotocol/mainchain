# Changelog

## [v1.3.24](https://github.com/argonprotocol/mainchain/compare/v1.3.23...29310689f681062caa25a8413a44254f5b0a5043) (2025-12-10)

### Fixes

* migrate argons per block to min
([2931068](https://github.com/argonprotocol/mainchain/commit/29310689f681062caa25a8413a44254f5b0a5043))

### [v1.3.23](https://github.com/argonprotocol/mainchain/compare/v1.3.22...v1.3.23) (2025-12-08)

#### Features

* update mining block growth trend
([ac0be7b](https://github.com/argonprotocol/mainchain/commit/ac0be7ba23032d0367ab39664030ae3f89558678))

### [v1.3.22](https://github.com/argonprotocol/mainchain/compare/v1.3.21...v1.3.22) (2025-12-03)

#### Features

* add frame id to digest log
([50228b3](https://github.com/argonprotocol/mainchain/commit/50228b331236bcc9a45dbb695e9abfbb67ad1015))
* Mining Frame transition to reward ticks
([e67bb6d](https://github.com/argonprotocol/mainchain/commit/e67bb6d6cdf56b154f86ea20ce5c9c7f64ec3610))

#### Fixes

* **mint:** don’t take min when cpi positive
([7ab2987](https://github.com/argonprotocol/mainchain/commit/7ab298711eeac627caa92290aac0eb3e634e3f8a))
* **treasury:** don’t roll off funders under min
([7839221](https://github.com/argonprotocol/mainchain/commit/78392216c5402ee05de354f88d4a43d46e49ca60))

### [v1.3.21](https://github.com/argonprotocol/mainchain/compare/v1.3.20...v1.3.21) (2025-11-20)

#### Features

* **proxy:** add a refunding fee type of proxy
([5520d31](https://github.com/argonprotocol/mainchain/commit/5520d31eecf09133ee92d7cd18d11cd0672fea7d))
* **client/nodejs:** preregister metadata
([c57a919](https://github.com/argonprotocol/mainchain/commit/c57a9198af68557811dbe417aa89aa6ed80bafb3))

#### Fixes

* handle fees with refunds
([b9cf0f8](https://github.com/argonprotocol/mainchain/commit/b9cf0f887abdec389469b1b0a8a5cc41622a5585))
* copilot fixes
([025ad0a](https://github.com/argonprotocol/mainchain/commit/025ad0a17af9292cc93ea9b035c2de00bc6952f3))
* **oracle:** handle timeframe mismatches for price
([ca37e67](https://github.com/argonprotocol/mainchain/commit/ca37e678fce1c38dab123a599712431f9bad2627))
* **localchain:** handle unreconnecting client
([6039592](https://github.com/argonprotocol/mainchain/commit/60395923190f7a306fe0ee3a4804c499944eccb8))
* **mining_slot:** schedule cohort size changes
([1c2458e](https://github.com/argonprotocol/mainchain/commit/1c2458e26986d773e2f88b54699249444588c077))

### [v1.3.20](https://github.com/argonprotocol/mainchain/compare/v1.3.19...v1.3.20) (2025-11-13)

#### Fixes

* **mining_slot:** fix mining nonce score history
([ce9d920](https://github.com/argonprotocol/mainchain/commit/ce9d920515bb345ab0a88fd3d2758bf830ba8768))

### [v1.3.19](https://github.com/argonprotocol/mainchain/compare/v1.3.18...v1.3.19) (2025-11-08)

### [v1.3.18](https://github.com/argonprotocol/mainchain/compare/v1.3.17...v1.3.18) (2025-11-06)

#### Fixes

* impose a penalty for recent block closers
([fa4a3a1](https://github.com/argonprotocol/mainchain/commit/fa4a3a1ba86bd80142e17c737a03ec5c9ab374e1))

### [v1.3.17](https://github.com/argonprotocol/mainchain/compare/v1.3.16...v1.3.17) (2025-11-04)

#### Fixes

* **mining_slot:** handle large negatives
([b8d14fd](https://github.com/argonprotocol/mainchain/commit/b8d14fd3a5120f47899b2e3147d39a5d5cb4beb9))

### [v1.3.16](https://github.com/argonprotocol/mainchain/compare/v1.3.15...v1.3.16) (2025-11-04)

#### Fixes

* miner nonce for wrong tick
([1ea2ca3](https://github.com/argonprotocol/mainchain/commit/1ea2ca3907c16ac2b3c65b1823870cd714b58ef5))

### [v1.3.15](https://github.com/argonprotocol/mainchain/compare/v1.3.14...v1.3.15) (2025-11-03)

#### Fixes

* use safe math
([87d4ad0](https://github.com/argonprotocol/mainchain/commit/87d4ad0e3056f50a1d0378c50aa533cdc3e1be9a))
* change mining slot max miners per cohort
([e7a0b00](https://github.com/argonprotocol/mainchain/commit/e7a0b00b932e730f8298c6b34479722f3a156a8f))
* **mining_slot:** adjust distribution over time
([fe02942](https://github.com/argonprotocol/mainchain/commit/fe029426b0adadb3948a2b284c2e230e33974ab4))
* **mining_slot:** fix ties in miner nonce scores
([34a242d](https://github.com/argonprotocol/mainchain/commit/34a242d8e0ca50655f5593635d55f7337088677c))
* **mining_slot:** fix ties in miner nonce scores
([aff9c54](https://github.com/argonprotocol/mainchain/commit/aff9c54d058d5c638cfc11329647e1a3811399ce))

### [v1.3.14](https://github.com/argonprotocol/mainchain/compare/v1.3.13...v1.3.14) (2025-11-02)

#### Fixes

* adjust some comments per feedback
([56363f4](https://github.com/argonprotocol/mainchain/commit/56363f4c50c5db7cc1aac194c4b9ac4b060d6947))
* update distribution of miners chosen
([7a771a6](https://github.com/argonprotocol/mainchain/commit/7a771a6fd8990033587865d3188980f42086bc47))

### [v1.3.13](https://github.com/argonprotocol/mainchain/compare/v1.3.12...v1.3.13) (2025-10-30)

#### Features

* **client/nodejs:** allow external progress
([5a99c96](https://github.com/argonprotocol/mainchain/commit/5a99c962ec04e4d35517f05711ec21bd9faf37e5))

### [v1.3.12](https://github.com/argonprotocol/mainchain/compare/v1.3.11...v1.3.12) (2025-10-27)

#### Fixes

* remove unneeded var per copilot feedback
([8887c64](https://github.com/argonprotocol/mainchain/commit/8887c6487be7ed20f67eb00320bf6d84ad8666bf))
* **node:** hanging sync for notary
([7c4fcb4](https://github.com/argonprotocol/mainchain/commit/7c4fcb4ea7486a3e93cf8d43d2876032264063e1))
* **client/nodejs:** add fee to btc release cost
([168fade](https://github.com/argonprotocol/mainchain/commit/168fade5c63a182cc28ff1818b884cd077591ba3))
* **treasury:** prebond not adding existing correct
([c7f3092](https://github.com/argonprotocol/mainchain/commit/c7f3092f77f33fbf57a1403314f83fdee68aa99f))
* **treasury:** fix double burn of uncollected
([071bb24](https://github.com/argonprotocol/mainchain/commit/071bb24308c51b4d8ff2dac3322266623656b008))

### [v1.3.11](https://github.com/argonprotocol/mainchain/compare/v1.3.10...v1.3.11) (2025-10-15)

#### Fixes

* increment spec
([d7a7cff](https://github.com/argonprotocol/mainchain/commit/d7a7cffe5e5e5185dafed8314065e76b49d9c0e1))
* **treasury:** remove infinite loop
([4ee4f7c](https://github.com/argonprotocol/mainchain/commit/4ee4f7cce4918c57b0c60c76f78bd10b54615ad1))

### [v1.3.10](https://github.com/argonprotocol/mainchain/compare/v1.3.9...v1.3.10) (2025-10-15)

#### Fixes

* **treasury:** migrations broke
([da22ef6](https://github.com/argonprotocol/mainchain/commit/da22ef68dc585973a528fba2890ef442d17f03ca))

### [v1.3.9](https://github.com/argonprotocol/mainchain/compare/v1.3.8...v1.3.9) (2025-10-12)

#### Fixes

* **migration:** fix comparison in migration
([f0c2efd](https://github.com/argonprotocol/mainchain/commit/f0c2efd7123b34db0f412c0d2d4fe470b8f67582))
* **treasury:** don’t roll vault earnings
([3f6cbb9](https://github.com/argonprotocol/mainchain/commit/3f6cbb9c5c79ff19819008f348b2ca62a25aa916))
* add revenue migration
([552fc5f](https://github.com/argonprotocol/mainchain/commit/552fc5fcd09f8e8f87b69d1ea0e08304da6ef7ba))
* **vaults:** migration bug
([b8e50de](https://github.com/argonprotocol/mainchain/commit/b8e50de742fb5215d71091c4f942233244e2b1fe))
* **bitcoin:** output tx for release
([9171dbc](https://github.com/argonprotocol/mainchain/commit/9171dbcaf338f2b87eab45bddaa59faff5db829c))
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
* **runtime:** remove 1 second minimum time
([87891be](https://github.com/argonprotocol/mainchain/commit/87891be737d029b084c23b065beec3ad7ca829c2))
* bitcoin wasm builds broken
([6411c57](https://github.com/argonprotocol/mainchain/commit/6411c571c4b7abdd1aa5bb826d647ec2cd1aebc5))
* **node:** don’t set best block in verifier
([03d3289](https://github.com/argonprotocol/mainchain/commit/03d32890cc079708dbe2a0f74814b5ed3d0878f0))
* **nodejs:** cohort bidder subscriptions broken
([dee6c33](https://github.com/argonprotocol/mainchain/commit/dee6c33d9a3524f77e2f31aa56b93c4a41b4587f))

### [v1.3.7](https://github.com/argonprotocol/mainchain/compare/v1.3.6...v1.3.7) (2025-08-14)

#### Features

* **client/nodejs:** add account minisecret
([d21d36a](https://github.com/argonprotocol/mainchain/commit/d21d36a82cc7139c3a5c42c0d367673691259b14))

#### Fixes

* **client/nodejs:** handle negative bigints
([1688fe4](https://github.com/argonprotocol/mainchain/commit/1688fe429e740fbc69709754af60d67c6b772324))
* **node:** improve fast sync speed
([b6ef61f](https://github.com/argonprotocol/mainchain/commit/b6ef61f9d1ad6bb81dedb65ffee3f8e019a83740))

### [v1.3.6](https://github.com/argonprotocol/mainchain/compare/v1.3.5...v1.3.6) (2025-08-05)

#### Features

* **client/nodejs:** cohort bidder callbacks
([7d017bd](https://github.com/argonprotocol/mainchain/commit/7d017bd7f592d40277bc138c18059ac9497878df))
* **liquidity_pool:** simplify prebond amount
([f4242c4](https://github.com/argonprotocol/mainchain/commit/f4242c4c6d079dc86982cfa8b9f20b4c5db522c3))
* **vaults:** earnings must be collected
([ab688ce](https://github.com/argonprotocol/mainchain/commit/ab688cebc6354ec66b8dfe4a488ad1f1f13ad028))
* **vaults:** don’t charge operator for bitcoin
([88262ba](https://github.com/argonprotocol/mainchain/commit/88262baa62ea9fbd498e60c6250fadb940bf3817))

#### Fixes

* **docker:** use mempool electrs
([19fddbe](https://github.com/argonprotocol/mainchain/commit/19fddbe93f5f26a5b20a7f3ea84bdd81c43f95d6))
* **oracle:** add a lifetime to price index
([382cbd8](https://github.com/argonprotocol/mainchain/commit/382cbd85434ebac9544522c65483f1b275bdd47a))
* **node:** don’t do duplicate check on reimports
([0a75855](https://github.com/argonprotocol/mainchain/commit/0a75855f45882d67f911629e361e26d5f0fa3588))
* **block_rewards:** grant fees to rewards account
([5bb00c5](https://github.com/argonprotocol/mainchain/commit/5bb00c5158be1340bf4107627d3d367a80832476))

### [v1.3.5](https://github.com/argonprotocol/mainchain/compare/v1.3.4...v1.3.5) (2025-07-28)

#### Fixes

* **docker:** use a named wallet to avoid conflict
([d1f9d89](https://github.com/argonprotocol/mainchain/commit/d1f9d897f84011aa84cfe4f7f824872b35562764))
* **node:** migration broke notebooks
([2c07ca0](https://github.com/argonprotocol/mainchain/commit/2c07ca022f31e65fecb190924b3dbad781def42c))
* **bitcoin/nodejs:** enable signet
([75d6857](https://github.com/argonprotocol/mainchain/commit/75d6857755bd7456716260f1d8d3ac7d3ca20053))

### [v1.3.4](https://github.com/argonprotocol/mainchain/compare/v1.3.3...v1.3.4) (2025-07-27)

#### Features

* **client/nodejs:** add progress callback to transaction submission
([cdc7d12](https://github.com/argonprotocol/mainchain/commit/cdc7d12f11cb83f86f90d120dbfa685fc6bdda8a))

#### Fixes

* **docker:** use correct image and user
([4009f95](https://github.com/argonprotocol/mainchain/commit/4009f953d01a44711f7fba9f16a8a4d98c67a6f5))
* **bitcoin/nodejs:** signing without derive issues
([f2e3831](https://github.com/argonprotocol/mainchain/commit/f2e3831af7ac17d13d94fdc3575c6107c9a5aa43))
* **bitcoin/nodejs:** fix browser compat
([69e364e](https://github.com/argonprotocol/mainchain/commit/69e364e9502c758051b36db87b514dbbcfa7e1de))
* **tests:** don’t loop forever in bitcoin test
([5860e89](https://github.com/argonprotocol/mainchain/commit/5860e8935dc0f87ffdb035efd56bcb9f5061fe06))

### [v1.3.3](https://github.com/argonprotocol/mainchain/compare/v1.3.2...v1.3.3) (2025-07-23)

#### Features

* **docker:** add voting cli and bitcoin mining
([3c580c6](https://github.com/argonprotocol/mainchain/commit/3c580c637918d89703e1ff7b1697db83ff4b0b7d))
* **client:** make nodejs libs browser friendly
([3d2df95](https://github.com/argonprotocol/mainchain/commit/3d2df95b5766b9a4729d3440f2307a2404c19ba8))

#### Fixes

* **mining_slot:** migration bug for next frame
([dc3ead1](https://github.com/argonprotocol/mainchain/commit/dc3ead15481b2a8b2f8b1a3d4c881feaf1dcd610))
* **node:** remove best block sync service check
([7dd7909](https://github.com/argonprotocol/mainchain/commit/7dd790988d16f38e77bbea2a51f3ddc7522e4809))

### [v1.3.2](https://github.com/argonprotocol/mainchain/compare/v1.3.1...v1.3.2) (2025-07-07)

#### Features

* **bitcoin/nodejs:** expose bitcoinjs primitives
([8730f84](https://github.com/argonprotocol/mainchain/commit/8730f84afd6effda4b845f552684cb674479c085))

#### Fixes

* **compute:** ensure solver version matches
([699dd3f](https://github.com/argonprotocol/mainchain/commit/699dd3ffa567afa8c9fec898d53ebdb6e373b2d0))
* **node:** backwards compatiblity.
([6579d52](https://github.com/argonprotocol/mainchain/commit/6579d52847c27729d80788dffbb6eca3a180c374))
* **node:** enable rocks db in node
([5a02820](https://github.com/argonprotocol/mainchain/commit/5a028208b676d779cb03ddb69a50891bee7d356a))

### [v1.3.1](https://github.com/argonprotocol/mainchain/compare/v1.3.0...v1.3.1) (2025-07-06)

#### Fixes

* **consensus:** log prehash for compute
([c229516](https://github.com/argonprotocol/mainchain/commit/c2295164102e7df997a6a4f10016b231548daea0))
* **actions:** add oracle for aarch to gh actions
([962e2cc](https://github.com/argonprotocol/mainchain/commit/962e2cc0e199eb66e6331985e7bb881e945b9ebf))

## [v1.3.0](https://github.com/argonprotocol/mainchain/compare/v1.2.0...v1.3.0) (2025-07-04)

### Features

* add a docker compose to run local testnet
([6d8c7f1](https://github.com/argonprotocol/mainchain/commit/6d8c7f1a1b6cf8a96ead05721efa9d9237ea80d6))
* **liquidity_pools:** vault operator prebonding
([e6066fe](https://github.com/argonprotocol/mainchain/commit/e6066febe05897393c648afca2bce75d5cba6772))
* **node:** allow multiple miners to submit a vote
([b98e8fa](https://github.com/argonprotocol/mainchain/commit/b98e8fa81e8982f7879601c048f19514886bfaa0))
* **bitcoin:** nodejs library for psbt
([83d2288](https://github.com/argonprotocol/mainchain/commit/83d2288eced197b87087736719662513cac17753))
* **vaults:** remove opened delay
([d874b04](https://github.com/argonprotocol/mainchain/commit/d874b043e30f226327b6ac390c1fec996bbabf46))

### Fixes

* **bitcoin:** wasm generation
([1ceaa00](https://github.com/argonprotocol/mainchain/commit/1ceaa00406b69d79ba13ac8d189de1975e5af578))
* **consensus:** ensure we have notebooks for block
([3dd8284](https://github.com/argonprotocol/mainchain/commit/3dd828436c1062fb69f74d5d335b0688bfc39de0))
* **import_queue:** make re-imports safe
([79ed9ce](https://github.com/argonprotocol/mainchain/commit/79ed9ceca8a004ba3621d1786fc563596ab744f9))
* **bitcoin_locks:** truncation in redemption price
([8b7726e](https://github.com/argonprotocol/mainchain/commit/8b7726e89b0e7f5d8d5deaece2edff0b685781b9))
* prevent underflow
([d8e7bf9](https://github.com/argonprotocol/mainchain/commit/d8e7bf987c30cedd8e936bd31fbca63fbb689c05))

## [v1.2.0](https://github.com/argonprotocol/mainchain/compare/v1.1.0...v1.2.0) (2025-06-11)

### Features

* **bitcoin:** use xpriv for bitcoin
([0847d50](https://github.com/argonprotocol/mainchain/commit/0847d50158d1decc25b1f935242f2edfa8981b69))
* **ci:** split docker from nodejs
([1e4d209](https://github.com/argonprotocol/mainchain/commit/1e4d2093845583e47eb619b1b95dd2f57e032c86))
* **bitcoin:** implement whitepaper unlock formula
([34581b9](https://github.com/argonprotocol/mainchain/commit/34581b91244db5116fd419889ef9d783293035e4))
* **bitcoin_lock:** ratcheting
([d47296e](https://github.com/argonprotocol/mainchain/commit/d47296e42e763ac182d496fe0a002441d70920e2))
* **mining_slot:** dynamic seats
([d874e82](https://github.com/argonprotocol/mainchain/commit/d874e82b2a2ef42614e739d7e9472318a99769c7))

### Fixes

* **client:** tests broken from xpriv
([7eddd59](https://github.com/argonprotocol/mainchain/commit/7eddd59d2058e9dfc2631954f98b05c6acc916ff))
* **ci:** attempt to fix github actions
([fa9d2fe](https://github.com/argonprotocol/mainchain/commit/fa9d2fedae733480edc7cf19ccfc4f8e51f2a353))
* remove nanoevents dependency
([8009109](https://github.com/argonprotocol/mainchain/commit/8009109481f8d75043b93a2dc5682b9383b1f718))
* **client/node:** don’t make deps optional
([1328e6e](https://github.com/argonprotocol/mainchain/commit/1328e6e1834d969bba01de135ca32b391f6d3919))
* end-to-end compile issue
([1a4e69d](https://github.com/argonprotocol/mainchain/commit/1a4e69d17c94973b5fafe554d953fff904df5a8e))

## [v1.1.0](https://github.com/argonprotocol/mainchain/compare/v1.0.18...v1.1.0) (2025-05-12)

### Features

* **vaults:** track revenue per frame
([0147fce](https://github.com/argonprotocol/mainchain/commit/0147fce4098aa9a59d03e135d145a0fc826dddf2))
* **client:** add bidding history to cohortbidder
([8a69d17](https://github.com/argonprotocol/mainchain/commit/8a69d17184682baa34d70262488eb6813c6ce865))
* **client:** ability to use a password file
([b08cfdd](https://github.com/argonprotocol/mainchain/commit/b08cfdd9f9a6065a525968d6b7d540a918c82dce))
* clear more space
([138a836](https://github.com/argonprotocol/mainchain/commit/138a836946658347963a795868c163ff348352e6))
* **mining_slot:** add tick to bids + mining reg
([4e8e475](https://github.com/argonprotocol/mainchain/commit/4e8e4755af2367e42e042be76e79b880c31141e4))
* upgrade to polkadot-sdk umbrella
([06f0e09](https://github.com/argonprotocol/mainchain/commit/06f0e098f65e19204d1da64e22e6dcf096a859ad))
* **client:** clarify storage height
([2007632](https://github.com/argonprotocol/mainchain/commit/200763227b16edb1acb6ec9d269511acfc520059))
* hyperbridge doc
([fd8a425](https://github.com/argonprotocol/mainchain/commit/fd8a425e82dfb7adc923805dafbb34818f12a108))
* **client:** add nodejs cli + bidding
([babcfb6](https://github.com/argonprotocol/mainchain/commit/babcfb6070d7d430ba13b39a0bbbbd7d9ffb2bcd))
* **mining-bonds:** Mining Bonds
([ab75e7f](https://github.com/argonprotocol/mainchain/commit/ab75e7fa8e5804d2e9df31ee5a880f7caed0f5e7))
* **notary/oracle:** verify key during insert
([ce966d5](https://github.com/argonprotocol/mainchain/commit/ce966d5cbcb27199300465726159358ca1b386c7))

### Fixes

* **vaults:** make all fees paid upfront
([c898195](https://github.com/argonprotocol/mainchain/commit/c89819521d782f415b01b4aad2068e67137894e2))
* **client:** cap budget by balance
([4286a17](https://github.com/argonprotocol/mainchain/commit/4286a172c15741fd3ba618ece025cdf959bc8456))
* **node:** check finalization before marking best
([f5dba60](https://github.com/argonprotocol/mainchain/commit/f5dba60efa43f226920537db005d09bde6f9608b))
* **gh:** glibc incapatability
([48a81d2](https://github.com/argonprotocol/mainchain/commit/48a81d24b6121199e932db2855af49288f9e3ab8))
* **testing:** bitcoin unavailable in docker
([6d8cc4e](https://github.com/argonprotocol/mainchain/commit/6d8cc4e4814fcbfa672df2a7abfe2d1ea59beb8a))
* **client:** rotation id still one off
([fe3d1d6](https://github.com/argonprotocol/mainchain/commit/fe3d1d6c7571b84374a85b698246179ea89ba4f7))
* **ga:** caching of files
([079232f](https://github.com/argonprotocol/mainchain/commit/079232f69f54b2b712a050685c490be48cda30c7))
* **node:** additional fast sync fixes
([45adbc5](https://github.com/argonprotocol/mainchain/commit/45adbc5c5c4c581311c66206f50ac453cd59db22))
* aarch64 can’t find openssl
([2ae1a4f](https://github.com/argonprotocol/mainchain/commit/2ae1a4fbb79633c49a0e6d6e41bd193f819d409e))
* check space
([b4f5107](https://github.com/argonprotocol/mainchain/commit/b4f51072e7e21cd81b2dc8cc231d91882ce8ebd7))
* **node:** handle state unavailable in sync
([34e0223](https://github.com/argonprotocol/mainchain/commit/34e0223b35db18508f144e71647b15391e268a91))
* mining rotations calculation
([58e39df](https://github.com/argonprotocol/mainchain/commit/58e39df794b965585a6ef38274a8d79054f2f9d4))
* **node:** fast sync bugs with state
([690ba42](https://github.com/argonprotocol/mainchain/commit/690ba42d141b4779c5074f37155968ce3517fc33))
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
* **node:** properly retieve aux data
([8f1eef5](https://github.com/argonprotocol/mainchain/commit/8f1eef59728f0bf981f0ae13a0cbe753c4a51e0e))
* **bitcoin:** allow https servers
([cadca32](https://github.com/argonprotocol/mainchain/commit/cadca327b4475fa7864a7f573fdb9963b831dc9d))
* crash if unable to allocate randomx for hash
([7fa84fb](https://github.com/argonprotocol/mainchain/commit/7fa84fbd8ae502aae36c79d563ca9b2ec112f43b))
* **node:** fast sync issues accessing state
([2453ba4](https://github.com/argonprotocol/mainchain/commit/2453ba4590d72aeb8f2337fe502ef39e66b0cee5))
* **srtool:** try updating rust version
([b39a781](https://github.com/argonprotocol/mainchain/commit/b39a781382d384848ba0f0d532c0856c70e2a430))

### [v1.0.18](https://github.com/argonprotocol/mainchain/compare/v1.0.17...v1.0.18) (2025-03-22)

#### Features

* **mint:** track per-cohort mint amounts
([5350fdd](https://github.com/argonprotocol/mainchain/commit/5350fddf176ee9ddf184be6762a81277192b4342))
* **proxy:** add a mining bid and vault proxy
([42f69c4](https://github.com/argonprotocol/mainchain/commit/42f69c4b6b3f63af2a278ace434e7d6cf9e849b4))
* **client:** auto-update with version changes
([5b8f8b4](https://github.com/argonprotocol/mainchain/commit/5b8f8b4e0a5bdbb456c3a736c06652416145877e))
* **block_rewards:** scale with target price
([23104e9](https://github.com/argonprotocol/mainchain/commit/23104e9e2f2cde69e4ea6421a0941aace93ce57c))
* bid pools for mining
([010b838](https://github.com/argonprotocol/mainchain/commit/010b838e836b5e09c2cdf3f860f0e8a9412032b1))

#### Fixes

* **vaults:** store apr for calculating prorata
([1bbcaa1](https://github.com/argonprotocol/mainchain/commit/1bbcaa1842e0e25dcef7722d34a41bd19fba6c13))
* **vaults:** migration broken due to stalled oblg
([d70eb4a](https://github.com/argonprotocol/mainchain/commit/d70eb4a6cb948af8a4d7a0de0d6c1ad5abaef2cf))
* **ci:** attempt to fix ci tests
([7130054](https://github.com/argonprotocol/mainchain/commit/7130054cde93709196bfa1b767ed89c9781c24cf))
* **mining_slot:** miners losing lookup index
([c55e6b0](https://github.com/argonprotocol/mainchain/commit/c55e6b01fc4519e42bcc6f52d2d2beea2f9af601))
* **vaults:** don’t distribute pool when 0 bonded
([d55d52d](https://github.com/argonprotocol/mainchain/commit/d55d52d172ad33c47da4504974e6705778b4728b))
* **sync:** warp sync not properly importing gaps
([0a184ce](https://github.com/argonprotocol/mainchain/commit/0a184ce102aca1462a0c02f6be7ec12e9815161f))
* **mining_slot:** allow full balance bid
([26e08ea](https://github.com/argonprotocol/mainchain/commit/26e08ea600ff6f39f7896adbea1322b264579059))
* **oracle:** don’t try http lookups on ci
([eb2fd34](https://github.com/argonprotocol/mainchain/commit/eb2fd34ccd2f2e5e14caf2ec559039558a6ae3f7))

### [v1.0.17](https://github.com/argonprotocol/mainchain/compare/v1.0.16...v1.0.17) (2025-03-14)

#### Fixes

* try runtime issue
([83fa5dd](https://github.com/argonprotocol/mainchain/commit/83fa5dd78f3d3795b73675fd2439ccba11ccd515))
* **mint:** use twal for liquidity in mint
([b997171](https://github.com/argonprotocol/mainchain/commit/b997171ae1bb4db99c1ebedfdbc331f4c372b4fc))
* **bitcoin:** allow unlock of mismatched utxo
([6ead704](https://github.com/argonprotocol/mainchain/commit/6ead7045ce43e3f002bcf551d635b774e9d23410))
* **mining_slot:** change bid target to 200%
([eefef90](https://github.com/argonprotocol/mainchain/commit/eefef90af50784d88e8bd9040a26fcf724dab2fa))
* **mining_slot:** use seal proof for vrf close
([321cf04](https://github.com/argonprotocol/mainchain/commit/321cf046ee67c2488cde879f6eae803783a6feb1))

### [v1.0.16](https://github.com/argonprotocol/mainchain/compare/v1.0.15...v1.0.16) (2025-02-26)

### [v1.0.15](https://github.com/argonprotocol/mainchain/compare/v1.0.14...v1.0.15) (2025-02-26)

#### Features

* add vrf close event
([4691608](https://github.com/argonprotocol/mainchain/commit/4691608df00f0f64e1400b156f53895cf7eec8af))

#### Fixes

* **mining_slot:** don’t allow vault mismatch
([5b5480b](https://github.com/argonprotocol/mainchain/commit/5b5480b4d7a00bd5979ff592dc4fb84949ff3ab6))
* **vaults:** available bonded argons wrong
([ab27a07](https://github.com/argonprotocol/mainchain/commit/ab27a07dfeb2c3b7e2f30690b63bb4285b84c680))
* **notebook:** sort notarizations correctly
([0d35a23](https://github.com/argonprotocol/mainchain/commit/0d35a2306d5c09131fb753c12b5a4a4b145063f0))

### [v1.0.14](https://github.com/argonprotocol/mainchain/compare/v1.0.13...v1.0.14) (2025-02-21)

#### Features

* indicate to user why xpriv needs rpc
([f3255d7](https://github.com/argonprotocol/mainchain/commit/f3255d7052b97447d8fea28065084c378322640e))

#### Fixes

* pin toolchchain to 1.0.84 for lint
([aeee3f0](https://github.com/argonprotocol/mainchain/commit/aeee3f08d88dd76b233f9f9747fe88687dd50530))
* **node:** don’t build on unfinalizable blocks
([f30240d](https://github.com/argonprotocol/mainchain/commit/f30240dc450758fb958c783a533201acf26d480f))
* **gh:** slow down block creation in tests
([bed41ec](https://github.com/argonprotocol/mainchain/commit/bed41ec62abb75cd28b7448d854f9e8754d40fc4))
* **mining_slot:** fix slot bidding arg
([02e5d14](https://github.com/argonprotocol/mainchain/commit/02e5d1457acf7c96301a7a960224b2661018c35d))
* **oracle:** delay retrying cpi if fails
([8373f47](https://github.com/argonprotocol/mainchain/commit/8373f474024347e0364dd641aae7a53d8bb346fd))
* **runtime:** add on-chain-release-build feature
([1b529ac](https://github.com/argonprotocol/mainchain/commit/1b529ac3f9d0f374742d062b5554bcfd4eefa054))
* **gh:** srtool not building metadata hash
([fb66221](https://github.com/argonprotocol/mainchain/commit/fb662216e8c2b4d9c19564adc78c9eb19a912eaf))
* **vaults:** return pending bonded argons on close
([954d3eb](https://github.com/argonprotocol/mainchain/commit/954d3eb9c12dd45a05b9b140b3dc0f01c2232361))

### [v1.0.13](https://github.com/argonprotocol/mainchain/compare/v1.0.12...v1.0.13) (2025-02-17)

#### Fixes

* **end-to-end:** tests needed cohort id
([1b9e313](https://github.com/argonprotocol/mainchain/commit/1b9e313494d83dcdaa6e9915406684e93871f302))
* **node:** attempt to fix warp sync with new api
([3c1ac8c](https://github.com/argonprotocol/mainchain/commit/3c1ac8cb6b56f8b9ddf20f9b20c76f3b94fddbf7))
* **vaults:** vault activation delay
([e04dcfc](https://github.com/argonprotocol/mainchain/commit/e04dcfcc188ea46276f8ea5024d7f14b1d6c1ff3))
* coindesk api removed
([c971269](https://github.com/argonprotocol/mainchain/commit/c971269fab1988f5db8ee2eec072db933e4e33d3))

### [v1.0.12](https://github.com/argonprotocol/mainchain/compare/v1.0.11...v1.0.12) (2025-02-12)

#### Features

* **block_rewards:** disable compute rewards
([fd393c7](https://github.com/argonprotocol/mainchain/commit/fd393c7c157d999651545cdba203f8ecf9d7e86f))
* **mining_slot:** disallow duplicate key reg
([4660021](https://github.com/argonprotocol/mainchain/commit/4660021aa2f77ddb0ca62cb712a9ac74fb433f82))
* **block_seal_spec:** reset difficulty every 10 blocks
([1a2fc75](https://github.com/argonprotocol/mainchain/commit/1a2fc75a2f54e3e1c6e9e61027d3ed9eff1ba0e4))

#### Fixes

* **hyperbridge:** upgrade hyperbridge to fix trie
([303c074](https://github.com/argonprotocol/mainchain/commit/303c074981e918b131d50ff3009413c6e6e5b267))
* **block_seal:** run xor closest before weight
([36c533d](https://github.com/argonprotocol/mainchain/commit/36c533d50869274e7dbbd64cdef0021a9369c81a))
* **ghactions:** don’t set as latest unless latest
([2d062be](https://github.com/argonprotocol/mainchain/commit/2d062be553580ae16f9f99ed72384ff4f27eb3a2))
* **node:** don’t stack same weight blocks
([301605a](https://github.com/argonprotocol/mainchain/commit/301605a2753ff2a6037aec01ebce1a8f79ecf8c2))
* **localchain:** add notebook archive to tests
([9588c35](https://github.com/argonprotocol/mainchain/commit/9588c35661e7372fd5c30be1e84cf06831dd6daf))

### [v1.0.11](https://github.com/argonprotocol/mainchain/compare/v1.0.10...v1.0.11) (2025-02-10)

#### Fixes

* separate runtimes
([5c5df56](https://github.com/argonprotocol/mainchain/commit/5c5df562356a7f2143e6e2fd0f99c35f4f00bbe4))

### [v1.0.10](https://github.com/argonprotocol/mainchain/compare/v1.0.9...v1.0.10) (2025-02-07)

#### Features

* **rewards:** increment rewards on ticks
([74418b1](https://github.com/argonprotocol/mainchain/commit/74418b13591069ad4141913643ef8f03ab4be435))

#### Fixes

* migration broken for clearing bonds
([7e5455e](https://github.com/argonprotocol/mainchain/commit/7e5455e8482f76ce19929ddad0fc53e32aebdfa8))
* attempt fix for windows release assets
([4e26676](https://github.com/argonprotocol/mainchain/commit/4e2667655de818c3c7cf903c5869d0300a423da8))
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

#### Fixes

* **runtime:** didn’t increment spec version
([d3a9dc9](https://github.com/argonprotocol/mainchain/commit/d3a9dc9cb08d5d8216d9ee4b8942fbe13b45233e))

### [v1.0.7](https://github.com/argonprotocol/mainchain/compare/v1.0.6...v1.0.7) (2025-01-27)

#### Features

* add sudo to update bid start
([1e0a492](https://github.com/argonprotocol/mainchain/commit/1e0a492e810e343305e286e9c49396f2c22474df))

#### Fixes

* **notary:** lock around best block
([8556cd8](https://github.com/argonprotocol/mainchain/commit/8556cd80fb44a92fdb59b41f6cc3810ef79d93af))
* **node:** don’t store bad sync for notary client
([43c3a66](https://github.com/argonprotocol/mainchain/commit/43c3a662145c7a3096649f22fda68220a56c1919))

### [v1.0.6](https://github.com/argonprotocol/mainchain/compare/v1.0.5...v1.0.6) (2025-01-24)

#### Fixes

* **node:** grandpa justification import
([a6482fa](https://github.com/argonprotocol/mainchain/commit/a6482fad883749bcf7512346a7fc952d956b36e9))
* **notary:** don’t fail audit for wrong tick
([d6d2975](https://github.com/argonprotocol/mainchain/commit/d6d29753c08333f20d0d32f5ab3139ccb5e2f735))

### [v1.0.5](https://github.com/argonprotocol/mainchain/compare/v1.0.4...v1.0.5) (2025-01-23)

#### Features

* **mining_slot:** correctly extend bids
([6ece2a1](https://github.com/argonprotocol/mainchain/commit/6ece2a148d4a987a6064bc8cfc2bb45f3cdadd2a))

#### Fixes

* **vaults:** time delay for funding changes
([4281d4f](https://github.com/argonprotocol/mainchain/commit/4281d4f57d971519012e2a44d075a1fed198857f))
* **node:** don’t save a failed audit for missing
([94f0a9d](https://github.com/argonprotocol/mainchain/commit/94f0a9dd2b278ce960c9d55ba5192a8265bb7597))

### [v1.0.4](https://github.com/argonprotocol/mainchain/compare/v1.0.3...v1.0.4) (2025-01-21)

#### Fixes

* needed to update spec_version
([bcc326f](https://github.com/argonprotocol/mainchain/commit/bcc326f9682691a3a0d56b093a3dc1e3a272d481))

### [v1.0.3](https://github.com/argonprotocol/mainchain/compare/v1.0.2...v1.0.3) (2025-01-21)

#### Fixes

* **block_seal:** don’t allow compute on vote
([c375b5b](https://github.com/argonprotocol/mainchain/commit/c375b5bde2c8d83a3ba165c64728a417f8c859f7))
* **vaults:** account for pending bitcoin
([733071b](https://github.com/argonprotocol/mainchain/commit/733071be1a1cf4ad39c2323473b6d329838c0e64))
* payload size should apply to all rpc
([b29c79a](https://github.com/argonprotocol/mainchain/commit/b29c79a2d1c79b51e980b13f082d2c9530febfb6))
* ensure grandpa rotation generates log
([881b8ad](https://github.com/argonprotocol/mainchain/commit/881b8ad0e5136be6b7b28fa4a3854494010f224c))
* only change grandpa once
([b3ec468](https://github.com/argonprotocol/mainchain/commit/b3ec468ba21d054a8f29842ecd651111099e9178))

### [v1.0.2](https://github.com/argonprotocol/mainchain/compare/v1.0.1...v1.0.2) (2025-01-18)

#### Features

* change token symbol
([6249c57](https://github.com/argonprotocol/mainchain/commit/6249c5765f6e546bbc587c6e7b990effb448fd14))

#### Fixes

* **node:** grandpa can’t prove finality
([63b86fd](https://github.com/argonprotocol/mainchain/commit/63b86fd4ab0690987e5e619b7e64b5a0810ab909))

### [v1.0.1](https://github.com/argonprotocol/mainchain/compare/v1.0.0...v1.0.1) (2025-01-16)

#### Features

* updated docs + chain spec for mainnet
([247c153](https://github.com/argonprotocol/mainchain/commit/247c1536782adaa0d0f875bf1e931cef6c7c220b))

#### Fixes

* lint issue
([76530bb](https://github.com/argonprotocol/mainchain/commit/76530bbf27227a1bde154a99b6df255d6a1b1382))
* **oracle:** dont adjust target price by usdc
([9904f7e](https://github.com/argonprotocol/mainchain/commit/9904f7edb98efc53b1eb7f9c95a6de1052360379))
* **block_seal_spec:** remove clamp
([8c1a0b2](https://github.com/argonprotocol/mainchain/commit/8c1a0b25d1e8c582543d3fcca82bbe8fc01f2afa))

## [v1.0.0](https://github.com/argonprotocol/mainchain/compare/v0.0.27...v1.0.0) (2025-01-15)

### Features

* hardcode cpi
([934a85a](https://github.com/argonprotocol/mainchain/commit/934a85ae4418aaa58b1df9adff6e71065948e22c))

### Fixes

* **oracle:** handle us cpi resp annual in latest
([0597b56](https://github.com/argonprotocol/mainchain/commit/0597b56c8f5dca6b18c32021ea5a263fa507e260))

### [v0.0.27](https://github.com/argonprotocol/mainchain/compare/v0.0.26...v0.0.27) (2025-01-14)

#### Fixes

* **node:** prom metrics wrong
([dd40341](https://github.com/argonprotocol/mainchain/commit/dd40341290d2ac1fd546fb9227a2138c57adb271))
* **seal_spec:** trim to 80th pctl of block times
([4c3458d](https://github.com/argonprotocol/mainchain/commit/4c3458da6ab4b402892507639be246206e6f5d8b))

### [v0.0.26](https://github.com/argonprotocol/mainchain/compare/v0.0.25...v0.0.26) (2025-01-13)

#### Features

* **mint:** spread out new mint over hour
([31d3a94](https://github.com/argonprotocol/mainchain/commit/31d3a9466278e491445c0aab515fab2cfab88e50))

#### Fixes

* **oracle:** default to target price if not found
([a0678fb](https://github.com/argonprotocol/mainchain/commit/a0678fba4fc9626d08f8bb98f06c007bb7fe4c2d))
* update metadata
([c5273ad](https://github.com/argonprotocol/mainchain/commit/c5273ad9bf1c623c8be770774d68186a0dac7fbf))
* **node:** memory leak for non-authority
([07729f8](https://github.com/argonprotocol/mainchain/commit/07729f82f7509fc8e5449aebebaa1e4360d39014))
* **consensus:** retain audits until finalized
([0ff4621](https://github.com/argonprotocol/mainchain/commit/0ff462170ca1bbe12e7df740195b298256c3f2d9))
* **node:** back off solving when max blocks
([2d4d167](https://github.com/argonprotocol/mainchain/commit/2d4d167d0d5c0b742533765374dd3a3c4dd3f5e2))
* **mining_slot:** cap ownership max amount at 80%
([15387e1](https://github.com/argonprotocol/mainchain/commit/15387e1e20e3ce2c42caacffcf32d7a3cabd2045))
* **node:** prometheus metrics for own blocks
([0528f5b](https://github.com/argonprotocol/mainchain/commit/0528f5be391046c130b3fec23ac24296e5770ade))

### [v0.0.25](https://github.com/argonprotocol/mainchain/compare/v0.0.24...v0.0.25) (2025-01-06)

#### Features

* **runtime:** reduce bond amount to 100 milligons
([6d041bb](https://github.com/argonprotocol/mainchain/commit/6d041bbb8eb83ea778d5bcf384d4bbf602da44ec))
* **oracle:** move cpi to env var
([68828c6](https://github.com/argonprotocol/mainchain/commit/68828c66f71bd5d873272109c893a63f8d306680))
* restore metrics
([7775622](https://github.com/argonprotocol/mainchain/commit/77756220d153a8fb1701014a804a384e6a3d6321))
* **node:** add earnings metrics
([7f7dc1e](https://github.com/argonprotocol/mainchain/commit/7f7dc1e4f3faab0b3ef7881bc912a4628b14b3f5))

#### Fixes

* **node:** reduce cpu spike for node
([c8f0322](https://github.com/argonprotocol/mainchain/commit/c8f03225f1269b5dcba0f8e5a95ddb87fd5b4737))
* **ticks:** max 5 blocks per tick
([0e43dbb](https://github.com/argonprotocol/mainchain/commit/0e43dbbed467d1978f4aba969c4b859b60377aae))
* **oracle:** lint for us cpi
([5f77b8a](https://github.com/argonprotocol/mainchain/commit/5f77b8ac887464c35a7de72e9d307f2a584b7cfe))
* naming tweak
([68d79fc](https://github.com/argonprotocol/mainchain/commit/68d79fc0e477d71d97aec4349bdc131fa13e4953))
* **bitcoin:** cli outputting hex xpub
([c5deaa9](https://github.com/argonprotocol/mainchain/commit/c5deaa933213b13b323e83f2d595e108fdf8b515))
* **node:** smarter wait for imported blocks
([0cbf2ba](https://github.com/argonprotocol/mainchain/commit/0cbf2ba45620212dc59d5900b782d321d8a45bcc))
* **mining:** starts slots after ticks vs blocks
([ff4428f](https://github.com/argonprotocol/mainchain/commit/ff4428f53acdf0735121492cd2a6a810d75db8e6))

### [v0.0.24](https://github.com/argonprotocol/mainchain/compare/v0.0.23...v0.0.24) (2024-12-19)

#### Fixes

* **node:** don’t download all notebooks on bootup
([014adc7](https://github.com/argonprotocol/mainchain/commit/014adc734e6bd7013dbe13acb5ee7de9029290f4))

### [v0.0.23](https://github.com/argonprotocol/mainchain/compare/v0.0.22...v0.0.23) (2024-12-19)

#### Features

* **node:** add prometheus metrics
([2d0d701](https://github.com/argonprotocol/mainchain/commit/2d0d7017c485a7dd8c1b3f0bb7b234323b9d4074))

#### Fixes

* **node:** reduce looping for notary_client
([b34f734](https://github.com/argonprotocol/mainchain/commit/b34f73417ddafbcd314f1669c843ed370addb3a6))
* **node:** notaries not dialed
([825965c](https://github.com/argonprotocol/mainchain/commit/825965c57ec58e730dea56a2ad833f3733a6121b))

### [v0.0.22](https://github.com/argonprotocol/mainchain/compare/v0.0.21...v0.0.22) (2024-12-16)

#### Features

* **notary:** archive hosts
([5df12d0](https://github.com/argonprotocol/mainchain/commit/5df12d0d31a8944c8be51db67e772c4ff299b143))

#### Fixes

* **localchain:** gh actions openssl build issues
([ea1186d](https://github.com/argonprotocol/mainchain/commit/ea1186d2f5d2e34963000bc1a88a09073e4d8468))
* **runtime:** reduce compute block time target
([27d6b69](https://github.com/argonprotocol/mainchain/commit/27d6b698ead28894376929d550a84b24be278cd2))
* **notary:** handle disconnecting rpc node
([9175116](https://github.com/argonprotocol/mainchain/commit/91751162b80446eaad544c2c52993588529c0e77))
* **node:** reconnect to notary to verify blocks
([a8f2383](https://github.com/argonprotocol/mainchain/commit/a8f23836ec947d7b4af46b236e8443bb8d2e0d93))

### [v0.0.21](https://github.com/argonprotocol/mainchain/compare/v0.0.20...v0.0.21) (2024-12-07)

#### Fixes

* **notary:** handle blocks re-downloaded
([359cbbb](https://github.com/argonprotocol/mainchain/commit/359cbbbdcfe9ff74f39852348cb6b43364f21a0f))

### [v0.0.20](https://github.com/argonprotocol/mainchain/compare/v0.0.19...v0.0.20) (2024-12-06)

#### Fixes

* **localchain:** improve lock hold durations
([025cbe4](https://github.com/argonprotocol/mainchain/commit/025cbe4aa6aa3606ad9e181c5da3879e0455bded))

### [v0.0.19](https://github.com/argonprotocol/mainchain/compare/v0.0.18...v0.0.19) (2024-12-05)

#### Fixes

* notary public key wrong in testnet
([8f20c4d](https://github.com/argonprotocol/mainchain/commit/8f20c4d2209d1ef7fc2ef10cdbc10ba23b2bb5ea))

### [v0.0.18](https://github.com/argonprotocol/mainchain/compare/v0.0.17...v0.0.18) (2024-12-05)

#### Features

* **oracle:** register keys
([958f3e1](https://github.com/argonprotocol/mainchain/commit/958f3e1332ceb6126796ebe5959002d4e8c595e0))
* **client:** add a wage protector
([c2bba70](https://github.com/argonprotocol/mainchain/commit/c2bba7038005251280a15f21829577359853d955))
* **mint:** allow bitcoin mint at cpi=0
([a4d7105](https://github.com/argonprotocol/mainchain/commit/a4d71058edae0a51ec358593a317b932de28216c))
* **node:** activate compute sooner in bootstrap
([828f283](https://github.com/argonprotocol/mainchain/commit/828f283a09570f62e34836bc35a55e58675dc3d1))
* **runtime:** temporarily disable grandpa w slots
([b01d5a0](https://github.com/argonprotocol/mainchain/commit/b01d5a0ba53394b5bc3da7fe3d663b6ced3a314d))
* **mint:** disable blocking mint if no reg miners
([27b3049](https://github.com/argonprotocol/mainchain/commit/27b3049bf6ee777f5a7c2da18616a3730430834a))
* **node:** change finalization to match voting
([aad495a](https://github.com/argonprotocol/mainchain/commit/aad495aae91f25b82eb10ff154a8e55e4fe6a5ac))
* **node:** remove compute notebook block sort
([e087392](https://github.com/argonprotocol/mainchain/commit/e08739228cad43b071b1d2181de0cb3197ae12c5))
* **mint:** return early if no registered miners
([02f259a](https://github.com/argonprotocol/mainchain/commit/02f259abcb0f34d0eea0698de18c57e91b795c12))
* **chain_transfer:** bridge scripts
([de5f351](https://github.com/argonprotocol/mainchain/commit/de5f351c9253de09c5be939f5ca6d830089d72a1))
* uniswap oracle for usdc prices
([a5e24e6](https://github.com/argonprotocol/mainchain/commit/a5e24e611ce45d874d017c0f038eb3d426ac02dc))
* **chain_transfer:** add ability to pause bridge
([3cfd210](https://github.com/argonprotocol/mainchain/commit/3cfd21014038a476fc2b610d187445cd6e643252))
* **runtime:** add a canary runtime
([1eb7a61](https://github.com/argonprotocol/mainchain/commit/1eb7a61e25183d29bef294d3fab99c8d842ff66c))
* **node:** add grandpa rpc
([c56c427](https://github.com/argonprotocol/mainchain/commit/c56c4272ec8253dafa2d0c2355c1c671fdf82bdd))
* change decimals to 6
([f8277eb](https://github.com/argonprotocol/mainchain/commit/f8277ebe93451b523eea93b688f00a1a160a6654))
* convert ticks to use unix epoch
([36d230e](https://github.com/argonprotocol/mainchain/commit/36d230e0f18e631a92da0e9b1b466028f02cde13))
* **runtime:** integrate hyperbridge to evm
([e5b8d35](https://github.com/argonprotocol/mainchain/commit/e5b8d3587b5ba285c96470a628f16fc1b1fde5f5))
* **node:** get vote block author from runtime
([ffee3b6](https://github.com/argonprotocol/mainchain/commit/ffee3b6584da349e72f2b9c99d17528f7bcefb01))
* **node:** add typing to genesis specs
([12fbef5](https://github.com/argonprotocol/mainchain/commit/12fbef5d007c692e25a6ff702fc7c091cc17fb4b))
* **runtime:** lower minimum vote start
([d7bfbab](https://github.com/argonprotocol/mainchain/commit/d7bfbab847742bf55db866fca01b2329f3e8c1f0))

#### Fixes

* build
([7628e02](https://github.com/argonprotocol/mainchain/commit/7628e02d9566eb03e019bd23d897fe7fdd1d5a31))
* tests timing out
([2801c78](https://github.com/argonprotocol/mainchain/commit/2801c78db1e6b34469c985757a34adfd64f2ea81))
* **node:** set initial miners to 100
([7153407](https://github.com/argonprotocol/mainchain/commit/71534073af4b7e7a2be7560303babda19e1706b9))
* **node:** pin blocks before broadcasting
([c29939f](https://github.com/argonprotocol/mainchain/commit/c29939faf83b8b4546a959d1a22486a3163e12d8))
* **block_rewards:** start with smaller rewards
([237971a](https://github.com/argonprotocol/mainchain/commit/237971a211fac9e770a7e11b1d1cabb4ad789554))
* **mint:** remove unlocked bitcoin from total
([2bcd738](https://github.com/argonprotocol/mainchain/commit/2bcd7380447bfa55f0da28181a3b465974bfa803))
* **node:** default block votes
([4c5f52d](https://github.com/argonprotocol/mainchain/commit/4c5f52d9a73d5de4d3b53a93b9d5d672c1933582))
* **e2e:** fix with minting now only to slots
([938efff](https://github.com/argonprotocol/mainchain/commit/938efff12b97fd7469cd8705016b3f94f30ed335))
* **node:** must audit notebooks in full client
([0560091](https://github.com/argonprotocol/mainchain/commit/056009137b219e71d85b61711eda4bb1caf3758f))
* **utxos:** check parent block for inherent reqt
([c7dd39a](https://github.com/argonprotocol/mainchain/commit/c7dd39a4ed3a933755082690e078909822f85a62))
* **price_index:** default argon ration to 1 if na
([a4a59b3](https://github.com/argonprotocol/mainchain/commit/a4a59b333bf03e24fb0a8808cb17e81c388b2996))
* **mining_slot:** remove miner zero
([52f33f1](https://github.com/argonprotocol/mainchain/commit/52f33f10b04b2314e49257e749aebf4ac2096de5))
* **mining_slot:** floor bids at min account bal
([ea6dd33](https://github.com/argonprotocol/mainchain/commit/ea6dd3384bbc1e768dca0ff703039b1c57c81fdb))
* **localchain:** vote with minimum amount
([1c3e371](https://github.com/argonprotocol/mainchain/commit/1c3e371e7a87cdd112d79192e973df6c44cfcf90))
* **node:** bug in import logic
([39f819f](https://github.com/argonprotocol/mainchain/commit/39f819f277bc03d465725b743571add01bcaffff))
* **node:** use full spectrum of u256 for nonce
([61dd7da](https://github.com/argonprotocol/mainchain/commit/61dd7da694ba10889aa8b1cd6c6bb48963b380f2))
* **block_seal:** sign full block
([e73cfc9](https://github.com/argonprotocol/mainchain/commit/e73cfc965b91a161bdf67b79e872294bafdb5d00))

### [v0.0.17](https://github.com/argonprotocol/mainchain/compare/v0.0.16...v0.0.17) (2024-10-25)

#### Fixes

* **node:** notary can’t catch up
([130990f](https://github.com/argonprotocol/mainchain/commit/130990f1c550cff872644fa38cf4c03352109ef3))

### [v0.0.16](https://github.com/argonprotocol/mainchain/compare/v0.0.15...v0.0.16) (2024-10-25)

#### Fixes

* rewards should be for notebook tick
([60a5b63](https://github.com/argonprotocol/mainchain/commit/60a5b63b1a28a750f49e43e16b889ee57174eced))

### [v0.0.15](https://github.com/argonprotocol/mainchain/compare/v0.0.14...v0.0.15) (2024-10-24)

#### Features

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

#### Fixes

* release overwrite protector
([3c2a037](https://github.com/argonprotocol/mainchain/commit/3c2a037071ce90cd69be8cecacdd07e65b0a9798))
* llvm 19.1.2 broke build on mac
([5f22086](https://github.com/argonprotocol/mainchain/commit/5f22086ffa71902ec243d030597a9c6377a91f3f))
* **ticks:** only allow a single block per tick
([cdf295a](https://github.com/argonprotocol/mainchain/commit/cdf295aae082adae7f72deb4ddc9517b48e9ccbd))

### [v0.0.14](https://github.com/argonprotocol/mainchain/compare/v0.0.13...v0.0.14) (2024-10-10)

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

### [v0.0.11](https://github.com/argonprotocol/mainchain/compare/v0.0.10...v0.0.11) (2024-10-07)

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
