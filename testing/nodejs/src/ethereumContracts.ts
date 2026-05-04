import { readFileSync } from 'node:fs';
import type { Abi, Hex } from 'viem';

type EthereumContractArtifact = {
  abi: Abi;
  bytecode: Hex;
};

function loadArtifact(fileName: string) {
  return JSON.parse(
    readFileSync(new URL(`./ethereum-contracts/${fileName}`, import.meta.url), 'utf8'),
  ) as EthereumContractArtifact;
}

const argonTokenArtifact = loadArtifact('ArgonToken.json');
const argonotTokenArtifact = loadArtifact('ArgonotToken.json');
const mintingGatewayArtifact = loadArtifact('MintingGateway.json');
const proxyAdminArtifact = loadArtifact('ProxyAdmin.json');
const transparentUpgradeableProxyArtifact = loadArtifact('TransparentUpgradeableProxy.json');

export {
  argonTokenArtifact,
  argonotTokenArtifact,
  mintingGatewayArtifact,
  proxyAdminArtifact,
  transparentUpgradeableProxyArtifact,
};
