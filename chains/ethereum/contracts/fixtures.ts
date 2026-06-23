import gatewayActivityHashingFixtureJson from './test/fixtures/gateway-activity-hashing.json' with { type: 'json' };
import type { Hex } from 'viem';
import type {
  MintingGatewayGlobalIssuanceCouncilRotated,
  MintingGatewayHashContext,
  MintingGatewayMintingAuthorityActivated,
  MintingGatewayMintingAuthorityCollateral,
  MintingGatewayMintingAuthorityDeactivated,
  MintingGatewayTransferOutOfArgonCanceled,
  MintingGatewayTransferOutOfArgonFinalized,
  MintingGatewayTransferToArgonStarted,
} from './index.js';

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
