import camelcaseKeys from 'camelcase-keys';
import { concatHex, type Hex, sha256 } from 'viem';
import type {
  EthereumLightClientExecutionPayloadHeader,
  EthereumLightClientHeader,
} from './EthereumBeaconTypes';
import { getExecutionClient, type EthereumExecutionSource } from './EthereumExecution';
import { loadExecutionHeader } from './EthereumProof';

const zeroRoot: Hex = `0x${'00'.repeat(32)}`;

export async function buildExecutionHeaderProof(
  finalizedHeader: EthereumLightClientHeader,
  executionSource?: EthereumExecutionSource,
  spec?: Record<string, string>,
) {
  let epoch: bigint | undefined;
  if (spec) epoch = BigInt(finalizedHeader.beacon.slot) / BigInt(spec.SLOTS_PER_EPOCH);

  const isGloas =
    epoch !== undefined &&
    spec?.GLOAS_FORK_EPOCH !== undefined &&
    epoch >= BigInt(spec.GLOAS_FORK_EPOCH);
  let isDeneb = finalizedHeader.execution?.blob_gas_used !== undefined;
  if (epoch !== undefined && spec?.DENEB_FORK_EPOCH !== undefined) {
    isDeneb = epoch >= BigInt(spec.DENEB_FORK_EPOCH);
  }

  if (finalizedHeader.execution) {
    if (isGloas) throw new Error('Gloas light-client header is missing execution_block_hash');

    return {
      executionHeader: buildExecutionPayloadHeader(finalizedHeader.execution, isDeneb),
      executionBranch: finalizedHeader.execution_branch.map(
        witness => witness.toLowerCase() as Hex,
      ),
    };
  }

  if (!spec || epoch === undefined) {
    throw new Error('Hash-only light-client headers require the beacon fork schedule');
  }
  if (!executionSource) {
    throw new Error('Hash-only light-client headers require an Ethereum execution source');
  }
  if (
    BigInt(finalizedHeader.beacon.slot) === 0n ||
    finalizedHeader.execution_block_hash.toLowerCase() === zeroRoot
  ) {
    throw new Error('Empty light-client headers cannot provide an execution anchor');
  }

  const header = await loadExecutionHeader(
    getExecutionClient(executionSource),
    finalizedHeader.execution_block_hash as Hex,
  );
  let branch = finalizedHeader.execution_branch.map(witness => witness.toLowerCase() as Hex);
  if (branch.length !== 11) {
    throw new Error('Invalid execution hash branch length');
  }

  let index = 2856;
  if (!isGloas) {
    if (spec.CAPELLA_FORK_EPOCH === undefined || epoch < BigInt(spec.CAPELLA_FORK_EPOCH)) {
      throw new Error('Execution header proofs require a Capella or later beacon slot');
    }
    if (spec.DENEB_FORK_EPOCH === undefined) {
      throw new Error('Historical hash-only headers require DENEB_FORK_EPOCH');
    }

    // The Gloas light-client spec proves historical block_hash directly, with zero padding.
    index = isDeneb ? 812 : 412;
    const padding = isDeneb ? 2 : 3;
    if (branch.slice(0, padding).some(root => root !== zeroRoot)) {
      throw new Error('Invalid historical execution branch padding');
    }
    branch = branch.slice(padding);
  }
  if (
    merkleBranchRoot(finalizedHeader.execution_block_hash.toLowerCase() as Hex, branch, index) !==
    finalizedHeader.beacon.body_root.toLowerCase()
  ) {
    throw new Error('Execution header does not match the beacon execution hash branch');
  }

  return {
    executionHeader: { Rlp: header.serialize() },
    executionBranch: branch,
  };
}

export async function getBeaconJson<T>(beaconApiUrl: string, path: string): Promise<T> {
  const response = await fetch(
    new URL(
      path.replace(/^\/+/, ''),
      beaconApiUrl.endsWith('/') ? beaconApiUrl : `${beaconApiUrl}/`,
    ),
  );

  if (!response.ok) {
    throw new Error(
      `Beacon API request failed for ${path}: ${response.status} ${response.statusText}`,
    );
  }

  return (await response.json()) as T;
}

export function normalizeBeaconStateBranch(
  branch: string[],
  slot: string,
  spec: Record<string, string>,
  kind: 'committee' | 'finality',
): string[] {
  const epoch = BigInt(slot) / BigInt(spec.SLOTS_PER_EPOCH);
  const isGloas = spec.GLOAS_FORK_EPOCH !== undefined && epoch >= BigInt(spec.GLOAS_FORK_EPOCH);
  if (isGloas) return branch;

  const isElectra =
    spec.ELECTRA_FORK_EPOCH !== undefined && epoch >= BigInt(spec.ELECTRA_FORK_EPOCH);
  let depth = isElectra ? 6 : 5;
  if (kind === 'finality') depth += 1;
  if (branch.length <= depth) return branch;

  if (branch.slice(0, branch.length - depth).some(root => root.toLowerCase() !== zeroRoot)) {
    throw new Error(`Invalid historical ${kind} branch padding`);
  }
  return branch.slice(-depth);
}

function buildExecutionPayloadHeader(
  header: EthereumLightClientExecutionPayloadHeader,
  isDeneb: boolean,
) {
  const { blobGasUsed, excessBlobGas, ...executionHeader } = camelcaseKeys(header);

  if (isDeneb) {
    if (blobGasUsed === undefined || excessBlobGas === undefined) {
      throw new Error('Deneb execution payload header is missing blob gas fields');
    }
    return {
      Deneb: {
        ...executionHeader,
        blobGasUsed,
        excessBlobGas,
      },
    };
  }

  return { Capella: executionHeader };
}

function merkleBranchRoot(leaf: Hex, branch: Hex[], index: number): Hex {
  for (const witness of branch) {
    leaf = sha256(concatHex(index & 1 ? [witness, leaf] : [leaf, witness]));
    index >>= 1;
  }
  return leaf;
}
