import { execFile } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import { getClient, type ArgonClient } from '@argonprotocol/mainchain';
import { createPublicClient, http } from 'viem';
import { deployRoot } from './src/hardhat.js';
import {
  deriveEstimatedMicrogonsPerEth,
  extractRuntimeSetupInputsFromManifest,
  parseActivationPricingRecommendation,
  resolveRuntimeSetupDefaults,
  type ActivationPricingMeasureReport,
  type RuntimeSetupManifest,
} from './src/runtimeSetup.js';
import {
  getOptionalArg,
  getOptionalBigInt,
  getRequiredArg,
  parseArgs,
  stringifyJson,
} from './src/cli.js';

const args = parseArgs(process.argv.slice(2));
const execFileAsync = promisify(execFile);

type RuntimePricingCliInputs = {
  argonRpcUrl: string;
  executionRpcUrl?: string;
  deploymentEnvironment?: string;
  estimatedMicrogonsPerEth?: bigint;
  estimatedWeiPerGas?: bigint;
  activationGasCost?: bigint;
  signatureGasCost?: bigint;
  measureReportPath?: string;
  outputPath?: string;
};

type PreparedRuntimePricingUpdate = {
  generatedAt: string;
  deploymentEnvironment?: string;
  targetCouncil: {
    source: 'latestQueuedCouncil' | 'activeCouncil';
    hash: string;
    memberCount: number;
  };
  pricing: {
    activationGasCost: bigint;
    signatureGasCost: bigint;
    quotedSingleActivationGas: bigint;
    activationBatchMarginalGas: bigint;
    sharedSignatureGasTotal: bigint;
    estimatedWeiPerGas: bigint;
    estimatedMicrogonsPerEth: bigint;
    argonUsdTargetPrice?: bigint;
    ethUsdPrice?: bigint;
    note: string;
  };
  extrinsic: {
    label: string;
    description: string;
    section: string;
    method: string;
    callHex: string;
    sudoCallHex: string;
  };
};

async function main() {
  const inputs = await loadCliInputs(args);
  console.error(
    `[runtime-pricing:prepare] Connecting to Argon RPC ${inputs.argonRpcUrl}`,
  );
  const client = await getClient(inputs.argonRpcUrl);

  try {
    console.error('[runtime-pricing:prepare] Loading current Ethereum council from Argon');
    const targetCouncil = await loadTargetCouncil(client);
    console.error(
      `[runtime-pricing:prepare] Preparing activation pricing for ${targetCouncil.memberCount} shared council signatures`,
    );
    const pricingRecommendation = await loadPricingRecommendation(
      inputs,
      targetCouncil.memberCount,
    );
    const defaults = resolveRuntimeSetupDefaults(inputs.deploymentEnvironment);
    const derivedPricingInputs =
      inputs.estimatedMicrogonsPerEth === undefined
        ? await deriveEstimatedMicrogonsPerEth(client, defaults)
        : undefined;
    const estimatedWeiPerGas =
      inputs.estimatedWeiPerGas ?? (await loadEstimatedWeiPerGas(inputs.executionRpcUrl, defaults));
    const estimatedMicrogonsPerEth =
      inputs.estimatedMicrogonsPerEth ?? derivedPricingInputs?.estimatedMicrogonsPerEth;

    if (estimatedMicrogonsPerEth === undefined) {
      throw new Error(
        'estimated-microgons-per-eth is required unless deployment-environment maps to a known public pricing source',
      );
    }

    console.error('[runtime-pricing:prepare] Building pricing update extrinsic');
    const call = client.tx.crosschainTransfer.setMintingAuthorityActivationRepaymentPricing(
      'Ethereum',
      {
        activationGasCost: pricingRecommendation.activationGasCost.toString(),
        signatureGasCost: pricingRecommendation.signatureGasCost.toString(),
        estimatedWeiPerGas: estimatedWeiPerGas.toString(),
        estimatedMicrogonsPerEth: estimatedMicrogonsPerEth.toString(),
      },
    );
    const sudoCall = client.tx.sudo.sudo(call);
    const plan: PreparedRuntimePricingUpdate = {
      generatedAt: new Date().toISOString(),
      deploymentEnvironment: inputs.deploymentEnvironment,
      targetCouncil,
      pricing: {
        activationGasCost: pricingRecommendation.activationGasCost,
        signatureGasCost: pricingRecommendation.signatureGasCost,
        quotedSingleActivationGas: pricingRecommendation.quotedSingleActivationGas,
        activationBatchMarginalGas: pricingRecommendation.activationBatchMarginalGas,
        sharedSignatureGasTotal: pricingRecommendation.sharedSignatureGasTotal,
        estimatedWeiPerGas,
        estimatedMicrogonsPerEth,
        argonUsdTargetPrice: derivedPricingInputs?.argonUsdTargetPrice,
        ethUsdPrice: derivedPricingInputs?.ethUsdPrice,
        note: pricingRecommendation.note,
      },
      extrinsic: {
        label: 'setMintingAuthorityActivationRepaymentPricing',
        description:
          'Update the runtime-side reimbursement quote inputs for Ethereum minting-authority activation relay.',
        section: call.method.section,
        method: call.method.method,
        callHex: call.method.toHex(),
        sudoCallHex: sudoCall.method.toHex(),
      },
    };

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

async function loadCliInputs(rawArgs: Record<string, string>) {
  const deploymentManifest = getOptionalArg(rawArgs, 'deployment-manifest');
  const deploymentEnvironment =
    getOptionalArg(rawArgs, 'deployment-environment') ??
    (await loadDeploymentEnvironment(deploymentManifest));

  return {
    argonRpcUrl: getRequiredArg(rawArgs, 'argon-rpc-url'),
    executionRpcUrl: getOptionalArg(rawArgs, 'execution-rpc-url'),
    deploymentEnvironment,
    estimatedMicrogonsPerEth: getOptionalBigInt(rawArgs, 'estimated-microgons-per-eth'),
    estimatedWeiPerGas: getOptionalBigInt(rawArgs, 'estimated-wei-per-gas'),
    activationGasCost: getOptionalBigInt(rawArgs, 'activation-gas-cost'),
    signatureGasCost: getOptionalBigInt(rawArgs, 'signature-gas-cost'),
    measureReportPath: getOptionalArg(rawArgs, 'measure-report'),
    outputPath: getOptionalArg(rawArgs, 'output'),
  } satisfies RuntimePricingCliInputs;
}

async function loadDeploymentEnvironment(deploymentManifest?: string) {
  if (!deploymentManifest) return undefined;

  const manifest = JSON.parse(await readFile(deploymentManifest, 'utf8')) as RuntimeSetupManifest;
  return extractRuntimeSetupInputsFromManifest(manifest).networkName;
}

async function loadTargetCouncil(client: ArgonClient) {
  const latestQueuedCouncilHash =
    await client.query.crosschainTransfer.latestQueuedCouncilHashByDestinationChain('Ethereum');
  const activeCouncilHash =
    await client.query.crosschainTransfer.activeGlobalIssuanceCouncilByDestinationChain('Ethereum');
  const targetCouncilHash = latestQueuedCouncilHash.isSome
    ? latestQueuedCouncilHash.unwrap()
    : activeCouncilHash.isSome
      ? activeCouncilHash.unwrap()
      : null;

  if (!targetCouncilHash) {
    throw new Error('Argon runtime does not have an active or queued Ethereum council');
  }

  const targetCouncil =
    await client.query.crosschainTransfer.globalIssuanceCouncilByHash(targetCouncilHash);
  if (targetCouncil.isNone) {
    throw new Error(`Argon runtime is missing council snapshot ${targetCouncilHash.toHex()}`);
  }

  return {
    source: latestQueuedCouncilHash.isSome ? 'latestQueuedCouncil' : 'activeCouncil',
    hash: targetCouncilHash.toHex(),
    memberCount: targetCouncil.unwrap().members.size,
  } as const;
}

async function loadPricingRecommendation(
  inputs: RuntimePricingCliInputs,
  targetSharedSignatureCount: number,
) {
  return (
    (await loadMeasureRecommendation(inputs.measureReportPath, targetSharedSignatureCount)) ??
    loadExplicitPricingRecommendation(inputs) ??
    (await loadGeneratedMeasureRecommendation(targetSharedSignatureCount))
  );
}

async function loadMeasureRecommendation(
  measureReportPath: string | undefined,
  targetSharedSignatureCount: number,
) {
  if (!measureReportPath) return undefined;

  const report = JSON.parse(
    await readFile(measureReportPath, 'utf8'),
  ) as ActivationPricingMeasureReport;
  return parseActivationPricingRecommendation(report, targetSharedSignatureCount);
}

function loadExplicitPricingRecommendation(inputs: RuntimePricingCliInputs) {
  const { activationGasCost, signatureGasCost } = inputs;
  if (activationGasCost === undefined && signatureGasCost === undefined) {
    return undefined;
  }
  if (activationGasCost === undefined || signatureGasCost === undefined) {
    throw new Error(
      'activation-gas-cost and signature-gas-cost must be provided together when overriding pricing directly',
    );
  }

  return {
    activationGasCost,
    signatureGasCost,
    quotedSingleActivationGas: 0n,
    activationBatchMarginalGas: activationGasCost,
    sharedSignatureGasTotal: signatureGasCost,
    note: 'Parsed from explicit CLI overrides.',
  };
}

async function loadGeneratedMeasureRecommendation(targetSharedSignatureCount: number) {
  console.error(
    '[runtime-pricing:prepare] Measuring activation pricing locally because no pricing override was provided',
  );
  const { stdout } = await execFileAsync(
    process.execPath,
    [...process.execArgv, 'measure.ts', '--json'],
    {
      cwd: deployRoot,
      maxBuffer: 1024 * 1024,
    },
  );

  const report = JSON.parse(stdout) as ActivationPricingMeasureReport;
  return parseActivationPricingRecommendation(report, targetSharedSignatureCount);
}

async function loadEstimatedWeiPerGas(
  executionRpcUrl: string | undefined,
  defaults: ReturnType<typeof resolveRuntimeSetupDefaults>,
) {
  const resolvedExecutionRpcUrl = executionRpcUrl?.trim() || defaults?.executionRpcUrl;
  if (!resolvedExecutionRpcUrl) {
    throw new Error(
      'execution-rpc-url is required unless deployment-environment maps to a default public endpoint',
    );
  }

  const executionClient = createPublicClient({ transport: http(resolvedExecutionRpcUrl) });
  return await executionClient.getGasPrice();
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
