import hardhatEthers from '@nomicfoundation/hardhat-ethers';
import { configVariable, defineConfig } from 'hardhat/config';

export default defineConfig({
  plugins: [hardhatEthers],
  solidity: {
    version: '0.8.24',
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
      evmVersion: 'cancun',
    },
  },
  networks: {
    hardhatMainnet: {
      type: 'edr-simulated',
      chainType: 'l1',
    },
    mainnet: {
      type: 'http',
      chainType: 'l1',
      url: process.env.ETHEREUM_RPC_URL ?? 'https://ethereum-rpc.publicnode.com',
      accounts: [configVariable('ETHEREUM_DEPLOYER_PRIVATE_KEY')],
    },
    testnet: {
      type: 'http',
      chainType: 'l1',
      url: process.env.TESTNET_ETHEREUM_RPC_URL ?? 'https://gnosis-rpc.publicnode.com',
      accounts: [configVariable('ETHEREUM_DEPLOYER_PRIVATE_KEY')],
    },
  },
  paths: {
    sources: './contracts',
    tests: './test',
    cache: './cache',
    artifacts: './artifacts',
  },
});
