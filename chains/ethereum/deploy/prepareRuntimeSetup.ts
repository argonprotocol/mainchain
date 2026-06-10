import { execFile } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import { getClient } from '@argonprotocol/mainchain';
import { deployRoot } from './src/hardhat.js';
import {
  extractRuntimeSetupInputsFromManifest,
  parseActivationPricingRecommendation,
  prepareCrosschainRuntimeSetup,
  type ActivationPricingMeasureReport,
  type PrepareCrosschainRuntimeSetupOptions,
  type RuntimeSetupManifest,
} from './src/runtimeSetup.js';
import { deriveRuntimeCouncilSnapshot } from './src/council.js';
import {
  getOptionalArg,
  getOptionalBigInt,
  getRequiredArg,
  parseArgs,
  stringifyJson,
} from './src/cli.js';

const args = parseArgs(process.argv.slice(2));
const execFileAsync = promisify(execFile);

type RuntimeSetupCliInputs = {
  argonRpcUrl: string;
  outputPath?: string;
  expectedChainId?: bigint;
  manifestCouncilAccounts: string[];
  manifestCouncilSigners: string[];
  prepareOptions: PrepareCrosschainRuntimeSetupOptions;
  verifyCouncil: {
    epochMicrogonsPerArgonot: bigint;
    totalWeight: bigint;
    weights: bigint[];
  };
};

type PricingRecommendationInputs = {
  measureReportPath?: string;
  activationGasCost?: bigint;
  signatureGasCost?: bigint;
  manifestCouncilSigners: string[];
  estimatedMicrogonsPerEth?: bigint;
  estimatedWeiPerGas?: bigint;
};

async function main() {
  const inputs = await loadCliInputs(args);
  const client = await getClient(inputs.argonRpcUrl);
  try {
    inputs.prepareOptions.initialCouncilMemberAccountIds = inputs.manifestCouncilAccounts;
    await assertInitialCouncilMatchesManifest(client, inputs);
    const plan = await prepareCrosschainRuntimeSetup(client, inputs.prepareOptions);

    if (
      inputs.expectedChainId !== undefined &&
      inputs.expectedChainId !== plan.externalChain.chainId
    ) {
      throw new Error(
        `deployment manifest chainId ${inputs.expectedChainId} does not match execution RPC chainId ${plan.externalChain.chainId}`,
      );
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

async function loadCliInputs(rawArgs: Record<string, string>) {
  const manifestInputs = await loadManifestInputs(getRequiredArg(rawArgs, 'deployment-manifest'));
  const gatewayAddress =
    getOptionalArg(rawArgs, 'gateway-address') ?? manifestInputs?.gatewayAddress;
  const argonTokenAddress =
    getOptionalArg(rawArgs, 'argon-token-address') ?? manifestInputs?.argonTokenAddress;
  const argonotTokenAddress =
    getOptionalArg(rawArgs, 'argonot-token-address') ?? manifestInputs?.argonotTokenAddress;
  const manifestCouncilAccounts = getRequiredManifestCouncilAccounts(manifestInputs);
  const manifestCouncilSigners = getRequiredManifestCouncilSigners(manifestInputs);
  const verifyCouncil = getRequiredManifestCouncilVerification(manifestInputs);

  if (!gatewayAddress || !argonTokenAddress || !argonotTokenAddress) {
    throw new Error(
      'gateway-address, argon-token-address, and argonot-token-address are required unless deployment-manifest provides them',
    );
  }

  const estimatedMicrogonsPerEth = getOptionalBigInt(rawArgs, 'estimated-microgons-per-eth');
  const estimatedWeiPerGas = getOptionalBigInt(rawArgs, 'estimated-wei-per-gas');
  const prepareOptions: PrepareCrosschainRuntimeSetupOptions = {
    executionRpcUrl: getOptionalArg(rawArgs, 'execution-rpc-url'),
    beaconApiUrl: getOptionalArg(rawArgs, 'beacon-api-url'),
    deploymentEnvironment: manifestInputs?.networkName,
    gatewayAddress,
    argonTokenAddress,
    argonotTokenAddress,
    estimatedMicrogonsPerEth,
    estimatedWeiPerGas,
    minimumMintingAuthorityValue: getOptionalBigInt(rawArgs, 'minimum-minting-authority-value'),
    forceSetGlobalIssuanceCouncilAfterNonce: getOptionalBigInt(rawArgs, 'force-set-after-nonce'),
    pricingRecommendation: await loadPricingRecommendation({
      measureReportPath: getOptionalArg(rawArgs, 'measure-report'),
      activationGasCost: getOptionalBigInt(rawArgs, 'activation-gas-cost'),
      signatureGasCost: getOptionalBigInt(rawArgs, 'signature-gas-cost'),
      manifestCouncilSigners,
      estimatedMicrogonsPerEth,
      estimatedWeiPerGas,
    }, manifestCouncilSigners.length),
  };

  return {
    argonRpcUrl: getRequiredArg(rawArgs, 'argon-rpc-url'),
    outputPath: getOptionalArg(rawArgs, 'output'),
    expectedChainId: manifestInputs?.chainId,
    manifestCouncilAccounts,
    manifestCouncilSigners,
    prepareOptions,
    verifyCouncil,
  } satisfies RuntimeSetupCliInputs;
}

async function loadManifestInputs(manifestPath: string) {
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8')) as RuntimeSetupManifest;
  return extractRuntimeSetupInputsFromManifest(manifest);
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

function getRequiredManifestCouncilSigners(
  manifestInputs: Awaited<ReturnType<typeof loadManifestInputs>>,
) {
  const signers = manifestInputs?.initialCouncilSigners?.filter(Boolean);
  if (!signers?.length) {
    throw new Error(
      'deployment-manifest must include initialCouncil.signers so bootstrap:prepare-runtime-setup can verify the council summary still matches the target Argon runtime',
    );
  }

  return signers;
}

function getRequiredManifestCouncilAccounts(
  manifestInputs: Awaited<ReturnType<typeof loadManifestInputs>>,
) {
  const accounts = manifestInputs?.initialCouncilAccounts?.filter(Boolean);
  if (!accounts?.length) {
    throw new Error(
      'deployment-manifest must include initialCouncil.accounts so bootstrap:prepare-runtime-setup can force-set the same council member accounts used during bootstrap deploy',
    );
  }

  return accounts;
}

function getRequiredManifestCouncilVerification(
  manifestInputs: Awaited<ReturnType<typeof loadManifestInputs>>,
) {
  if (
    !manifestInputs?.initialCouncilHash ||
    manifestInputs.initialCouncilEpochMicrogonsPerArgonot === undefined ||
    manifestInputs.initialCouncilTotalWeight === undefined ||
    !manifestInputs.initialCouncilWeights?.length
  ) {
    throw new Error(
      'deployment-manifest must include the full initialCouncil summary so bootstrap:prepare-runtime-setup can verify it still matches the target Argon runtime',
    );
  }

  return {
    epochMicrogonsPerArgonot: manifestInputs.initialCouncilEpochMicrogonsPerArgonot,
    totalWeight: manifestInputs.initialCouncilTotalWeight,
    weights: manifestInputs.initialCouncilWeights,
  };
}

async function loadPricingRecommendation(
  inputs: PricingRecommendationInputs,
  targetSharedSignatureCount: number,
) {
  return (
    (await loadMeasureRecommendation(inputs.measureReportPath, targetSharedSignatureCount)) ??
    loadExplicitPricingRecommendation(inputs) ??
    (await loadGeneratedMeasureRecommendation(inputs, targetSharedSignatureCount))
  );
}

async function loadGeneratedMeasureRecommendation(
  inputs: PricingRecommendationInputs,
  targetSharedSignatureCount: number,
) {
  if (!shouldGenerateMeasureRecommendation(inputs)) {
    return undefined;
  }

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

function loadExplicitPricingRecommendation(inputs: PricingRecommendationInputs) {
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

function shouldGenerateMeasureRecommendation(inputs: PricingRecommendationInputs) {
  return (
    !inputs.measureReportPath &&
    inputs.activationGasCost === undefined &&
    inputs.signatureGasCost === undefined &&
    ((inputs.manifestCouncilSigners.length ?? 0) > 0 ||
      inputs.estimatedMicrogonsPerEth !== undefined ||
      inputs.estimatedWeiPerGas !== undefined)
  );
}

async function assertInitialCouncilMatchesManifest(
  client: Awaited<ReturnType<typeof getClient>>,
  inputs: RuntimeSetupCliInputs,
) {
  const runtimeCouncil = await deriveRuntimeCouncilSnapshot(client, inputs.manifestCouncilAccounts);
  const runtimeSigners = runtimeCouncil.members.map(member => member.signer.toLowerCase());
  const manifestSigners = inputs.manifestCouncilSigners.map(signer => signer.toLowerCase());
  const runtimeWeights = runtimeCouncil.members.map(member => member.weight);

  if (
    runtimeSigners.length !== manifestSigners.length ||
    runtimeSigners.some((signer, index) => signer !== manifestSigners[index]) ||
    runtimeWeights.length !== inputs.verifyCouncil.weights.length ||
    runtimeWeights.some((weight, index) => weight !== inputs.verifyCouncil.weights[index]) ||
    runtimeCouncil.totalWeight !== inputs.verifyCouncil.totalWeight ||
    runtimeCouncil.epochMicrogonsPerArgonot !== inputs.verifyCouncil.epochMicrogonsPerArgonot
  ) {
    throw new Error(
      'The target Argon runtime council snapshot no longer matches the council used during bootstrap:deploy; redeploy the Ethereum bootstrap contracts from the current runtime state',
    );
  }
}

void main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
