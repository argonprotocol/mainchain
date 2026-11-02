import type { ContractFunctionArgs } from 'viem';
import type { argonBuyAndTransferAbi } from '@argonprotocol/ethereum-contracts/generated';
import argonBuyAndTransferArtifact from './artifacts/contracts/ArgonBuyAndTransfer.sol/ArgonBuyAndTransfer.json' with { type: 'json' };
import argonTokenArtifact from './artifacts/contracts/ArgonToken.sol/ArgonToken.json' with { type: 'json' };
import argonotTokenArtifact from './artifacts/contracts/ArgonotToken.sol/ArgonotToken.json' with { type: 'json' };
import stabilizationPoolSubscriberArtifact from './artifacts/contracts/StabilizationPoolSubscriber.sol/StabilizationPoolSubscriber.json' with { type: 'json' };
import stabilizationPoolHookArtifact from './artifacts/contracts/StabilizationPoolHook.sol/StabilizationPoolHook.json' with { type: 'json' };
import uniswapV4PriceQuoterArtifact from './artifacts/contracts/UniswapV4PriceQuoter.sol/UniswapV4PriceQuoter.json' with { type: 'json' };
import mintingGatewayArtifact from './artifacts/contracts/MintingGateway.sol/MintingGateway.json' with { type: 'json' };
import proxyAdminArtifact from './artifacts/contracts/ProxyArtifacts.sol/ProxyAdmin.json' with { type: 'json' };
import transparentUpgradeableProxyArtifact from './artifacts/contracts/ProxyArtifacts.sol/TransparentUpgradeableProxy.json' with { type: 'json' };

export const MINTING_GATEWAY_RUNTIME_DECIMALS = 6;
export const MINTING_GATEWAY_TOKEN_DECIMALS = 18;
export const MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE =
  10n ** BigInt(MINTING_GATEWAY_TOKEN_DECIMALS - MINTING_GATEWAY_RUNTIME_DECIMALS);

export * from './generated.js';
export * from './hashing.js';

export {
  argonBuyAndTransferArtifact,
  argonTokenArtifact,
  argonotTokenArtifact,
  mintingGatewayArtifact,
  stabilizationPoolSubscriberArtifact,
  stabilizationPoolHookArtifact,
  uniswapV4PriceQuoterArtifact,
  proxyAdminArtifact,
  transparentUpgradeableProxyArtifact,
};

export type ArgonBuyAndTransferRequest = ContractFunctionArgs<
  typeof argonBuyAndTransferAbi,
  'nonpayable',
  'buyAndTransfer'
>[0];
