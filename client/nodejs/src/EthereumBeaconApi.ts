import { type Hex } from 'viem';
import type { EthereumLightClientHeader } from './EthereumBeaconTypes';

export function buildExecutionHeaderProof(finalizedHeader: EthereumLightClientHeader) {
  return {
    executionHeader: buildExecutionPayloadHeader(finalizedHeader.execution),
    executionBranch: finalizedHeader.execution_branch.map(witness => witness.toLowerCase() as Hex),
  };
}

export async function getBeaconJson<T>(beaconApiUrl: string, path: string): Promise<T> {
  const response = await fetch(
    new URL(path, beaconApiUrl.endsWith('/') ? beaconApiUrl : `${beaconApiUrl}/`),
  );

  if (!response.ok) {
    throw new Error(
      `Beacon API request failed for ${path}: ${response.status} ${response.statusText}`,
    );
  }

  return (await response.json()) as T;
}

function buildExecutionPayloadHeader(header: EthereumLightClientHeader['execution']) {
  const executionHeader = {
    parentHash: header.parent_hash,
    feeRecipient: header.fee_recipient,
    stateRoot: header.state_root,
    receiptsRoot: header.receipts_root,
    logsBloom: header.logs_bloom,
    prevRandao: header.prev_randao,
    blockNumber: header.block_number,
    gasLimit: header.gas_limit,
    gasUsed: header.gas_used,
    timestamp: header.timestamp,
    extraData: header.extra_data,
    baseFeePerGas: header.base_fee_per_gas,
    blockHash: header.block_hash,
    transactionsRoot: header.transactions_root,
    withdrawalsRoot: header.withdrawals_root,
  };

  if (header.blob_gas_used && header.excess_blob_gas) {
    return {
      Deneb: {
        ...executionHeader,
        blobGasUsed: header.blob_gas_used,
        excessBlobGas: header.excess_blob_gas,
      },
    };
  }

  return { Capella: executionHeader };
}
