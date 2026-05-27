import { Keyring } from '@polkadot/keyring';
import { afterAll, describe, expect, it } from 'vitest';
import { EvmContracts } from '../index';
import {
  EthereumProofE2eHarness,
  getReadyEthereumGatewayUpdates,
  TestMintingAuthorityActor,
  TestMintingGateway,
  SKIP_E2E,
  teardown,
  TestEthereum,
} from '@argonprotocol/testing';
import { repeatByteHex, waitForExecutionBlockAtOrAbove } from './ethereumE2eTestUtils';
import { toHex, type Hex } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';

const {
  hashMintingGatewayActivateMintingAuthorityApproval,
  hashMintingGatewayGlobalIssuanceCouncil,
  MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
} = EvmContracts;

type Harness = Awaited<ReturnType<typeof EthereumProofE2eHarness.launch>>;
type AuthorityActor = InstanceType<typeof TestMintingAuthorityActor>;
type Gateway = InstanceType<typeof TestMintingGateway>;
type GatewayUpdateBatch = Awaited<ReturnType<typeof getReadyEthereumGatewayUpdates>>;
type TransferRequest = Awaited<
  ReturnType<AuthorityActor['collateralizeFirstPendingTransferOut']>
>['transferRequest'];

const TEST_ACCOUNT = {
  address: '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
  balance: '100ETH',
  privateKey: '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
} as const;

const TRANSFER_AMOUNT_RUNTIME_UNITS = 10_000n;
const TRANSFER_AMOUNT_BASE_UNITS =
  TRANSFER_AMOUNT_RUNTIME_UNITS * MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE;
const PROOF_RELAYER_URI = '//Charlie';
const ACTIVATION_RELAYER_URI = '//Ferdie';
const QUEUE_RELAY_OPERATOR_URI = '//Dave';
const ROUNDTRIP_ETHEREUM_RECIPIENT_PRIVATE_KEY = repeatByteHex('03', 32);
const TEST_VAULT_XPUB =
  'tpubD8t2diXwgDwRaNt8NNY6pb19U3SwmUzxFhFtSaKb79cfkPqqWX8vSqPzsW2NkhkMsxye6fuB2wNqs5sGTZPpM63UaAb3e69LvNcFpci6JZt';
const QUEUE_RELAY_COUNCIL_PRIVATE_KEY = repeatByteHex('01', 32);
const QUEUE_RELAY_AUTHORITY_PRIVATE_KEY = repeatByteHex('02', 32);

describe.skipIf(SKIP_E2E || !TestEthereum.isInstalled())('Ethereum proof e2e', () => {
  describe.sequential('roundtrip saga', () => {
    let harness!: Harness;
    let authorityActor!: AuthorityActor;
    let gateway!: Gateway;
    const activationRelayer = new Keyring({ type: 'sr25519' }).createFromUri(
      ACTIVATION_RELAYER_URI,
    );
    const outboundSender = new Keyring({ type: 'sr25519' }).createFromUri('//Eve');
    const argonReturnRecipient = new Keyring({ type: 'sr25519' }).createFromUri('//Bob');
    const ethereumRecipient = privateKeyToAccount(ROUNDTRIP_ETHEREUM_RECIPIENT_PRIVATE_KEY);
    let activeCouncilHash = '' as Hex;
    let batch!: GatewayUpdateBatch;
    let councilApprovalSignature = '' as Hex;
    const mintingAuthorityMicronots = 1_000_000n;
    let transferId = '' as Hex;
    let transferRequest!: TransferRequest;
    let collateralizationSignature = '' as Hex;
    const returnAmountRuntimeUnits = 4_000n;
    const activationRepaymentPricing = {
      activationGasCost: 100_000,
      signatureGasCost: 50_000,
      estimatedWeiPerGas: 1_000_000_000,
      estimatedMicrogonsPerEth: 1_000_000,
    };
    const expectedActivationRepaymentDue = 150n;

    afterAll(async () => {
      await teardown();
    });

    it('launches the devnet and configures council, gateway, and queued activation', async () => {
      harness = await EthereumProofE2eHarness.launch({
        testAccount: TEST_ACCOUNT,
        proofRelayerUri: PROOF_RELAYER_URI,
      });
      authorityActor = new TestMintingAuthorityActor(harness, {
        operatorUri: QUEUE_RELAY_OPERATOR_URI,
        councilPrivateKey: QUEUE_RELAY_COUNCIL_PRIVATE_KEY,
        authorityPrivateKey: QUEUE_RELAY_AUTHORITY_PRIVATE_KEY,
      });

      expect(
        await harness.publicClient.getBalance({ address: harness.deployer.address }),
      ).toBeGreaterThan(0n);

      await authorityActor.prepareOperator({
        freeBalance: 3_000_000_000n,
        ownershipBalance: 5_000_000n,
        committedArgonots: 1_000_000n,
        bitcoinXpub: TEST_VAULT_XPUB,
      });
      await authorityActor.registerCouncilSigner();

      const councilSetup = await authorityActor.forceSingleMemberCouncil();
      activeCouncilHash = councilSetup.activeCouncilHash;
      const configuredCouncil = councilSetup.currentCouncil;

      gateway = await TestMintingGateway.deploy(harness, {
        deployerPrivateKey: TEST_ACCOUNT.privateKey,
        initialMicrogonsPerArgonot: councilSetup.activeCouncil.epochMicrogonsPerArgonot.toBigInt(),
      });
      authorityActor.attachGateway(gateway);

      await harness.configureEthereumRuntime(gateway, {
        kind: 'outbound',
        activationPricing: activationRepaymentPricing,
        minimumMintingAuthorityValue: 1n,
      });
      const syncGatewayCouncilReceipt = await gateway.forceUpdateActiveCouncil(
        configuredCouncil,
        councilSetup.activeCouncil.epochMicrogonsPerArgonot.toBigInt(),
      );
      expect(syncGatewayCouncilReceipt.status).toBe('success');

      await authorityActor.registerMintingAuthority(mintingAuthorityMicronots);

      const approval = await authorityActor.approveActivationQueueEntry();
      batch = approval.batch;
      councilApprovalSignature = approval.councilApprovalSignature;

      expect(batch.updates).toHaveLength(1);
      expect(batch.firstQueueNonce).toBe(1n);
      expect(batch.lastQueueNonce).toBe(1n);
      expect(approval.approvalQueueEntry.isSome).toBe(true);

      const contractCouncil = await gateway.globalIssuanceCouncil();
      const expectedGatewayCouncilHash = hashMintingGatewayGlobalIssuanceCouncil({
        signers: configuredCouncil.signers,
        weights: configuredCouncil.weights,
        epochMicrogonsPerArgonot: councilSetup.activeCouncil.epochMicrogonsPerArgonot.toBigInt(),
      });

      expect(expectedGatewayCouncilHash).toBe(activeCouncilHash);
      expect(contractCouncil[2]).toBe(activeCouncilHash);
      expect(
        hashMintingGatewayActivateMintingAuthorityApproval(
          { chainId: BigInt(harness.chain.id), gatewayAddress: gateway.gatewayAddress },
          {
            queueNonce: 1n,
            approvingCouncilHash: activeCouncilHash,
            previousUpdateHash: `0x${'00'.repeat(32)}`,
            target: {
              microgonCollateral: 0n,
              micronotCollateral: mintingAuthorityMicronots,
              signingKey: authorityActor.authoritySigner.address,
            },
          },
        ),
      ).toBe(approval.approvalQueueEntry.unwrap().approvalHash.toHex());
      expect(batch.updates[0]?.signatures[0]?.toLowerCase()).toBe(
        councilApprovalSignature.toLowerCase(),
      );
    }, 420_000);

    it('relays the activation and proves it back to Argon', async () => {
      if (!harness || !authorityActor || !gateway || !batch) {
        throw new Error('roundtrip setup checkpoint did not complete');
      }

      const activeHarness = harness;
      const activationRelayerBefore = await activeHarness.mainchainClient.query.system.account(
        activationRelayer.address,
      );
      const relayReceipt = await gateway.relayReadyApprovals(batch, activationRelayer.address);
      expect(await gateway.argonApprovalsNonce()).toBe(1n);

      const [microgonCollateral, authorityMicronotCollateral] = await gateway.authorityCollateral(
        authorityActor.authoritySigner.address,
      );
      expect(microgonCollateral).toBe(0n);
      expect(authorityMicronotCollateral).toBe(mintingAuthorityMicronots);

      const nextBatch = await getReadyEthereumGatewayUpdates(
        activeHarness.mainchainClient,
        activeHarness.publicClient,
      );
      expect(nextBatch.updates).toEqual([]);
      expect(nextBatch.argonApprovalsNonce).toBe(1n);

      await activeHarness.waitForExecutionFinalizedAfter(relayReceipt.blockNumber);
      await activeHarness.fundProofRelayer();
      await activeHarness.syncVerifierThrough(relayReceipt.blockNumber);

      const { payload, result } = await activeHarness.proveGatewayActivity(
        gateway.gatewayAddress,
        relayReceipt.blockNumber,
      );

      expect(payload.previousGatewayActivityNonce).toBe(0n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
      expect(payload.activities).toHaveLength(1);
      expect(payload.activities[0]?.kind).toBe('MintingAuthorityActivated');
      expect(
        result.events.some(event =>
          activeHarness.mainchainClient.events.crosschainTransfer.MintingAuthorityActivationFinalized.is(
            event,
          ),
        ),
      ).toBe(true);
      expect(
        result.events.some(event =>
          activeHarness.mainchainClient.events.crosschainTransfer.GatewayStateAdvanced.is(event),
        ),
      ).toBe(true);

      const authorityOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.mintingAuthoritiesBySigner(
          authorityActor.authoritySigner.address,
        );
      expect(authorityOption.isSome).toBe(true);
      const authority = authorityOption.unwrap();
      expect(authority.state.isActive).toBe(true);
      expect(
        result.events.some(event =>
          activeHarness.mainchainClient.events.crosschainTransfer.MintingAuthorityActivationCompleted.is(
            event,
          ),
        ),
      ).toBe(true);

      const gatewayStateOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.gatewayStateBySourceChain(
          'Ethereum',
        );
      expect(gatewayStateOption.isSome).toBe(true);
      expect(gatewayStateOption.unwrap().gatewayActivityNonce.toBigInt()).toBe(1n);
      expect(gatewayStateOption.unwrap().argonApprovalsNonce.toBigInt()).toBe(1n);
      const activationRelayerAfter = await activeHarness.mainchainClient.query.system.account(
        activationRelayer.address,
      );
      expect(
        activationRelayerAfter.data.free.toBigInt() - activationRelayerBefore.data.free.toBigInt(),
      ).toBe(expectedActivationRepaymentDue);
    }, 420_000);

    it('transfers Argon to Ethereum, collateralizes it, and proves finalization back', async () => {
      if (!harness || !authorityActor || !gateway) {
        throw new Error('activation checkpoint did not complete');
      }

      const activeHarness = harness;
      const outboundSenderFunding =
        TRANSFER_AMOUNT_RUNTIME_UNITS +
        activeHarness.mainchainClient.consts.balances.existentialDeposit.toBigInt() +
        10_000n;
      await activeHarness.forceSetBalance(outboundSender.address, outboundSenderFunding);
      await activeHarness.submit(
        activeHarness.mainchainClient.tx.crosschainTransfer.transferOut(
          'Ethereum',
          'Argon',
          ethereumRecipient.address,
          TRANSFER_AMOUNT_RUNTIME_UNITS,
        ),
        outboundSender,
      );

      const collateralizedTransfer = await authorityActor.collateralizeFirstPendingTransferOut();
      transferId = collateralizedTransfer.transferId;
      transferRequest = collateralizedTransfer.transferRequest;
      collateralizationSignature = collateralizedTransfer.collateralizationSignature;

      expect(collateralizedTransfer.pendingRequest.remainingCollateral.toBigInt()).toBe(
        TRANSFER_AMOUNT_RUNTIME_UNITS,
      );
      expect(
        collateralizedTransfer.result.events.some(event =>
          activeHarness.mainchainClient.events.crosschainTransfer.TransferOutReady.is(event),
        ),
      ).toBe(true);

      const readyTransferOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.transferOutById(transferId);
      expect(readyTransferOption.isSome).toBe(true);
      expect(readyTransferOption.unwrap().state.isReady).toBe(true);

      const finalizeTransferReceipt = await gateway.finalizeTransferOut({
        transferRequest,
        collateralizationSignature,
        micronotCollateral: collateralizedTransfer.micronotCollateral,
      });
      expect(await gateway.isFinalizedTransferOut(transferRequest)).toBe(true);
      expect(await gateway.argonBalance(ethereumRecipient.address)).toBe(
        TRANSFER_AMOUNT_BASE_UNITS,
      );

      await activeHarness.waitForExecutionFinalizedAfter(finalizeTransferReceipt.blockNumber);
      await activeHarness.syncVerifierThrough(finalizeTransferReceipt.blockNumber);

      const { payload, result } = await activeHarness.proveGatewayActivity(
        gateway.gatewayAddress,
        finalizeTransferReceipt.blockNumber,
      );
      expect(payload.previousGatewayActivityNonce).toBe(1n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
      expect(payload.activities).toHaveLength(1);
      expect(payload.activities[0]?.kind).toBe('TransferOutOfArgonFinalized');
      expect(
        result.events.some(event =>
          activeHarness.mainchainClient.events.crosschainTransfer.TransferOutFinalized.is(event),
        ),
      ).toBe(true);

      const finalizedTransferOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.transferOutById(transferId);
      expect(finalizedTransferOption.isNone).toBe(true);
      expect(
        await activeHarness.mainchainClient.query.crosschainTransfer.pendingCollateralizationRequestsByChain(
          'Ethereum',
        ),
      ).toHaveLength(0);

      const remainingAuthorityOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.mintingAuthoritiesBySigner(
          authorityActor.authoritySigner.address,
        );
      expect(remainingAuthorityOption.isSome).toBe(true);
      expect(remainingAuthorityOption.unwrap().gatewayRemainingMicronotCollateral.toBigInt()).toBe(
        mintingAuthorityMicronots - TRANSFER_AMOUNT_RUNTIME_UNITS,
      );
      expect(remainingAuthorityOption.unwrap().pendingReservedMicronotCollateral.toBigInt()).toBe(
        0n,
      );

      const gatewayStateOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.gatewayStateBySourceChain(
          'Ethereum',
        );
      expect(gatewayStateOption.isSome).toBe(true);
      expect(gatewayStateOption.unwrap().gatewayActivityNonce.toBigInt()).toBe(2n);
    }, 420_000);

    it('transfers from Ethereum back to Argon and settles it', async () => {
      if (!harness || !gateway || !transferRequest) {
        throw new Error('outbound finalization checkpoint did not complete');
      }

      const activeHarness = harness;
      const fundEthereumRecipientResult = await gateway.fundExecutionAccount(
        ethereumRecipient.address,
        10n ** 16n,
      );
      expect(fundEthereumRecipientResult.status).toBe('success');

      const returnTransferRecipientBefore =
        await activeHarness.mainchainClient.query.system.account(argonReturnRecipient.address);
      const returnTransferReceipt = await gateway.startTransferToArgon({
        account: ethereumRecipient,
        amountRuntimeUnits: returnAmountRuntimeUnits,
        recipientArgonAddress: toHex(argonReturnRecipient.publicKey, { size: 32 }),
      });

      await activeHarness.waitForExecutionFinalizedAfter(BigInt(returnTransferReceipt.blockNumber));
      await activeHarness.syncVerifierThrough(BigInt(returnTransferReceipt.blockNumber));

      const { payload, result } = await activeHarness.proveGatewayActivity(
        gateway.gatewayAddress,
        BigInt(returnTransferReceipt.blockNumber),
      );
      expect(payload.previousGatewayActivityNonce).toBe(2n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 3n, end: 3n });
      expect(payload.activities).toHaveLength(1);
      expect(payload.activities[0]?.kind).toBe('TransferToArgonStarted');
      expect(
        result.events.some(event =>
          activeHarness.mainchainClient.events.crosschainTransfer.TransferToArgonSettled.is(event),
        ),
      ).toBe(true);

      const returnTransferRecipientAfter = await activeHarness.mainchainClient.query.system.account(
        argonReturnRecipient.address,
      );
      expect(
        returnTransferRecipientAfter.data.free.toBigInt() -
          returnTransferRecipientBefore.data.free.toBigInt(),
      ).toBe(returnAmountRuntimeUnits);

      const finalGatewayStateOption =
        await activeHarness.mainchainClient.query.crosschainTransfer.gatewayStateBySourceChain(
          'Ethereum',
        );
      expect(finalGatewayStateOption.isSome).toBe(true);
      expect(finalGatewayStateOption.unwrap().gatewayActivityNonce.toBigInt()).toBe(3n);
    }, 420_000);
  });

  describe('committee transition regression', () => {
    afterAll(async () => {
      await teardown();
    });

    it('proves a burn after the first minimal sync committee transition', async () => {
      const harness = await EthereumProofE2eHarness.launch({
        testAccount: TEST_ACCOUNT,
        proofRelayerUri: PROOF_RELAYER_URI,
      });

      const gateway = await TestMintingGateway.deploy(harness, {
        deployerPrivateKey: TEST_ACCOUNT.privateKey,
        seedArgonAmountBaseUnits: TRANSFER_AMOUNT_BASE_UNITS,
        seedArgonRecipient: harness.deployer.address,
      });
      await harness.configureEthereumRuntime(gateway, {
        kind: 'inbound-only',
      });
      const bob = new Keyring({ type: 'sr25519' }).createFromUri('//Bob');

      await waitForExecutionBlockAtOrAbove(harness.publicClient, 73n);

      const burnReceipt = await gateway.startTransferToArgon({
        account: harness.deployer,
        amountRuntimeUnits: TRANSFER_AMOUNT_RUNTIME_UNITS,
        recipientArgonAddress: toHex(bob.publicKey, { size: 32 }),
      });
      const burnBlockNumber = BigInt(burnReceipt.blockNumber);

      await harness.waitForExecutionFinalizedAfter(burnBlockNumber);
      await harness.syncVerifierThrough(burnBlockNumber);
      await harness.fundBurnAccount(TRANSFER_AMOUNT_RUNTIME_UNITS);
      await harness.fundProofRelayer();

      const relayerBefore = await harness.mainchainClient.query.system.account(
        harness.proofRelayer.address,
      );
      const { payload, result } = await harness.proveGatewayActivity(
        gateway.gatewayAddress,
        burnBlockNumber,
      );

      expect(payload.previousGatewayActivityNonce).toBe(0n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
      expect(payload.executionBlockNumberRange).toEqual({
        start: burnBlockNumber,
        end: burnBlockNumber,
      });
      expect(
        result.events.some(event =>
          harness.mainchainClient.events.crosschainTransfer.TransferToArgonSettled.is(event),
        ),
      ).toBe(true);
      expect(
        result.events.some(event =>
          harness.mainchainClient.events.crosschainTransfer.GatewayStateAdvanced.is(event),
        ),
      ).toBe(true);
      expect(result.finalFee ?? 0n).toBe(0n);

      const relayerAfter = await harness.mainchainClient.query.system.account(
        harness.proofRelayer.address,
      );
      expect(relayerAfter.data.free.toBigInt()).toBe(relayerBefore.data.free.toBigInt());
    }, 420_000);
  });
});
