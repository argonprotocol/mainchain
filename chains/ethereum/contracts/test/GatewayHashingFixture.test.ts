import { AbiCoder, keccak256, toBeHex } from 'ethers';
import { describe, expect, it } from 'vitest';
import { mintingGatewayActivityHashingFixture as fixture } from '../fixtures.js';
import {
  appendMintingGatewayActivityRoot,
  hashMintingGatewayActivityBlockLocator,
  hashMintingGatewayGlobalIssuanceCouncilRotatedActivity,
  hashMintingGatewayMintingAuthorityActivatedActivity,
  hashMintingGatewayMintingAuthorityDeactivatedActivity,
  hashMintingGatewayTransferOutOfArgonCanceledActivity,
  hashMintingGatewayTransferOutOfArgonFinalizedActivity,
  hashMintingGatewayTransferToArgonStartedActivity,
} from '../hashing.js';

describe('MintingGateway hashing fixture', () => {
  it('matches the shared gateway hashing vectors', () => {
    expect(
      hashMintingGatewayTransferToArgonStartedActivity(
        fixture.context,
        fixture.activities.transferToArgonStarted,
      ),
    ).toEqual(fixture.leafHashes.transferToArgonStarted);
    expect(
      hashMintingGatewayMintingAuthorityActivatedActivity(
        fixture.context,
        fixture.activities.mintingAuthorityActivated,
      ),
    ).toEqual(fixture.leafHashes.mintingAuthorityActivated);
    expect(
      hashMintingGatewayGlobalIssuanceCouncilRotatedActivity(
        fixture.context,
        fixture.activities.globalIssuanceCouncilRotated,
      ),
    ).toEqual(fixture.leafHashes.globalIssuanceCouncilRotated);
    expect(
      hashMintingGatewayMintingAuthorityDeactivatedActivity(
        fixture.context,
        fixture.activities.mintingAuthorityDeactivated,
      ),
    ).toEqual(fixture.leafHashes.mintingAuthorityDeactivated);
    expect(
      hashMintingGatewayTransferOutOfArgonCanceledActivity(
        fixture.context,
        fixture.activities.transferOutOfArgonCanceled,
      ),
    ).toEqual(fixture.leafHashes.transferOutOfArgonCanceled);
    expect(
      hashMintingGatewayTransferOutOfArgonFinalizedActivity(
        fixture.context,
        fixture.activities.transferOutOfArgonFinalized,
      ),
    ).toEqual(fixture.leafHashes.transferOutOfArgonFinalized);

    let activityRoot = fixture.activityRootSeed;
    for (const { name, root } of fixture.roots) {
      activityRoot = appendMintingGatewayActivityRoot(activityRoot, fixture.leafHashes[name]);
      expect(activityRoot).toEqual(root);
    }
    expect(activityRoot).toEqual(fixture.finalRoot);

    expect(
      hashMintingGatewayActivityBlockLocator({
        blockNumber: fixture.blockNumber,
        startGatewayActivityNonce: fixture.startGatewayActivityNonce,
        endGatewayActivityNonce: fixture.endGatewayActivityNonce,
        activityRoot: fixture.finalRoot,
      }),
    ).toEqual(fixture.locatorHash);

    const rangeSlotKey = keccak256(
      AbiCoder.defaultAbiCoder().encode(
        ['uint256', 'uint256'],
        [fixture.locatorIndex, fixture.mappingSlot],
      ),
    );
    expect(rangeSlotKey).toEqual(fixture.rangeSlotKey);
    expect(toBeHex(BigInt(rangeSlotKey) + 1n, 32)).toEqual(fixture.rootSlotKey);

    const rangeSlotValue =
      `0x0000000000000000${toBeHex(fixture.endGatewayActivityNonce, 8).slice(2)}` +
      `${toBeHex(fixture.startGatewayActivityNonce, 8).slice(2)}` +
      `${toBeHex(fixture.blockNumber, 8).slice(2)}`;
    expect(rangeSlotValue).toEqual(fixture.rangeSlotValue);
  });
});
