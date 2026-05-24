import { expect, it } from 'vitest';
import {
  hashMintingGatewayActivateMintingAuthority,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  hashMintingGatewayActivateMintingAuthorityApproval,
  type MintingGatewayMintingAuthorityActivationTarget,
} from '../EvmContracts';
import { getReadyEthereumGatewayUpdates } from '@argonprotocol/testing';
import type { Hex } from 'viem';

const ZERO_HASH: Hex = `0x${'00'.repeat(32)}`;

it('builds the contiguous ready activation prefix from the Argon approval queue', async () => {
  const gatewayAddress = repeatHex('11', 20);
  const councilHash = repeatHex('22', 32);
  const authoritySigningKey = repeatHex('33', 20);
  const councilSignerA = repeatHex('44', 20);
  const councilSignerB = repeatHex('55', 20);
  const activationTarget: MintingGatewayMintingAuthorityActivationTarget = {
    microgonCollateral: 1_500n,
    micronotCollateral: 250n,
    signingKey: authoritySigningKey,
  };
  const activationPayloadHash = hashMintingGatewayActivateMintingAuthority(
    { chainId: 1n, gatewayAddress },
    activationTarget,
  );
  const activationApprovalHash = hashMintingGatewayActivateMintingAuthorityApproval(
    { chainId: 1n, gatewayAddress },
    {
      queueNonce: 1n,
      approvingCouncilHash: councilHash,
      previousUpdateHash: ZERO_HASH,
      target: activationTarget,
    },
  );
  const secondApprovalHash = hashMintingGatewayActivateMintingAuthorityApproval(
    { chainId: 1n, gatewayAddress },
    {
      queueNonce: 2n,
      approvingCouncilHash: councilHash,
      previousUpdateHash: activationApprovalHash,
      target: activationTarget,
    },
  );

  const readyEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: true,
      asMintingAuthorityActivation: hexCodec(authoritySigningKey),
      type: 'MintingAuthorityActivation',
    },
    targetPayloadHash: hexCodec(activationPayloadHash),
    previousApprovalHash: hexCodec(ZERO_HASH),
    approvalHash: hexCodec(activationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('66', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('77', 65))],
    ]),
  };
  const notReadyEntry = {
    ...readyEntry,
    approvalHash: hexCodec(secondApprovalHash),
    previousApprovalHash: hexCodec(activationApprovalHash),
    approvedTotalWeight: amount(60n),
    signatures: new Map([[hexCodec(councilSignerB), hexCodec(repeatHex('99', 65))]]),
  };
  const client = {
    query: {
      crosschainTransfer: {
        chainConfigBySourceChain: async () =>
          some({
            isEvm: true,
            asEvm: {
              chainId: amount(1n),
              gateway: hexCodec(gatewayAddress),
              argonToken: hexCodec(repeatHex('aa', 20)),
              argonotToken: hexCodec(repeatHex('bb', 20)),
            },
          }),
        activeGlobalIssuanceCouncilByDestinationChain: async () => some(hexCodec(councilHash)),
        globalIssuanceCouncilByHash: async () =>
          some({
            totalWeight: amount(100n),
            members: new Map([
              [hexCodec(councilSignerB), { weight: amount(30n) }],
              [hexCodec(councilSignerA), { weight: amount(70n) }],
            ]),
          }),
        councilApprovalQueueByDestinationChainAndNonce: async (_chain: string, nonce: bigint) => {
          if (nonce === 1n) return some(readyEntry);
          if (nonce === 2n) return some(notReadyEntry);
          return none();
        },
        mintingAuthoritiesBySigner: async () =>
          some({
            destinationChain: { type: 'Ethereum' },
            gatewayRemainingMicrogonCollateral: amount(activationTarget.microgonCollateral),
            gatewayRemainingMicronotCollateral: amount(activationTarget.micronotCollateral),
          }),
      },
    },
  };
  const gatewayClient = {
    readContract: async ({ functionName }: { functionName: string }) => {
      if (functionName === 'argonApprovalsNonce') return 0n;
      if (functionName === 'argonApprovalsHash') return ZERO_HASH;
      if (functionName === 'paused') return false;
      throw new Error(`Unexpected function ${functionName}`);
    },
  };

  const batch = await getReadyEthereumGatewayUpdates(client as any, gatewayClient as any);

  expect(batch.currentCouncil).toEqual({
    signers: [councilSignerA, councilSignerB],
    weights: [70n, 30n],
  });
  expect(batch.firstQueueNonce).toBe(1n);
  expect(batch.lastQueueNonce).toBe(1n);
  expect(batch.updates).toEqual([
    {
      queueNonce: 1n,
      kind: 1,
      payload: encodeMintingGatewayMintingAuthorityActivationTarget(activationTarget),
      signatures: [repeatHex('77', 65), repeatHex('66', 65)],
    },
  ]);
});

it('returns no updates while the gateway is paused', async () => {
  let queueLookups = 0;
  const client = {
    query: {
      crosschainTransfer: {
        chainConfigBySourceChain: async () =>
          some({
            isEvm: true,
            asEvm: {
              chainId: amount(1n),
              gateway: hexCodec(repeatHex('11', 20)),
              argonToken: hexCodec(repeatHex('aa', 20)),
              argonotToken: hexCodec(repeatHex('bb', 20)),
            },
          }),
        activeGlobalIssuanceCouncilByDestinationChain: async () =>
          some(hexCodec(repeatHex('22', 32))),
        globalIssuanceCouncilByHash: async () =>
          some({
            totalWeight: amount(100n),
            members: new Map([[hexCodec(repeatHex('44', 20)), { weight: amount(100n) }]]),
          }),
        councilApprovalQueueByDestinationChainAndNonce: async () => {
          queueLookups += 1;
          return none();
        },
      },
    },
  };
  const gatewayClient = {
    readContract: async ({ functionName }: { functionName: string }) => {
      if (functionName === 'argonApprovalsNonce') return 4n;
      if (functionName === 'argonApprovalsHash') return repeatHex('33', 32);
      if (functionName === 'paused') return true;
      throw new Error(`Unexpected function ${functionName}`);
    },
  };

  const batch = await getReadyEthereumGatewayUpdates(client as any, gatewayClient as any);

  expect(batch.paused).toBe(true);
  expect(batch.updates).toEqual([]);
  expect(queueLookups).toBe(0);
});

function amount(value: bigint) {
  return {
    toBigInt: () => value,
  };
}

function hexCodec(hex: Hex) {
  return {
    toHex: () => hex,
    toString: () => hex,
  };
}

function some<T>(value: T) {
  return {
    isNone: false,
    isSome: true,
    unwrap: () => value,
  };
}

function none() {
  return {
    isNone: true,
    isSome: false,
    unwrap: () => {
      throw new Error('Tried to unwrap None');
    },
  };
}

function repeatHex(byte: string, size: number): Hex {
  return `0x${byte.repeat(size)}`;
}
