import { expect, it } from 'vitest';
import {
  hashMintingGatewayActivateMintingAuthority,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  encodeMintingGatewayMintingAuthorityDeactivateTarget,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayGatewayUpdateApproval,
  type MintingGatewayMintingAuthorityActivationTarget,
} from '../EvmContracts';
import { getReadyEthereumGatewayUpdates } from '@argonprotocol/testing';
import { keccak256, type Hex } from 'viem';

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

  expect(batch.currentCouncilHash).toBe(councilHash);
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

it('does not relay entries that only have a simple majority of council weight', async () => {
  const gatewayAddress = repeatHex('11', 20);
  const councilHash = repeatHex('22', 32);
  const authoritySigningKey = repeatHex('33', 20);
  const councilSignerA = repeatHex('44', 20);
  const councilSignerB = repeatHex('55', 20);
  const councilSignerC = repeatHex('66', 20);
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
  const entry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: true,
      asMintingAuthorityActivation: hexCodec(authoritySigningKey),
      type: 'MintingAuthorityActivation',
    },
    targetPayloadHash: hexCodec(activationPayloadHash),
    previousApprovalHash: hexCodec(ZERO_HASH),
    approvalHash: hexCodec(activationApprovalHash),
    approvedTotalWeight: amount(60n),
    signatures: new Map([[hexCodec(councilSignerA), hexCodec(repeatHex('77', 65))]]),
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
              [hexCodec(councilSignerA), { weight: amount(60n) }],
              [hexCodec(councilSignerB), { weight: amount(20n) }],
              [hexCodec(councilSignerC), { weight: amount(20n) }],
            ]),
          }),
        councilApprovalQueueByDestinationChainAndNonce: async (_chain: string, nonce: bigint) => {
          if (nonce === 1n) return some(entry);
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

  expect(batch.updates).toEqual([]);
  expect(batch.firstQueueNonce).toBeUndefined();
  expect(batch.lastQueueNonce).toBeUndefined();
});

it('relays quorum-approved deactivations in the contiguous ready batch', async () => {
  const gatewayAddress = repeatHex('11', 20);
  const councilHash = repeatHex('22', 32);
  const authoritySigningKey = repeatHex('33', 20);
  const nextAuthoritySigningKey = repeatHex('34', 20);
  const councilSignerA = repeatHex('44', 20);
  const councilSignerB = repeatHex('55', 20);
  const deactivationPayload = encodeMintingGatewayMintingAuthorityDeactivateTarget({
    signingKey: authoritySigningKey,
  });
  const deactivationPayloadHash = keccak256(deactivationPayload);
  const activationTarget: MintingGatewayMintingAuthorityActivationTarget = {
    microgonCollateral: 1_500n,
    micronotCollateral: 250n,
    signingKey: nextAuthoritySigningKey,
  };
  const deactivationApprovalHash = hashMintingGatewayGatewayUpdateApproval(
    { chainId: 1n, gatewayAddress },
    {
      queueNonce: 1n,
      approvingCouncilHash: councilHash,
      kind: 2,
      targetId: signingKeyTargetId(authoritySigningKey),
      targetPayloadHash: deactivationPayloadHash,
      previousUpdateHash: ZERO_HASH,
    },
  );
  const activationApprovalHash = hashMintingGatewayActivateMintingAuthorityApproval(
    { chainId: 1n, gatewayAddress },
    {
      queueNonce: 2n,
      approvingCouncilHash: councilHash,
      previousUpdateHash: deactivationApprovalHash,
      target: activationTarget,
    },
  );
  const activationPayloadHash = hashMintingGatewayActivateMintingAuthority(
    { chainId: 1n, gatewayAddress },
    activationTarget,
  );
  const deactivationEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: false,
      isMintingAuthorityDeactivation: true,
      asMintingAuthorityDeactivation: hexCodec(authoritySigningKey),
      type: 'MintingAuthorityDeactivation',
    },
    targetPayloadHash: hexCodec(deactivationPayloadHash),
    previousApprovalHash: hexCodec(ZERO_HASH),
    approvalHash: hexCodec(deactivationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('66', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('67', 65))],
    ]),
  };
  const activationEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: true,
      asMintingAuthorityActivation: hexCodec(nextAuthoritySigningKey),
      type: 'MintingAuthorityActivation',
    },
    targetPayloadHash: hexCodec(activationPayloadHash),
    previousApprovalHash: hexCodec(deactivationApprovalHash),
    approvalHash: hexCodec(activationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('77', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('88', 65))],
    ]),
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
          if (nonce === 1n) return some(deactivationEntry);
          if (nonce === 2n) return some(activationEntry);
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

  expect(batch.firstQueueNonce).toBe(1n);
  expect(batch.lastQueueNonce).toBe(2n);
  expect(batch.updates).toEqual([
    {
      queueNonce: 1n,
      kind: 2,
      payload: deactivationPayload,
      signatures: [],
    },
    {
      queueNonce: 2n,
      kind: 1,
      payload: encodeMintingGatewayMintingAuthorityActivationTarget(activationTarget),
      signatures: [repeatHex('88', 65), repeatHex('77', 65)],
    },
  ]);
});

it('includes a trailing quorum-approved deactivation instead of peeling it off', async () => {
  const gatewayAddress = repeatHex('11', 20);
  const councilHash = repeatHex('22', 32);
  const authoritySigningKey = repeatHex('33', 20);
  const nextAuthoritySigningKey = repeatHex('34', 20);
  const councilSignerA = repeatHex('44', 20);
  const councilSignerB = repeatHex('55', 20);
  const activationTarget: MintingGatewayMintingAuthorityActivationTarget = {
    microgonCollateral: 1_500n,
    micronotCollateral: 250n,
    signingKey: nextAuthoritySigningKey,
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
  const deactivationPayload = encodeMintingGatewayMintingAuthorityDeactivateTarget({
    signingKey: authoritySigningKey,
  });
  const deactivationPayloadHash = keccak256(deactivationPayload);
  const deactivationApprovalHash = hashMintingGatewayGatewayUpdateApproval(
    { chainId: 1n, gatewayAddress },
    {
      queueNonce: 2n,
      approvingCouncilHash: councilHash,
      kind: 2,
      targetId: signingKeyTargetId(authoritySigningKey),
      targetPayloadHash: deactivationPayloadHash,
      previousUpdateHash: activationApprovalHash,
    },
  );
  const activationEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: true,
      asMintingAuthorityActivation: hexCodec(nextAuthoritySigningKey),
      type: 'MintingAuthorityActivation',
    },
    targetPayloadHash: hexCodec(activationPayloadHash),
    previousApprovalHash: hexCodec(ZERO_HASH),
    approvalHash: hexCodec(activationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('77', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('88', 65))],
    ]),
  };
  const deactivationEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: false,
      isMintingAuthorityDeactivation: true,
      asMintingAuthorityDeactivation: hexCodec(authoritySigningKey),
      type: 'MintingAuthorityDeactivation',
    },
    targetPayloadHash: hexCodec(deactivationPayloadHash),
    previousApprovalHash: hexCodec(activationApprovalHash),
    approvalHash: hexCodec(deactivationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('66', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('67', 65))],
    ]),
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
          if (nonce === 1n) return some(activationEntry);
          if (nonce === 2n) return some(deactivationEntry);
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

  expect(batch.firstQueueNonce).toBe(1n);
  expect(batch.lastQueueNonce).toBe(2n);
  expect(batch.updates).toEqual([
    {
      queueNonce: 1n,
      kind: 1,
      payload: encodeMintingGatewayMintingAuthorityActivationTarget(activationTarget),
      signatures: [],
    },
    {
      queueNonce: 2n,
      kind: 2,
      payload: deactivationPayload,
      signatures: [repeatHex('67', 65), repeatHex('66', 65)],
    },
  ]);
});

it('rejects malformed quorum-approved deactivations', async () => {
  const gatewayAddress = repeatHex('11', 20);
  const councilHash = repeatHex('22', 32);
  const authoritySigningKey = repeatHex('33', 20);
  const nextAuthoritySigningKey = repeatHex('34', 20);
  const councilSignerA = repeatHex('44', 20);
  const councilSignerB = repeatHex('55', 20);
  const activationTarget: MintingGatewayMintingAuthorityActivationTarget = {
    microgonCollateral: 1_500n,
    micronotCollateral: 250n,
    signingKey: nextAuthoritySigningKey,
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
  const deactivationPayload = encodeMintingGatewayMintingAuthorityDeactivateTarget({
    signingKey: authoritySigningKey,
  });
  const deactivationPayloadHash = keccak256(deactivationPayload);
  const malformedDeactivationApprovalHash = hashMintingGatewayGatewayUpdateApproval(
    { chainId: 1n, gatewayAddress },
    {
      queueNonce: 2n,
      approvingCouncilHash: councilHash,
      kind: 2,
      targetId: signingKeyTargetId(authoritySigningKey),
      targetPayloadHash: deactivationPayloadHash,
      previousUpdateHash: ZERO_HASH,
    },
  );
  const activationEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: true,
      asMintingAuthorityActivation: hexCodec(nextAuthoritySigningKey),
      type: 'MintingAuthorityActivation',
    },
    targetPayloadHash: hexCodec(activationPayloadHash),
    previousApprovalHash: hexCodec(ZERO_HASH),
    approvalHash: hexCodec(activationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('77', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('88', 65))],
    ]),
  };
  const malformedDeactivationEntry = {
    approvingCouncilHash: hexCodec(councilHash),
    target: {
      isMintingAuthorityActivation: false,
      isMintingAuthorityDeactivation: true,
      asMintingAuthorityDeactivation: hexCodec(authoritySigningKey),
      type: 'MintingAuthorityDeactivation',
    },
    targetPayloadHash: hexCodec(deactivationPayloadHash),
    previousApprovalHash: hexCodec(activationApprovalHash),
    approvalHash: hexCodec(malformedDeactivationApprovalHash),
    approvedTotalWeight: amount(90n),
    signatures: new Map([
      [hexCodec(councilSignerB), hexCodec(repeatHex('66', 65))],
      [hexCodec(councilSignerA), hexCodec(repeatHex('67', 65))],
    ]),
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
          if (nonce === 1n) return some(activationEntry);
          if (nonce === 2n) return some(malformedDeactivationEntry);
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

  await expect(getReadyEthereumGatewayUpdates(client as any, gatewayClient as any)).rejects.toThrow(
    'approval hash does not match deactivation',
  );
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

function signingKeyTargetId(signingKey: Hex): Hex {
  return `0x${signingKey.slice(2).padStart(64, '0').toLowerCase()}`;
}
