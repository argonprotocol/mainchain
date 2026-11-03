import gatewayActivityHashingFixtureJson from './test/fixtures/gateway-activity-hashing.json' with { type: 'json' };
import { parseAbi, type Hex } from 'viem';
import poolManagerArtifact from '@uniswap/v4-core/out/PoolManager.sol/PoolManager.json' with { type: 'json' };
import positionManagerArtifact from './artifacts/@uniswap/v4-periphery/src/PositionManager.sol/PositionManager.json' with { type: 'json' };
import usdcArtifact from './artifacts/contracts/test/UniswapFixtures.sol/TestUSDC.json' with { type: 'json' };
import routerFixture from './test/fixtures/UniversalRouter2.1.2.json' with { type: 'json' };
import permit2Fixture from './test/fixtures/Permit2.json' with { type: 'json' };
import type {
  EvmContractArtifact,
  MintingGatewayGlobalIssuanceCouncilRotated,
  MintingGatewayHashContext,
  MintingGatewayMintingAuthorityActivated,
  MintingGatewayMintingAuthorityCollateral,
  MintingGatewayMintingAuthorityDeactivated,
  MintingGatewayTransferOutOfArgonCanceled,
  MintingGatewayTransferOutOfArgonFinalized,
  MintingGatewayTransferToArgonStarted,
} from './index.js';

/** Pinned local-development dependencies for the canonical stabilization deployment helpers. */
export const uniswapV4FixtureArtifacts: Record<
  'poolManager' | 'positionManager' | 'router' | 'permit2' | 'usdc',
  EvmContractArtifact
> = {
  poolManager: {
    abi: poolManagerArtifact.abi as EvmContractArtifact['abi'],
    bytecode: poolManagerArtifact.bytecode.object as Hex,
  },
  positionManager: positionManagerArtifact as EvmContractArtifact,
  router: {
    abi: parseAbi([
      'constructor((address permit2,address weth9,address v2Factory,address v3Factory,bytes32 pairInitCodeHash,bytes32 poolInitCodeHash,address v4PoolManager,address v3NFTPositionManager,address v4PositionManager,address spokePool) params)',
    ]),
    bytecode: routerFixture.bytecode as Hex,
  },
  permit2: { abi: [], bytecode: permit2Fixture.bytecode as Hex },
  usdc: usdcArtifact as EvmContractArtifact,
};

type GatewayActivityHashFixtureName =
  | 'transferToArgonStarted'
  | 'mintingAuthorityActivated'
  | 'globalIssuanceCouncilRotated'
  | 'mintingAuthorityDeactivated'
  | 'transferOutOfArgonCanceled'
  | 'transferOutOfArgonFinalized';

export type MintingGatewayActivityHashingFixture = {
  context: MintingGatewayHashContext;
  mappingSlot: bigint;
  maxGatewayActivitiesPerBlock: bigint;
  maxMintingAuthorizationsPerTransfer: bigint;
  locatorIndex: bigint;
  activityRootSeed: Hex;
  blockNumber: bigint;
  startGatewayActivityNonce: bigint;
  endGatewayActivityNonce: bigint;
  rangeSlotKey: Hex;
  rootSlotKey: Hex;
  rangeSlotValue: Hex;
  activities: {
    transferToArgonStarted: MintingGatewayTransferToArgonStarted;
    mintingAuthorityActivated: MintingGatewayMintingAuthorityActivated;
    globalIssuanceCouncilRotated: MintingGatewayGlobalIssuanceCouncilRotated;
    mintingAuthorityDeactivated: MintingGatewayMintingAuthorityDeactivated;
    transferOutOfArgonCanceled: MintingGatewayTransferOutOfArgonCanceled;
    transferOutOfArgonFinalized: Omit<
      MintingGatewayTransferOutOfArgonFinalized,
      'mintingCollateral'
    > & {
      mintingCollateral: MintingGatewayMintingAuthorityCollateral[];
    };
  };
  leafHashes: Record<GatewayActivityHashFixtureName, Hex>;
  roots: Array<{ name: GatewayActivityHashFixtureName; root: Hex }>;
  finalRoot: Hex;
  locatorHash: Hex;
};

function fixtureValue(value: unknown): unknown {
  if (typeof value === 'string' && /^\d+$/.test(value)) {
    return BigInt(value);
  }

  if (Array.isArray(value)) {
    return value.map(fixtureValue);
  }

  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, fixtureValue(entry)]),
    );
  }

  return value;
}

export const mintingGatewayActivityHashingFixture = fixtureValue(
  gatewayActivityHashingFixtureJson,
) as MintingGatewayActivityHashingFixture;
