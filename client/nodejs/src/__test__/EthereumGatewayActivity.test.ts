import v4Purchase from '../../../../chains/ethereum/fixtures/uniswap-v4-purchase.json';
import v4Refund from '../../../../chains/ethereum/fixtures/uniswap-v4-purchase-refund.json';
import type { EthereumEventLog } from '../EthereumProof';
import { expect, it } from 'vitest';
import { MintingGatewayEvents } from '../EvmContracts';
import {
  appendEthereumGatewayActivityRoot,
  hashEthereumGatewayActivity,
  decodeEthereumGatewayActivityLog,
  decodeEthereumTransferToArgonStartedLog,
  findEthereumTransferToArgonStartedLogIndexes,
} from '../EthereumGatewayActivity';
import {
  createGlobalIssuanceCouncilRotatedBlockLog,
  createTransferToArgonStartedBlockLog,
  repeatHex,
} from './ethereumProofTestUtils';

it('decodes TransferToArgonStarted logs into gateway activity state', async () => {
  const log = createTransferToArgonStartedBlockLog({
    gatewayAddress: repeatHex('77', 20),
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 7n,
    argonAccountId: repeatHex('33', 32),
  });

  expect(
    decodeEthereumTransferToArgonStartedLog({
      topics: log.topics,
      data: log.data,
    }),
  ).toEqual({
    from: repeatHex('11', 20),
    token: repeatHex('22', 20),
    amount: 250n,
    argonAccountId: repeatHex('33', 32),
    gatewayState: {
      gatewayActivityNonce: 7n,
      argonApprovalsNonce: 0n,
      argonCirculation: 750n,
      argonotCirculation: 2_000n,
    },
  });
});

it('decodes non-transfer gateway activity logs', async () => {
  const log = createGlobalIssuanceCouncilRotatedBlockLog({
    gatewayAddress: repeatHex('77', 20),
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 1,
    nonce: 3n,
  });

  expect(
    decodeEthereumGatewayActivityLog({
      topics: log.topics,
      data: log.data,
    }),
  ).toMatchObject({
    kind: MintingGatewayEvents.GlobalIssuanceCouncilRotated.name,
    gatewayState: {
      gatewayActivityNonce: 3n,
    },
  });
});

it('finds all TransferToArgonStarted log indexes for a gateway receipt', async () => {
  const gatewayAddress = repeatHex('77', 20);
  const matchingFirst = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    argonAccountId: repeatHex('31', 32),
  });
  const nonMatching = createGlobalIssuanceCouncilRotatedBlockLog({
    gatewayAddress,
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 1,
    nonce: 2n,
  });
  const matchingSecond = createTransferToArgonStartedBlockLog({
    gatewayAddress,
    txHash: repeatHex('10', 32),
    transactionIndex: 0,
    logIndex: 2,
    nonce: 3n,
    argonAccountId: repeatHex('32', 32),
  });

  expect(
    findEthereumTransferToArgonStartedLogIndexes(
      {
        transactionHash: repeatHex('10', 32),
        logs: [
          {
            address: matchingFirst.address,
            topics: matchingFirst.topics,
          },
          {
            address: nonMatching.address,
            topics: nonMatching.topics,
          },
          {
            address: matchingSecond.address,
            topics: matchingSecond.topics,
          },
        ],
      },
      gatewayAddress,
    ),
  ).toEqual([0, 2]);
});

it.each([v4Purchase, v4Refund])(
  'decodes native purchases and reconstructs their gateway storage commitment',
  fixture => {
    const eventLog = fixture.eventLog as EthereumEventLog;
    const decoded = decodeEthereumGatewayActivityLog(eventLog);
    expect(decoded.kind).toBe('TransferToArgonFromSwapStarted');
    if (decoded.kind !== 'TransferToArgonFromSwapStarted') throw new Error('Expected purchase');
    expect(decoded.amount).toBe(decoded.purchase.argonBought / 1_000_000_000_000n);
    expect(() => decodeEthereumTransferToArgonStartedLog(eventLog)).toThrow(
      'Expected TransferToArgonStarted',
    );
    expect(
      appendEthereumGatewayActivityRoot(
        fixture.activityRootSeed as `0x${string}`,
        hashEthereumGatewayActivity(
          {
            chainId: BigInt(fixture.chainId),
            gatewayAddress: eventLog.address,
          },
          decoded,
        ),
      ),
    ).toBe(fixture.activityRoot);
  },
);
