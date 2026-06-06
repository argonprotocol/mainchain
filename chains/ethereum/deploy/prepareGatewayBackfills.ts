import { writeFile } from 'node:fs/promises';
import {
  buildGatewayExecutionHeaderBackfills,
  getClient,
  type ArgonClient,
} from '@argonprotocol/mainchain';
import { getAddress, type Address } from 'viem';
import { resolveRuntimeSetupDefaults } from './src/runtimeSetup.js';
import {
  getOptionalArg,
  getOptionalBigInt,
  getRequiredArg,
  parseArgs,
  stringifyJson,
} from './src/cli.js';

const args = parseArgs(process.argv.slice(2));

type GatewayBackfillCliInputs = {
  argonRpcUrl: string;
  executionRpcUrl: string;
  beaconApiUrl: string;
  gatewayAddress: Address;
  throughExecutionBlockNumber?: bigint;
  outputPath?: string;
};

type PreparedGatewayBackfill = {
  label: string;
  description: string;
  targetExecutionBlockNumber: bigint;
  executionBlockNumber: bigint;
  executionBlockHash: string;
  expectedBeaconRoot: string;
  beaconSlot: bigint;
  callHex: string;
  sudoCallHex: string;
};

type PreparedGatewayBackfills = {
  gatewayAddress: Address;
  throughExecutionBlockNumber?: bigint;
  requiredBackfills: PreparedGatewayBackfill[];
  sudoBatch?: {
    label: string;
    description: string;
    callHex: string;
    sudoCallHex: string;
  };
};

type PreparedCall = ReturnType<ArgonClient['tx']['utility']['batchAll']>;
type PreparedCalls = Parameters<ArgonClient['tx']['utility']['batchAll']>[0];

async function main() {
  const inputs = await loadCliInputs(args);
  const client = await getClient(inputs.argonRpcUrl);

  try {
    const payloads = await buildGatewayExecutionHeaderBackfills(client, {
      beaconApiUrl: inputs.beaconApiUrl,
      executionRpcUrl: inputs.executionRpcUrl,
      gatewayAddress: inputs.gatewayAddress,
      throughExecutionBlockNumber: inputs.throughExecutionBlockNumber,
    });

    const calls: PreparedCalls = [];
    const requiredBackfills: PreparedGatewayBackfill[] = [];

    for (const payload of payloads) {
      const call = client.tx.ethereumVerifier.importTrustedExecutionHeaderBackfill(
        payload.expectedBeaconRoot,
        payload.header,
        payload.executionHeaderProof,
      );

      calls.push(call);
      requiredBackfills.push(
        describeBackfillExtrinsic(client, call, {
          targetExecutionBlockNumber: payload.targetExecutionBlockNumber,
          executionBlockNumber: payload.executionBlockNumber,
          executionBlockHash: payload.executionBlockHash,
          expectedBeaconRoot: payload.expectedBeaconRoot,
          beaconSlot: BigInt(payload.header.slot),
        }),
      );
    }

    const plan: PreparedGatewayBackfills = {
      gatewayAddress: inputs.gatewayAddress,
      throughExecutionBlockNumber: inputs.throughExecutionBlockNumber,
      requiredBackfills,
    };

    if (calls.length > 0) {
      const batchCall = client.tx.utility.batchAll(
        calls as Parameters<typeof client.tx.utility.batchAll>[0],
      );
      const sudoBatchCall = client.tx.sudo.sudo(batchCall);

      plan.sudoBatch = {
        label: 'batchAll',
        description:
          'Submit one sudo-wrapped batch containing every gateway-proof backfill still needed.',
        callHex: batchCall.method.toHex(),
        sudoCallHex: sudoBatchCall.method.toHex(),
      };
    }

    const output = stringifyJson(plan);
    if (inputs.outputPath) {
      await writeFile(inputs.outputPath, `${output}\n`);
    } else {
      console.log(output);
    }
  } finally {
    await client.disconnect().catch(() => undefined);
  }
}

async function loadCliInputs(rawArgs: Record<string, string>): Promise<GatewayBackfillCliInputs> {
  const deploymentEnvironment = getOptionalArg(rawArgs, 'deployment-environment');
  const defaults = resolveRuntimeSetupDefaults(deploymentEnvironment);
  const gatewayAddress = getRequiredArg(rawArgs, 'gateway-address');
  const executionRpcUrl = getOptionalArg(rawArgs, 'execution-rpc-url') ?? defaults?.executionRpcUrl;
  const beaconApiUrl = getOptionalArg(rawArgs, 'beacon-api-url') ?? defaults?.beaconApiUrl;

  if (!executionRpcUrl) {
    throw new Error(
      'execution-rpc-url is required unless deployment-environment maps to a default public endpoint',
    );
  }
  if (!beaconApiUrl) {
    throw new Error(
      'beacon-api-url is required unless deployment-environment maps to a default public endpoint',
    );
  }

  return {
    argonRpcUrl: getRequiredArg(rawArgs, 'argon-rpc-url'),
    executionRpcUrl,
    beaconApiUrl,
    gatewayAddress: getAddress(gatewayAddress),
    throughExecutionBlockNumber: getOptionalBigInt(rawArgs, 'through-execution-block-number'),
    outputPath: getOptionalArg(rawArgs, 'output'),
  };
}

function describeBackfillExtrinsic(
  client: ArgonClient,
  call: PreparedCall,
  details: {
    targetExecutionBlockNumber: bigint;
    executionBlockNumber: bigint;
    executionBlockHash: string;
    expectedBeaconRoot: string;
    beaconSlot: bigint;
  },
): PreparedGatewayBackfill {
  const sudoCall = client.tx.sudo.sudo(call);

  return {
    label: `importTrustedExecutionHeaderBackfill:${details.targetExecutionBlockNumber.toString()}`,
    description:
      `Backfill the missing execution anchor that first covers gateway locator block ` +
      `${details.targetExecutionBlockNumber.toString()}.`,
    targetExecutionBlockNumber: details.targetExecutionBlockNumber,
    executionBlockNumber: details.executionBlockNumber,
    executionBlockHash: details.executionBlockHash,
    expectedBeaconRoot: details.expectedBeaconRoot,
    beaconSlot: details.beaconSlot,
    callHex: call.method.toHex(),
    sudoCallHex: sudoCall.method.toHex(),
  };
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
