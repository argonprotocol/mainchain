import {
  Contract,
  ContractFactory,
  getAddress,
  ZeroAddress,
  type Provider,
  type Signer,
  type InterfaceAbi,
} from 'ethers';
import { EvmContracts } from '@argonprotocol/mainchain';
import type { Address } from 'viem';

/** Canonical purchase contracts already deployed on the signer's execution chain. */
export interface PurchaseDependencies {
  usdc: Address;
  router: Address;
  permit2: Address;
}

export async function deployBuyAndTransfer(
  signer: Signer,
  gatewayAddress: string,
  argonTokenAddress: string,
  config: PurchaseDependencies,
) {
  const provider = signer.provider;
  if (!provider) throw new Error('Purchase deployment requires a provider');
  await validatePurchaseDependencies(provider, config);
  for (const address of [gatewayAddress, argonTokenAddress]) {
    if (getAddress(address) === ZeroAddress) throw new Error('Purchase dependency cannot be zero');
    if ((await provider.getCode(address)) === '0x') {
      throw new Error(`Purchase dependency ${address} has no deployed code`);
    }
  }
  const constructorArgs = [
    getAddress(config.usdc),
    argonTokenAddress,
    gatewayAddress,
    getAddress(config.router),
    getAddress(config.permit2),
  ];
  const factory = new ContractFactory(
    EvmContracts.argonBuyAndTransferArtifact.abi as InterfaceAbi,
    EvmContracts.argonBuyAndTransferArtifact.bytecode,
    signer,
  );
  const contract = await factory.deploy(...constructorArgs);
  await contract.waitForDeployment();
  return {
    address: await contract.getAddress(),
    constructorArgs,
    deploymentTxHash: contract.deploymentTransaction()?.hash,
  };
}

/** Validate before the fixture creates its gateway, so invalid inputs cause no deployments. */
export async function validatePurchaseDependencies(
  provider: Provider,
  config: PurchaseDependencies,
) {
  for (const address of [config.usdc, config.router, config.permit2]) {
    if (address === ZeroAddress) throw new Error('Purchase dependency cannot be zero');
    if ((await provider.getCode(address)) === '0x') {
      throw new Error(`Purchase dependency ${address} has no deployed code`);
    }
  }
  const usdc = new Contract(config.usdc, ['function decimals() view returns (uint8)'], provider);
  if ((await usdc.decimals()) !== 6n) throw new Error('Purchase USDC must have 6 decimals');

  const router = new Contract(
    config.router,
    ['function poolManager() view returns (address)'],
    provider,
  );
  const poolManager = getAddress(await router.poolManager());
  if (poolManager === ZeroAddress) throw new Error('Purchase dependency cannot be zero');
  if ((await provider.getCode(poolManager)) === '0x') {
    throw new Error(`Purchase PoolManager ${poolManager} has no deployed code`);
  }
}
