import { readFile } from 'node:fs/promises';
import { getAddress, ZeroAddress } from 'ethers';
import type { HardhatEthers } from '@nomicfoundation/hardhat-ethers/types';

export async function deployBuyAndTransfer(
  ethers: HardhatEthers,
  gatewayAddress: string,
  argonTokenAddress: string,
  configPath: string,
) {
  const config = JSON.parse(await readFile(configPath, 'utf8'));
  const routerAddress = getAddress(config.router);
  const router = await ethers.getContractAt(
    ['function poolManager() view returns (address)'],
    routerAddress,
  );
  const poolManagerAddress = getAddress(await router.getFunction('poolManager')());
  const constructorArgs = [
    getAddress(config.usdc),
    argonTokenAddress,
    gatewayAddress,
    routerAddress,
    getAddress(config.permit2),
  ];
  for (const address of [...constructorArgs, poolManagerAddress]) {
    if (address === ZeroAddress) throw new Error('Purchase dependency cannot be zero');
    if ((await ethers.provider.getCode(address)) === '0x') {
      throw new Error(`Purchase dependency ${address} has no deployed code`);
    }
  }
  const factory = await ethers.getContractFactory('ArgonBuyAndTransfer');
  const contract = await factory.deploy(...constructorArgs);
  await contract.waitForDeployment();
  return {
    address: await contract.getAddress(),
    constructorArgs,
    deploymentTxHash: contract.deploymentTransaction()?.hash,
  };
}
