This Pallet allows transfers to both Localchain and EVM-based chains via hyperbridge.

## Localchain

You can exchange assets back and forth with the Localchain using the `send_to_localchain` function in this pallet, and a
corresponding localchain transaction to send back to mainchain.

## Hyperbridge

Hyperbridge is a cross-chain bridge that allows for the transfer of assets between different blockchains. It is a
decentralized, trustless, and secure solution that enables users to move assets between different chains without the
need for a centralized intermediary. More here: https://docs.hyperbridge.network/

A token transfer ui is available here: https://app.hyperbridge.network/

### Relayer

There's a hyperbridge script that will all you to start a local relayer to retrieve the initial genesis state:
`start_pallet_relayer.sh`

### Configuration

Configuration details relevant to transactions are below:

#### Testnet

Contract addresses for the hyperbridge contracts: https://docs.hyperbridge.network/developers/evm/contracts/testnet

Token Gateway is 0xFcDa26cA021d5535C3059547390E6cCd8De7acA6

##### EVM

- Ethereum => Evm 11155111,
- Optimism => Evm 11155420,
- Arbitrum => Evm 421614,
- Base => Evm 84532,
- Polygon => Evm 80001,
- BinanceSmartChain => Evm 97,
- Gnosis => Evm 10200,

##### Substrate

- Hyperbridge Paseo => Polkadot(4009),

#### Mainnet

Contract addresses for the hyperbridge contracts:https://docs.hyperbridge.network/developers/evm/contracts/mainnet

Token Gateway is 0xFd413e3AFe560182C4471F4d143A96d3e259B6dE

##### EVM

- Ethereum => Evm 1,
- Arbitrum => Evm 42161,
- Optimism => Evm 10,
- Base => Evm 8453,
- BinanceSmartChain => Evm 56,
- Gnosis => Evm 100,

##### Substrate

- Hyperbridge Polkadot => Polkadot(3367),
- Hyperbridge Kusama => Kusama(3340),
