import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { network } from 'hardhat';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { getAddress } from 'viem';
import { JsonRpcProvider, Wallet, type InterfaceAbi } from 'ethers';
import {
  CREATE2_DEPLOYER,
  CREATE2_DEPLOYER_CODE,
  TestEthereum,
  uniswapV4FixtureArtifacts,
  prepareStabilizationPool,
  publishStabilizationPool,
} from '../../../../testing/nodejs/lib/index.js';

const connection = await network.create();
const { ethers } = connection;
const deployerPrivateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
const rpc = createServer(async (request, response) => {
  let body = '';
  for await (const chunk of request) body += chunk;
  const messages = JSON.parse(body);
  const results = await Promise.all(
    (Array.isArray(messages) ? messages : [messages]).map(async message => {
      try {
        return {
          jsonrpc: '2.0',
          id: message.id,
          result: await connection.provider.request({
            method: message.method,
            params: message.params,
          }),
        };
      } catch (error) {
        const {
          code = -32000,
          message: detail,
          data,
        } = error as Error & { code?: number; data?: unknown };
        return { jsonrpc: '2.0', id: message.id, error: { code, message: detail, data } };
      }
    }),
  );
  response.setHeader('Content-Type', 'application/json');
  response.end(JSON.stringify(Array.isArray(messages) ? results : results[0]));
});
let executionRpcUrl: string;

beforeAll(async () => {
  await new Promise<void>((resolve, reject) => {
    rpc.once('error', reject);
    rpc.listen(0, '127.0.0.1', resolve);
  });
  executionRpcUrl = `http://127.0.0.1:${(rpc.address() as AddressInfo).port}`;
});

afterAll(async () => {
  await new Promise<void>((resolve, reject) =>
    rpc.close(error => {
      if (error) reject(error);
      else resolve();
    }),
  );
  await connection.close();
});

describe('TestEthereum purchase fixture', () => {
  it('binds the adapter to the final gateway implementation and canonical dependencies', async () => {
    const [owner] = await ethers.getSigners();
    const ethereum = new TestEthereum();
    ethereum.executionRpcUrl = executionRpcUrl;
    ethereum.chainId = '0x7a69';
    const usdc = await ethers.getContractAt(
      'TestUSDC',
      await ethereum.deployUsdcFixture(deployerPrivateKey),
    );
    expect(await usdc.decimals()).toBe(6n);
    const poolManager = await new ethers.ContractFactory(
      uniswapV4FixtureArtifacts.poolManager.abi as InterfaceAbi,
      uniswapV4FixtureArtifacts.poolManager.bytecode,
      owner,
    ).deploy(owner.address);
    const permit2 = await new ethers.ContractFactory(
      [],
      uniswapV4FixtureArtifacts.permit2.bytecode,
      owner,
    ).deploy();
    const router = await new ethers.ContractFactory(
      uniswapV4FixtureArtifacts.router.abi as InterfaceAbi,
      uniswapV4FixtureArtifacts.router.bytecode,
      owner,
    ).deploy([
      permit2.target,
      usdc.target,
      ethers.ZeroAddress,
      ethers.ZeroAddress,
      ethers.ZeroHash,
      ethers.ZeroHash,
      poolManager.target,
      ethers.ZeroAddress,
      ethers.ZeroAddress,
      ethers.ZeroAddress,
    ]);
    const fixture = await ethereum.deployMintingGatewayFixture({
      deployerPrivateKey,
      purchase: {
        usdc: getAddress(await usdc.getAddress()),
        router: getAddress(await router.getAddress()),
        permit2: getAddress(await permit2.getAddress()),
      },
    });

    const gateway = await ethers.getContractAt('MintingGateway', fixture.gatewayAddress);
    expect(await gateway.swapContract()).not.toBe(ethers.ZeroAddress);
    expect(await gateway.swapContract()).toBe(ethers.getAddress(fixture.buyAndTransferAddress!));
    const adapter = await ethers.getContractAt(
      'ArgonBuyAndTransfer',
      fixture.buyAndTransferAddress!,
    );
    expect(await adapter.gateway()).toBe(ethers.getAddress(fixture.gatewayAddress));
    expect(await adapter.argon()).toBe(ethers.getAddress(fixture.argonTokenAddress));
    expect(await adapter.usdc()).toBe(ethers.getAddress(await usdc.getAddress()));
    expect(await adapter.poolManager()).toBe(await poolManager.getAddress());
    expect(await adapter.router()).toBe(await router.getAddress());
    expect(await adapter.permit2()).toBe(await permit2.getAddress());
    expect(await gateway.migrationCompleted()).toBe(false);

    const positionManager = await new ethers.ContractFactory(
      uniswapV4FixtureArtifacts.positionManager.abi as InterfaceAbi,
      uniswapV4FixtureArtifacts.positionManager.bytecode,
      owner,
    ).deploy(poolManager.target, permit2.target, 100_000, owner.address, usdc.target);
    await connection.provider.request({
      method: 'hardhat_setCode',
      params: [CREATE2_DEPLOYER, CREATE2_DEPLOYER_CODE],
    });
    const provider = new JsonRpcProvider(executionRpcUrl);
    const wallet = new Wallet(deployerPrivateKey, provider);
    try {
      const deployment = {
        ...(await prepareStabilizationPool(provider, {
          chainId: '31337',
          argonToken: fixture.argonTokenAddress,
          usdc: await usdc.getAddress(),
          positionManager: await positionManager.getAddress(),
          tickSpacing: 10,
          sqrtPriceX96: (1n << 96n).toString(),
        })),
        transactions: {},
      };
      await publishStabilizationPool(wallet, deployment, async () => {});
      const hook = await ethers.getContractAt(
        'StabilizationPoolHook',
        deployment.pool.oracleAddress,
      );
      expect(await hook.poolId()).toBe(deployment.pool.poolId);
      expect(await hook.argonToken()).toBe(ethers.getAddress(fixture.argonTokenAddress));
    } finally {
      provider.destroy();
    }
  });

  it('keeps ordinary fixtures usable without purchase dependencies', async () => {
    const ethereum = new TestEthereum();
    ethereum.executionRpcUrl = executionRpcUrl;
    ethereum.chainId = '0x7a69';
    const fixture = await ethereum.deployMintingGatewayFixture({ deployerPrivateKey });
    const gateway = await ethers.getContractAt('MintingGateway', fixture.gatewayAddress);
    expect(await gateway.swapContract()).toBe(ethers.ZeroAddress);
    expect(await gateway.argonToken()).toBe(ethers.getAddress(fixture.argonTokenAddress));
    expect(await gateway.argonotToken()).toBe(ethers.getAddress(fixture.argonotTokenAddress));
  });

  it('rejects an 18-decimal USDC configuration before deploying a gateway', async () => {
    const [owner] = await ethers.getSigners();
    const token = await ethers.deployContract('TestArgon');
    const address = getAddress(await token.getAddress());
    const ethereum = new TestEthereum();
    ethereum.executionRpcUrl = executionRpcUrl;
    ethereum.chainId = '0x7a69';
    const nonceBefore = await ethers.provider.getTransactionCount(owner.address);

    await expect(
      ethereum.deployMintingGatewayFixture({
        deployerPrivateKey,
        purchase: { usdc: address, router: address, permit2: address },
      }),
    ).rejects.toThrow('Purchase USDC must have 6 decimals');
    expect(await ethers.provider.getTransactionCount(owner.address)).toBe(nonceBefore);
  });
});
