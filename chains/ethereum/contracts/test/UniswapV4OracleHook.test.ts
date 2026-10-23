import { createRequire } from 'node:module';
import { Contract } from 'ethers';
import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { expectCustomError } from './assertions.js';

const require = createRequire(import.meta.url);
const connection = await network.create();
const { ethers } = connection;
const Q128 = 1n << 128n;

afterAll(async () => connection.close());

async function fixture() {
  const [owner] = await ethers.getSigners();
  // Use Uniswap's published PoolManager, settlement routers and ERC20 test artifacts unchanged.
  async function deployUniswap(name: string, args: unknown[]) {
    const artifact = require(`@uniswap/v4-core/out/${name}.sol/${name}.json`);
    const factory = new ethers.ContractFactory(artifact.abi, artifact.bytecode.object, owner);
    const contract = await factory.deploy(...args);
    await contract.waitForDeployment();
    return contract as Contract;
  }
  const manager = await deployUniswap('PoolManager', [owner.address]);
  const liquidityRouter = await deployUniswap('PoolModifyLiquidityTest', [manager.target]);
  const swapRouter = await deployUniswap('PoolSwapTest', [manager.target]);
  const tokens = await Promise.all([
    deployUniswap('TestERC20', [1n << 120n]),
    deployUniswap('TestERC20', [1n << 120n]),
  ]);
  tokens.sort((a, b) =>
    String(a.target).toLowerCase().localeCompare(String(b.target).toLowerCase()),
  );
  for (const token of tokens) {
    await (await token.approve(liquidityRouter.target, ethers.MaxUint256)).wait();
    await (await token.approve(swapRouter.target, ethers.MaxUint256)).wait();
  }

  const deployer = await ethers.deployContract('OracleHookTestDeployer');
  const hookFactory = await ethers.getContractFactory('UniswapV4OracleHook');
  const initCode = (
    await hookFactory.getDeployTransaction(
      manager.target,
      tokens[0].target,
      tokens[1].target,
      500,
      10,
    )
  ).data!;
  const initCodeHash = ethers.keccak256(initCode);
  let salt = 0n;
  let hookAddress: string;
  for (;;) {
    hookAddress = ethers.getCreate2Address(
      await deployer.getAddress(),
      ethers.toBeHex(salt, 32),
      initCodeHash,
    );
    if ((BigInt(hookAddress) & 0x3fffn) === 0x1a80n) break;
    salt += 1n;
  }
  await (await deployer.deploy(ethers.toBeHex(salt, 32), initCode)).wait();
  const hook = hookFactory.attach(hookAddress) as Contract;
  const key = {
    currency0: tokens[0].target,
    currency1: tokens[1].target,
    fee: 500,
    tickSpacing: 10,
    hooks: hookAddress,
  };
  await (await manager.initialize(key, 1n << 96n)).wait();
  const block = await ethers.provider.getBlock('latest');
  const start = block!.timestamp + 10;
  await ethers.provider.send('evm_setNextBlockTimestamp', [start]);
  const position = {
    tickLower: -1000,
    tickUpper: 1000,
    liquidityDelta: 1n << 64n,
    salt: ethers.ZeroHash,
  };
  await (
    await liquidityRouter[
      'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)'
    ](key, position, '0x')
  ).wait();
  return { owner, manager, hook, key, position, liquidityRouter, swapRouter, start };
}

describe('UniswapV4OracleHook with Uniswap v3 Oracle library', () => {
  it('accounts for additions, removals, zero liquidity and quiet intervals', async () => {
    const { hook, key, position, liquidityRouter, start } = await fixture();
    const modify =
      liquidityRouter[
        'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)'
      ];
    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 20]);
    await (await modify(key, position, '0x')).wait(); // Double liquidity.
    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 40]);
    await (
      await modify(key, { ...position, liquidityDelta: -2n * position.liquidityDelta }, '0x')
    ).wait();
    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 50]);
    await ethers.provider.send('evm_mine', []);

    const [ticks, seconds] = await hook.observe([50, 40, 30, 20, 10, 0]);
    expect(Array.from(ticks)).to.deep.equal([0n, 0n, 0n, 0n, 0n, 0n]);
    expect(seconds[1] - seconds[0]).to.equal(10n * (1n << 64n));
    expect(seconds[2] - seconds[0]).to.equal(20n * (1n << 64n));
    expect(seconds[3] - seconds[2]).to.equal(10n * (1n << 63n));
    expect(seconds[4] - seconds[2]).to.equal(20n * (1n << 63n));
    expect(seconds[5] - seconds[4]).to.equal(10n * Q128);
  });

  it('records the pre-swap tick and extrapolates the new tick after a swap', async () => {
    const { manager, hook, key, swapRouter, start } = await fixture();
    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 20]);
    await (
      await swapRouter.swap(
        key,
        { zeroForOne: true, amountSpecified: -(1n << 58n), sqrtPriceLimitX96: 4295128740n },
        { takeClaims: false, settleUsingBurn: false },
        '0x',
      )
    ).wait();
    const poolId = await hook.poolId();
    const slot = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(['bytes32', 'uint256'], [poolId, 6]),
    );
    const state = BigInt(await manager['extsload(bytes32)'](slot));
    const tick = BigInt.asIntN(24, state >> 160n);
    expect(tick < 0n).to.equal(true);
    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 40]);
    await ethers.provider.send('evm_mine', []);
    const [ticks] = await hook.observe([40, 20, 10, 0]);
    expect(ticks[1] - ticks[0]).to.equal(0n);
    expect(ticks[2] - ticks[1]).to.equal(tick * 10n);
    expect(ticks[3] - ticks[1]).to.equal(tick * 20n);
  });

  it('retains a full hour through rollover at one observation per second', async () => {
    const { hook, key, position, liquidityRouter, start } = await fixture();
    const modify =
      liquidityRouter[
        'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)'
      ];
    await expectCustomError(hook.observe([3600]), hook, 'OLD');
    for (let elapsed = 1; elapsed <= 3600; elapsed++) {
      await ethers.provider.send('evm_setNextBlockTimestamp', [start + elapsed]);
      await (await modify(key, { ...position, liquidityDelta: 0 }, '0x')).wait();
    }
    const before = await hook.observe([3600, 0]);
    expect(before[1][1] - before[1][0]).to.equal(3600n * (1n << 64n));

    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 3601]);
    await (await modify(key, { ...position, liquidityDelta: 0 }, '0x')).wait();
    const after = await hook.observe([3600, 0]);
    expect(after[1][1] - after[1][0]).to.equal(3600n * (1n << 64n));
    expect(after[1][0] - before[1][0]).to.equal(1n << 64n);
    await expectCustomError(hook.observe([3601]), hook, 'OLD');
  });

  it('writes once for same-block actions and rolls back history when a swap fails', async () => {
    const { hook, key, position, liquidityRouter, swapRouter, start } = await fixture();
    const modify =
      liquidityRouter[
        'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)'
      ];
    const initialIndex = await hook.observationIndex();
    await ethers.provider.send('evm_setAutomine', [false]);
    try {
      await ethers.provider.send('evm_setNextBlockTimestamp', [start + 10]);
      const add = await modify(key, position, '0x', { gasLimit: 1_000_000 });
      const swap = await swapRouter.swap(
        key,
        { zeroForOne: true, amountSpecified: -(1n << 58n), sqrtPriceLimitX96: 4295128740n },
        { takeClaims: false, settleUsingBurn: false },
        '0x',
        { gasLimit: 1_000_000 },
      );
      await ethers.provider.send('evm_mine', []);
      await add.wait();
      await swap.wait();
    } finally {
      await ethers.provider.send('evm_setAutomine', [true]);
    }
    expect(await hook.observationIndex()).to.equal(initialIndex + 1n);
    const [ticks, seconds] = await hook.observe([10, 0]);
    expect(ticks[1] - ticks[0]).to.equal(0n);
    expect(seconds[1] - seconds[0]).to.equal(10n * (1n << 64n));

    await ethers.provider.send('evm_setNextBlockTimestamp', [start + 20]);
    await expect(
      swapRouter.swap(
        key,
        { zeroForOne: true, amountSpecified: -1n, sqrtPriceLimitX96: 1n << 96n },
        { takeClaims: false, settleUsingBurn: false },
        '0x',
      ),
    ).rejects.toThrow();
    expect(await hook.observationIndex()).to.equal(initialIndex + 1n);
  });

  it('rejects direct writes and attachment to another pool', async () => {
    const { hook, owner, key, manager } = await fixture();
    await expectCustomError(
      hook.afterInitialize(owner.address, key, 1n << 96n, 0),
      hook,
      'OnlyPoolManager',
    );
    await expect(manager.initialize({ ...key, fee: 3000 }, 1n << 96n)).rejects.toThrow();
    expect(await hook.token0()).to.equal(key.currency0);
    expect(await hook.token1()).to.equal(key.currency1);
  });
});
