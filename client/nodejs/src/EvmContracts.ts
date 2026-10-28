import type { Abi, Hex } from 'viem';
import {
  MINTING_GATEWAY_RUNTIME_DECIMALS,
  MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
  MINTING_GATEWAY_TOKEN_DECIMALS,
  argonTokenArtifact as rawArgonTokenArtifact,
  argonotTokenArtifact as rawArgonotTokenArtifact,
  mintingGatewayArtifact as rawMintingGatewayArtifact,
  proxyAdminArtifact as rawProxyAdminArtifact,
  transparentUpgradeableProxyArtifact as rawTransparentUpgradeableProxyArtifact,
} from '@argonprotocol/ethereum-contracts';
import {
  mintingGatewayActivityHashingFixture,
  type MintingGatewayActivityHashingFixture,
} from '@argonprotocol/ethereum-contracts/fixtures';
import {
  ArgonTokenEvents,
  ArgonotTokenEvents,
  CanonicalMintableBurnableERC20Events,
  ICanonicalTokenEvents,
  MintingGatewayEvents,
  ProxyAdminEvents,
  TransparentUpgradeableProxyEvents,
  argonTokenAbi,
  argonotTokenAbi,
  canonicalMintableBurnableErc20Abi,
  iCanonicalTokenAbi,
  mintingGatewayAbi,
  proxyAdminAbi,
  transparentUpgradeableProxyAbi,
} from '@argonprotocol/ethereum-contracts/generated';
import {
  MINTING_GATEWAY_UPDATE_KINDS,
  encodeMintingGatewayCouncilSnapshot,
  encodeMintingGatewayGlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  appendMintingGatewayActivityRoot,
  hashMintingGatewayActivateMintingAuthority,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayActivityBlockLocator,
  hashMintingGatewayGatewayUpdateApproval,
  hashMintingGatewayGlobalIssuanceCouncilRotatedActivity,
  hashMintingGatewayGlobalIssuanceCouncil,
  hashMintingGatewayMintingAuthorityActivatedActivity,
  hashMintingGatewayMintingAuthorityDeactivatedActivity,
  hashMintingGatewayMintingAuthority,
  hashMintingGatewayMintingAuthorization as rawHashMintingGatewayMintingAuthorization,
  hashMintingGatewayRotateGlobalIssuanceCouncil,
  hashMintingGatewayRotateGlobalIssuanceCouncilApproval,
  hashMintingGatewayTransferOutOfArgonCanceledActivity,
  hashMintingGatewayTransferOutOfArgonFinalizedActivity,
  hashMintingGatewayTransferOutOfArgonRequest as rawHashMintingGatewayTransferOutOfArgonRequest,
  hashMintingGatewayTransferToArgonStartedActivity,
} from '@argonprotocol/ethereum-contracts/hashing';

export type * from '@argonprotocol/ethereum-contracts/generated';
export type * from '@argonprotocol/ethereum-contracts/hashing';

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
  mintingGatewayActivityHashingFixture,
  MintingGatewayEvents,
  ProxyAdminEvents,
  TransparentUpgradeableProxyEvents,
  appendMintingGatewayActivityRoot,
  argonTokenAbi,
  argonotTokenAbi,
  canonicalMintableBurnableErc20Abi,
  encodeMintingGatewayCouncilSnapshot,
  encodeMintingGatewayGlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  hashMintingGatewayActivateMintingAuthority,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayActivityBlockLocator,
  hashMintingGatewayGatewayUpdateApproval,
  hashMintingGatewayGlobalIssuanceCouncil,
  hashMintingGatewayGlobalIssuanceCouncilRotatedActivity,
  hashMintingGatewayMintingAuthority,
  hashMintingGatewayMintingAuthorityActivatedActivity,
  hashMintingGatewayMintingAuthorityDeactivatedActivity,
  hashMintingGatewayRotateGlobalIssuanceCouncil,
  hashMintingGatewayRotateGlobalIssuanceCouncilApproval,
  hashMintingGatewayTransferOutOfArgonCanceledActivity,
  hashMintingGatewayTransferOutOfArgonFinalizedActivity,
  hashMintingGatewayTransferToArgonStartedActivity,
  iCanonicalTokenAbi,
  mintingGatewayAbi,
  proxyAdminAbi,
  transparentUpgradeableProxyAbi,
};

export type { MintingGatewayActivityHashingFixture };
