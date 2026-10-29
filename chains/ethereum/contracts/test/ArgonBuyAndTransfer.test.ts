import { writeFile } from 'node:fs/promises';
import { network } from 'hardhat';
import type { TransactionReceipt } from 'ethers';
import { afterAll, describe, expect, it } from 'vitest';
import v4ManagerArtifact from '@uniswap/v4-core/out/PoolManager.sol/PoolManager.json' with { type: 'json' };
import v4LiquidityArtifact from '@uniswap/v4-core/out/PoolModifyLiquidityTest.sol/PoolModifyLiquidityTest.json' with { type: 'json' };
import v4HookArtifact from '@uniswap/v4-core/out/MockHooks.sol/MockHooks.json' with { type: 'json' };
import routerArtifact from './fixtures/UniversalRouter2.1.2.json' with { type: 'json' };
import permit2Artifact from './fixtures/Permit2.json' with { type: 'json' };
import { createMPT, createMerkleProof } from '@ethereumjs/mpt';
import { expectCustomError } from './assertions.js';
import {
  appendMintingGatewayActivityRoot,
  MintingGatewayEvents,
  hashMintingGatewayTransferToArgonFromSwapStartedActivity,
} from '../index.js';

const connection = await network.create();
const { ethers } = connection;
const SCALE = 10n ** 12n;
const Q96 = 1n << 96n;

afterAll(async () => {
  await connection.close();
});

async function fixture(
  zeroForOne = true,
  narrowLiquidity = false,
  hooks = ethers.ZeroAddress,
  fee = 3000,
  liquidityAmount = 10n ** 17n,
) {
  const [buyer, guardian] = await ethers.getSigners();
  const gatewayFactory = await ethers.getContractFactory('MintingGateway');
  const bootstrap = await gatewayFactory.deploy(
    ethers.ZeroAddress,
    ethers.ZeroAddress,
    ethers.ZeroAddress,
  );
  const proxyFactory = await ethers.getContractFactory('TransparentUpgradeableProxy');
  const proxy = await proxyFactory.deploy(
    await bootstrap.getAddress(),
    buyer.address,
    gatewayFactory.interface.encodeFunctionData('initialize', [
      buyer.address,
      guardian.address,
      ethers.id('council'),
      1,
      1,
      1_000_000,
    ]),
  );
  const gateway = await ethers.getContractAt('MintingGateway', await proxy.getAddress());
  const argon = await (
    await ethers.getContractFactory('ArgonToken')
  ).deploy(await gateway.getAddress());
  const argonot = await (
    await ethers.getContractFactory('ArgonotToken')
  ).deploy(await gateway.getAddress());
  let usdc = await (await ethers.getContractFactory('TestUSDC')).deploy();
  while (BigInt(await usdc.getAddress()) < BigInt(await argon.getAddress()) !== zeroForOne) {
    usdc = await (await ethers.getContractFactory('TestUSDC')).deploy();
  }

  const manager = await new ethers.ContractFactory(
    v4ManagerArtifact.abi,
    v4ManagerArtifact.bytecode.object,
    buyer,
  ).deploy(buyer.address);
  const permit2 = await new ethers.ContractFactory(
    ['function allowance(address,address,address) view returns (uint160,uint48,uint48)'],
    permit2Artifact.bytecode,
    buyer,
  ).deploy();
  expect(ethers.keccak256(routerArtifact.bytecode)).toBe(routerArtifact.creationCodeHash);
  expect(ethers.keccak256(permit2Artifact.bytecode)).toBe(permit2Artifact.creationCodeHash);
  const router = await new ethers.ContractFactory(
    [
      'constructor((address permit2,address weth9,address v2Factory,address v3Factory,bytes32 pairInitCodeHash,bytes32 poolInitCodeHash,address v4PoolManager,address v3NFTPositionManager,address v4PositionManager,address spokePool) params)',
    ],
    routerArtifact.bytecode,
    buyer,
  ).deploy([
    await permit2.getAddress(),
    await usdc.getAddress(),
    ethers.ZeroAddress,
    ethers.ZeroAddress,
    ethers.ZeroHash,
    ethers.ZeroHash,
    await manager.getAddress(),
    ethers.ZeroAddress,
    ethers.ZeroAddress,
    ethers.ZeroAddress,
  ]);
  const purchase = await (
    await ethers.getContractFactory('ArgonBuyAndTransfer')
  ).deploy(
    await usdc.getAddress(),
    await argon.getAddress(),
    await gateway.getAddress(),
    await router.getAddress(),
    await permit2.getAddress(),
  );
  const implementation = await gatewayFactory.deploy(
    await argon.getAddress(),
    await argonot.getAddress(),
    await purchase.getAddress(),
  );
  const adminSlot = await ethers.provider.getStorage(
    await gateway.getAddress(),
    '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103',
  );
  const proxyAdmin = await ethers.getContractAt(
    'ProxyAdmin',
    ethers.getAddress(`0x${adminSlot.slice(-40)}`),
  );
  await proxyAdmin.getFunction('upgradeAndCall')(
    await gateway.getAddress(),
    await implementation.getAddress(),
    '0x',
  );
  await gateway.getFunction('migrate')(
    { recipients: [buyer.address], amounts: [1_000_000n * 10n ** 18n] },
    { recipients: [], amounts: [] },
  );

  const token0 = zeroForOne ? await usdc.getAddress() : await argon.getAddress();
  const token1 = zeroForOne ? await argon.getAddress() : await usdc.getAddress();
  const sqrtPrice = zeroForOne ? Q96 * 1_000_000n : Q96 / 1_000_000n;
  const deadline = BigInt((await ethers.provider.getBlock('latest'))!.timestamp + 3600);
  const key = { currency0: token0, currency1: token1, fee, tickSpacing: 60, hooks };
  let tickLower = -887220;
  let tickUpper = 887220;
  if (narrowLiquidity) {
    tickLower = zeroForOne ? 276240 : -276360;
    tickUpper = zeroForOne ? 276360 : -276240;
  }
  await manager.getFunction('initialize')(key, sqrtPrice);
  const liquidity = await new ethers.ContractFactory(
    v4LiquidityArtifact.abi,
    v4LiquidityArtifact.bytecode.object,
    buyer,
  ).deploy(await manager.getAddress());
  await usdc.getFunction('approve')(await liquidity.getAddress(), ethers.MaxUint256);
  await argon.getFunction('approve')(await liquidity.getAddress(), ethers.MaxUint256);
  await liquidity.getFunction(
    'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)',
  )(
    key,
    {
      tickLower,
      tickUpper,
      liquidityDelta: liquidityAmount,
      salt: ethers.ZeroHash,
    },
    '0x',
  );
  const request = {
    argonAccountId: ethers.encodeBytes32String('argon-recipient'),
    maxUsdcIn: (narrowLiquidity ? 1_000n : 110n) * 10n ** 6n,
    minMicrogonsOut: 100n * 10n ** 6n,
    deadline,
  };
  const buyAndTransfer = purchase.getFunction('buyAndTransfer');
  await usdc.getFunction('approve')(await purchase.getAddress(), request.maxUsdcIn);
  return {
    buyer,
    guardian,
    gateway,
    purchase,
    argon,
    usdc,
    router,
    permit2,
    buyAndTransfer,
    key,
    request,
    sqrtPrice,
    liquidityAmount,
    manager,
    liquidity,
  };
}

describe('native Uniswap v4 buyAndTransfer', () => {
  it.each([true, false])(
    'burns only purchased whole microgons and returns dust (USDC token0: %s)',
    async zeroForOne => {
      const f = await fixture(zeroForOne);
      await f.argon.getFunction('transfer')(await f.purchase.getAddress(), 7n * SCALE);
      await f.usdc.getFunction('transfer')(await f.purchase.getAddress(), 17n);
      await f.usdc.getFunction('transfer')(await f.router.getAddress(), 19n);
      const argonBefore = await f.argon.getFunction('balanceOf')(f.buyer.address);
      const usdcBefore = await f.usdc.getFunction('balanceOf')(f.buyer.address);
      const supplyBefore = await f.argon.getFunction('totalSupply')();
      const receipt = await (await f.buyAndTransfer(f.request, f.key, '0x')).wait();
      const log = receipt.logs
        .map((x: any) => f.gateway.interface.parseLog(x))
        .find((x: any) => x?.name === 'TransferToArgonFromSwapStarted');
      expect(log).toBeDefined();
      await verifyReceiptProof(receipt, false, zeroForOne);

      const { purchase, token, amount, gatewayState } = log.args;
      const swaps = receipt.logs.filter(
        (entry: any) =>
          entry.address === purchase.poolManager &&
          entry.topics[0] === f.manager.interface.getEvent('Swap')!.topicHash,
      );
      expect(swaps).toHaveLength(1);
      const swap = f.manager.interface.parseLog(swaps[0])!;
      expect(purchase.poolManager).toBe(await f.manager.getAddress());
      expect(purchase.poolId).toBe(swap.args.id);
      expect(purchase.sqrtPriceBeforeX96).toBe(f.sqrtPrice);
      expect(purchase.sqrtPriceAfterX96).toBe(swap.args.sqrtPriceX96);
      const purchaseLog = receipt.logs.find(
        (entry: TransactionReceipt['logs'][number]) =>
          entry.topics[0] === MintingGatewayEvents.TransferToArgonFromSwapStarted.topic,
      )!;
      expect(ethers.getBytes(purchaseLog.data)).toHaveLength(512);
      const purchaseBlock = await ethers.provider.getBlock(receipt.blockHash);
      expect(purchase.timestamp).toBe(BigInt(purchaseBlock!.timestamp));
      if (zeroForOne) expect(purchase.sqrtPriceAfterX96).toBeLessThan(purchase.sqrtPriceBeforeX96);
      else expect(purchase.sqrtPriceAfterX96).toBeGreaterThan(purchase.sqrtPriceBeforeX96);
      expect(purchase.buyer).toBe(f.buyer.address);
      expect(purchase.argonAccountId).toBe(f.request.argonAccountId);
      expect(purchase.usdcSpent).toBe(f.request.maxUsdcIn);
      expect(amount).toBe(purchase.argonBought / SCALE);
      expect(amount).toBeGreaterThanOrEqual(f.request.minMicrogonsOut);
      expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(
        usdcBefore - purchase.usdcSpent,
      );
      expect(await f.argon.getFunction('balanceOf')(f.buyer.address)).toBe(
        argonBefore + (purchase.argonBought % SCALE),
      );
      expect(await f.argon.getFunction('totalSupply')()).toBe(supplyBefore - amount * SCALE);
      expect(await f.argon.getFunction('balanceOf')(await f.purchase.getAddress())).toBe(
        7n * SCALE,
      );
      expect(await f.usdc.getFunction('balanceOf')(await f.purchase.getAddress())).toBe(17n);
      expect(await f.usdc.getFunction('balanceOf')(await f.router.getAddress())).toBe(19n);
      expect(
        await f.usdc.getFunction('allowance')(
          await f.purchase.getAddress(),
          await f.permit2.getAddress(),
        ),
      ).toBe(0n);
      expect(
        (
          await f.permit2.getFunction('allowance')(
            await f.purchase.getAddress(),
            await f.usdc.getAddress(),
            await f.router.getAddress(),
          )
        )[0],
      ).toBe(0n);
      expect(
        await f.argon.getFunction('allowance')(
          await f.purchase.getAddress(),
          await f.gateway.getAddress(),
        ),
      ).toBe(0n);
      expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(1n);
      const activityHash = hashMintingGatewayTransferToArgonFromSwapStartedActivity(
        {
          chainId: (await ethers.provider.getNetwork()).chainId,
          gatewayAddress: (await f.gateway.getAddress()) as `0x${string}`,
        },
        {
          purchase: purchase.toObject(),
          token,
          amount,
          gatewayState: gatewayState.toObject(),
        } as any,
      );
      const locator = await f.gateway.getFunction('activityBlockLocators')(1);
      expect(locator.activityRoot).toBe(
        appendMintingGatewayActivityRoot(ethers.ZeroHash as `0x${string}`, activityHash),
      );
    },
  );

  it.each([true, false])(
    'rejects request floors below the live gateway minimum before spending (USDC token0: %s)',
    async zeroForOne => {
      const f = await fixture(zeroForOne);
      const snapshot = await ethers.provider.send('evm_snapshot', []);
      const receipt = await (await f.buyAndTransfer(f.request, f.key, '0x')).wait();
      const event = receipt.logs
        .map((log: any) => f.gateway.interface.parseLog(log))
        .find((log: any) => log?.name === 'TransferToArgonFromSwapStarted');
      const bound = event.args.amount;
      await ethers.provider.send('evm_revert', [snapshot]);

      const usdcBefore = await f.usdc.getFunction('balanceOf')(f.buyer.address);
      const supplyBefore = await f.argon.getFunction('totalSupply')();
      await f.gateway.getFunction('setMinimumSwapMicrogons')(bound + 1n);
      await f.usdc.getFunction('approve')(await f.purchase.getAddress(), 0);
      await expectCustomError(
        f.buyAndTransfer({ ...f.request, minMicrogonsOut: bound }, f.key, '0x'),
        f.purchase,
        'PurchaseTooSmall',
      );
      expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(usdcBefore);
      expect(await f.argon.getFunction('totalSupply')()).toBe(supplyBefore);
      expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(0n);

      await f.gateway.getFunction('setMinimumSwapMicrogons')(bound);
      await f.usdc.getFunction('approve')(await f.purchase.getAddress(), f.request.maxUsdcIn);
      await (await f.buyAndTransfer({ ...f.request, minMicrogonsOut: bound }, f.key, '0x')).wait();
      expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(1n);
    },
  );

  it.each([true, false])(
    'refunds unused USDC when native liquidity is exhausted (USDC token0: %s)',
    async zeroForOne => {
      const f = await fixture(zeroForOne, true);
      const usdcBefore = await f.usdc.getFunction('balanceOf')(f.buyer.address);
      const request = f.request;
      const receipt = await (await f.buyAndTransfer(request, f.key, '0x')).wait();
      const event = receipt.logs
        .map((log: any) => f.gateway.interface.parseLog(log))
        .find((log: any) => log?.name === 'TransferToArgonFromSwapStarted');
      await verifyReceiptProof(receipt, true, zeroForOne);
      const swap = receipt.logs
        .filter((log: any) => log.address === event.args.purchase.poolManager)
        .map((log: any) => f.manager.interface.parseLog(log))
        .find((log: any) => log?.name === 'Swap');
      expect(swap.args.liquidity).toBe(0n);
      expect(event.args.amount).toBeGreaterThanOrEqual(
        await f.gateway.getFunction('minimumSwapMicrogons')(),
      );
      expect(event.args.purchase.usdcSpent).toBeLessThan(request.maxUsdcIn);
      expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(
        usdcBefore - event.args.purchase.usdcSpent,
      );
      expect(await f.usdc.getFunction('balanceOf')(await f.purchase.getAddress())).toBe(0n);
      expect(
        await f.usdc.getFunction('allowance')(
          await f.purchase.getAddress(),
          await f.permit2.getAddress(),
        ),
      ).toBe(0n);
      expect(
        (
          await f.permit2.getFunction('allowance')(
            await f.purchase.getAddress(),
            await f.usdc.getAddress(),
            await f.router.getAddress(),
          )
        )[0],
      ).toBe(0n);
    },
  );

  it('rolls back the native swap on minimum-output and paused-gateway failures', async () => {
    const f = await fixture();
    const before = await f.usdc.getFunction('balanceOf')(f.buyer.address);
    await expect(
      f.buyAndTransfer({ ...f.request, minMicrogonsOut: 200n * 10n ** 6n }, f.key, '0x'),
    ).rejects.toThrow();
    await f.gateway.connect(f.guardian).getFunction('pause')();
    await expect(f.buyAndTransfer(f.request, f.key, '0x')).rejects.toThrow();
    expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(before);
    expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(0n);
    expect(await f.usdc.getFunction('balanceOf')(await f.purchase.getAddress())).toBe(0n);
    await f.gateway.getFunction('unpause')();
    await (await f.buyAndTransfer(f.request, f.key, '0x')).wait();
    expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(1n);
  });

  it('rejects invalid requests before spending', async () => {
    const f = await fixture();
    for (const request of [
      { ...f.request, deadline: 0 },
      { ...f.request, argonAccountId: ethers.ZeroHash },
      { ...f.request, minMicrogonsOut: 0 },
      { ...f.request, maxUsdcIn: 0 },
    ])
      await expectCustomError(f.buyAndTransfer(request, f.key, '0x'), f.purchase, 'InvalidRequest');
    expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(0n);
  });

  it.each([true, false])(
    'buys at least 100 ARGON for less than 100 USDC (USDC token0: %s)',
    async zeroForOne => {
      const f = await fixture(zeroForOne);
      const key = { ...f.key, fee: 500 };
      const price = zeroForOne ? f.sqrtPrice * 2n : f.sqrtPrice / 2n;
      await f.manager.getFunction('initialize')(key, price);
      await f.liquidity.getFunction('modifyLiquidity')(
        key,
        {
          tickLower: -887220,
          tickUpper: 887220,
          liquidityDelta: f.liquidityAmount,
          salt: ethers.ZeroHash,
        },
        '0x',
      );
      const request = { ...f.request, maxUsdcIn: 50n * 10n ** 6n };
      const receipt = await (await f.buyAndTransfer(request, key, '0x')).wait();
      const event = receipt.logs
        .map((log: any) => f.gateway.interface.parseLog(log))
        .find((log: any) => log?.name === 'TransferToArgonFromSwapStarted');
      expect(event.args.purchase.usdcSpent).toBe(50n * 10n ** 6n);
      expect(event.args.amount).toBeGreaterThanOrEqual(100n * 10n ** 6n);
    },
  );

  it.each([true, false])(
    'rolls back output below the required minimum despite pre-existing ARGN (USDC token0: %s)',
    async zeroForOne => {
      const f = await fixture(zeroForOne, true, ethers.ZeroAddress, 3000, 10n ** 14n);
      const preExistingArgon = 100n * 10n ** 18n;
      await f.argon.getFunction('transfer')(await f.purchase.getAddress(), preExistingArgon);
      const usdcBefore = await f.usdc.getFunction('balanceOf')(f.buyer.address);
      const argonBefore = await f.argon.getFunction('balanceOf')(f.buyer.address);
      const supplyBefore = await f.argon.getFunction('totalSupply')();
      await expect(f.buyAndTransfer(f.request, f.key, '0x')).rejects.toThrow();
      expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(usdcBefore);
      expect(await f.argon.getFunction('balanceOf')(f.buyer.address)).toBe(argonBefore);
      expect(await f.argon.getFunction('totalSupply')()).toBe(supplyBefore);
      expect(await f.argon.getFunction('balanceOf')(await f.purchase.getAddress())).toBe(
        preExistingArgon,
      );
      expect(await f.usdc.getFunction('balanceOf')(await f.purchase.getAddress())).toBe(0n);
      expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(0n);
    },
  );
});

it('selects a new canonical v4 pool without an Ethereum reward allowlist', async () => {
  const f = await fixture(true, false, ethers.ZeroAddress, 500);
  const receipt = await (await f.buyAndTransfer(f.request, f.key, '0x')).wait();
  const event = receipt.logs
    .map((log: any) => f.gateway.interface.parseLog(log))
    .find((log: any) => log?.name === 'TransferToArgonFromSwapStarted');
  const purchase = event.args.purchase;
  const poolId = ethers.keccak256(
    ethers.AbiCoder.defaultAbiCoder().encode(
      ['tuple(address currency0,address currency1,uint24 fee,int24 tickSpacing,address hooks)'],
      [f.key],
    ),
  );
  expect(purchase.poolManager).toBe(await f.manager.getAddress());
  expect(purchase.poolId).toBe(poolId);
  expect(purchase.sqrtPriceBeforeX96).toBe(f.sqrtPrice);
  expect(purchase.sqrtPriceAfterX96).toBeLessThan(f.sqrtPrice);
  expect(event.args.amount).toBeGreaterThanOrEqual(f.request.minMicrogonsOut);
});

it('rejects uninitialized or noncanonical v4 pools before spending', async () => {
  const f = await fixture();
  const before = await f.usdc.getFunction('balanceOf')(f.buyer.address);
  for (const key of [
    { ...f.key, fee: 500 },
    { ...f.key, currency0: ethers.ZeroAddress },
    { ...f.key, currency1: ethers.ZeroAddress },
    { ...f.key, currency0: f.key.currency1, currency1: f.key.currency0 },
  ]) {
    await expectCustomError(f.buyAndTransfer(f.request, key, '0x'), f.purchase, 'InvalidPool');
  }
  expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(before);
  expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(0n);
});

it.each([true, false])(
  'buys canonical tokens through an independently chosen v4 hook and rolls back hook failures (USDC token0: %s)',
  async zeroForOne => {
    const hooks = zeroForOne
      ? '0x10000000000000000000000000000000000000C0'
      : '0x20000000000000000000000000000000000000C0';
    await ethers.provider.send('hardhat_setCode', [hooks, v4HookArtifact.deployedBytecode.object]);
    const f = await fixture(zeroForOne, false, hooks);
    const hook = new ethers.Contract(hooks, v4HookArtifact.abi, f.buyer);
    const beforeSwap = hook.interface.getFunction('beforeSwap')!.selector;
    const buyerUsdc = await f.usdc.getFunction('balanceOf')(f.buyer.address);
    const argonSupply = await f.argon.getFunction('totalSupply')();

    await hook.getFunction('setReturnValue')(beforeSwap, '0xdeadbeef');
    await expect(f.buyAndTransfer(f.request, f.key, '0x')).rejects.toThrow();
    expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(buyerUsdc);
    expect(await f.argon.getFunction('totalSupply')()).toBe(argonSupply);
    expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(0n);

    await hook.getFunction('setReturnValue')(beforeSwap, beforeSwap);
    const receipt = await (await f.buyAndTransfer(f.request, f.key, '0x')).wait();
    const event = receipt.logs
      .map((log: any) => f.gateway.interface.parseLog(log))
      .find((log: any) => log?.name === 'TransferToArgonFromSwapStarted');
    expect(event.args.token).toBe(await f.argon.getAddress());
    expect(event.args.purchase.usdcToken).toBe(await f.usdc.getAddress());
    expect(event.args.purchase.usdcSpent).toBeGreaterThan(0n);
    expect(event.args.purchase.usdcSpent).toBeLessThanOrEqual(f.request.maxUsdcIn);
    expect(event.args.amount).toBeGreaterThanOrEqual(f.request.minMicrogonsOut);
    expect(await f.usdc.getFunction('balanceOf')(f.buyer.address)).toBe(
      buyerUsdc - event.args.purchase.usdcSpent,
    );
    expect(await f.argon.getFunction('totalSupply')()).toBe(
      argonSupply - event.args.amount * SCALE,
    );
    expect(await f.gateway.getFunction('gatewayActivityNonce')()).toBe(1n);
    await verifyReceiptProof(receipt, false, false);
  },
);

async function verifyReceiptProof(
  receipt: TransactionReceipt,
  refund: boolean,
  saveFixture: boolean,
) {
  const encodedReceipt = ethers.getBytes(
    ethers.concat([
      ethers.toBeHex(receipt.type),
      ethers.encodeRlp([
        '0x01',
        ethers.toBeHex(receipt.cumulativeGasUsed),
        receipt.logsBloom,
        receipt.logs.map(x => [x.address, [...x.topics], x.data]),
      ]),
    ]),
  );
  expect(receipt.index).toBe(0);
  const receiptKey = Uint8Array.from([0x80]);
  const trie = await createMPT();
  await trie.put(receiptKey, encodedReceipt);
  const proof = await createMerkleProof(trie, receiptKey);
  expect(Math.max(...proof.map(x => x.length))).toBeLessThanOrEqual(4096);
  if (process.env.UPDATE_UNISWAP_FIXTURES === '1' && saveFixture) {
    const eventLog = receipt.logs.find(
      x => x.topics[0] === MintingGatewayEvents.TransferToArgonFromSwapStarted.topic,
    )!;
    const gateway = await ethers.getContractAt('MintingGateway', eventLog.address);
    const locatorIndex = await gateway.getFunction('latestActivityBlockLocatorIndex')();
    expect(locatorIndex).toBe(1n);
    const locator = await gateway.getFunction('activityBlockLocators')(locatorIndex);
    const block = await ethers.provider.getBlock(receipt.blockHash);
    const purchaseEvent = gateway.interface.parseLog(eventLog)!;
    expect(purchaseEvent.args.purchase.timestamp).toBe(BigInt(block!.timestamp));
    await writeFile(
      new URL(`../../fixtures/uniswap-v4-purchase${refund ? '-refund' : ''}.json`, import.meta.url),
      JSON.stringify(
        {
          description: `Local native Uniswap v4 purchase${refund ? ' with unused-input refund' : ''} through Universal Router 2.1.2 and Permit2`,
          chainId: (await ethers.provider.getNetwork()).chainId.toString(),
          activityRootSeed: ethers.ZeroHash,
          activityRoot: locator.activityRoot,
          transactionIndex: receipt.index,
          receiptsRoot: ethers.hexlify(trie.root()),
          encodedReceipt: ethers.hexlify(encodedReceipt),
          nodes: proof.map(x => ethers.hexlify(x)),
          eventLog: {
            address: eventLog.address,
            topics: [...eventLog.topics],
            data: eventLog.data,
          },
        },
        null,
        2,
      ) + '\n',
    );
  }
}
