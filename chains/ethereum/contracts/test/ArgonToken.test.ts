import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { expectCustomError } from './assertions.js';

const connection = await network.create();
const { ethers } = connection;

afterAll(async () => {
  await connection.close();
});

describe('ArgonToken', () => {
  it('binds to the gateway at construction time and does not expose mutable gateway setup', async () => {
    const [, adminSafe, holder] = await ethers.getSigners();

    const gatewayImplementationFactory = await ethers.getContractFactory('MintingGateway');
    const gatewayImplementation = (await gatewayImplementationFactory.deploy(
      ethers.ZeroAddress,
      ethers.ZeroAddress,
    )) as any;
    await gatewayImplementation.waitForDeployment();

    const initializeData = gatewayImplementationFactory.interface.encodeFunctionData('initialize', [
      adminSafe.address,
      adminSafe.address,
    ]);

    const gatewayProxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
    const gatewayProxy = (await gatewayProxyFactory.deploy(
      await gatewayImplementation.getAddress(),
      adminSafe.address,
      initializeData,
    )) as any;
    await gatewayProxy.waitForDeployment();

    const tokenFactory = await ethers.getContractFactory('ArgonToken');
    const token = (await tokenFactory.deploy(await gatewayProxy.getAddress())) as any;
    await token.waitForDeployment();

    expect(await token.gateway()).to.equal(await gatewayProxy.getAddress());
    expect(token.interface.hasFunction('burn(uint256)')).to.equal(false);
    expect(token.interface.hasFunction('setGateway(address)')).to.equal(false);
    expect(token.interface.hasFunction('finalizeGateway()')).to.equal(false);

    await expectCustomError(token.connect(holder).mint(holder.address, 1n), token, 'OnlyGateway', [
      holder.address,
    ]);
  });
});
