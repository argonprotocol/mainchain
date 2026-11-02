import { createRequire } from 'node:module';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { Contract } from 'ethers';
import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { expectCustomError } from './assertions.js';
import {
  CREATE2_DEPLOYER,
  CREATE2_DEPLOYER_CODE,
  prepareStabilizationPool,
  publishStabilizationPool,
  type StabilizationPoolDeployment,
} from '../../deploy/src/stabilization.js';

const require = createRequire(import.meta.url);
const connection = await network.create();
const { ethers } = connection;
const Q96 = 1n << 96n;

afterAll(async () => connection.close());

describe('Stabilization pool publication and directional LP fees', () => {
  it('exports the hook, subscriber and price quoter for source verification', async () => {
    const { deployment } = await fixture();
    const outputDir = await mkdtemp(join(tmpdir(), 'stabilization-verification-'));
    try {
      const manifestPath = join(outputDir, 'manifest.json');
      await writeFile(manifestPath, JSON.stringify(deployment));
      await promisify(execFile)(process.execPath, [
        '--import',
        'tsx',
        fileURLToPath(new URL('../../deploy/outputEtherscanFiles.ts', import.meta.url)),
        '--deployment-manifest',
        manifestPath,
      ]);

      const artifactNames: Record<string, string> = {
        stabilizationPoolHook: 'StabilizationPoolHook',
        stabilizationPoolSubscriber: 'StabilizationPoolSubscriber',
        uniswapV4PriceQuoter: 'UniswapV4PriceQuoter',
      };
      for (const [name, contract] of Object.entries(deployment.contracts)) {
        const input = JSON.parse(
          await readFile(join(outputDir, `${name}.standard-input.json`), 'utf8'),
        );
        if (name === 'stabilizationPoolHook') {
          expect(input.sources['project/contracts/StabilizationPoolHook.sol']).toBeDefined();
          expect(input.sources['project/contracts/UniswapV4OracleHook.sol']).toBeUndefined();
        }
        if (name === 'uniswapV4PriceQuoter') {
          expect(input.sources['project/contracts/UniswapV4PriceQuoter.sol']).toBeDefined();
          expect(Object.keys(input.sources).some(path => path.endsWith('/BaseV4Quoter.sol'))).toBe(
            true,
          );
        }
        const encoded = (
          await readFile(join(outputDir, `${name}.constructor-args.txt`), 'utf8')
        ).trim();
        const factory = await ethers.getContractFactory(artifactNames[name]);
        expect(`${factory.bytecode}${encoded}`).toBe(contract.initCode);
      }
    } finally {
      await rm(outputDir, { recursive: true, force: true });
    }
  });

  for (const argonIsToken0 of [true, false]) {
    it(`charges 10% sells and 0.5% buys with ARGN as currency${argonIsToken0 ? 0 : 1}`, async () => {
      const { owner, poolManager, argon, usdc, deployment } = await fixture(argonIsToken0);
      const nonceBefore = await ethers.provider.getTransactionCount(owner.address);
      expect(await ethers.provider.getCode(deployment.pool.oracleAddress)).toBe('0x');

      await publishStabilizationPool(ethers, deployment, async () => {});
      const hook = await ethers.getContractAt(
        'StabilizationPoolHook',
        deployment.pool.oracleAddress,
      );
      expect(await hook.poolId()).toBe(deployment.pool.poolId);
      expect(await hook.argonToken()).toBe(await argon.getAddress());
      expect(await ethers.provider.getTransactionCount(owner.address)).toBe(nonceBefore + 4);
      const quoter = await ethers.getContractAt(
        'UniswapV4PriceQuoter',
        deployment.contracts.uniswapV4PriceQuoter.address,
      );
      expect(await quoter.poolManager()).toBe(await poolManager.getAddress());

      const liquidityRouter = await deployCore('PoolModifyLiquidityTest', [poolManager.target]);
      const swapRouter = await deployCore('PoolSwapTest', [poolManager.target]);
      for (const token of [argon, usdc]) {
        await (await token.approve(liquidityRouter.target, ethers.MaxUint256)).wait();
        await (await token.approve(swapRouter.target, ethers.MaxUint256)).wait();
      }
      const key = deployment.pool.key;
      await (
        await liquidityRouter[
          'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)'
        ](
          key,
          {
            tickLower: -1000,
            tickUpper: 1000,
            liquidityDelta: 1_000_000_000n,
            salt: ethers.ZeroHash,
          },
          '0x',
        )
      ).wait();
      await (
        await liquidityRouter[
          'modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int256,bytes32),bytes)'
        ](
          key,
          { tickLower: -10, tickUpper: 10, liquidityDelta: 1_000_000_000n, salt: ethers.ZeroHash },
          '0x',
        )
      ).wait();

      const poolStateSlot = ethers.keccak256(
        ethers.AbiCoder.defaultAbiCoder().encode(
          ['bytes32', 'uint256'],
          [deployment.pool.poolId, 6],
        ),
      );
      const start = (await ethers.provider.getBlock('latest'))!.timestamp;
      let elapsed = 0;
      for (const sellingArgon of [true, false]) {
        for (const exactInput of [true, false]) {
          const zeroForOne = sellingArgon === argonIsToken0;
          elapsed += 10;
          await ethers.provider.send('evm_setNextBlockTimestamp', [start + elapsed]);

          const stateBefore = await poolManager['extsload(bytes32)'](poolStateSlot);
          const observationIndexBefore = await hook.observationIndex();
          const quoteParams = { poolKey: key, zeroForOne, exactAmount: 1_000_000n, hookData: '0x' };
          let quote;
          if (exactInput) {
            quote = await quoter.quoteExactInputSingle.staticCall(quoteParams);
          } else {
            quote = await quoter.quoteExactOutputSingle.staticCall(quoteParams);
          }
          expect(quote.gasEstimate).toBeGreaterThan(0n);
          expect(await poolManager['extsload(bytes32)'](poolStateSlot)).toBe(stateBefore);
          expect(await hook.observationIndex()).toBe(observationIndexBefore);

          if (sellingArgon && exactInput) {
            // Even a submitted quote rolls back its pool and hook writes before returning.
            const quoteReceipt = await (await quoter.quoteExactInputSingle(quoteParams)).wait();
            expect(quoteReceipt!.logs).toHaveLength(0);
            expect(await poolManager['extsload(bytes32)'](poolStateSlot)).toBe(stateBefore);
            expect(await hook.observationIndex()).toBe(observationIndexBefore);
            expect(await quoter.msgSender()).toBe(ethers.ZeroAddress);
          }

          const transaction = await swapRouter.swap(
            key,
            {
              zeroForOne,
              amountSpecified: exactInput ? -1_000_000n : 1_000_000n,
              sqrtPriceLimitX96: zeroForOne
                ? 4295128740n
                : 1461446703485210103287273052203988822378723970341n,
            },
            { takeClaims: false, settleUsingBurn: false },
            '0x',
          );
          const receipt = await transaction.wait();
          const swap = receipt.logs
            .filter((log: { address: string }) => log.address === poolManager.target)
            .map((log: { topics: string[]; data: string }) => poolManager.interface.parseLog(log))
            .find((event: { name: string } | null) => event?.name === 'Swap');
          expect(swap.args.fee).toBe(sellingArgon ? 100_000n : 5_000n);
          expect(swap.args.amount0 < 0n).toBe(zeroForOne);
          let quoteAmount;
          if (exactInput) {
            quoteAmount = zeroForOne ? swap.args.amount1 : swap.args.amount0;
          } else {
            quoteAmount = -(zeroForOne ? swap.args.amount0 : swap.args.amount1);
          }
          expect(quote[0]).toBe(quoteAmount);
          expect(quote.sqrtPriceX96After).toBe(swap.args.sqrtPriceX96);
          expect(quote.sqrtPriceX96After).not.toBe(BigInt(stateBefore) & ((1n << 160n) - 1n));
          if (sellingArgon && !exactInput) {
            expect(swap.args.liquidity).toBe(1_000_000_000n);
          }

          // Core credits the fee to LP growth in the input currency; no treasury receives it.
          const feeGrowthSlot = ethers.toBeHex(BigInt(poolStateSlot) + (zeroForOne ? 1n : 2n), 32);
          expect(BigInt(await poolManager['extsload(bytes32)'](feeGrowthSlot))).toBeGreaterThan(0n);
        }
      }

      const [, secondsPerLiquidity] = await hook.observe([elapsed, 0]);
      expect(secondsPerLiquidity[1] - secondsPerLiquidity[0]).toBeGreaterThan(0n);
      const nonceAfterTrading = await ethers.provider.getTransactionCount(owner.address);
      await publishStabilizationPool(ethers, deployment, async () => {});
      expect(await ethers.provider.getTransactionCount(owner.address)).toBe(nonceAfterTrading);
    });
  }

  it('rejects unauthenticated callbacks and propagates an unfillable quote', async () => {
    const { deployment } = await fixture();
    await publishStabilizationPool(ethers, deployment, async () => {});
    const quoter = await ethers.getContractAt(
      'UniswapV4PriceQuoter',
      deployment.contracts.uniswapV4PriceQuoter.address,
    );
    const params = {
      poolKey: deployment.pool.key,
      zeroForOne: true,
      exactAmount: 1n,
      hookData: '0x',
    };
    await expectCustomError(quoter._quoteSingle.staticCall(params, true), quoter, 'NotSelf');
    await expectCustomError(quoter.unlockCallback.staticCall('0x'), quoter, 'NotPoolManager');

    const reason = quoter.interface.encodeErrorResult('NotEnoughLiquidity', [
      deployment.pool.poolId,
    ]);
    await expectCustomError(
      quoter.quoteExactOutputSingle.staticCall(params),
      quoter,
      'UnexpectedRevertBytes',
      [reason],
    );
    expect(await quoter.msgSender()).toBe(ethers.ZeroAddress);
  });

  it('recovers a confirmed deployment after losing the progress write', async () => {
    const { owner, deployment } = await fixture();
    await expect(
      publishStabilizationPool(ethers, deployment, async () => {
        throw new Error('progress write failed');
      }),
    ).rejects.toThrow('progress write failed');
    expect(
      await ethers.provider.getCode(deployment.contracts.stabilizationPoolHook.address),
    ).not.toBe('0x');
    expect(
      await ethers.provider.getCode(deployment.contracts.stabilizationPoolSubscriber.address),
    ).toBe('0x');

    const restarted: StabilizationPoolDeployment = { ...deployment, transactions: {} };
    const nonceBefore = await ethers.provider.getTransactionCount(owner.address);
    await publishStabilizationPool(ethers, restarted, async () => {});
    expect(await ethers.provider.getTransactionCount(owner.address)).toBe(nonceBefore + 3);
    const subscriber = await ethers.getContractAt(
      'StabilizationPoolSubscriber',
      restarted.contracts.stabilizationPoolSubscriber.address,
    );
    expect(await subscriber.positionManager()).toBe(deployment.pool.positionManager);
    const quoter = await ethers.getContractAt(
      'UniswapV4PriceQuoter',
      deployment.contracts.uniswapV4PriceQuoter.address,
    );
    expect(await quoter.poolManager()).toBe(deployment.pool.poolManager);
  });

  it('stops for an unexpected chain or a pool initialized at an unreviewed price', async () => {
    const { owner, poolManager, deployment } = await fixture();
    const nonceBefore = await ethers.provider.getTransactionCount(owner.address);
    await expect(
      publishStabilizationPool(
        ethers,
        {
          ...deployment,
          network: { chainId: '1' },
        },
        async () => {},
      ),
    ).rejects.toThrow('chainId mismatch');
    expect(await ethers.provider.getTransactionCount(owner.address)).toBe(nonceBefore);

    await expect(
      publishStabilizationPool(ethers, deployment, async () => {
        if (deployment.transactions.uniswapV4PriceQuoter)
          throw new Error('stopped before initialization');
      }),
    ).rejects.toThrow('stopped before initialization');
    await (await poolManager.initialize(deployment.pool.key, Q96 * 2n)).wait();
    await expect(publishStabilizationPool(ethers, deployment, async () => {})).rejects.toThrow(
      'different price',
    );
  });
});

async function fixture(argonIsToken0 = true) {
  const [owner] = await ethers.getSigners();
  const poolManager = await deployCore('PoolManager', [owner.address]);
  const positionManager = await ethers.deployContract('PositionManager', [
    poolManager.target,
    owner.address,
    100_000,
    owner.address,
    owner.address,
  ]);
  const argon = await ethers.deployContract('TestArgon');
  let usdc = await ethers.deployContract('TestUSDC');
  while (BigInt(await argon.getAddress()) < BigInt(await usdc.getAddress()) !== argonIsToken0) {
    usdc = await ethers.deployContract('TestUSDC');
  }

  // Install the standard proxy's actual runtime; deployments below execute its CREATE2 opcode.
  await ethers.provider.send('hardhat_setCode', [CREATE2_DEPLOYER, CREATE2_DEPLOYER_CODE]);
  const plan = await prepareStabilizationPool(ethers, {
    chainId: (await ethers.provider.getNetwork()).chainId.toString(),
    argonToken: await argon.getAddress(),
    usdc: await usdc.getAddress(),
    positionManager: await positionManager.getAddress(),
    tickSpacing: 10,
    sqrtPriceX96: Q96.toString(),
  });
  const deployment: StabilizationPoolDeployment = { ...plan, transactions: {} };
  return { owner, poolManager, argon, usdc, deployment };
}

async function deployCore(name: string, args: unknown[]) {
  const [owner] = await ethers.getSigners();
  const artifact = require(`@uniswap/v4-core/out/${name}.sol/${name}.json`);
  const factory = new ethers.ContractFactory(artifact.abi, artifact.bytecode.object, owner);
  const contract = await factory.deploy(...args);
  await contract.waitForDeployment();
  return contract as Contract;
}
