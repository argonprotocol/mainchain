import { getEthereumBeaconSyncBootstrapTx, type ArgonClient } from '@argonprotocol/mainchain';
import {
  deriveMintingAuthorityActivationPricingRecommendation,
  type MintingAuthorityActivationPricingRecommendation,
} from './pricing.js';
import { createPublicClient, getAddress, http, type Address } from 'viem';

export type RuntimeSetupManifest = {
  network?: {
    name?: string;
    chainId?: string;
  };
  initialCouncil?: {
    accounts?: string[];
    councilHash?: string;
    epochMicrogonsPerArgonot?: string;
    signers?: string[];
    totalWeight?: string;
    weights?: string[];
  };
  contracts?: {
    mintingGateway?: {
      address?: string;
    };
    argonToken?: {
      address?: string;
    };
    argonotToken?: {
      address?: string;
    };
  };
};

type RuntimeSetupEnvironment = 'mainnet' | 'testnet';

type RuntimeSetupDefaults = {
  beaconApiUrl: string;
  executionRpcUrl: string;
  runtimeSetupEnvironment: RuntimeSetupEnvironment;
};

export type ActivationPricingMeasureReport = {
  activationPricingMeasurements?: {
    singleActivationGas: string;
    batchActivationGas: string;
    batchActivationCount: number;
    sharedSignatureCount: number;
  };
  activationPricingRecommendation?: {
    activationGasCost: string;
    signatureGasCost: string;
  };
};

export type PrepareCrosschainRuntimeSetupOptions = {
  executionRpcUrl?: string;
  beaconApiUrl?: string;
  deploymentEnvironment?: string;
  gatewayAddress: string;
  argonTokenAddress: string;
  argonotTokenAddress: string;
  estimatedMicrogonsPerEth?: bigint;
  estimatedWeiPerGas?: bigint;
  minimumMintingAuthorityValue?: bigint;
  initialCouncilMemberAccountIds?: string[];
  forceSetGlobalIssuanceCouncilAfterNonce?: bigint;
  pricingRecommendation?: MintingAuthorityActivationPricingRecommendation;
};

export type PreparedRuntimeExtrinsic = {
  label: string;
  description: string;
  section: string;
  method: string;
  callHex: string;
  sudoCallHex: string;
};

export type PreparedCrosschainRuntimeSetup = {
  externalChain: {
    chainId: bigint;
    beaconApiUrl: string;
    executionRpcUrl: string;
    estimatedWeiPerGas: bigint;
    gatewayAddress: Address;
    argonTokenAddress: Address;
    argonotTokenAddress: Address;
    runtimeSetupEnvironment?: RuntimeSetupEnvironment;
  };
  pricing?: {
    activationGasCost: bigint;
    signatureGasCost: bigint;
    estimatedWeiPerGas: bigint;
    estimatedMicrogonsPerEth: bigint;
    argonUsdTargetPrice?: bigint;
    ethUsdPrice?: bigint;
  };
  extrinsics: PreparedRuntimeExtrinsic[];
  batch: {
    label: string;
    description: string;
    callHex: string;
    sudoCallHex: string;
  };
};

type PreparedCall = ReturnType<ArgonClient['tx']['utility']['batchAll']>;
type PreparedCalls = Parameters<ArgonClient['tx']['utility']['batchAll']>[0];

const MICROGONS_PER_ARGON = 1_000_000n;
const USD_FIXED_POINT_SCALE = 10n ** 18n;
const PUBLIC_RUNTIME_SETUP_DEFAULTS: Record<RuntimeSetupEnvironment, RuntimeSetupDefaults> = {
  mainnet: {
    executionRpcUrl: 'https://ethereum-rpc.publicnode.com',
    beaconApiUrl: 'https://ethereum-beacon-api.publicnode.com',
    runtimeSetupEnvironment: 'mainnet',
  },
  testnet: {
    executionRpcUrl: 'https://gnosis-rpc.publicnode.com',
    beaconApiUrl: 'https://gnosis-beacon-api.publicnode.com',
    runtimeSetupEnvironment: 'testnet',
  },
};

export async function prepareCrosschainRuntimeSetup(
  client: ArgonClient,
  options: PrepareCrosschainRuntimeSetupOptions,
): Promise<PreparedCrosschainRuntimeSetup> {
  const defaults = resolveRuntimeSetupDefaults(options.deploymentEnvironment);
  const executionRpcUrl = options.executionRpcUrl?.trim() || defaults?.executionRpcUrl;
  if (!executionRpcUrl) {
    throw new Error(
      'executionRpcUrl is required unless the deployment environment maps to a default public endpoint; local/dev networks must pass it explicitly',
    );
  }

  const beaconApiUrl = options.beaconApiUrl?.trim() || defaults?.beaconApiUrl;
  if (!beaconApiUrl) {
    throw new Error(
      'beaconApiUrl is required unless the deployment environment maps to a default public endpoint; local/dev networks must pass it explicitly',
    );
  }

  const gatewayAddress = getAddress(options.gatewayAddress);
  const argonTokenAddress = getAddress(options.argonTokenAddress);
  const argonotTokenAddress = getAddress(options.argonotTokenAddress);
  const executionClient = createPublicClient({ transport: http(executionRpcUrl) });
  const chainId = BigInt(await executionClient.getChainId());
  const estimatedWeiPerGas = options.estimatedWeiPerGas ?? (await executionClient.getGasPrice());
  const derivedPricingInputs =
    options.pricingRecommendation && options.estimatedMicrogonsPerEth === undefined
      ? await deriveEstimatedMicrogonsPerEth(client, defaults)
      : undefined;

  const extrinsics: PreparedRuntimeExtrinsic[] = [];
  const innerCalls: PreparedCalls = [];
  const crosschainTransfer = client.tx.crosschainTransfer;

  const setChainConfig = crosschainTransfer.setChainConfig('Ethereum', {
    Evm: {
      chainId: chainId.toString(),
      gateway: gatewayAddress,
      argonToken: argonTokenAddress,
      argonotToken: argonotTokenAddress,
    },
  });
  innerCalls.push(setChainConfig);
  extrinsics.push(
    describeExtrinsic(client, setChainConfig, {
      label: 'setChainConfig',
      description: 'Seed the Ethereum gateway and token addresses into runtime chain config.',
    }),
  );

  let pricing: PreparedCrosschainRuntimeSetup['pricing'] | undefined;
  const estimatedMicrogonsPerEth =
    options.estimatedMicrogonsPerEth ?? derivedPricingInputs?.estimatedMicrogonsPerEth;
  if (options.pricingRecommendation || estimatedMicrogonsPerEth !== undefined) {
    if (!options.pricingRecommendation || estimatedMicrogonsPerEth === undefined) {
      throw new Error(
        'pricingRecommendation and estimatedMicrogonsPerEth must be provided together when preparing repayment pricing; local/dev networks should pass estimatedMicrogonsPerEth explicitly',
      );
    }

    pricing = {
      activationGasCost: options.pricingRecommendation.activationGasCost,
      signatureGasCost: options.pricingRecommendation.signatureGasCost,
      estimatedWeiPerGas,
      estimatedMicrogonsPerEth,
      argonUsdTargetPrice: derivedPricingInputs?.argonUsdTargetPrice,
      ethUsdPrice: derivedPricingInputs?.ethUsdPrice,
    };
    const setPricing = crosschainTransfer.setMintingAuthorityActivationRepaymentPricing(
      'Ethereum',
      {
        activationGasCost: pricing.activationGasCost.toString(),
        signatureGasCost: pricing.signatureGasCost.toString(),
        estimatedWeiPerGas: pricing.estimatedWeiPerGas.toString(),
        estimatedMicrogonsPerEth: pricing.estimatedMicrogonsPerEth.toString(),
      },
    );
    innerCalls.push(setPricing);
    extrinsics.push(
      describeExtrinsic(client, setPricing, {
        label: 'setMintingAuthorityActivationRepaymentPricing',
        description:
          'Set the runtime-side reimbursement quote inputs for Ethereum minting-authority activation relay.',
      }),
    );
  }

  if (options.minimumMintingAuthorityValue !== undefined) {
    const setMinimumValue = crosschainTransfer.setMinimumMintingAuthorityValue(
      'Ethereum',
      options.minimumMintingAuthorityValue.toString(),
    );
    innerCalls.push(setMinimumValue);
    extrinsics.push(
      describeExtrinsic(client, setMinimumValue, {
        label: 'setMinimumMintingAuthorityValue',
        description: 'Set the minimum required collateral value for Ethereum minting authorities.',
      }),
    );
  }

  if (options.initialCouncilMemberAccountIds?.length) {
    const forceSetGlobalIssuanceCouncil = crosschainTransfer.forceSetGlobalIssuanceCouncil(
      'Ethereum',
      (options.forceSetGlobalIssuanceCouncilAfterNonce ?? 0n).toString(),
      options.initialCouncilMemberAccountIds,
    );
    innerCalls.push(forceSetGlobalIssuanceCouncil);
    extrinsics.push(
      describeExtrinsic(client, forceSetGlobalIssuanceCouncil, {
        label: 'forceSetGlobalIssuanceCouncil',
        description:
          'Install the initial Ethereum Global Issuance Council on the existing runtime.',
      }),
    );
  }

  const forceCheckpoint = await getEthereumBeaconSyncBootstrapTx(client, beaconApiUrl);
  innerCalls.push(forceCheckpoint);
  extrinsics.push(
    describeExtrinsic(client, forceCheckpoint, {
      label: 'forceCheckpoint',
      description: 'Bootstrap the Ethereum verifier from the target beacon API.',
    }),
  );

  const batchCall = client.tx.utility.batchAll(
    innerCalls as Parameters<typeof client.tx.utility.batchAll>[0],
  );
  const sudoBatchCall = client.tx.sudo.sudo(batchCall);
  return {
    externalChain: {
      chainId,
      beaconApiUrl,
      executionRpcUrl,
      estimatedWeiPerGas,
      gatewayAddress,
      argonTokenAddress,
      argonotTokenAddress,
      runtimeSetupEnvironment: defaults?.runtimeSetupEnvironment,
    },
    pricing,
    extrinsics,
    batch: {
      label: 'batchAll',
      description: 'Submit one sudo-wrapped batch containing all prepared runtime setup calls.',
      callHex: batchCall.method.toHex(),
      sudoCallHex: sudoBatchCall.method.toHex(),
    },
  };
}

export function extractRuntimeSetupInputsFromManifest(manifest: RuntimeSetupManifest) {
  const gatewayAddress = manifest.contracts?.mintingGateway?.address;
  const argonTokenAddress = manifest.contracts?.argonToken?.address;
  const argonotTokenAddress = manifest.contracts?.argonotToken?.address;
  const chainId = manifest.network?.chainId ? BigInt(manifest.network.chainId) : undefined;
  const networkName = manifest.network?.name;

  if (!gatewayAddress || !argonTokenAddress || !argonotTokenAddress) {
    throw new Error(
      'deployment manifest is missing one or more runtime bootstrap addresses: mintingGateway, argonToken, argonotToken',
    );
  }

  return {
    gatewayAddress,
    argonTokenAddress,
    argonotTokenAddress,
    chainId,
    networkName,
    initialCouncilAccounts: manifest.initialCouncil?.accounts?.filter(Boolean),
    initialCouncilHash: manifest.initialCouncil?.councilHash,
    initialCouncilTotalWeight: manifest.initialCouncil?.totalWeight
      ? BigInt(manifest.initialCouncil.totalWeight)
      : undefined,
    initialCouncilWeights: manifest.initialCouncil?.weights?.filter(Boolean).map(BigInt),
    initialCouncilEpochMicrogonsPerArgonot: manifest.initialCouncil?.epochMicrogonsPerArgonot
      ? BigInt(manifest.initialCouncil.epochMicrogonsPerArgonot)
      : undefined,
    initialCouncilSigners: manifest.initialCouncil?.signers?.filter(Boolean),
  };
}

export function parseActivationPricingRecommendation(
  report: ActivationPricingMeasureReport,
): MintingAuthorityActivationPricingRecommendation {
  if (report.activationPricingRecommendation) {
    if (report.activationPricingMeasurements) {
      return deriveMintingAuthorityActivationPricingRecommendation({
        singleActivationGas: BigInt(report.activationPricingMeasurements.singleActivationGas),
        batchActivationGas: BigInt(report.activationPricingMeasurements.batchActivationGas),
        batchActivationCount: report.activationPricingMeasurements.batchActivationCount,
        sharedSignatureCount: report.activationPricingMeasurements.sharedSignatureCount,
      });
    }

    return {
      activationGasCost: BigInt(report.activationPricingRecommendation.activationGasCost),
      signatureGasCost: BigInt(report.activationPricingRecommendation.signatureGasCost),
      quotedSingleActivationGas:
        BigInt(report.activationPricingRecommendation.activationGasCost) +
        BigInt(report.activationPricingRecommendation.signatureGasCost),
      activationBatchMarginalGas: BigInt(report.activationPricingRecommendation.activationGasCost),
      sharedSignatureGasTotal: BigInt(report.activationPricingRecommendation.signatureGasCost),
      note: 'Parsed from gas measure report recommendation. Regenerate the report if the gateway gas surface changes.',
    };
  }

  if (!report.activationPricingMeasurements) {
    throw new Error(
      'Measure report must include activationPricingRecommendation or activationPricingMeasurements',
    );
  }

  return deriveMintingAuthorityActivationPricingRecommendation({
    singleActivationGas: BigInt(report.activationPricingMeasurements.singleActivationGas),
    batchActivationGas: BigInt(report.activationPricingMeasurements.batchActivationGas),
    batchActivationCount: report.activationPricingMeasurements.batchActivationCount,
    sharedSignatureCount: report.activationPricingMeasurements.sharedSignatureCount,
  });
}

function describeExtrinsic(
  client: ArgonClient,
  call: PreparedCall,
  details: {
    label: string;
    description: string;
  },
): PreparedRuntimeExtrinsic {
  const sudoCall = client.tx.sudo.sudo(call);

  return {
    label: details.label,
    description: details.description,
    section: call.method.section,
    method: call.method.method,
    callHex: call.method.toHex(),
    sudoCallHex: sudoCall.method.toHex(),
  };
}

function resolveRuntimeSetupDefaults(deploymentEnvironment?: string) {
  const normalizedDeploymentEnvironment = normalizeRuntimeSetupEnvironment(deploymentEnvironment);
  if (normalizedDeploymentEnvironment) {
    return PUBLIC_RUNTIME_SETUP_DEFAULTS[normalizedDeploymentEnvironment];
  }

  return undefined;
}

function normalizeRuntimeSetupEnvironment(value?: string): RuntimeSetupEnvironment | undefined {
  const normalized = value?.trim().toLowerCase();

  if (normalized === 'mainnet') {
    return 'mainnet';
  }
  if (normalized === 'testnet') {
    return 'testnet';
  }

  return undefined;
}

async function deriveEstimatedMicrogonsPerEth(
  client: ArgonClient,
  defaults?: RuntimeSetupDefaults,
) {
  if (!defaults) {
    throw new Error(
      'Cannot derive estimatedMicrogonsPerEth without a known public runtime setup environment; local/dev networks must provide --estimated-microgons-per-eth explicitly',
    );
  }

  const currentPriceIndex = await client.query.priceIndex.current();
  if (currentPriceIndex.isNone) {
    throw new Error(
      'Cannot derive estimatedMicrogonsPerEth because priceIndex.current is empty on the target Argon chain',
    );
  }

  const argonUsdTargetPrice = currentPriceIndex.unwrap().argonUsdTargetPrice.toBigInt();
  if (argonUsdTargetPrice <= 0n) {
    throw new Error(
      'Cannot derive estimatedMicrogonsPerEth because argonUsdTargetPrice is zero on the target Argon chain',
    );
  }

  const ethUsdPrice = await getCoinbaseSpotPrice('ETH-USD');

  return {
    argonUsdTargetPrice,
    ethUsdPrice,
    estimatedMicrogonsPerEth: (ethUsdPrice * MICROGONS_PER_ARGON) / argonUsdTargetPrice,
  };
}

async function getCoinbaseSpotPrice(pair: 'ETH-USD') {
  const response = await fetch(`https://api.coinbase.com/v2/prices/${pair}/spot`);
  if (!response.ok) {
    throw new Error(
      `Coinbase spot price request failed for ${pair}: ${response.status} ${response.statusText}`,
    );
  }

  const payload = (await response.json()) as {
    data?: {
      amount?: string;
    };
  };
  const amount = payload.data?.amount?.trim();
  if (!amount) {
    throw new Error(`Coinbase spot price response for ${pair} did not include an amount`);
  }

  return parseUsdDecimalToFixedPoint(amount);
}

function parseUsdDecimalToFixedPoint(value: string) {
  const [wholePart, fractionPart = ''] = value.split('.');
  const normalizedWholePart = wholePart.trim();
  const normalizedFractionPart = fractionPart.trim();

  if (!/^\d+$/.test(normalizedWholePart) || !/^\d*$/.test(normalizedFractionPart)) {
    throw new Error(`Invalid USD decimal value: ${value}`);
  }

  const paddedFraction = `${normalizedFractionPart}000000000000000000`.slice(0, 18);
  return BigInt(normalizedWholePart) * USD_FIXED_POINT_SCALE + BigInt(paddedFraction || '0');
}
