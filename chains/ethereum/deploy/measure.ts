import { Wallet } from 'ethers';
import type { Address, Hex } from 'viem';
import { contractsRoot } from './src/hardhat.js';
import { deriveMintingAuthorityActivationPricingRecommendation } from './src/pricing.js';
import { parseArgs, stringifyJson } from './src/cli.js';
import {
  encodeMintingGatewayGlobalIssuanceCouncilRotateTarget,
  encodeMintingGatewayMintingAuthorityActivationTarget,
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayGlobalIssuanceCouncil,
  hashMintingGatewayMintingAuthorization,
  hashMintingGatewayRotateGlobalIssuanceCouncilApproval,
  MINTING_GATEWAY_UPDATE_KINDS,
  type MintingGatewayCouncilSnapshot,
  type MintingGatewayGatewayUpdate,
  type MintingGatewayGlobalIssuanceCouncilRotateTarget,
  type MintingGatewayHashContext,
  type MintingGatewayMigrationAssetDistribution,
  type MintingGatewayMintingAuthorityActivationTarget,
  type MintingGatewayMintingAuthorization,
  type MintingGatewayTransferOutOfArgonProof,
  type MintingGatewayTransferOutOfArgonRequest,
} from '@argonprotocol/ethereum-contracts/hashing';

const MINTING_GATEWAY_RUNTIME_DECIMALS = 6;
const MINTING_GATEWAY_TOKEN_DECIMALS = 18;
const SCALE = 10n ** BigInt(MINTING_GATEWAY_TOKEN_DECIMALS - MINTING_GATEWAY_RUNTIME_DECIMALS);
const ERC1967_ADMIN_SLOT = '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103';
const COUNCIL_WEIGHTS = [40n, 30n, 20n, 10n] as const;
const MICROGONS_PER_ARGONOT = 1_000_000n;
const LOCAL_GAS_CAP = 16_777_216n;
const args = parseArgs(process.argv.slice(2));
process.chdir(contractsRoot);
const { network } = await import('hardhat');
const connection = await network.create();
const { ethers } = connection;

type SignerLike = {
  address: Address;
  signMessage(message: Uint8Array): Promise<Hex>;
  signTypedData?(
    domain: Record<string, unknown>,
    types: Record<string, Array<{ name: string; type: string }>>,
    value: Record<string, unknown>,
  ): Promise<Hex>;
};

type AccountSigner = SignerLike;

type TransactionReceiptLike = {
  gasUsed: bigint;
};

type TransactionLike = {
  wait(): Promise<TransactionReceiptLike>;
};

type ContractWithDeployment = {
  getAddress(): Promise<Address>;
  waitForDeployment(): Promise<unknown>;
  deploymentTransaction(): TransactionLike | null;
};

type GatewayContract = {
  getAddress(): Promise<Address>;
  connect(signer: AccountSigner): GatewayContract;
  getFunction(name: 'argonApprovalsHash'): () => Promise<Hex>;
  globalIssuanceCouncil(): Promise<{ councilHash: Hex; epochMicrogonsPerArgonot: bigint }>;
  migrate(
    argonMigration: MintingGatewayMigrationAssetDistribution,
    argonotMigration: MintingGatewayMigrationAssetDistribution,
  ): Promise<TransactionLike>;
  startTransferToArgon(
    token: Address,
    amount: bigint,
    argonAccountId: Hex,
    deadline: bigint,
    v: number,
    r: Hex,
    s: Hex,
  ): Promise<TransactionLike>;
  finalizeTransferOutOfArgon(
    request: MintingGatewayTransferOutOfArgonRequest,
    proof: MintingGatewayTransferOutOfArgonProof,
  ): Promise<TransactionLike>;
  cancelTransferOutOfArgon(
    request: MintingGatewayTransferOutOfArgonRequest,
  ): Promise<TransactionLike>;
  applyGatewayUpdates: ((
    currentCouncil: MintingGatewayCouncilSnapshot,
    updates: readonly MintingGatewayGatewayUpdate[],
    relayerArgonAccountId: Hex,
  ) => Promise<TransactionLike>) & {
    estimateGas(
      currentCouncil: MintingGatewayCouncilSnapshot,
      updates: readonly MintingGatewayGatewayUpdate[],
      relayerArgonAccountId: Hex,
    ): Promise<bigint>;
  };
};

type ProxyAdminContract = {
  connect(signer: AccountSigner): ProxyAdminContract;
  upgradeAndCall(proxy: Address, implementation: Address, data: Hex): Promise<TransactionLike>;
};

type TokenContract = ContractWithDeployment & {
  getFunction(name: 'nonces'): (owner: Address) => Promise<bigint>;
};

type ContractFactory<TContract extends ContractWithDeployment> = {
  deploy(...args: unknown[]): Promise<TContract>;
  interface: {
    encodeFunctionData(name: string, args: readonly unknown[]): Hex;
  };
};

type Council = {
  wallets: SignerLike[];
  signers: Address[];
  weights: bigint[];
  memberCount: bigint;
  totalWeight: bigint;
  hash: Hex;
  epochMicrogonsPerArgonot: bigint;
  snapshot: MintingGatewayCouncilSnapshot;
  quorumSigners: SignerLike[];
};

type DeployStack = Awaited<ReturnType<typeof deployGatewayStack>>;

type GasRow = {
  scenario: string;
  measurement: string;
  gas: string;
  weiAt10Gwei: string;
  ethAt10Gwei: string;
  weiAt20Gwei: string;
  ethAt20Gwei: string;
  note: string;
};

type GasMeasurementScenario = {
  scenario: string;
  measurement: string;
  gasUsed: bigint;
  note: string;
};

type GasMeasurementReport = {
  generatedAt: string;
  setupScenarios: GasMeasurementScenario[];
  userScenarios: GasMeasurementScenario[];
  activationPricingMeasurements: {
    singleActivationGas: bigint;
    batchActivationGas: bigint;
    batchActivationCount: number;
    sharedSignatureCount: number;
    oneMemberSingleActivationGas: bigint;
    oneMemberSharedSignatureCount: number;
    smallCouncilSingleActivationGas: bigint;
    smallCouncilSharedSignatureCount: number;
  };
  activationPricingRecommendation: ReturnType<
    typeof deriveMintingAuthorityActivationPricingRecommendation
  >;
};

await main();

async function main() {
  try {
    const report = await measureGasReport();

    if (args.json === 'true') {
      console.log(stringifyJson(report));
      return;
    }

    const setupRows = report.setupScenarios.map(toGasRow);
    const userRows = report.userScenarios.map(toGasRow);

    console.log('\nOne-Time Setup And Council Paths');
    console.table(setupRows);

    console.log('\nUser Paths');
    console.table(userRows);

    console.log('\nNotes');
    console.log('- 100-member council scenarios use equal weights, so quorum needs 90 signatures.');
    console.log('- startTransferToArgon includes ERC-2612 permit and burn in one transaction.');
    console.log('- finalizeTransferOutOfArgon scales with the number of minting authorizations.');
    console.log(
      '- chained queue hashes mainly help when multiple council-approved items share one council segment.',
    );
    console.log(
      '- weiAt10Gwei / weiAt20Gwei and ethAt10Gwei / ethAt20Gwei are gas * gasPrice only.',
    );
    console.log(
      `- recommended activationGasCost: ${formatInteger(report.activationPricingRecommendation.activationGasCost)}`,
    );
    console.log(
      `- recommended signatureGasCost: ${formatInteger(report.activationPricingRecommendation.signatureGasCost)}`,
    );
  } finally {
    await connection.close();
  }
}

async function measureGasReport(): Promise<GasMeasurementReport> {
  const oneCouncil = createCouncil(1);
  const fourCouncil = createCouncil(4);
  const hundredCouncil = createCouncil(100);

  const oneSetup = await deployFixture(oneCouncil);
  const fourSetup = await deployGatewayStack(fourCouncil);
  const hundredSetup = await deployGatewayStack(hundredCouncil);

  const fourUpgradeGas = await getGasUsed(
    fourSetup.proxyAdmin
      .connect(fourSetup.adminSafe)
      .upgradeAndCall(
        await fourSetup.gateway.getAddress(),
        await fourSetup.gatewayFinalImplementation.getAddress(),
        '0x',
      ),
  );
  const hundredUpgradeGas = await getGasUsed(
    hundredSetup.proxyAdmin
      .connect(hundredSetup.adminSafe)
      .upgradeAndCall(
        await hundredSetup.gateway.getAddress(),
        await hundredSetup.gatewayFinalImplementation.getAddress(),
        '0x',
      ),
  );

  await fourSetup.gateway
    .connect(fourSetup.adminSafe)
    .migrate(
      { recipients: [fourSetup.holder.address], amounts: [1_000n * SCALE] },
      { recipients: [fourSetup.holder.address], amounts: [2_000n * SCALE] },
    );
  await hundredSetup.gateway
    .connect(hundredSetup.adminSafe)
    .migrate(
      { recipients: [hundredSetup.holder.address], amounts: [1_000n * SCALE] },
      { recipients: [hundredSetup.holder.address], amounts: [2_000n * SCALE] },
    );
  const hundredBatchSetup = await deployFixture(hundredCouncil);

  const oneActivationGas = await measureMintingAuthorityActivationGas(oneSetup, 1n);
  const fourActivationGas = await measureMintingAuthorityActivationGas(fourSetup, 1n);
  const hundredActivationGas = await measureMintingAuthorityActivationGas(hundredSetup, 1n);
  const hundredActivationBatchGas = await measureMintingAuthorityActivationBatchGas(
    hundredBatchSetup,
    1n,
    3,
  );
  const fourRotationGas = await measureCouncilRotationGas(fourSetup, 2n, createCouncil(4));
  const hundredRotationGas = await measureCouncilRotationGas(hundredSetup, 2n, createCouncil(100));

  return {
    generatedAt: new Date().toISOString(),
    setupScenarios: [
      createGasMeasurementScenario(
        'Proxy deploy + initialize (4 council members)',
        fourSetup.proxyDeployGas,
      ),
      createGasMeasurementScenario(
        'Proxy deploy + initialize (100 council members)',
        hundredSetup.proxyDeployGas,
      ),
      createGasMeasurementScenario(
        'Upgrade to final implementation (4 council members)',
        fourUpgradeGas,
      ),
      createGasMeasurementScenario(
        'Upgrade to final implementation (100 council members)',
        hundredUpgradeGas,
      ),
      createGasMeasurementScenario(
        'Minting authority activation update (1 member, 1 signature)',
        oneActivationGas,
      ),
      createGasMeasurementScenario(
        'Minting authority activation update (4 members, 3 signatures)',
        fourActivationGas,
      ),
      createGasMeasurementScenario(
        'Minting authority activation update (100 members, 90 signatures)',
        hundredActivationGas,
      ),
      createGasMeasurementScenario(
        'Minting authority activation batch (100 members, 3 activations, 90 signatures once)',
        hundredActivationBatchGas,
      ),
      createGasMeasurementScenario(
        'Council rotation update (4 -> 4 members, 3 signatures)',
        fourRotationGas.gas,
        fourRotationGas.measurement,
        fourRotationGas.note,
      ),
      createGasMeasurementScenario(
        'Council rotation update (100 -> 100 members, 90 signatures)',
        hundredRotationGas.gas,
        hundredRotationGas.measurement,
        hundredRotationGas.note,
      ),
    ],
    userScenarios: await measureUserScenarios(),
    activationPricingMeasurements: {
      singleActivationGas: hundredActivationGas,
      batchActivationGas: hundredActivationBatchGas,
      batchActivationCount: 3,
      sharedSignatureCount: hundredCouncil.quorumSigners.length,
      oneMemberSingleActivationGas: oneActivationGas,
      oneMemberSharedSignatureCount: oneCouncil.quorumSigners.length,
      smallCouncilSingleActivationGas: fourActivationGas,
      smallCouncilSharedSignatureCount: fourCouncil.quorumSigners.length,
    },
    activationPricingRecommendation: deriveMintingAuthorityActivationPricingRecommendation({
      singleActivationGas: hundredActivationGas,
      batchActivationGas: hundredActivationBatchGas,
      batchActivationCount: 3,
      sharedSignatureCount: hundredCouncil.quorumSigners.length,
      oneMemberSingleActivationGas: oneActivationGas,
      oneMemberSharedSignatureCount: oneCouncil.quorumSigners.length,
      smallCouncilSingleActivationGas: fourActivationGas,
      smallCouncilSharedSignatureCount: fourCouncil.quorumSigners.length,
    }),
  };
}

async function measureUserScenarios() {
  const argonAccountId = ethers.encodeBytes32String('argon-account-1') as Hex;
  const transferFixture = await deployFixture(createCouncil(4));
  const permitDeadline = BigInt((await ethers.provider.getBlock('latest'))!.timestamp) + 3600n;
  const permit = await signPermit(
    transferFixture.holder,
    transferFixture.argon,
    transferFixture.holder.address,
    await transferFixture.gateway.getAddress(),
    250n * SCALE,
    permitDeadline,
    'Argon',
  );
  const startTransferGas = await getGasUsed(
    transferFixture.gateway
      .connect(transferFixture.holder)
      .startTransferToArgon(
        await transferFixture.argon.getAddress(),
        250n,
        argonAccountId,
        permitDeadline,
        permit.v,
        permit.r as Hex,
        permit.s as Hex,
      ),
  );

  const singleAuthorityFixture = await deployFixture(createCouncil(4));
  const singleAuthority = await activateMintingAuthority(singleAuthorityFixture, 1n, 1);
  const singleRequest = await buildTransferOutOfArgonRequest(singleAuthorityFixture, 1n, 50n);
  const finalizeOneAuthorityGas = await getGasUsed(
    singleAuthorityFixture.gateway
      .connect(singleAuthorityFixture.outsider)
      .finalizeTransferOutOfArgon(singleRequest, {
        authorizations: [
          await buildMintingAuthorization(
            singleAuthorityFixture.gateway,
            singleRequest,
            singleAuthority.signingWallet,
            80n,
            0n,
          ),
        ],
      }),
  );

  const multiAuthorityFixture = await deployFixture(createCouncil(4));
  const authorityOne = await activateMintingAuthority(multiAuthorityFixture, 1n, 1);
  const authorityTwo = await activateMintingAuthority(multiAuthorityFixture, 2n, 2);
  const authorityThree = await activateMintingAuthority(multiAuthorityFixture, 3n, 3);
  const multiRequest = await buildTransferOutOfArgonRequest(multiAuthorityFixture, 1n, 50n);
  const multiAuthorizations = await Promise.all(
    [authorityOne, authorityTwo, authorityThree]
      .sort((left, right) => left.signingWallet.address.localeCompare(right.signingWallet.address))
      .map(authority =>
        buildMintingAuthorization(
          multiAuthorityFixture.gateway,
          multiRequest,
          authority.signingWallet,
          80n,
          0n,
        ),
      ),
  );
  const finalizeThreeAuthorityGas = await getGasUsed(
    multiAuthorityFixture.gateway
      .connect(multiAuthorityFixture.outsider)
      .finalizeTransferOutOfArgon(multiRequest, { authorizations: multiAuthorizations }),
  );

  const cancelFixture = await deployFixture(createCouncil(4));
  const chainId = BigInt((await ethers.provider.getNetwork()).chainId);
  const activeCouncil = await cancelFixture.gateway.globalIssuanceCouncil();
  const expiredRequest: MintingGatewayTransferOutOfArgonRequest = {
    argonAccountId: ethers.encodeBytes32String('account-cancel') as Hex,
    argonTransferNonce: 99n,
    chainId,
    microgonsPerArgonot: activeCouncil.epochMicrogonsPerArgonot,
    recipient: cancelFixture.recipient.address,
    validUntilBlock: 0n,
    token: await cancelFixture.argon.getAddress(),
    amount: 25n,
    mintingAuthorityTip: 1n,
  };
  const cancelGas = await getGasUsed(
    cancelFixture.gateway.connect(cancelFixture.outsider).cancelTransferOutOfArgon(expiredRequest),
  );

  return [
    createGasMeasurementScenario('startTransferToArgon', startTransferGas),
    createGasMeasurementScenario(
      'finalizeTransferOutOfArgon (1 authorization)',
      finalizeOneAuthorityGas,
    ),
    createGasMeasurementScenario(
      'finalizeTransferOutOfArgon (3 authorizations)',
      finalizeThreeAuthorityGas,
    ),
    createGasMeasurementScenario('cancelTransferOutOfArgon', cancelGas),
  ];
}

async function deployFixture(council: Council) {
  const stack = await deployGatewayStack(council);

  await stack.proxyAdmin
    .connect(stack.adminSafe)
    .upgradeAndCall(
      await stack.gateway.getAddress(),
      await stack.gatewayFinalImplementation.getAddress(),
      '0x',
    );

  await stack.gateway
    .connect(stack.adminSafe)
    .migrate(
      { recipients: [stack.holder.address], amounts: [1_000n * SCALE] },
      { recipients: [stack.holder.address], amounts: [2_000n * SCALE] },
    );

  return stack;
}

async function deployGatewayStack(council: Council) {
  const [, adminSafe, guardian, holder, recipient, outsider] =
    (await ethers.getSigners()) as AccountSigner[];

  const gatewayFactory = (await ethers.getContractFactory(
    'MintingGateway',
  )) as unknown as ContractFactory<ContractWithDeployment>;
  const gatewayBootstrapImplementation = await gatewayFactory.deploy(
    ethers.ZeroAddress,
    ethers.ZeroAddress,
  );
  const gatewayBootstrapImplementationTx = requireDeploymentTransaction(
    'MintingGateway bootstrap implementation',
    gatewayBootstrapImplementation.deploymentTransaction(),
  );
  await gatewayBootstrapImplementation.waitForDeployment();

  const initializeData = gatewayFactory.interface.encodeFunctionData('initialize', [
    adminSafe.address,
    guardian.address,
    council.hash,
    council.memberCount,
    council.totalWeight,
    council.epochMicrogonsPerArgonot,
  ]);

  const gatewayProxyFactory = (await ethers.getContractFactory(
    'TransparentUpgradeableProxy',
  )) as unknown as ContractFactory<ContractWithDeployment>;
  const gatewayProxy = await gatewayProxyFactory.deploy(
    await gatewayBootstrapImplementation.getAddress(),
    adminSafe.address,
    initializeData,
  );
  const gatewayProxyTx = requireDeploymentTransaction(
    'TransparentUpgradeableProxy',
    gatewayProxy.deploymentTransaction(),
  );
  await gatewayProxy.waitForDeployment();

  const gateway = (await ethers.getContractAt(
    'MintingGateway',
    await gatewayProxy.getAddress(),
  )) as unknown as GatewayContract;

  const proxyAdminStorage = await ethers.provider.getStorage(
    await gatewayProxy.getAddress(),
    ERC1967_ADMIN_SLOT,
  );
  const proxyAdminAddress = ethers.getAddress(`0x${proxyAdminStorage.slice(-40)}`) as Address;
  const proxyAdmin = (await ethers.getContractAt(
    'ProxyAdmin',
    proxyAdminAddress,
  )) as unknown as ProxyAdminContract;

  const argonFactory = (await ethers.getContractFactory(
    'ArgonToken',
  )) as unknown as ContractFactory<TokenContract>;
  const argonotFactory = (await ethers.getContractFactory(
    'ArgonotToken',
  )) as unknown as ContractFactory<TokenContract>;

  const argon = await argonFactory.deploy(await gatewayProxy.getAddress());
  await argon.waitForDeployment();
  const argonot = await argonotFactory.deploy(await gatewayProxy.getAddress());
  await argonot.waitForDeployment();

  const gatewayFinalImplementation = await gatewayFactory.deploy(
    await argon.getAddress(),
    await argonot.getAddress(),
  );
  await gatewayFinalImplementation.waitForDeployment();

  return {
    adminSafe,
    guardian,
    holder,
    recipient,
    outsider,
    council,
    gateway,
    proxyAdmin,
    argon,
    argonot,
    gatewayFinalImplementation,
    proxyDeployGas: (await gatewayProxyTx.wait()).gasUsed,
    bootstrapImplementationDeployGas: (await gatewayBootstrapImplementationTx.wait()).gasUsed,
  };
}

async function activateMintingAuthority(
  fixture: DeployStack,
  queueNonce: bigint,
  authorityNumber: number,
) {
  const signingWallet = Wallet.createRandom() as unknown as SignerLike;
  const target = {
    microgonCollateral: 1_000n,
    micronotCollateral: 200n,
    signingKey: signingWallet.address,
  } satisfies MintingGatewayMintingAuthorityActivationTarget;
  const previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
  const approvalHash = hashMintingGatewayActivateMintingAuthorityApproval(
    await getGatewayHashContext(fixture.gateway),
    {
      queueNonce,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash,
      target,
    },
  );
  const signatures = await signApprovalHash(fixture.council.quorumSigners, approvalHash);
  const payload = encodeMintingGatewayMintingAuthorityActivationTarget(target);

  await fixture.gateway.applyGatewayUpdates(
    fixture.council.snapshot,
    [
      {
        queueNonce,
        kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
        payload,
        signatures,
      },
    ],
    ethers.encodeBytes32String(`relayer-${authorityNumber}`) as Hex,
  );

  return {
    signingWallet,
  };
}

async function measureMintingAuthorityActivationGas(fixture: DeployStack, queueNonce: bigint) {
  const signingWallet = Wallet.createRandom() as unknown as SignerLike;
  const target = {
    microgonCollateral: 1_000n,
    micronotCollateral: 200n,
    signingKey: signingWallet.address,
  } satisfies MintingGatewayMintingAuthorityActivationTarget;
  const previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
  const approvalHash = hashMintingGatewayActivateMintingAuthorityApproval(
    await getGatewayHashContext(fixture.gateway),
    {
      queueNonce,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash,
      target,
    },
  );
  const signatures = await signApprovalHash(fixture.council.quorumSigners, approvalHash);
  const payload = encodeMintingGatewayMintingAuthorityActivationTarget(target);

  return getGasUsed(
    fixture.gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      [
        {
          queueNonce,
          kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
          payload,
          signatures,
        },
      ],
      ethers.encodeBytes32String('relayer-activation') as Hex,
    ),
  );
}

async function measureMintingAuthorityActivationBatchGas(
  fixture: DeployStack,
  startQueueNonce: bigint,
  activationCount: number,
) {
  let previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
  const updates: MintingGatewayGatewayUpdate[] = [];
  const hashContext = await getGatewayHashContext(fixture.gateway);

  for (let index = 0; index < activationCount; ++index) {
    const queueNonce = startQueueNonce + BigInt(index);
    const target = {
      microgonCollateral: 1_000n,
      micronotCollateral: 200n,
      signingKey: (Wallet.createRandom() as unknown as SignerLike).address,
    } satisfies MintingGatewayMintingAuthorityActivationTarget;
    const approvalHash = hashMintingGatewayActivateMintingAuthorityApproval(hashContext, {
      queueNonce,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash,
      target,
    });
    const signatures =
      index + 1 === activationCount
        ? await signApprovalHash(fixture.council.quorumSigners, approvalHash)
        : [];
    const payload = encodeMintingGatewayMintingAuthorityActivationTarget(target);

    updates.push({
      queueNonce,
      kind: MINTING_GATEWAY_UPDATE_KINDS.mintingAuthorityActivate,
      payload,
      signatures,
    });
    previousUpdateHash = approvalHash;
  }

  return getGasUsed(
    fixture.gateway.applyGatewayUpdates(
      fixture.council.snapshot,
      updates,
      ethers.encodeBytes32String('relayer-activation-batch') as Hex,
    ),
  );
}

async function measureCouncilRotationGas(
  fixture: DeployStack,
  queueNonce: bigint,
  nextCouncil: Council,
) {
  const previousUpdateHash = await fixture.gateway.getFunction('argonApprovalsHash')();
  const approvalHash = hashMintingGatewayRotateGlobalIssuanceCouncilApproval(
    await getGatewayHashContext(fixture.gateway),
    {
      queueNonce,
      approvingCouncilHash: fixture.council.hash,
      previousUpdateHash,
      target: {
        council: nextCouncil.snapshot,
        epochMicrogonsPerArgonot: nextCouncil.epochMicrogonsPerArgonot,
      },
    },
  );
  const signatures = await signApprovalHash(fixture.council.quorumSigners, approvalHash);
  const payload = encodeMintingGatewayGlobalIssuanceCouncilRotateTarget({
    council: nextCouncil.snapshot,
    epochMicrogonsPerArgonot: nextCouncil.epochMicrogonsPerArgonot,
  } satisfies MintingGatewayGlobalIssuanceCouncilRotateTarget);

  let estimate: bigint;

  try {
    estimate = await fixture.gateway.applyGatewayUpdates.estimateGas(
      fixture.council.snapshot,
      [
        {
          queueNonce,
          kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
          payload,
          signatures,
        },
      ],
      ethers.encodeBytes32String('relayer-rotation') as Hex,
    );
  } catch (error) {
    const gasCapError = parseGasCapError(error);
    if (!gasCapError) throw error;

    return {
      gas: gasCapError.estimatedGas,
      measurement: 'estimate',
      note: `exceeds local ${formatInteger(gasCapError.gasCap)} gas cap`,
    };
  }

  if (estimate > LOCAL_GAS_CAP) {
    return {
      gas: estimate,
      measurement: 'estimate',
      note: `exceeds local ${formatInteger(LOCAL_GAS_CAP)} gas cap`,
    };
  }

  return {
    gas: await getGasUsed(
      fixture.gateway.applyGatewayUpdates(
        fixture.council.snapshot,
        [
          {
            queueNonce,
            kind: MINTING_GATEWAY_UPDATE_KINDS.globalIssuanceCouncilRotate,
            payload,
            signatures,
          },
        ],
        ethers.encodeBytes32String('relayer-rotation') as Hex,
      ),
    ),
    measurement: 'actual',
    note: '',
  };
}

async function buildTransferOutOfArgonRequest(
  fixture: DeployStack,
  transferNonce: bigint,
  amount: bigint,
): Promise<MintingGatewayTransferOutOfArgonRequest> {
  const chainId = BigInt((await ethers.provider.getNetwork()).chainId);
  const activeCouncil = await fixture.gateway.globalIssuanceCouncil();

  return {
    argonAccountId: ethers.encodeBytes32String(`account-${transferNonce}`) as Hex,
    argonTransferNonce: transferNonce,
    chainId,
    microgonsPerArgonot: activeCouncil.epochMicrogonsPerArgonot,
    recipient: fixture.recipient.address,
    validUntilBlock: 1_000_000n,
    token: await fixture.argon.getAddress(),
    amount,
    mintingAuthorityTip: 5n,
  };
}

async function buildMintingAuthorization(
  gateway: { getAddress(): Promise<Address> },
  request: MintingGatewayTransferOutOfArgonRequest,
  signingWallet: SignerLike,
  microgonCollateral: bigint,
  micronotCollateral: bigint,
): Promise<MintingGatewayMintingAuthorization> {
  const authorizationHash = hashMintingGatewayMintingAuthorization(
    await getGatewayHashContext(gateway),
    {
      request,
      microgonCollateral,
      micronotCollateral,
    },
  );
  const signature = await signingWallet.signMessage(ethers.getBytes(authorizationHash));

  return { microgonCollateral, micronotCollateral, signature };
}

async function signPermit(
  signer: SignerLike,
  token: {
    getAddress(): Promise<Address>;
    getFunction(name: 'nonces'): (owner: Address) => Promise<bigint>;
  },
  owner: Address,
  spender: Address,
  value: bigint,
  deadline: bigint,
  name: string,
) {
  if (!signer.signTypedData) {
    throw new Error('Signer does not support signTypedData');
  }

  const { chainId } = await ethers.provider.getNetwork();
  const nonce = await token.getFunction('nonces')(owner);
  const signature = await signer.signTypedData(
    {
      name,
      version: '1',
      chainId,
      verifyingContract: await token.getAddress(),
    },
    {
      Permit: [
        { name: 'owner', type: 'address' },
        { name: 'spender', type: 'address' },
        { name: 'value', type: 'uint256' },
        { name: 'nonce', type: 'uint256' },
        { name: 'deadline', type: 'uint256' },
      ],
    },
    {
      owner,
      spender,
      value,
      nonce,
      deadline,
    },
  );

  return ethers.Signature.from(signature);
}

async function signApprovalHash(signers: SignerLike[], approvalHash: Hex) {
  return Promise.all(signers.map(signer => signer.signMessage(ethers.getBytes(approvalHash))));
}

function createCouncil(memberCount: number): Council {
  const wallets = Array.from(
    { length: memberCount },
    () => Wallet.createRandom() as unknown as SignerLike,
  ).sort((left, right) => left.address.toLowerCase().localeCompare(right.address.toLowerCase()));
  const weights =
    memberCount === 4 ? [...COUNCIL_WEIGHTS] : Array.from({ length: memberCount }, () => 1n);
  const signers = wallets.map(wallet => wallet.address);
  const totalWeight = weights.reduce((sum, weight) => sum + weight, 0n);
  const quorumCount = getQuorumCount(weights);

  return {
    wallets,
    signers,
    weights,
    memberCount: BigInt(memberCount),
    totalWeight,
    hash: hashMintingGatewayGlobalIssuanceCouncil({
      signers,
      weights,
      epochMicrogonsPerArgonot: MICROGONS_PER_ARGONOT,
    }),
    epochMicrogonsPerArgonot: MICROGONS_PER_ARGONOT,
    snapshot: { signers, weights },
    quorumSigners: wallets.slice(0, quorumCount),
  };
}

async function getGatewayHashContext(gateway: {
  getAddress(): Promise<Address>;
}): Promise<MintingGatewayHashContext> {
  const { chainId } = await ethers.provider.getNetwork();

  return {
    chainId,
    gatewayAddress: await gateway.getAddress(),
  };
}

function getQuorumCount(weights: bigint[]) {
  const totalWeight = weights.reduce((sum, weight) => sum + weight, 0n);
  let signerWeight = 0n;

  for (let index = 0; index < weights.length; ++index) {
    signerWeight += weights[index] ?? 0n;
    const signerCount = index + 1;
    const unsignedMemberCount = weights.length - signerCount;

    if (signerWeight * 100n >= totalWeight * 90n) return signerCount;
    if (unsignedMemberCount <= 2 && signerWeight * 100n >= totalWeight * 80n) {
      return signerCount;
    }
  }

  throw new Error('Unable to find quorum count');
}

async function getGasUsed(action: Promise<TransactionLike>) {
  const tx = await action;
  const receipt = await tx.wait();
  return receipt.gasUsed;
}

function createGasMeasurementScenario(
  scenario: string,
  gasUsed: bigint,
  measurement = 'actual',
  note = '',
): GasMeasurementScenario {
  return { scenario, measurement, gasUsed, note };
}

function toGasRow(measurement: GasMeasurementScenario): GasRow {
  return {
    scenario: measurement.scenario,
    measurement: measurement.measurement,
    gas: formatInteger(measurement.gasUsed),
    weiAt10Gwei: formatWeiAtGwei(measurement.gasUsed, 10n),
    ethAt10Gwei: formatEthAtGwei(measurement.gasUsed, 10n),
    weiAt20Gwei: formatWeiAtGwei(measurement.gasUsed, 20n),
    ethAt20Gwei: formatEthAtGwei(measurement.gasUsed, 20n),
    note: measurement.note,
  };
}

function formatWeiAtGwei(gasUsed: bigint, gwei: bigint) {
  return formatInteger(gasUsed * gwei * 1_000_000_000n);
}

function formatEthAtGwei(gasUsed: bigint, gwei: bigint) {
  const wei = gasUsed * gwei * 1_000_000_000n;
  const whole = wei / 1_000_000_000_000_000_000n;
  const fraction = (wei % 1_000_000_000_000_000_000n).toString().padStart(18, '0').slice(0, 6);
  return `${whole}.${fraction} ETH`;
}

function formatInteger(value: bigint) {
  return value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

function parseGasCapError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const match = message.match(/transaction gas limit \((\d+)\) is greater than the cap \((\d+)\)/);
  if (!match) return null;

  return {
    estimatedGas: BigInt(match[1]),
    gasCap: BigInt(match[2]),
  };
}

function requireDeploymentTransaction(
  contractName: string,
  transaction: TransactionLike | null,
): TransactionLike {
  if (!transaction) {
    throw new Error(`Missing deployment transaction for ${contractName}`);
  }

  return transaction;
}
