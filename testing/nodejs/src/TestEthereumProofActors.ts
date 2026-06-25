import {
  type ArgonClient,
  buildGatewayActivityProofPayload,
  decodeAddress,
  dispatchErrorToString,
  EvmContracts,
  getEthereumBeaconSyncBootstrapTx,
  Keyring,
  type KeyringPair,
  toFixedNumber,
  TxSubmitter,
  U8aFixed,
  Vault,
  Vec,
} from '@argonprotocol/mainchain';
import { privateKeyToAccount } from 'viem/accounts';
import {
  createPublicClient,
  createWalletClient,
  defineChain,
  encodeFunctionData,
  type Hex,
  http,
  toHex,
} from 'viem';
import { getReadyEthereumGatewayUpdates } from './EthereumGatewayQueue';
import {
  mineLaterExecutionAnchorReceipt,
  signGatewayPermit,
  syncEthereumVerifierUntilAnchorCovers,
  toArgonKeccakSignature,
  waitForExecutionReceipt,
  waitForFinalizedBeaconExecutionAtOrAbove,
} from './EthereumE2eUtils';
import TestEthereum from './TestEthereum';
import TestMainchain from './TestMainchain';

const { argonTokenAbi, mintingGatewayAbi, MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE } = EvmContracts;
const MINIMAL_BOOTSTRAP_FINALIZED_SLOT = 64n;

type DeployerAccount = ReturnType<typeof privateKeyToAccount>;
type EthereumChain = ReturnType<typeof defineChain>;
type EthereumPublicClient = {
  getBalance: (args: { address: Hex }) => Promise<bigint>;
  getBlock: (...args: any[]) => Promise<any>;
  getBlockNumber: () => Promise<bigint>;
  readContract: (...args: any[]) => Promise<unknown>;
  waitForTransactionReceipt: (args: {
    hash: Hex;
  }) => Promise<{ status: string; blockNumber: bigint }>;
};
type EthereumWalletClient = {
  sendTransaction: (...args: any[]) => Promise<Hex>;
  writeContract: (...args: any[]) => Promise<Hex>;
};
type GatewayDeployment = Awaited<ReturnType<TestEthereum['deployMintingGatewayFixture']>>;
type RuntimeSetupMode =
  | {
      kind: 'inbound-only';
    }
  | {
      kind: 'outbound';
      activationPricing: {
        activationGasCost: bigint | number;
        signatureGasCost: bigint | number;
        estimatedWeiPerGas: bigint | number;
        estimatedMicrogonsPerEth: bigint | number;
      };
      minimumMintingAuthorityValue?: bigint;
    };

export class EthereumProofE2eHarness {
  public readonly sudoSigner = new Keyring({ type: 'sr25519' }).createFromUri('//Alice');
  public readonly deployer: DeployerAccount;
  public readonly chain: EthereumChain;
  public readonly publicClient: EthereumPublicClient;
  public readonly walletClient: EthereumWalletClient;
  public readonly mainchainClient: ArgonClient;
  public readonly proofRelayer: KeyringPair;

  private constructor(
    public readonly ethereum: TestEthereum,
    public readonly endpoints: Awaited<ReturnType<TestEthereum['launch']>>,
    public readonly mainchain: TestMainchain,
    mainchainClient: ArgonClient,
    deployerPrivateKey: Hex,
    proofRelayerUri: string,
  ) {
    this.mainchainClient = mainchainClient;
    this.deployer = privateKeyToAccount(deployerPrivateKey);
    this.chain = defineChain({
      id: Number.parseInt(endpoints.chainId, 16),
      name: 'argon-test-ethereum',
      nativeCurrency: {
        name: 'Ether',
        symbol: 'ETH',
        decimals: 18,
      },
      rpcUrls: {
        default: {
          http: [endpoints.executionRpcUrl],
        },
      },
    });
    this.publicClient = createPublicClient({
      chain: this.chain,
      transport: http(endpoints.executionRpcUrl),
    });
    this.walletClient = createWalletClient({
      account: this.deployer,
      chain: this.chain,
      transport: http(endpoints.executionRpcUrl),
    });
    this.proofRelayer = new Keyring({ type: 'sr25519' }).createFromUri(proofRelayerUri);
  }

  static async launch(args: {
    testAccount: { address: Hex; balance: string; privateKey: Hex };
    proofRelayerUri: string;
  }) {
    const ethereum = new TestEthereum();
    const endpoints = await ethereum.launch({
      consensusClient: 'lodestar',
      preset: 'minimal',
      secondsPerSlot: 1,
      prefundedAccounts: {
        [args.testAccount.address]: {
          balance: args.testAccount.balance,
        },
      },
    });

    const mainchain = new TestMainchain();
    await mainchain.launch();
    const mainchainClient = await mainchain.client();

    return new EthereumProofE2eHarness(
      ethereum,
      endpoints,
      mainchain,
      mainchainClient,
      args.testAccount.privateKey,
      args.proofRelayerUri,
    );
  }

  async submit(tx: unknown, signer: KeyringPair) {
    const result = await new TxSubmitter(this.mainchainClient, tx as never, signer).submit();
    await result.waitForInFirstBlock;
    return result;
  }

  async sudoSubmit(tx: unknown) {
    const result = await this.submit(
      this.mainchainClient.tx.sudo.sudo(tx as never),
      this.sudoSigner,
    );
    const sudoEvent = result.events.find(event => this.mainchainClient.events.sudo.Sudid.is(event));
    if (!sudoEvent || !this.mainchainClient.events.sudo.Sudid.is(sudoEvent)) {
      throw new Error('sudo did not emit sudo.Sudid');
    }

    const sudoResult = sudoEvent.data[0] as unknown as {
      isErr: boolean;
      asErr: Parameters<typeof dispatchErrorToString>[1];
    };
    if (sudoResult.isErr) {
      throw new Error(
        `sudo failed: ${dispatchErrorToString(this.mainchainClient, sudoResult.asErr)}`,
      );
    }

    return result;
  }

  async syncVerifierThrough(minimumExecutionBlockNumber: bigint) {
    await syncEthereumVerifierUntilAnchorCovers(
      this.mainchainClient,
      this.sudoSigner,
      this.endpoints.beaconApiUrl,
      minimumExecutionBlockNumber,
    );
  }

  async proveGatewayActivity(gatewayAddress: Hex, throughExecutionBlockNumber: bigint) {
    const payload = await buildGatewayActivityProofPayload(this.mainchainClient, {
      executionRpcUrl: this.endpoints.executionRpcUrl,
      gatewayAddress,
      throughExecutionBlockNumber,
    });
    if (!payload) {
      throw new Error('Expected uncovered gateway activity to prove');
    }

    const result = await this.submit(
      this.mainchainClient.tx.crosschainTransfer.proveGatewayActivity(
        'Ethereum',
        payload.previousGatewayActivityNonce,
        payload.proof,
      ),
      this.proofRelayer,
    );

    return { payload, result };
  }

  async configureEthereumRuntime(gateway: TestMintingGateway, mode: RuntimeSetupMode) {
    const calls = [
      this.mainchainClient.tx.crosschainTransfer.setChainConfig('Ethereum', {
        Evm: {
          chainId: BigInt(this.chain.id).toString(),
          gateway: gateway.gatewayAddress,
          argonToken: gateway.argonTokenAddress,
          argonotToken: gateway.argonotTokenAddress,
        },
      }),
    ];

    if (mode.kind === 'outbound') {
      const activationPricing = {
        activationGasCost: BigInt(mode.activationPricing.activationGasCost),
        signatureGasCost: BigInt(mode.activationPricing.signatureGasCost),
        estimatedWeiPerGas: BigInt(mode.activationPricing.estimatedWeiPerGas),
        estimatedMicrogonsPerEth: BigInt(mode.activationPricing.estimatedMicrogonsPerEth),
      };

      calls.push(
        this.mainchainClient.tx.crosschainTransfer.setMintingAuthorityActivationRepaymentPricing(
          'Ethereum',
          {
            activationGasCost: activationPricing.activationGasCost.toString(),
            signatureGasCost: activationPricing.signatureGasCost.toString(),
            estimatedWeiPerGas: activationPricing.estimatedWeiPerGas.toString(),
            estimatedMicrogonsPerEth: activationPricing.estimatedMicrogonsPerEth.toString(),
          },
        ),
      );

      if (mode.minimumMintingAuthorityValue !== undefined) {
        calls.push(
          this.mainchainClient.tx.crosschainTransfer.setMinimumMintingAuthorityValue(
            'Ethereum',
            mode.minimumMintingAuthorityValue.toString(),
          ),
        );
      }
    }

    calls.push(
      await getEthereumBeaconSyncBootstrapTx(this.mainchainClient, this.endpoints.beaconApiUrl),
    );

    const result = await this.sudoSubmit(this.mainchainClient.tx.utility.batchAll(calls as never));
    return { result };
  }

  async fundBurnAccount(amount: bigint) {
    const burnAccount =
      this.mainchainClient.consts.crosschainTransfer.ethereumBurnAccount.toString();

    // For the seeded-gateway tests we need the local burn account to mirror the Ethereum-side
    // migrated circulation exactly, without an extra existential-deposit cushion skewing the
    // gateway circulation check.
    return this.forceSetBalance(burnAccount, amount);
  }

  async fundProofRelayer(
    amount = this.mainchainClient.consts.balances.existentialDeposit.toBigInt() + 1_000_000n,
  ) {
    return this.submit(
      this.mainchainClient.tx.balances.transferAllowDeath(this.proofRelayer.address, amount),
      this.sudoSigner,
    );
  }

  async forceSetBalance(address: string, amount: bigint) {
    return this.sudoSubmit(this.mainchainClient.tx.balances.forceSetBalance(address, amount));
  }

  async forceSetOwnership(address: string, amount: bigint) {
    return this.sudoSubmit(this.mainchainClient.tx.ownership.forceSetBalance(address, amount));
  }

  async waitForExecutionFinalizedAfter(minimumExecutionBlockNumber: bigint) {
    const laterReceipt = await mineLaterExecutionAnchorReceipt(
      this.walletClient,
      this.chain,
      this.ethereum,
      this.deployer,
      minimumExecutionBlockNumber,
    );
    await waitForFinalizedBeaconExecutionAtOrAbove(
      this.ethereum,
      BigInt(laterReceipt.blockNumber),
      {
        // Matches the apps/pr/gateway-proof bootstrap guard for minimal devnets.
        minimumFinalizedSlot: MINIMAL_BOOTSTRAP_FINALIZED_SLOT,
      },
    );
    return laterReceipt;
  }
}

export class TestMintingGateway {
  private constructor(
    public readonly harness: EthereumProofE2eHarness,
    public readonly deployment: GatewayDeployment,
  ) {}

  static async deploy(
    harness: EthereumProofE2eHarness,
    options: Parameters<TestEthereum['deployMintingGatewayFixture']>[0],
  ) {
    const deployment = await harness.ethereum.deployMintingGatewayFixture(options);
    return new TestMintingGateway(harness, deployment);
  }

  get gatewayAddress() {
    return this.deployment.gatewayAddress;
  }

  get argonTokenAddress() {
    return this.deployment.argonTokenAddress;
  }

  get argonotTokenAddress() {
    return this.deployment.argonotTokenAddress;
  }

  async startTransferToArgon(args: {
    account: DeployerAccount;
    amountRuntimeUnits: bigint;
    recipientArgonAddress: Hex;
  }) {
    const permitDeadline = (await this.harness.publicClient.getBlock()).timestamp + 3600n;
    const permitNonce = (await this.harness.publicClient.readContract({
      address: this.argonTokenAddress,
      abi: argonTokenAbi,
      functionName: 'nonces',
      args: [args.account.address],
    })) as bigint;
    const permitSignature = await signGatewayPermit({
      account: args.account,
      chainId: this.harness.chain.id,
      tokenAddress: this.argonTokenAddress,
      gatewayAddress: this.gatewayAddress,
      owner: args.account.address,
      value: args.amountRuntimeUnits * MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
      nonce: permitNonce,
      deadline: permitDeadline,
    });
    const transactionHash = await createWalletClient({
      account: args.account,
      chain: this.harness.chain,
      transport: http(this.harness.endpoints.executionRpcUrl),
    }).sendTransaction({
      account: args.account,
      chain: this.harness.chain,
      to: this.gatewayAddress,
      data: encodeFunctionData({
        abi: mintingGatewayAbi,
        functionName: 'startTransferToArgon',
        args: [
          this.argonTokenAddress,
          args.amountRuntimeUnits,
          args.recipientArgonAddress,
          permitDeadline,
          permitSignature.v,
          permitSignature.r,
          permitSignature.s,
        ],
      }),
    });

    return waitForExecutionReceipt(this.harness.ethereum, transactionHash);
  }

  async forceUpdateActiveCouncil(
    replacementCouncil: { signers: Hex[]; weights: bigint[] },
    nextMicrogonsPerArgonot: bigint,
  ) {
    return this.harness.publicClient.waitForTransactionReceipt({
      hash: await this.harness.walletClient.writeContract({
        account: this.harness.deployer,
        chain: this.harness.chain,
        address: this.gatewayAddress,
        abi: mintingGatewayAbi,
        functionName: 'forceUpdateActiveCouncil',
        args: [replacementCouncil, nextMicrogonsPerArgonot],
      }),
    });
  }

  async relayReadyApprovals(
    batch: Awaited<ReturnType<typeof getReadyEthereumGatewayUpdates>>,
    operatorAddress: string,
  ) {
    return this.harness.publicClient.waitForTransactionReceipt({
      hash: await this.harness.walletClient.writeContract({
        account: this.harness.deployer,
        chain: this.harness.chain,
        address: this.gatewayAddress,
        abi: mintingGatewayAbi,
        functionName: 'applyGatewayUpdates',
        args: [
          batch.currentCouncil,
          batch.updates,
          toHex(decodeAddress(operatorAddress), { size: 32 }),
        ],
      }),
    });
  }

  async argonApprovalsNonce(): Promise<bigint> {
    return (await this.harness.publicClient.readContract({
      address: this.gatewayAddress,
      abi: mintingGatewayAbi,
      functionName: 'argonApprovalsNonce',
    })) as bigint;
  }

  async globalIssuanceCouncil(): Promise<[Hex[], bigint[], Hex]> {
    return (await this.harness.publicClient.readContract({
      address: this.gatewayAddress,
      abi: mintingGatewayAbi,
      functionName: 'globalIssuanceCouncil',
    })) as [Hex[], bigint[], Hex];
  }

  async authorityCollateral(signingKey: Hex): Promise<[bigint, bigint]> {
    return (await this.harness.publicClient.readContract({
      address: this.gatewayAddress,
      abi: mintingGatewayAbi,
      functionName: 'mintingAuthorityCollateralRemaining',
      args: [signingKey],
    })) as [bigint, bigint];
  }

  async finalizeTransferOut(args: {
    transferRequest: {
      argonAccountId: Hex;
      argonTransferNonce: bigint;
      chainId: bigint;
      microgonsPerArgonot: bigint;
      recipient: Hex;
      validUntilBlock: bigint;
      token: Hex;
      amount: bigint;
      mintingAuthorityTip: bigint;
    };
    collateralizationSignature: Hex;
    micronotCollateral: bigint;
  }) {
    return this.harness.publicClient.waitForTransactionReceipt({
      hash: await this.harness.walletClient.writeContract({
        account: this.harness.deployer,
        chain: this.harness.chain,
        address: this.gatewayAddress,
        abi: mintingGatewayAbi,
        functionName: 'finalizeTransferOutOfArgon',
        args: [
          args.transferRequest,
          {
            authorizations: [
              {
                microgonCollateral: 0n,
                micronotCollateral: args.micronotCollateral,
                signature: args.collateralizationSignature,
              },
            ],
          },
        ],
      }),
    });
  }

  async isFinalizedTransferOut(
    transferRequest: Parameters<typeof EvmContracts.hashMintingGatewayTransferOutOfArgonRequest>[0],
  ): Promise<boolean> {
    return (await this.harness.publicClient.readContract({
      address: this.gatewayAddress,
      abi: mintingGatewayAbi,
      functionName: 'finalizedTransferOutOfArgonIds',
      args: [EvmContracts.hashMintingGatewayTransferOutOfArgonRequest(transferRequest)],
    })) as boolean;
  }

  async argonBalance(address: Hex): Promise<bigint> {
    return (await this.harness.publicClient.readContract({
      address: this.argonTokenAddress,
      abi: argonTokenAbi,
      functionName: 'balanceOf',
      args: [address],
    })) as bigint;
  }

  async fundExecutionAccount(address: Hex, value: bigint) {
    return this.harness.publicClient.waitForTransactionReceipt({
      hash: await this.harness.walletClient.sendTransaction({
        account: this.harness.deployer,
        chain: this.harness.chain,
        to: address,
        value,
      }),
    });
  }
}

export class TestMintingAuthorityActor {
  public readonly operator: KeyringPair;
  public readonly councilSigner: DeployerAccount;
  public readonly authoritySigner: DeployerAccount;
  private gateway?: TestMintingGateway;

  constructor(
    public readonly harness: EthereumProofE2eHarness,
    args: {
      operatorUri: string;
      councilPrivateKey: Hex;
      authorityPrivateKey: Hex;
    },
  ) {
    this.operator = new Keyring({ type: 'sr25519' }).createFromUri(args.operatorUri);
    this.councilSigner = privateKeyToAccount(args.councilPrivateKey);
    this.authoritySigner = privateKeyToAccount(args.authorityPrivateKey);
  }

  attachGateway(gateway: TestMintingGateway) {
    this.gateway = gateway;
  }

  async prepareOperator(args: {
    freeBalance: bigint;
    ownershipBalance: bigint;
    committedArgonots: bigint;
    bitcoinXpub: string;
  }) {
    await this.harness.forceSetBalance(this.operator.address, args.freeBalance);
    await this.harness.forceSetOwnership(this.operator.address, args.ownershipBalance);

    const vault = await Vault.create(this.harness.mainchainClient, this.operator, {
      securitization: 1_000_000_000n,
      securitizationRatio: 1,
      annualPercentRate: 0.05,
      baseFee: 0n,
      bitcoinXpub: args.bitcoinXpub,
      treasuryProfitSharing: 0,
      bonusSharingPercent: 0,
    });
    await vault.getVault();

    await this.harness.sudoSubmit(
      this.harness.mainchainClient.tx.priceIndex.setOperator(this.operator.address),
    );

    const currentTick = await this.harness.mainchainClient.query.ticks.currentTick();
    await this.harness.submit(
      this.harness.mainchainClient.tx.priceIndex.submit({
        btcUsdPrice: toFixedNumber(60_000, 18),
        argonotUsdPrice: toFixedNumber(1, 18),
        argonUsdPrice: toFixedNumber(1, 18),
        argonUsdTargetPrice: toFixedNumber(1, 18),
        argonTimeWeightedAverageLiquidity: toFixedNumber(1_000_000, 18),
        tick: currentTick.toBigInt(),
      }),
      this.operator,
    );

    await this.harness.submit(
      this.harness.mainchainClient.tx.vaults.setCommittedArgonots(args.committedArgonots),
      this.operator,
    );
  }

  async registerCouncilSigner() {
    return this.harness.submit(
      this.harness.mainchainClient.tx.crosschainTransfer.registerCouncilSigner(
        'Ethereum',
        this.councilSigner.address,
        toArgonKeccakSignature(
          await this.councilSigner.signMessage({
            message: { raw: toHex(this.registrationMessage('argon/council-signer/v2')) },
          }),
        ),
      ),
      this.operator,
    );
  }

  async forceSingleMemberCouncil() {
    await this.harness.sudoSubmit(
      this.harness.mainchainClient.tx.crosschainTransfer.forceSetGlobalIssuanceCouncil(
        'Ethereum',
        0,
        [this.operator.address],
      ),
    );

    const activeCouncilHashOption =
      await this.harness.mainchainClient.query.crosschainTransfer.activeGlobalIssuanceCouncilByDestinationChain(
        'Ethereum',
      );
    if (activeCouncilHashOption.isNone) {
      throw new Error('Expected active Ethereum council hash');
    }

    const activeCouncilOption =
      await this.harness.mainchainClient.query.crosschainTransfer.globalIssuanceCouncilByHash(
        activeCouncilHashOption.unwrap(),
      );
    if (activeCouncilOption.isNone) {
      throw new Error('Expected active Ethereum council');
    }

    const activeCouncil = activeCouncilOption.unwrap();
    const currentCouncil = [...activeCouncil.members.entries()]
      .map(([signer, member]) => ({
        signer: signer.toHex(),
        weight: member.weight.toBigInt(),
      }))
      .sort((left, right) => left.signer.localeCompare(right.signer));

    return {
      activeCouncilHash: activeCouncilHashOption.unwrap().toHex(),
      activeCouncil,
      currentCouncil: {
        signers: currentCouncil.map(member => member.signer),
        weights: currentCouncil.map(member => member.weight),
      },
    };
  }

  async registerMintingAuthority(micronotCollateral: bigint) {
    return this.harness.submit(
      this.harness.mainchainClient.tx.crosschainTransfer.registerMintingAuthority(
        'Ethereum',
        this.authoritySigner.address,
        toArgonKeccakSignature(
          await this.authoritySigner.signMessage({
            message: {
              raw: toHex(this.registrationMessage('argon/minting-authority-signer/v2')),
            },
          }),
        ),
        0n,
        micronotCollateral,
      ),
      this.operator,
    );
  }

  async approveActivationQueueEntry(queueNonce = 1n) {
    const approvalQueueEntry =
      await this.harness.mainchainClient.query.crosschainTransfer.councilApprovalQueueByDestinationChainAndNonce(
        'Ethereum',
        queueNonce,
      );
    if (approvalQueueEntry.isNone) {
      throw new Error(`Expected queue nonce ${queueNonce} to exist`);
    }

    const councilApprovalSignature = await this.councilSigner.signMessage({
      message: {
        raw: approvalQueueEntry.unwrap().approvalHash.toHex(),
      },
    });
    await this.harness.submit(
      this.harness.mainchainClient.tx.crosschainTransfer.approveQueueEntries(
        'Ethereum',
        new Vec(this.harness.mainchainClient.registry, U8aFixed.with(520), [
          new U8aFixed(
            this.harness.mainchainClient.registry,
            toArgonKeccakSignature(councilApprovalSignature),
            520,
          ),
        ]),
      ),
      this.operator,
    );

    const batch = await getReadyEthereumGatewayUpdates(
      this.harness.mainchainClient,
      this.harness.publicClient,
    );

    return {
      approvalQueueEntry,
      councilApprovalSignature,
      batch,
    };
  }

  async collateralizeFirstPendingTransferOut() {
    const gateway = this.requireGateway();
    const pendingRequests =
      await this.harness.mainchainClient.query.crosschainTransfer.pendingCollateralizationRequestsByChain(
        'Ethereum',
      );
    if (pendingRequests.length === 0) {
      throw new Error('Expected a pending collateralization request');
    }

    const pendingRequest = pendingRequests[0];
    const transferId = pendingRequest.transferId.toHex();
    const transferOption =
      await this.harness.mainchainClient.query.crosschainTransfer.transferOutById(transferId);
    if (transferOption.isNone) {
      throw new Error(`Expected transfer out ${transferId} to exist`);
    }

    const transfer = transferOption.unwrap();
    const transferRequest = {
      argonAccountId: transfer.argonAccountId.toHex(),
      argonTransferNonce: transfer.argonTransferNonce.toBigInt(),
      chainId: BigInt(this.harness.chain.id),
      microgonsPerArgonot: transfer.microgonsPerArgonot.toBigInt(),
      recipient: transfer.destinationAccount.toHex(),
      validUntilBlock: transfer.validUntilEthereumBlock.toBigInt(),
      token: gateway.argonTokenAddress,
      amount: transfer.amount.toBigInt(),
      mintingAuthorityTip: transfer.mintingAuthorityTip.toBigInt(),
    };
    const micronotCollateral = transfer.amount.toBigInt();
    const collateralizationHash = EvmContracts.hashMintingGatewayMintingAuthorization(
      { chainId: BigInt(this.harness.chain.id), gatewayAddress: gateway.gatewayAddress },
      {
        request: transferRequest,
        microgonCollateral: 0n,
        micronotCollateral,
      },
    );
    const collateralizationSignature = await this.authoritySigner.signMessage({
      message: {
        raw: collateralizationHash,
      },
    });

    const result = await this.harness.submit(
      this.harness.mainchainClient.tx.crosschainTransfer.collateralizeTransfer(
        transferId,
        toArgonKeccakSignature(collateralizationSignature),
        0n,
        micronotCollateral,
      ),
      this.operator,
    );

    return {
      pendingRequest,
      transferId,
      transferRequest,
      micronotCollateral,
      collateralizationSignature,
      result,
    };
  }

  private registrationMessage(prefix: string) {
    const prefixBytes = this.harness.mainchainClient.registry.createType('Bytes', prefix).toU8a();
    const destinationChainBytes = this.harness.mainchainClient.registry
      .createType('PalletCrosschainTransferSourceChain', 'Ethereum')
      .toU8a();
    const operatorAccountIdBytes = this.harness.mainchainClient.registry
      .createType('AccountId32', this.operator.address)
      .toU8a();

    return concatBytes(prefixBytes, destinationChainBytes, operatorAccountIdBytes);
  }

  private requireGateway() {
    if (!this.gateway) {
      throw new Error('Minting authority actor requires an attached TestMintingGateway');
    }

    return this.gateway;
  }
}

function concatBytes(...parts: Uint8Array[]) {
  const totalLength = parts.reduce((sum, part) => sum + part.length, 0);
  const bytes = new Uint8Array(totalLength);
  let offset = 0;

  for (const part of parts) {
    bytes.set(part, offset);
    offset += part.length;
  }

  return bytes;
}
