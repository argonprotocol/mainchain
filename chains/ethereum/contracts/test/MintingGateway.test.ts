import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { expectCustomError, expectEvent } from './assertions.js';

const SCALE = 1_000_000_000_000n;
const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const connection = await network.create();
const { ethers } = connection;

afterAll(async () => {
  await connection.close();
});

describe('MintingGateway', () => {
  async function deployGatewayStack() {
    const [, adminSafe, guardian, holder, recipient] = await ethers.getSigners();

    const gatewayImplementationFactory = await ethers.getContractFactory('MintingGateway');
    const gatewayBootstrapImplementation = (await gatewayImplementationFactory.deploy(
      ethers.ZeroAddress,
      ethers.ZeroAddress,
    )) as any;
    await gatewayBootstrapImplementation.waitForDeployment();

    const initializeData = gatewayImplementationFactory.interface.encodeFunctionData('initialize', [
      adminSafe.address,
      guardian.address,
    ]);

    const gatewayProxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
    const gatewayProxy = (await gatewayProxyFactory.deploy(
      await gatewayBootstrapImplementation.getAddress(),
      adminSafe.address,
      initializeData,
    )) as any;
    await gatewayProxy.waitForDeployment();

    const gateway = (await ethers.getContractAt(
      'MintingGateway',
      await gatewayProxy.getAddress(),
    )) as any;

    const proxyAdminStorage = await ethers.provider.getStorage(
      await gatewayProxy.getAddress(),
      ERC1967_ADMIN_SLOT,
    );
    const proxyAdminAddress = ethers.getAddress(`0x${proxyAdminStorage.slice(-40)}`);
    const proxyAdmin = (await ethers.getContractAt('ProxyAdmin', proxyAdminAddress)) as any;

    const argonFactory = await ethers.getContractFactory('ArgonToken');
    const argonotFactory = await ethers.getContractFactory('ArgonotToken');

    const argon = (await argonFactory.deploy(await gatewayProxy.getAddress())) as any;
    const argonot = (await argonotFactory.deploy(await gatewayProxy.getAddress())) as any;
    await Promise.all([argon.waitForDeployment(), argonot.waitForDeployment()]);

    const gatewayFinalImplementation = (await gatewayImplementationFactory.deploy(
      await argon.getAddress(),
      await argonot.getAddress(),
    )) as any;
    await gatewayFinalImplementation.waitForDeployment();

    return {
      adminSafe,
      guardian,
      holder,
      recipient,
      gateway,
      proxyAdmin,
      gatewayFinalImplementation,
      argon,
      argonot,
    };
  }

  async function deployFixture() {
    const stack = await deployGatewayStack();

    await stack.proxyAdmin
      .connect(stack.adminSafe)
      .upgradeAndCall(
        await stack.gateway.getAddress(),
        await stack.gatewayFinalImplementation.getAddress(),
        '0x',
      );

    await stack.gateway
      .connect(stack.adminSafe)
      .adminMintBatch(await stack.argon.getAddress(), [stack.holder.address], [1_000n]);

    await stack.gateway
      .connect(stack.adminSafe)
      .adminMintBatch(await stack.argonot.getAddress(), [stack.holder.address], [2_000n]);

    return stack;
  }

  it('burns canonical tokens and increments one nonce stream per account', async () => {
    const { argon, argonot, gateway, holder } = await deployFixture();
    const destination = ethers.encodeBytes32String('argon-destination');

    await expectCustomError(
      gateway.connect(holder).burnForTransfer(await argon.getAddress(), 250n, destination),
      argon,
      'ERC20InsufficientAllowance',
      [await gateway.getAddress(), 0n, 250n * SCALE],
    );

    await argon.connect(holder).approve(await gateway.getAddress(), 250n * SCALE);

    await expectEvent(
      gateway.connect(holder).burnForTransfer(await argon.getAddress(), 250n, destination),
      gateway,
      'BurnForTransfer',
      [holder.address, await argon.getAddress(), 250n, destination, 1n],
    );

    expect(await argon.balanceOf(holder.address)).to.equal(750n * SCALE);
    expect(await argon.allowance(holder.address, await gateway.getAddress())).to.equal(0n);
    expect(await gateway.accountNonces(holder.address)).to.equal(1n);

    await argonot.connect(holder).approve(await gateway.getAddress(), 10n * SCALE);

    await expectEvent(
      gateway.connect(holder).burnForTransfer(await argonot.getAddress(), 10n, destination),
      gateway,
      'BurnForTransfer',
      [holder.address, await argonot.getAddress(), 10n, destination, 2n],
    );

    expect(await argonot.allowance(holder.address, await gateway.getAddress())).to.equal(0n);
    expect(await gateway.accountNonces(holder.address)).to.equal(2n);
  });

  it('rejects unsupported tokens and zero-amount burns', async () => {
    const { gateway, holder } = await deployFixture();
    const destination = ethers.encodeBytes32String('argon-destination');

    const extraTokenFactory = await ethers.getContractFactory('ArgonToken');
    const extraToken = (await extraTokenFactory.deploy(await gateway.getAddress())) as any;
    await extraToken.waitForDeployment();

    await expectCustomError(
      gateway.connect(holder).burnForTransfer(await extraToken.getAddress(), 1n, destination),
      gateway,
      'UnsupportedToken',
      [await extraToken.getAddress()],
    );

    await expectCustomError(
      gateway.connect(holder).burnForTransfer(await extraToken.getAddress(), 0n, destination),
      gateway,
      'UnsupportedToken',
      [await extraToken.getAddress()],
    );
  });

  it('keeps gateway owner actions on the admin safe', async () => {
    const { adminSafe, guardian, gateway, proxyAdmin, gatewayFinalImplementation, argon, argonot } =
      await deployGatewayStack();

    expect(await gateway.owner()).to.equal(adminSafe.address);
    expect(await gateway.guardian()).to.equal(guardian.address);

    await proxyAdmin
      .connect(adminSafe)
      .upgradeAndCall(
        await gateway.getAddress(),
        await gatewayFinalImplementation.getAddress(),
        '0x',
      );

    expect(await gateway.argonToken()).to.equal(await argon.getAddress());
    expect(await gateway.argonotToken()).to.equal(await argonot.getAddress());
  });

  it('starts on a bootstrap implementation and upgrades once tokens exist', async () => {
    const { adminSafe, gateway, proxyAdmin, gatewayFinalImplementation, argon, argonot } =
      await deployGatewayStack();

    expect(await gateway.argonToken()).to.equal(ethers.ZeroAddress);
    expect(await gateway.argonotToken()).to.equal(ethers.ZeroAddress);

    await proxyAdmin
      .connect(adminSafe)
      .upgradeAndCall(
        await gateway.getAddress(),
        await gatewayFinalImplementation.getAddress(),
        '0x',
      );

    expect(await gateway.argonToken()).to.equal(await argon.getAddress());
    expect(await gateway.argonotToken()).to.equal(await argonot.getAddress());
  });

  it('assigns proxy admin ownership to the admin safe for now', async () => {
    const { adminSafe, proxyAdmin } = await deployGatewayStack();

    expect(await proxyAdmin.owner()).to.equal(adminSafe.address);
  });

  it('rejects zero-amount burns for configured tokens', async () => {
    const { argon, gateway, holder } = await deployFixture();
    const destination = ethers.encodeBytes32String('argon-destination');

    await expectCustomError(
      gateway.connect(holder).burnForTransfer(await argon.getAddress(), 0n, destination),
      gateway,
      'ZeroAmount',
    );
  });

  it('runs admin minting through the gateway with the admin safe', async () => {
    const { adminSafe, argon, gateway, recipient } = await deployFixture();

    await gateway
      .connect(adminSafe)
      .adminMintBatch(await argon.getAddress(), [recipient.address], [50n]);

    expect(await argon.balanceOf(recipient.address)).to.equal(50n * SCALE);
  });

  it('lets the admin safe rotate the guardian and rejects unauthorized changes', async () => {
    const { adminSafe, gateway, guardian, holder } = await deployFixture();

    await expectCustomError(
      gateway.connect(holder).setGuardian(holder.address),
      gateway,
      'OwnableUnauthorizedAccount',
      [holder.address],
    );

    await expectCustomError(
      gateway.connect(adminSafe).setGuardian(ethers.ZeroAddress),
      gateway,
      'ZeroGuardian',
    );

    await expectEvent(
      gateway.connect(adminSafe).setGuardian(holder.address),
      gateway,
      'GuardianUpdated',
      [guardian.address, holder.address],
    );

    expect(await gateway.guardian()).to.equal(holder.address);
  });

  it('lets the guardian pause immediately while only the admin safe can unpause', async () => {
    const { adminSafe, argon, gateway, guardian, holder, recipient } = await deployFixture();
    const destination = ethers.encodeBytes32String('argon-destination');

    await gateway.connect(guardian).pause();

    await argon.connect(holder).approve(await gateway.getAddress(), 10n * SCALE);

    await expectCustomError(
      gateway.connect(holder).burnForTransfer(await argon.getAddress(), 10n, destination),
      gateway,
      'EnforcedPause',
    );

    await expectCustomError(
      gateway.connect(guardian).unpause(),
      gateway,
      'OwnableUnauthorizedAccount',
      [guardian.address],
    );

    await gateway.connect(adminSafe).unpause();
    await gateway
      .connect(adminSafe)
      .adminMintBatch(await argon.getAddress(), [recipient.address], [1n]);

    expect(await argon.balanceOf(recipient.address)).to.equal(1n * SCALE);
  });
});
