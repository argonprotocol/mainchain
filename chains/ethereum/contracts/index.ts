export const MINTING_GATEWAY_BURN_FOR_TRANSFER_EVENT_NAME = 'BurnForTransfer' as const;
export const MINTING_GATEWAY_RUNTIME_DECIMALS = 6;
export const MINTING_GATEWAY_TOKEN_DECIMALS = 18;
export const MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE =
  10n ** BigInt(MINTING_GATEWAY_TOKEN_DECIMALS - MINTING_GATEWAY_RUNTIME_DECIMALS);

export { default as argonTokenArtifact } from './artifacts/contracts/ArgonToken.sol/ArgonToken.json';
export { default as argonotTokenArtifact } from './artifacts/contracts/ArgonotToken.sol/ArgonotToken.json';
export { default as mintingGatewayArtifact } from './artifacts/contracts/MintingGateway.sol/MintingGateway.json';
export { default as proxyAdminArtifact } from './artifacts/contracts/ProxyArtifacts.sol/ProxyAdmin.json';
export { default as transparentUpgradeableProxyArtifact } from './artifacts/contracts/ProxyArtifacts.sol/TransparentUpgradeableProxy.json';
