import hardhatEthers from '@nomicfoundation/hardhat-ethers';
import { configVariable, defineConfig } from 'hardhat/config';

export default defineConfig({
  plugins: [hardhatEthers],
  solidity: {
    npmFilesToBuild: ['@uniswap/v4-periphery/src/PositionManager.sol'],
    compilers: [
      {
        version: '0.8.35',
        settings: {
          optimizer: {
            enabled: true,
            runs: 200,
          },
          evmVersion: 'cancun',
        },
      },
      // PositionManager pins solc 0.8.26 and uses the upstream IR compilation profile.
      {
        version: '0.8.26',
        settings: {
          optimizer: { enabled: true, runs: 500 },
          viaIR: true,
          evmVersion: 'cancun',
        },
      },
    ],
    overrides: {
      'contracts/test/StabilizationPoolHookInvariant.sol': {
        version: '0.8.26',
        settings: {
          optimizer: { enabled: true, runs: 200 },
          evmVersion: 'cancun',
        },
      },
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
      url: process.env.TESTNET_ETHEREUM_RPC_URL ?? 'https://ethereum-sepolia-rpc.publicnode.com',
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
