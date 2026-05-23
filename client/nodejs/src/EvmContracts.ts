import type { Abi, Hex } from 'viem';
import {
  ArgonTokenEvents,
  ArgonotTokenEvents,
  CanonicalMintableBurnableERC20Events,
  ICanonicalTokenEvents,
  MINTING_GATEWAY_RUNTIME_DECIMALS,
  MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
  MINTING_GATEWAY_TOKEN_DECIMALS,
  MINTING_GATEWAY_UPDATE_KINDS,
  MintingGatewayEvents,
  ProxyAdminEvents,
  TransparentUpgradeableProxyEvents,
  argonTokenAbi,
  argonTokenArtifact as rawArgonTokenArtifact,
  argonotTokenAbi,
  argonotTokenArtifact as rawArgonotTokenArtifact,
  canonicalMintableBurnableErc20Abi,
  encodeMintingGatewayCouncilSnapshot,
  encodeMintingGatewayGlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  hashMintingGatewayActivateMintingAuthority,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayGatewayUpdateApproval,
  hashMintingGatewayGlobalIssuanceCouncil,
  hashMintingGatewayMintingAuthority,
  hashMintingGatewayMintingAuthorityDeactivation,
  hashMintingGatewayMintingAuthorization as rawHashMintingGatewayMintingAuthorization,
  hashMintingGatewayRotateGlobalIssuanceCouncil,
  hashMintingGatewayRotateGlobalIssuanceCouncilApproval,
  hashMintingGatewayTransferOutOfArgonRequest as rawHashMintingGatewayTransferOutOfArgonRequest,
  iCanonicalTokenAbi,
  mintingGatewayAbi,
  mintingGatewayArtifact as rawMintingGatewayArtifact,
  proxyAdminAbi,
  proxyAdminArtifact as rawProxyAdminArtifact,
  transparentUpgradeableProxyAbi,
  transparentUpgradeableProxyArtifact as rawTransparentUpgradeableProxyArtifact,
} from '@argonprotocol/ethereum-contracts';

export type * from '@argonprotocol/ethereum-contracts';

export type EvmContractArtifact = {
  abi: Abi;
  bytecode: Hex;
};

export const argonTokenArtifact = rawArgonTokenArtifact as EvmContractArtifact;
export const argonotTokenArtifact = rawArgonotTokenArtifact as EvmContractArtifact;
export const mintingGatewayArtifact = rawMintingGatewayArtifact as EvmContractArtifact;
export const proxyAdminArtifact = rawProxyAdminArtifact as EvmContractArtifact;
export const transparentUpgradeableProxyArtifact =
  rawTransparentUpgradeableProxyArtifact as EvmContractArtifact;

export type MintingGatewayTransferOutOfArgonRequest = Parameters<
  typeof rawHashMintingGatewayTransferOutOfArgonRequest
>[0];

export function hashMintingGatewayTransferOutOfArgonRequest(
  request: MintingGatewayTransferOutOfArgonRequest,
) {
  return rawHashMintingGatewayTransferOutOfArgonRequest(request);
}

export function hashMintingGatewayMintingAuthorization(
  gateway: Parameters<typeof rawHashMintingGatewayMintingAuthorization>[0],
  authorization: Omit<
    Parameters<typeof rawHashMintingGatewayMintingAuthorization>[1],
    'request'
  > & {
    request: MintingGatewayTransferOutOfArgonRequest;
  },
) {
  return rawHashMintingGatewayMintingAuthorization(gateway, authorization);
}

export {
  ArgonTokenEvents,
  ArgonotTokenEvents,
  CanonicalMintableBurnableERC20Events,
  ICanonicalTokenEvents,
  MINTING_GATEWAY_RUNTIME_DECIMALS,
  MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
  MINTING_GATEWAY_TOKEN_DECIMALS,
  MINTING_GATEWAY_UPDATE_KINDS,
  MintingGatewayEvents,
  ProxyAdminEvents,
  TransparentUpgradeableProxyEvents,
  argonTokenAbi,
  argonotTokenAbi,
  canonicalMintableBurnableErc20Abi,
  encodeMintingGatewayCouncilSnapshot,
  encodeMintingGatewayGlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  hashMintingGatewayActivateMintingAuthority,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayGatewayUpdateApproval,
  hashMintingGatewayGlobalIssuanceCouncil,
  hashMintingGatewayMintingAuthority,
  hashMintingGatewayMintingAuthorityDeactivation,
  hashMintingGatewayRotateGlobalIssuanceCouncil,
  hashMintingGatewayRotateGlobalIssuanceCouncilApproval,
  iCanonicalTokenAbi,
  mintingGatewayAbi,
  proxyAdminAbi,
  transparentUpgradeableProxyAbi,
};
