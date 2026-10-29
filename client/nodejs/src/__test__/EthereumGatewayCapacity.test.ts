import { execFile } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { Metadata, TypeRegistry } from '@polkadot/types';
import { expect, it } from 'vitest';
import { getAddress, size, toHex } from 'viem';
import runtimeMetadata from '../../metadata.json';
import { createTransferOutOfArgonFinalizedBlockLog, repeatHex } from './ethereumProofTestUtils';

it('fits every activity allowed by the contract block cap within the bundled runtime envelope', async () => {
  const registry = new TypeRegistry();
  const metadata = new Metadata(registry, runtimeMetadata.result as `0x${string}`);
  const constants = metadata.asLatest.pallets.find(pallet =>
    pallet.name.eq('CrosschainTransfer'),
  )!.constants;
  const maximumActivities = registry
    .createType(
      'u32',
      constants.find(constant => constant.name.eq('MaxActivitiesPerGatewayProof'))!.value,
    )
    .toNumber();
  const maximumDataBytes = registry
    .createType(
      'u32',
      constants.find(constant => constant.name.eq('max_gateway_activity_log_data_bytes'))!.value,
    )
    .toNumber();
  const cwd = fileURLToPath(new URL('../../../../chains/ethereum/deploy/', import.meta.url));
  const { stdout } = await promisify(execFile)(
    process.execPath,
    ['--import', 'tsx', 'measure.ts', '--json'],
    { cwd },
  );
  const report = JSON.parse(stdout) as {
    gatewayActivityCapacity: {
      contractActivityLimit: number;
      mintingAuthorizationsPerTransfer: number;
    };
  };
  const { contractActivityLimit, mintingAuthorizationsPerTransfer } =
    report.gatewayActivityCapacity;
  const largestLog = createTransferOutOfArgonFinalizedBlockLog({
    gatewayAddress: repeatHex('11', 20),
    txHash: repeatHex('22', 32),
    transactionIndex: 0,
    logIndex: 0,
    nonce: 1n,
    amount: 1n,
    mintingCollateral: Array.from({ length: mintingAuthorizationsPerTransfer }, (_, index) => ({
      signingKey: getAddress(toHex(index + 1, { size: 20 })),
      microgonCollateral: 1n,
      micronotCollateral: 0n,
    })),
  });

  expect(contractActivityLimit).toBeGreaterThan(0);
  expect(contractActivityLimit).toBeLessThanOrEqual(maximumActivities);
  expect(contractActivityLimit * size(largestLog.data)).toBeLessThanOrEqual(maximumDataBytes);
}, 120_000);
