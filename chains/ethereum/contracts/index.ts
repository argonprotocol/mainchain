import argonTokenArtifact from './artifacts/contracts/ArgonToken.sol/ArgonToken.json' with {
  type: 'json',
};
import argonotTokenArtifact from './artifacts/contracts/ArgonotToken.sol/ArgonotToken.json' with {
  type: 'json',
};
import mintingGatewayArtifact from './artifacts/contracts/MintingGateway.sol/MintingGateway.json' with {
  type: 'json',
};
import mintingGatewayV2Artifact from './artifacts/contracts/MintingGatewayV2.sol/MintingGatewayV2.json' with {
  type: 'json',
};
import proxyAdminArtifact from './artifacts/contracts/ProxyArtifacts.sol/ProxyAdmin.json' with {
  type: 'json',
};
import transparentUpgradeableProxyArtifact from './artifacts/contracts/ProxyArtifacts.sol/TransparentUpgradeableProxy.json' with {
  type: 'json',
};

export const MINTING_GATEWAY_BURN_FOR_TRANSFER_EVENT_NAME = 'BurnForTransfer' as const;
export const MINTING_GATEWAY_V2_TRANSFER_TO_ARGON_STARTED_EVENT_NAME =
  'TransferToArgonStarted' as const;
export const MINTING_GATEWAY_RUNTIME_DECIMALS = 6;
export const MINTING_GATEWAY_TOKEN_DECIMALS = 18;
export const MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE =
  10n ** BigInt(MINTING_GATEWAY_TOKEN_DECIMALS - MINTING_GATEWAY_RUNTIME_DECIMALS);

export * from './generated.js';
export * from './hashing.js';

export {
  argonTokenArtifact,
  argonotTokenArtifact,
  mintingGatewayArtifact,
  mintingGatewayV2Artifact,
  proxyAdminArtifact,
  transparentUpgradeableProxyArtifact,
};
