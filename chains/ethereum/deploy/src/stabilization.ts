import {
  AbiCoder,
  Contract,
  ContractFactory,
  Interface,
  NonceManager,
  concat,
  getAddress,
  getCreate2Address,
  keccak256,
  toBeHex,
  ZeroAddress,
  ZeroHash,
  type ContractRunner,
  type InterfaceAbi,
  type Signer,
} from 'ethers';
import { EvmContracts } from '@argonprotocol/mainchain';

// Uniswap's HookMiner uses this standard, permissionless CREATE2 deployment proxy.
// https://github.com/Arachnid/deterministic-deployment-proxy
export const CREATE2_DEPLOYER = '0x4e59b44847b379578588920cA78FbF26c0B4956C';
export const CREATE2_DEPLOYER_CODE =
  '0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe03601600081602082378035828234f58015156039578182fd5b8082525050506014600cf3';

export interface StabilizationPoolConfig {
  /** Explicit Ethereum chain ID, checked before preparing or publishing transactions. */
  chainId: string;
  /** Canonical 18-decimal ARGN and 6-decimal USDC contract addresses. */
  argonToken: string;
  usdc: string;
  /** Approved pinned Uniswap NFT manager; its immutable poolManager supplies the core address. */
  positionManager: string;
  /** Positive tick interval; independent of the directional swap fees. */
  tickSpacing: number;
  /** Initial sqrt(raw token1 / raw token0) * 2^96, accounting for token ordering and decimals. */
  sqrtPriceX96: string;
}

/** Prepare deterministic deployments and pool initialization without submitting transactions. */
export async function prepareStabilizationPool(
  runner: ContractRunner,
  config: StabilizationPoolConfig,
) {
  const provider = runner.provider;
  if (!provider) throw new Error('Stabilization preparation requires a provider');
  const { chainId } = await provider.getNetwork();
  if (BigInt(config.chainId) !== chainId) throw new Error('Stabilization config chainId mismatch');
  if (!Number.isInteger(config.tickSpacing)) throw new Error('tickSpacing must be an integer');
  if (config.tickSpacing < 1) throw new Error('tickSpacing must be positive');
  if (config.tickSpacing > 32_767) throw new Error('tickSpacing exceeds Uniswap v4 bounds');

  const sqrtPriceX96 = BigInt(config.sqrtPriceX96);
  if (sqrtPriceX96 < 4295128739n) throw new Error('sqrtPriceX96 is below Uniswap v4 bounds');
  if (sqrtPriceX96 >= 1461446703485210103287273052203988822378723970342n) {
    throw new Error('sqrtPriceX96 exceeds Uniswap v4 bounds');
  }

  const argonToken = getAddress(config.argonToken);
  const usdc = getAddress(config.usdc);
  const positionManager = getAddress(config.positionManager);
  if (argonToken === usdc) throw new Error('ARGN and USDC must be distinct');

  const manager = new Contract(
    positionManager,
    ['function poolManager() view returns (address)'],
    runner,
  );
  const poolManager = getAddress(await manager.getFunction('poolManager')());
  const dependencyCodeHashes: Record<string, string> = {};
  for (const address of [argonToken, usdc, positionManager, poolManager]) {
    if (address === ZeroAddress) throw new Error('Pool dependencies cannot be zero');
    const code = await provider.getCode(address);
    if (code === '0x') throw new Error(`Pool dependency ${address} has no deployed code`);
    dependencyCodeHashes[address] = keccak256(code);
  }

  for (const [address, decimals] of [
    [argonToken, 18],
    [usdc, 6],
  ] as const) {
    const token = new Contract(address, ['function decimals() view returns (uint8)'], runner);
    if ((await token.getFunction('decimals')()) !== BigInt(decimals)) {
      throw new Error(`Token ${address} must have ${decimals} decimals`);
    }
  }
  if ((await provider.getCode(CREATE2_DEPLOYER)) !== CREATE2_DEPLOYER_CODE) {
    throw new Error(
      `Standard CREATE2 deployer is missing or has unexpected code at ${CREATE2_DEPLOYER}`,
    );
  }

  const hookFactory = new ContractFactory(
    EvmContracts.stabilizationPoolHookArtifact.abi as InterfaceAbi,
    EvmContracts.stabilizationPoolHookArtifact.bytecode,
    runner,
  );
  const hookConstructorArgs = [poolManager, argonToken, usdc, config.tickSpacing];
  const hookInitCode = (await hookFactory.getDeployTransaction(...hookConstructorArgs)).data;
  const hookInitCodeHash = keccak256(hookInitCode);
  let hookSalt = ZeroHash;
  let hookAddress = ZeroAddress;
  for (let salt = 0; salt < 1_000_000; salt += 1) {
    hookSalt = toBeHex(salt, 32);
    hookAddress = getCreate2Address(CREATE2_DEPLOYER, hookSalt, hookInitCodeHash);
    if ((BigInt(hookAddress) & 0x3fffn) === 0x1a80n) break;
  }
  if ((BigInt(hookAddress) & 0x3fffn) !== 0x1a80n) throw new Error('Hook salt search exhausted');

  const subscriberFactory = new ContractFactory(
    EvmContracts.stabilizationPoolSubscriberArtifact.abi as InterfaceAbi,
    EvmContracts.stabilizationPoolSubscriberArtifact.bytecode,
    runner,
  );
  const subscriberConstructorArgs = [positionManager];
  const subscriberInitCode = (
    await subscriberFactory.getDeployTransaction(...subscriberConstructorArgs)
  ).data;
  const subscriberAddress = getCreate2Address(
    CREATE2_DEPLOYER,
    ZeroHash,
    keccak256(subscriberInitCode),
  );

  const quoterFactory = new ContractFactory(
    EvmContracts.uniswapV4PriceQuoterArtifact.abi as InterfaceAbi,
    EvmContracts.uniswapV4PriceQuoterArtifact.bytecode,
    runner,
  );
  const quoterConstructorArgs = [poolManager];
  const quoterInitCode = (await quoterFactory.getDeployTransaction(...quoterConstructorArgs)).data;
  const quoterAddress = getCreate2Address(CREATE2_DEPLOYER, ZeroHash, keccak256(quoterInitCode));

  const contracts = {
    stabilizationPoolHook: {
      address: hookAddress,
      constructorArgs: hookConstructorArgs,
      salt: hookSalt,
      initCode: hookInitCode,
    },
    stabilizationPoolSubscriber: {
      address: subscriberAddress,
      constructorArgs: subscriberConstructorArgs,
      salt: ZeroHash,
      initCode: subscriberInitCode,
    },
    uniswapV4PriceQuoter: {
      address: quoterAddress,
      constructorArgs: quoterConstructorArgs,
      salt: ZeroHash,
      initCode: quoterInitCode,
    },
  };

  const argonIsToken0 = BigInt(argonToken) < BigInt(usdc);
  const poolKey = {
    currency0: argonIsToken0 ? argonToken : usdc,
    currency1: argonIsToken0 ? usdc : argonToken,
    fee: 0x800000,
    tickSpacing: config.tickSpacing,
    hooks: hookAddress,
  };
  const poolKeyType =
    'tuple(address currency0,address currency1,uint24 fee,int24 tickSpacing,address hooks)';
  const poolId = keccak256(AbiCoder.defaultAbiCoder().encode([poolKeyType], [poolKey]));
  const poolManagerInterface = new Interface([
    `function initialize(${poolKeyType},uint160) returns (int24)`,
  ]);

  return {
    network: { chainId: chainId.toString() },
    dependencyCodeHashes,
    contracts,
    pool: {
      poolManager,
      positionManager,
      poolId,
      key: poolKey,
      sqrtPriceX96: sqrtPriceX96.toString(),
      buyFee: 5_000,
      sellFee: 100_000,
      oracleAddress: hookAddress,
    },
    initialization: {
      to: poolManager,
      data: poolManagerInterface.encodeFunctionData('initialize', [poolKey, sqrtPriceX96]),
    },
  };
}

export type StabilizationPoolDeployment = Awaited<ReturnType<typeof prepareStabilizationPool>> & {
  /** Confirmed transactions, persisted after each step. Existing deterministic deployments are reused. */
  transactions: Record<string, string>;
};

/** Publish a prepared bundle; retries reuse the same contract addresses and initialized pool. */
export async function publishStabilizationPool(
  deployer: Signer,
  deployment: StabilizationPoolDeployment,
  saveProgress: () => Promise<void>,
) {
  const provider = deployer.provider;
  if (!provider) throw new Error('Stabilization publication requires a provider');
  const { chainId } = await provider.getNetwork();
  if (chainId !== BigInt(deployment.network.chainId))
    throw new Error('Deployment chainId mismatch');
  if ((await provider.getCode(CREATE2_DEPLOYER)) !== CREATE2_DEPLOYER_CODE) {
    throw new Error('Unexpected CREATE2 deployer code');
  }
  for (const [address, hash] of Object.entries(deployment.dependencyCodeHashes)) {
    if (keccak256(await provider.getCode(address)) !== hash) {
      throw new Error(`Dependency code changed at ${address}; prepare a new deployment`);
    }
  }

  // Instant local mining can confirm the next transaction before a provider's nonce cache expires.
  const publisher = new NonceManager(deployer);

  for (const [name, contract] of Object.entries(deployment.contracts)) {
    if (
      getCreate2Address(CREATE2_DEPLOYER, contract.salt, keccak256(contract.initCode)) !==
      contract.address
    ) {
      throw new Error(`Invalid deterministic address for ${name}`);
    }
    if ((await provider.getCode(contract.address)) !== '0x') continue;

    const transaction = await publisher.sendTransaction({
      to: CREATE2_DEPLOYER,
      data: concat([contract.salt, contract.initCode]),
    });
    const receipt = await transaction.wait();
    if (!receipt?.status) throw new Error(`${name} deployment failed`);
    deployment.transactions[name] = receipt.hash;
    await saveProgress();
  }

  // PoolManager stores sqrtPriceX96 in the low 160 bits of pools[poolId].slot0.
  const poolManager = new Contract(
    deployment.pool.poolManager,
    ['function extsload(bytes32) view returns (bytes32)'],
    provider,
  );
  const slot = keccak256(
    AbiCoder.defaultAbiCoder().encode(['bytes32', 'uint256'], [deployment.pool.poolId, 6]),
  );
  const sqrtPriceX96 =
    BigInt(await poolManager.getFunction('extsload')(slot)) & ((1n << 160n) - 1n);
  if (sqrtPriceX96 !== 0n) {
    // An existing pool may already have traded. Only our recorded initialization permits that case.
    if (
      !deployment.transactions.initialize &&
      sqrtPriceX96 !== BigInt(deployment.pool.sqrtPriceX96)
    ) {
      throw new Error(
        'Pool already initialized at a different price; inspect its Initialize event before registering',
      );
    }
    return;
  }

  const transaction = await publisher.sendTransaction(deployment.initialization);
  const receipt = await transaction.wait();
  if (!receipt?.status) throw new Error('Pool initialization failed');
  deployment.transactions.initialize = receipt.hash;
  await saveProgress();
}
