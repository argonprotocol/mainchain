import { createRequire } from 'node:module';
import { Contract } from 'ethers';
import { network } from 'hardhat';
import { afterAll, describe, expect, it } from 'vitest';
import { expectCustomError } from './assertions.js';

const require = createRequire(import.meta.url);
const connection = await network.create();
const { ethers } = connection;
const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const argonAccountId = `0x${'42'.repeat(32)}`;
const poolKeyAbiType =
  'tuple(address currency0,address currency1,uint24 fee,int24 tickSpacing,address hooks)';

afterAll(async () => connection.close());

async function fixture() {
  const [owner, recipient] = await ethers.getSigners();

  async function deployCore(name: string, args: unknown[]) {
    const artifact = require(`@uniswap/v4-core/out/${name}.sol/${name}.json`);
    const factory = new ethers.ContractFactory(artifact.abi, artifact.bytecode.object, owner);
    return (await factory.deploy(...args)) as Contract;
  }

  const poolManager = await deployCore('PoolManager', [owner.address]);
  const positionManager = await ethers.deployContract('PositionManager', [
    poolManager.target,
    owner.address,
    100_000,
    owner.address,
    owner.address,
  ]);
  const subscriber = await ethers.deployContract('StabilizationPoolSubscriber', [
    positionManager.target,
  ]);

  const tokens = await Promise.all([
    deployCore('TestERC20', [1n << 120n]),
    deployCore('TestERC20', [1n << 120n]),
  ]);
  if (BigInt(await tokens[0].getAddress()) > BigInt(await tokens[1].getAddress())) {
    tokens.reverse();
  }

  // Let PositionManager settle from its own balance; these tests need no Permit2 substitute.
  for (const token of tokens) {
    await (await token.transfer(positionManager.target, 1n << 100n)).wait();
  }

  const poolKey = {
    currency0: tokens[0].target,
    currency1: tokens[1].target,
    fee: 500,
    tickSpacing: 10,
    hooks: ethers.ZeroAddress,
  };
  await (await poolManager.initialize(poolKey, 1n << 96n)).wait();

  async function modifyLiquidities(actions: string, params: string[]) {
    return (
      await positionManager.modifyLiquidities(
        abiCoder.encode(['bytes', 'bytes[]'], [actions, params]),
        ethers.MaxUint256,
      )
    ).wait();
  }

  const settlementParams = tokens.map(token =>
    abiCoder.encode(['address', 'uint256', 'bool'], [token.target, 0, false]),
  );
  const liquidity = 1n << 64n;
  await modifyLiquidities('0x020b0b', [
    abiCoder.encode(
      [poolKeyAbiType, 'int24', 'int24', 'uint256', 'uint128', 'uint128', 'address', 'bytes'],
      [poolKey, -100, 100, liquidity, (1n << 128n) - 1n, (1n << 128n) - 1n, owner.address, '0x'],
    ),
    ...settlementParams,
  ]);

  const tokenId = 1n;
  await (
    await positionManager.subscribe(
      tokenId,
      subscriber.target,
      abiCoder.encode(['bytes32'], [argonAccountId]),
    )
  ).wait();

  return {
    owner,
    recipient,
    positionManager,
    poolManager,
    subscriber,
    tokenId,
    poolKey,
    liquidity,
    modifyLiquidities,
    settlementParams,
  };
}

describe('StabilizationPoolSubscriber', () => {
  it('detects liquidity round trips, ignores fee collection, and keeps the beneficiary', async () => {
    const {
      positionManager,
      subscriber,
      tokenId,
      liquidity,
      modifyLiquidities,
      settlementParams,
      poolKey,
      owner,
    } = await fixture();
    expect(await subscriber.subscriptions(tokenId)).toEqual([argonAccountId, 1n]);

    const decrease = (amount: bigint) =>
      abiCoder.encode(
        ['uint256', 'uint256', 'uint128', 'uint128', 'bytes'],
        [tokenId, amount, 0, 0, '0x'],
      );
    const take = abiCoder.encode(
      ['address', 'address', 'address'],
      [poolKey.currency0, poolKey.currency1, owner.address],
    );

    await modifyLiquidities('0x0111', [decrease(0n), take]);
    expect((await subscriber.subscriptions(tokenId)).revision).toBe(1n);

    await modifyLiquidities('0x0111', [decrease(liquidity / 2n), take]);
    expect((await subscriber.subscriptions(tokenId)).revision).toBe(2n);

    await modifyLiquidities('0x000b0b', [
      abiCoder.encode(
        ['uint256', 'uint256', 'uint128', 'uint128', 'bytes'],
        [tokenId, liquidity / 2n, (1n << 128n) - 1n, (1n << 128n) - 1n, '0x'],
      ),
      ...settlementParams,
    ]);
    expect(await positionManager.getPositionLiquidity(tokenId)).toBe(liquidity);
    expect(await subscriber.subscriptions(tokenId)).toEqual([argonAccountId, 3n]);
  });

  it('keeps stale unsubscribe state harmless and changes revision on re-enrollment and burn', async () => {
    const { owner, recipient, positionManager, subscriber, tokenId, modifyLiquidities, poolKey } =
      await fixture();
    await (await positionManager.transferFrom(owner.address, recipient.address, tokenId)).wait();
    expect(await positionManager.subscriber(tokenId)).toBe(ethers.ZeroAddress);
    expect(await subscriber.subscriptions(tokenId)).toEqual([argonAccountId, 1n]);

    const nextAccount = `0x${'24'.repeat(32)}`;
    await (
      await (positionManager.connect(recipient) as Contract).subscribe(
        tokenId,
        subscriber.target,
        abiCoder.encode(['bytes32'], [nextAccount]),
      )
    ).wait();
    expect(await subscriber.subscriptions(tokenId)).toEqual([nextAccount, 2n]);
    await (
      await (positionManager.connect(recipient) as Contract).transferFrom(
        recipient.address,
        owner.address,
        tokenId,
      )
    ).wait();
    await (
      await positionManager.subscribe(
        tokenId,
        subscriber.target,
        abiCoder.encode(['bytes32'], [argonAccountId]),
      )
    ).wait();
    await modifyLiquidities('0x0311', [
      abiCoder.encode(['uint256', 'uint128', 'uint128', 'bytes'], [tokenId, 0, 0, '0x']),
      abiCoder.encode(
        ['address', 'address', 'address'],
        [poolKey.currency0, poolKey.currency1, owner.address],
      ),
    ]);
    expect((await subscriber.subscriptions(tokenId)).revision).toBe(4n);
    expect(await positionManager.subscriber(tokenId)).toBe(ethers.ZeroAddress);
  });

  it('authenticates callbacks and rejects malformed enrollment without changing state', async () => {
    const { positionManager, subscriber, tokenId } = await fixture();
    await expectCustomError(
      subscriber.notifySubscribe(tokenId, abiCoder.encode(['bytes32'], [argonAccountId])),
      subscriber,
      'OnlyPositionManager',
    );
    await expectCustomError(
      subscriber.notifyModifyLiquidity(tokenId, 1, 0),
      subscriber,
      'OnlyPositionManager',
    );
    await expectCustomError(
      subscriber.notifyBurn(tokenId, ethers.ZeroAddress, 0, 0, 0),
      subscriber,
      'OnlyPositionManager',
    );
    await expectCustomError(
      subscriber.notifyUnsubscribe(tokenId),
      subscriber,
      'OnlyPositionManager',
    );
    await (await positionManager.unsubscribe(tokenId)).wait();
    await expect(positionManager.subscribe(tokenId, subscriber.target, '0x')).rejects.toThrow();
    expect(await positionManager.subscriber(tokenId)).toBe(ethers.ZeroAddress);
    expect((await subscriber.subscriptions(tokenId)).revision).toBe(1n);
    await expect(
      positionManager.subscribe(
        tokenId,
        subscriber.target,
        abiCoder.encode(['bytes32'], [ethers.ZeroHash]),
      ),
    ).rejects.toThrow();
  });

  it('matches the storage paths used by Argon against real Uniswap deployment state', async () => {
    const { positionManager, poolManager, subscriber, tokenId, poolKey, liquidity } =
      await fixture();
    const slot = (mappingKey: bigint, seed: bigint) =>
      ethers.keccak256(abiCoder.encode(['uint256', 'uint256'], [mappingKey, seed]));

    const subscriptionSlot = slot(tokenId, 0n);
    expect(await ethers.provider.getStorage(subscriber.target, subscriptionSlot)).toBe(
      argonAccountId,
    );
    expect(
      BigInt(await ethers.provider.getStorage(subscriber.target, BigInt(subscriptionSlot) + 1n)),
    ).toBe(1n);
    expect(await ethers.provider.getStorage(positionManager.target, slot(tokenId, 7n))).toBe(
      ethers.zeroPadValue(await subscriber.getAddress(), 32).toLowerCase(),
    );

    const poolId = ethers.keccak256(abiCoder.encode([poolKeyAbiType], [poolKey]));
    const info = `${poolId.slice(0, 52)}${ethers.toBeHex(100, 3).slice(2)}${ethers.toBeHex((1 << 24) - 100, 3).slice(2)}01`;
    expect(await ethers.provider.getStorage(positionManager.target, slot(tokenId, 9n))).toBe(info);

    const positionId = ethers.solidityPackedKeccak256(
      ['address', 'int24', 'int24', 'bytes32'],
      [positionManager.target, -100, 100, ethers.toBeHex(tokenId, 32)],
    );
    const poolState = BigInt(slot(BigInt(poolId), 6n));
    const positionSlot = ethers.keccak256(
      abiCoder.encode(['bytes32', 'uint256'], [positionId, poolState + 6n]),
    );
    expect(BigInt(await ethers.provider.getStorage(poolManager.target, positionSlot))).toBe(
      liquidity,
    );
  });
});
