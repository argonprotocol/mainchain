import { network } from 'hardhat';
import { JsonRpcProvider } from 'ethers';
import { expect, it } from 'vitest';
import mainnetManifest from '../../deploy/mainnet/deployment-manifest.json' with { type: 'json' };
import testnetManifest from '../../deploy/testnet/deployment-manifest.json' with { type: 'json' };

const IMPLEMENTATION_SLOT = '0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc';
const ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const rpcUrl = process.env.GATEWAY_UPGRADE_RPC_URL;

it.runIf(rpcUrl)(
  'preserves the deployed gateway state when its existing proxy is upgraded',
  async () => {
    const name = process.env.GATEWAY_UPGRADE_NETWORK ?? 'mainnet';
    if (name !== 'mainnet' && name !== 'testnet') throw new Error('Unknown gateway network');
    const manifest = name === 'mainnet' ? mainnetManifest : testnetManifest;
    const remote = new JsonRpcProvider(rpcUrl);
    const blockNumber = process.env.GATEWAY_UPGRADE_BLOCK
      ? Number(process.env.GATEWAY_UPGRADE_BLOCK)
      : await remote.getBlockNumber();
    expect((await remote.getNetwork()).chainId).toBe(BigInt(manifest.network.chainId));
    remote.destroy();

    // Transactions and impersonation are confined to this local fork.
    const connection = await network.create({
      network: 'hardhatMainnet',
      override: {
        chainId: Number(manifest.network.chainId),
        forking: { url: rpcUrl!, blockNumber },
      },
    });
    try {
      const { ethers } = connection;
      const proxyAddress = manifest.contracts.mintingGateway.address;
      const gateway = await ethers.getContractAt('MintingGateway', proxyAddress);
      const adminAddress = ethers.getAddress(
        `0x${(await ethers.provider.getStorage(proxyAddress, ADMIN_SLOT)).slice(-40)}`,
      );
      const admin = await ethers.getContractAt('ProxyAdmin', adminAddress);
      const adminOwner = await admin.getFunction('owner')();
      const fields = [
        'owner',
        'guardian',
        'paused',
        'argonToken',
        'argonotToken',
        'gatewayActivityNonce',
        'argonApprovalsNonce',
        'argonApprovalsHash',
        'latestActivityBlockLocatorIndex',
        'migrationCompleted',
        'globalIssuanceCouncil',
        'maxTransferOutMicrogonsPerArgonot',
      ];
      const before = await Promise.all(fields.map(field => gateway.getFunction(field)()));
      const slots = Array.from({ length: 13 }, (_, index) => index);
      const storageBefore = await Promise.all(
        slots.map(slot => ethers.provider.getStorage(proxyAddress, slot)),
      );
      const latestLocator: bigint = await gateway.getFunction('latestActivityBlockLocatorIndex')();
      const locators = [...new Set([1n, latestLocator])].filter(index => index <= latestLocator);
      const locatorsBefore = await Promise.all(
        locators.map(index => gateway.getFunction('activityBlockLocators')(index)),
      );

      const [purchaseAuthority] = await ethers.getSigners();
      const replacement = await (
        await ethers.getContractFactory('MintingGateway')
      ).deploy(
        await gateway.getFunction('argonToken')(),
        await gateway.getFunction('argonotToken')(),
        purchaseAuthority.address,
      );
      await replacement.waitForDeployment();
      await ethers.provider.send('hardhat_setBalance', [adminOwner, ethers.toQuantity(10n ** 18n)]);
      const owner = await ethers.getImpersonatedSigner(adminOwner);
      const transaction = await admin.connect(owner).getFunction('upgradeAndCall')(
        proxyAddress,
        await replacement.getAddress(),
        '0x',
      );
      await transaction.wait();

      expect(await Promise.all(fields.map(field => gateway.getFunction(field)()))).toEqual(before);
      expect(
        await Promise.all(slots.map(slot => ethers.provider.getStorage(proxyAddress, slot))),
      ).toEqual(storageBefore);
      expect(
        await Promise.all(
          locators.map(index => gateway.getFunction('activityBlockLocators')(index)),
        ),
      ).toEqual(locatorsBefore);
      expect(await ethers.provider.getStorage(proxyAddress, IMPLEMENTATION_SLOT)).toBe(
        ethers.zeroPadValue(await replacement.getAddress(), 32).toLowerCase(),
      );
      expect(await gateway.getFunction('swapContract')()).toBe(purchaseAuthority.address);
      expect(await gateway.getFunction('minimumSwapMicrogons')()).toBe(100_000_000n);

      const trace = await ethers.provider.send('debug_traceTransaction', [
        transaction.hash,
        {
          disableMemory: true,
          disableStorage: true,
        },
      ]);
      const writtenSlots: string[] = [];
      for (const step of trace.structLogs) {
        if (step.op === 'SSTORE') {
          writtenSlots.push(
            ethers.toBeHex(BigInt(`0x${step.stack.at(-1).replace(/^0x/, '')}`), 32),
          );
        }
      }
      expect(writtenSlots).toEqual([IMPLEMENTATION_SLOT]);
    } finally {
      await connection.close();
    }
  },
);
