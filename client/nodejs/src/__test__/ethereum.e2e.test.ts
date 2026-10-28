import { Keyring } from '@polkadot/keyring';
import { afterAll, describe, expect, it } from 'vitest';
import {
  buildEthereumEventProof,
  buildGatewayActivityReceiptProofPayloads,
  buildGatewayActivityStorageProofs,
  captureGatewayActivityStorageProofs,
  type CapturedGatewayActivityStorageProof,
  discoverMissingGatewayActivityLocators,
  EvmContracts,
  getLatestArgonFinalizedExecutionHeader,
} from '../index';
import {
  EthereumProofE2eHarness,
  getReadyEthereumGatewayUpdates,
  TestMintingAuthorityActor,
  TestMintingGateway,
  SKIP_E2E,
  teardown,
  TestEthereum,
  waitForExecutionReceipt,
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
// @ts-expect-error private constructor
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

// Partial returns leave at least the existential deposit; isolate proofs from dust handling.
const TRANSFER_AMOUNT_RUNTIME_UNITS = 100_000n;
const TRANSFER_AMOUNT_BASE_UNITS =
  TRANSFER_AMOUNT_RUNTIME_UNITS * MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE;
const PROOF_RELAYER_URI = '//Charlie';
const ACTIVATION_RELAYER_URI = '//Ferdie';
const QUEUE_RELAY_OPERATOR_URI = '//Dave';
const ROUNDTRIP_ETHEREUM_RECIPIENT_PRIVATE_KEY = repeatByteHex('03', 32);
const TEST_VAULT_XPUB_BYTES_HEX =
  '0x043587cf015436d724800000008f5e2b22b8e08d61a920bc2006ccd532b6e1304ce07b3a28d86c3595db6fed2303ca6a577de236ac2477e0fc7b6e93ba5df2e4556845952446645114d002c4add2';
const QUEUE_RELAY_COUNCIL_PRIVATE_KEY = repeatByteHex('01', 32);
const QUEUE_RELAY_AUTHORITY_PRIVATE_KEY = repeatByteHex('02', 32);

describe.skipIf(SKIP_E2E || !TestEthereum.isInstalled())('Ethereum proof e2e', () => {
  describe('roundtrip saga', () => {
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
    const returnAmountRuntimeUnits = 40_000n;
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
        bitcoinXpubBytesHex: TEST_VAULT_XPUB_BYTES_HEX,
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

    it('rejects a proven admin receipt without pausing, then proves activation with receipts', async () => {
      if (!harness || !authorityActor || !gateway || !batch) {
        throw new Error('roundtrip setup checkpoint did not complete');
      }

      const activeHarness = harness;
      const activationRelayerBefore = await activeHarness.mainchainClient.query.system.account(
        activationRelayer.address,
      );
      const relayReceipt = await gateway.relayReadyApprovals(batch, activationRelayer.address);
      expect(await gateway.argonApprovalsNonce()).toBe(1n);

      const guardianReceipt = await waitForExecutionReceipt(
        activeHarness.ethereum,
        await activeHarness.walletClient.writeContract({
          account: activeHarness.deployer,
          chain: activeHarness.chain,
          address: gateway.gatewayAddress,
          abi: EvmContracts.mintingGatewayAbi,
          functionName: 'setGuardian',
          args: [activeHarness.deployer.address],
        }),
      );
      expect(guardianReceipt.status).toBe('success');
      expect(guardianReceipt.logs).toHaveLength(1);

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

      await activeHarness.waitForExecutionFinalizedAfter(guardianReceipt.blockNumber);
      await activeHarness.fundProofRelayer();
      await activeHarness.syncVerifierThrough(guardianReceipt.blockNumber);

      const anchor = await getLatestArgonFinalizedExecutionHeader(activeHarness.mainchainClient);
      const adminProof = await buildEthereumEventProof(
        { executionRpcUrl: activeHarness.endpoints.executionRpcUrl },
        anchor,
        [
          [
            {
              txHash: guardianReceipt.transactionHash,
              logIndexes: [guardianReceipt.logs[0].logIndex],
            },
          ],
        ],
      );
      // This authentic receipt contains no journal nonce and must not stop honest relayers.
      for (let attempt = 0; attempt < 2; attempt++) {
        await expect(
          activeHarness.submit(
            activeHarness.mainchainClient.tx.crosschainTransfer.proveGatewayActivity(
              'Ethereum',
              0n,
              adminProof,
            ),
            activeHarness.proofRelayer,
          ),
        ).rejects.toThrow('InvalidGatewayActivity');
        const pause =
          await activeHarness.mainchainClient.query.crosschainTransfer.gatewaySyncPauseBySourceChain(
            'Ethereum',
          );
        const state =
          await activeHarness.mainchainClient.query.crosschainTransfer.gatewayStateBySourceChain(
            'Ethereum',
          );
        expect(pause.isNone).toBe(true);
        expect(state.isNone).toBe(true);
        if (attempt === 0) {
          await activeHarness.sudoSubmit(
            activeHarness.mainchainClient.tx.crosschainTransfer.unpauseGateway('Ethereum'),
          );
        }
      }
      const proofOptions = {
        executionRpcUrl: activeHarness.endpoints.executionRpcUrl,
        gatewayAddress: gateway.gatewayAddress,
        locators: await discoverMissingGatewayActivityLocators({
          executionRpcUrl: activeHarness.endpoints.executionRpcUrl,
          gatewayAddress: gateway.gatewayAddress,
          finalizedExecutionBlockNumber: anchor.blockNumber,
          minimumGatewayActivityNonce: 1n,
        }),
      };
      const captures = await captureGatewayActivityStorageProofs({
        ...proofOptions,
        executionBlock: await activeHarness.publicClient.getBlock({
          blockNumber: anchor.blockNumber,
        }),
      });
      const storage = await buildGatewayActivityStorageProofs(activeHarness.mainchainClient, {
        ...proofOptions,
        locators: captures,
      });
      expect(storage.payloads).toHaveLength(1);
      expect(
        storage.payloads[0].proof.storageProof.executionBlockProof.targetToAnchorHeaderChain,
      ).toEqual([]);
      const { payloads } = await buildGatewayActivityReceiptProofPayloads(
        activeHarness.mainchainClient,
        proofOptions,
      );

      expect(payloads).toHaveLength(1);
      const payload = payloads[0];
      const result = await activeHarness.submit(
        activeHarness.mainchainClient.tx.crosschainTransfer.proveGatewayActivity(
          'Ethereum',
          payload.previousGatewayActivityNonce,
          payload.proof,
        ),
        activeHarness.proofRelayer,
      );
      expect(result.finalFee ?? 0n).toBe(0n);
      expect(payload.previousGatewayActivityNonce).toBe(0n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
      expect(payload.activities).toHaveLength(1);
      expect(payload.activities[0]?.kind).toBe('MintingAuthorityActivated');
      expect(payload.proof.blocks).toHaveLength(1);
      expect(payload.proof.blocks[0].receiptLogs).toHaveLength(1);
      // Another signer rules out a stale account nonce as the rejection reason.
      const relayerAfterReceipt = await activeHarness.mainchainClient.query.system.account(
        outboundSender.address,
      );
      expect(relayerAfterReceipt.nonce.toBigInt()).toBe(0n);
      await expect(
        activeHarness.submit(
          activeHarness.mainchainClient.tx.crosschainTransfer.proveGatewayActivityStorage(
            'Ethereum',
            storage.payloads[0].previousGatewayActivityNonce,
            storage.payloads[0].proof,
          ),
          outboundSender,
        ),
      ).rejects.toThrow(/outdated/i);
      const relayerAfterStaleProof = await activeHarness.mainchainClient.query.system.account(
        outboundSender.address,
      );
      expect(relayerAfterStaleProof.data.free.toBigInt()).toBe(
        relayerAfterReceipt.data.free.toBigInt(),
      );
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

      const { payloads, results } = await activeHarness.proveGatewayActivities(
        gateway.gatewayAddress,
        finalizeTransferReceipt.blockNumber,
      );
      const payload = payloads.at(-1)!;
      const result = results.at(-1)!;
      expect(payloads).toHaveLength(1);
      expect(results).toHaveLength(1);
      expect(payload.previousGatewayActivityNonce).toBe(1n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 2n, end: 2n });
      expect(payload.activities).toHaveLength(1);
      expect(payload.activities[0]?.kind).toBe('TransferOutOfArgonFinalized');
      expect(payload.proof.locatorIndex).toBeGreaterThan(0n);
      expect(payload.proof.activityLogs).toHaveLength(1);
      expect(payload.proof.storageProof.slots).toHaveLength(2);
      expect(payload.proof.storageProof.executionBlockProof.targetToAnchorHeaderChain).toEqual([]);
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

    it('relays cached locator proofs through the nearest retained anchors and rejects tampered witnesses', async () => {
      if (!harness || !gateway || !transferRequest) {
        throw new Error('outbound finalization checkpoint did not complete');
      }

      const activeHarness = harness;
      const client = activeHarness.mainchainClient;
      const fundEthereumRecipientResult = await gateway.fundExecutionAccount(
        ethereumRecipient.address,
        10n ** 16n,
      );
      expect(fundEthereumRecipientResult.status).toBe('success');

      const recipientBeforeStorageSettlement = await client.query.system.account(
        argonReturnRecipient.address,
      );
      const firstReturnTransferReceipt = await gateway.startTransferToArgon({
        account: ethereumRecipient,
        amountRuntimeUnits: returnAmountRuntimeUnits,
        recipientArgonAddress: toHex(argonReturnRecipient.publicKey, { size: 32 }),
      });
      const secondReturnTransferReceipt = await gateway.startTransferToArgon({
        account: ethereumRecipient,
        amountRuntimeUnits: returnAmountRuntimeUnits,
        recipientArgonAddress: toHex(argonReturnRecipient.publicKey, { size: 32 }),
      });
      expect(firstReturnTransferReceipt.status).toBe('success');
      expect(secondReturnTransferReceipt.status).toBe('success');
      expect(secondReturnTransferReceipt.transactionHash).not.toBe(
        firstReturnTransferReceipt.transactionHash,
      );
      expect(secondReturnTransferReceipt.blockNumber).toBeGreaterThan(
        firstReturnTransferReceipt.blockNumber,
      );

      // Capture finalized witnesses before Argon has an anchor that can authenticate them.
      await activeHarness.waitForExecutionFinalizedAfter(secondReturnTransferReceipt.blockNumber);
      const proofOptions = {
        executionRpcUrl: activeHarness.endpoints.executionRpcUrl,
        gatewayAddress: gateway.gatewayAddress,
      };
      const finalizedExecutionBlock = await activeHarness.publicClient.getBlock({
        blockTag: 'finalized',
      });
      const locators = await discoverMissingGatewayActivityLocators({
        ...proofOptions,
        finalizedExecutionBlockNumber: finalizedExecutionBlock.number,
        minimumGatewayActivityNonce: 3n,
      });
      expect(locators).toHaveLength(2);
      const perLocatorBlockWitnesses: CapturedGatewayActivityStorageProof[] = [];
      for (const locator of locators) {
        const locatorBlock = await activeHarness.publicClient.getBlock({
          blockNumber: locator.blockNumber,
        });
        const locatorWitnesses = await captureGatewayActivityStorageProofs({
          ...proofOptions,
          executionBlock: locatorBlock,
          locators: [locator],
        });
        perLocatorBlockWitnesses.push(...locatorWitnesses);
      }

      const sharedBlockWitnesses = await captureGatewayActivityStorageProofs({
        ...proofOptions,
        executionBlock: await activeHarness.publicClient.getBlock({
          blockNumber: secondReturnTransferReceipt.blockNumber,
        }),
        locators,
      });
      const buildBeforeAnchorSync = await buildGatewayActivityStorageProofs(client, {
        ...proofOptions,
        locators: perLocatorBlockWitnesses,
      });
      expect(buildBeforeAnchorSync.payloads).toEqual([]);
      expect(buildBeforeAnchorSync.deferredLocators).toEqual(perLocatorBlockWitnesses);
      expect(buildBeforeAnchorSync.recaptureLocators).toEqual([]);

      // Retain two anchors so the test can distinguish the nearest usable one from the latest.
      await activeHarness.syncVerifierThrough(secondReturnTransferReceipt.blockNumber);
      const firstUsableAnchor = await getLatestArgonFinalizedExecutionHeader(client);
      const laterReceipt = await activeHarness.waitForExecutionFinalizedAfter(
        firstUsableAnchor.blockNumber,
      );
      await activeHarness.syncVerifierThrough(laterReceipt.blockNumber);
      const newestAnchor = await getLatestArgonFinalizedExecutionHeader(client);
      expect(newestAnchor.blockNumber).toBeGreaterThan(firstUsableAnchor.blockNumber);

      // A modified header makes ancestry unreachable; this is not a real chain reorganization.
      const firstHeaderRlp = perLocatorBlockWitnesses[0].executionHeaderRlp;
      let changedLastByte = '00';
      if (firstHeaderRlp.endsWith('00')) changedLastByte = '01';
      const unreachableHeaderWitness = {
        ...perLocatorBlockWitnesses[0],
        executionHeaderRlp: `${firstHeaderRlp.slice(0, -2)}${changedLastByte}` as Hex,
      };
      const buildRequiringRecapture = await buildGatewayActivityStorageProofs(client, {
        ...proofOptions,
        locators: [unreachableHeaderWitness, perLocatorBlockWitnesses[1]],
      });
      expect(buildRequiringRecapture.payloads).toEqual([]);
      expect(buildRequiringRecapture.recaptureLocators).toEqual([unreachableHeaderWitness]);
      expect(buildRequiringRecapture.deferredLocators).toEqual([perLocatorBlockWitnesses[1]]);

      // Verify anchor selection independently for per-locator and shared-block witnesses.
      const perLocatorBlockBuild = await buildGatewayActivityStorageProofs(client, {
        ...proofOptions,
        locators: buildBeforeAnchorSync.deferredLocators,
      });
      const sharedBlockBuild = await buildGatewayActivityStorageProofs(client, {
        ...proofOptions,
        locators: sharedBlockWitnesses,
      });
      expect(perLocatorBlockBuild.payloads).toHaveLength(2);
      expect(sharedBlockBuild.payloads).toHaveLength(2);
      expect(perLocatorBlockBuild.deferredLocators).toEqual([]);
      expect(sharedBlockBuild.deferredLocators).toEqual([]);

      const anchorChecks = [
        {
          payload: perLocatorBlockBuild.payloads[0],
          witnessBlockNumber: firstReturnTransferReceipt.blockNumber,
        },
        {
          payload: sharedBlockBuild.payloads[1],
          witnessBlockNumber: secondReturnTransferReceipt.blockNumber,
        },
      ];
      const retainedAnchors = (
        await client.query.ethereumVerifier.executionHeaderAnchorsByBlockNumber.entries()
      )
        .flatMap(([, entry]) => (entry.isSome ? [entry.unwrap()] : []))
        .sort((left, right) => Number(left.blockNumber.toBigInt() - right.blockNumber.toBigInt()));
      for (const { payload, witnessBlockNumber } of anchorChecks) {
        const expectedAnchor = retainedAnchors.find(
          anchor => anchor.blockNumber.toBigInt() >= witnessBlockNumber,
        )!;
        const executionProof = payload.proof.storageProof.executionBlockProof;
        expect(executionProof.anchorBlockHash).toBe(expectedAnchor.blockHash.toHex());
        expect(executionProof.anchorBlockHash).not.toBe(newestAnchor.blockHash);
        expect(executionProof.targetToAnchorHeaderChain).toHaveLength(
          Number(expectedAnchor.blockNumber.toBigInt() - witnessBlockNumber),
        );
      }
      expect(
        anchorChecks.some(
          ({ payload }) =>
            payload.proof.storageProof.executionBlockProof.targetToAnchorHeaderChain.length > 0,
        ),
      ).toBe(true);

      // Recapture only the blocked locator; the second locator keeps its cached witness.
      const recapturedWitnesses = await captureGatewayActivityStorageProofs({
        ...proofOptions,
        executionBlock: await activeHarness.publicClient.getBlock({
          blockNumber: newestAnchor.blockNumber,
        }),
        locators: buildRequiringRecapture.recaptureLocators,
      });
      const recapturedBuild = await buildGatewayActivityStorageProofs(client, {
        ...proofOptions,
        locators: recapturedWitnesses,
      });
      expect(recapturedBuild.payloads).toHaveLength(1);
      expect(recapturedBuild.deferredLocators).toEqual([]);
      expect(recapturedBuild.recaptureLocators).toEqual([]);
      expect(
        recapturedBuild.payloads[0].proof.storageProof.executionBlockProof
          .targetToAnchorHeaderChain,
      ).toEqual([]);
      const firstReturnPayload = recapturedBuild.payloads[0];
      const secondReturnPayload = sharedBlockBuild.payloads[1];

      // Altering a proven storage value must not advance the cursor or credit the recipient.
      const tamperedStorageProof = structuredClone(firstReturnPayload.proof);
      tamperedStorageProof.storageProof.slots[1].value = repeatByteHex('00', 32);
      await expect(
        activeHarness.submit(
          client.tx.crosschainTransfer.proveGatewayActivityStorage(
            'Ethereum',
            2n,
            tamperedStorageProof,
          ),
          activeHarness.proofRelayer,
        ),
      ).rejects.toThrow('InvalidProof');
      const stateAfterInvalidProof =
        await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');
      expect(stateAfterInvalidProof.unwrap().gatewayActivityNonce.toBigInt()).toBe(2n);
      const recipientAfterInvalidProof = await client.query.system.account(
        argonReturnRecipient.address,
      );
      expect(recipientAfterInvalidProof.data.free.toBigInt()).toBe(
        recipientBeforeStorageSettlement.data.free.toBigInt(),
      );

      // Submit the recaptured and cached proofs in activity-nonce order.
      const expectedReturns = [
        {
          payload: firstReturnPayload,
          blockNumber: firstReturnTransferReceipt.blockNumber,
          activityNonce: 3n,
        },
        {
          payload: secondReturnPayload,
          blockNumber: secondReturnTransferReceipt.blockNumber,
          activityNonce: 4n,
        },
      ];
      expect(secondReturnPayload.proof.locatorIndex).toBeGreaterThan(
        firstReturnPayload.proof.locatorIndex,
      );
      let expectedBackingBalance = TRANSFER_AMOUNT_RUNTIME_UNITS;
      for (const { payload, blockNumber, activityNonce } of expectedReturns) {
        expect(payload.previousGatewayActivityNonce).toBe(activityNonce - 1n);
        expect(payload.gatewayActivityNonceRange).toEqual({
          start: activityNonce,
          end: activityNonce,
        });
        expect(payload.executionBlockNumberRange).toEqual({ start: blockNumber, end: blockNumber });
        expect(payload.activities).toHaveLength(1);
        expect(payload.activities[0]?.kind).toBe('TransferToArgonStarted');
        expect(payload.proof.locatorIndex).toBeGreaterThan(0n);
        expect(payload.proof.activityLogs).toHaveLength(1);
        expect(payload.proof.storageProof.slots).toHaveLength(2);

        const result = await activeHarness.submit(
          client.tx.crosschainTransfer.proveGatewayActivityStorage(
            'Ethereum',
            payload.previousGatewayActivityNonce,
            payload.proof,
          ),
          activeHarness.proofRelayer,
        );
        expect(
          result.events.some(event =>
            client.events.crosschainTransfer.GatewayStateAdvanced.is(event),
          ),
        ).toBe(true);
        expect(
          result.events.some(event =>
            client.events.crosschainTransfer.TransferToArgonSettled.is(event),
          ),
        ).toBe(true);
        expectedBackingBalance -= returnAmountRuntimeUnits;
        const backing = await client.query.system.account(
          client.consts.crosschainTransfer.ethereumBurnAccount,
        );
        expect(backing.data.free.toBigInt()).toBe(expectedBackingBalance);
      }

      // Inclusion can arrive before the API's shared storage subscription observes the new nonce.
      await expect
        .poll(
          async () => {
            const state =
              await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');
            return state.unwrap().gatewayActivityNonce.toBigInt();
          },
          { timeout: 10_000 },
        )
        .toBe(4n);
      const receiptReplayCheck = await buildGatewayActivityReceiptProofPayloads(client, {
        ...proofOptions,
        locators,
      });
      expect(receiptReplayCheck.payloads).toEqual([]);
      expect(receiptReplayCheck.deferredLocators).toEqual([]);

      const recipientAfterStorageSettlement = await client.query.system.account(
        argonReturnRecipient.address,
      );
      expect(
        recipientAfterStorageSettlement.data.free.toBigInt() -
          recipientBeforeStorageSettlement.data.free.toBigInt(),
      ).toBe(returnAmountRuntimeUnits * 2n);

      const finalGatewayStateOption =
        await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');
      expect(finalGatewayStateOption.isSome).toBe(true);
      expect(finalGatewayStateOption.unwrap().gatewayActivityNonce.toBigInt()).toBe(4n);
    }, 420_000);

    it('accepts receipt proofs after storage settlement without replaying earlier locators', async () => {
      if (!harness || !gateway) throw new Error('roundtrip setup checkpoint did not complete');
      const client = harness.mainchainClient;
      const recipientBeforeReceiptSettlement = await client.query.system.account(
        argonReturnRecipient.address,
      );
      const nextReturnReceipt = await gateway.startTransferToArgon({
        account: ethereumRecipient,
        amountRuntimeUnits: 10_000n,
        recipientArgonAddress: toHex(argonReturnRecipient.publicKey, { size: 32 }),
      });
      await harness.waitForExecutionFinalizedAfter(nextReturnReceipt.blockNumber);
      await harness.syncVerifierThrough(nextReturnReceipt.blockNumber);
      const receiptProofAnchor = await getLatestArgonFinalizedExecutionHeader(client);
      const receiptProofOptions = {
        executionRpcUrl: harness.endpoints.executionRpcUrl,
        gatewayAddress: gateway.gatewayAddress,
        locators: await discoverMissingGatewayActivityLocators({
          executionRpcUrl: harness.endpoints.executionRpcUrl,
          gatewayAddress: gateway.gatewayAddress,
          finalizedExecutionBlockNumber: receiptProofAnchor.blockNumber,
          minimumGatewayActivityNonce: 1n,
        }),
      };
      const receiptBuild = await buildGatewayActivityReceiptProofPayloads(
        client,
        receiptProofOptions,
      );
      expect(receiptBuild.payloads).toHaveLength(1);
      const nextReturnPayload = receiptBuild.payloads[0];
      expect(nextReturnPayload.previousGatewayActivityNonce).toBe(4n);
      expect(nextReturnPayload.gatewayActivityNonceRange).toEqual({ start: 5n, end: 5n });
      expect(nextReturnPayload.activities).toHaveLength(1);
      const relayerBefore = await client.query.system.account(harness.proofRelayer.address);
      const result = await harness.submit(
        client.tx.crosschainTransfer.proveGatewayActivity('Ethereum', 4n, nextReturnPayload.proof),
        harness.proofRelayer,
      );
      expect(result.finalFee ?? 0n).toBe(0n);
      expect(
        result.events.some(event =>
          client.events.crosschainTransfer.TransferToArgonSettled.is(event),
        ),
      ).toBe(true);
      await expect
        .poll(
          async () => {
            const state =
              await client.query.crosschainTransfer.gatewayStateBySourceChain('Ethereum');
            return state.unwrap().gatewayActivityNonce.toBigInt();
          },
          { timeout: 10_000 },
        )
        .toBe(5n);
      const recipientAfterReceiptSettlement = await client.query.system.account(
        argonReturnRecipient.address,
      );
      expect(
        recipientAfterReceiptSettlement.data.free.toBigInt() -
          recipientBeforeReceiptSettlement.data.free.toBigInt(),
      ).toBe(10_000n);
      const relayerAfter = await client.query.system.account(harness.proofRelayer.address);
      expect(relayerAfter.data.free.toBigInt()).toBe(relayerBefore.data.free.toBigInt());
      const receiptReplayCheck = await buildGatewayActivityReceiptProofPayloads(
        client,
        receiptProofOptions,
      );
      expect(receiptReplayCheck.payloads).toEqual([]);
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
      const { payloads, results } = await harness.proveGatewayActivities(
        gateway.gatewayAddress,
        burnBlockNumber,
      );
      const payload = payloads.at(-1)!;
      const result = results.at(-1)!;

      expect(payloads).toHaveLength(1);
      expect(results).toHaveLength(1);
      expect(payload.previousGatewayActivityNonce).toBe(0n);
      expect(payload.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
      expect(payload.executionBlockNumberRange).toEqual({
        start: burnBlockNumber,
        end: burnBlockNumber,
      });
      expect(payload.proof.locatorIndex).toBeGreaterThan(0n);
      expect(payload.proof.activityLogs).toHaveLength(1);
      expect(payload.proof.storageProof.slots).toHaveLength(2);
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
