import { Keyring } from '@polkadot/keyring';
import { afterEach, describe, expect, it } from 'vitest';
import {
  TxSubmitter,
  buildEthereumEventProof,
  getEthereumBeaconSyncBootstrapTx,
  getNextEthereumBeaconSyncTxs,
} from '../index';
import type {
  EthereumBeaconBlockResponse,
  EthereumBeaconHeaderDetailsResponse,
} from '../EthereumBeaconTypes';
import type { Hex, RpcTransactionReceipt } from 'viem';
import {
  createPublicClient,
  createWalletClient,
  defineChain,
  encodeEventTopics,
  encodeFunctionData,
  http,
  toHex,
} from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import {
  argonTokenArtifact,
  mintingGatewayArtifact,
  SKIP_E2E,
  sudo,
  teardown,
  TestEthereum,
  TestMainchain,
} from '@argonprotocol/testing';

const TEST_ACCOUNT = {
  address: '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
  balance: '100ETH',
  privateKey: '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
} as const;

const RUNTIME_TO_ERC20_SCALE = 1_000_000_000_000n;
const TRANSFER_AMOUNT_BASE_UNITS = 250n;

afterEach(async () => {
  await teardown();
});

describe.skipIf(SKIP_E2E || !TestEthereum.isInstalled())('Ethereum proof e2e', () => {
  it('boots a real ethereum devnet and proves a MintingGateway burn into CrosschainTransfer', async () => {
    const ethereum = new TestEthereum();
    const endpoints = await ethereum.launch({
      secondsPerSlot: 1,
      prefundedAccounts: {
        [TEST_ACCOUNT.address]: {
          balance: TEST_ACCOUNT.balance,
        },
      },
    });

    const mainchain = new TestMainchain();
    const mainchainReady = mainchain.launch();

    const chain = defineChain({
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

    const account = privateKeyToAccount(TEST_ACCOUNT.privateKey);
    const publicClient = createPublicClient({
      chain,
      transport: http(endpoints.executionRpcUrl),
    });
    const walletClient = createWalletClient({
      account,
      chain,
      transport: http(endpoints.executionRpcUrl),
    });

    expect(await publicClient.getBalance({ address: account.address })).toBeGreaterThan(0n);

    const gatewayDeployment = await ethereum.deployMintingGatewayFixture({
      deployerPrivateKey: TEST_ACCOUNT.privateKey,
      seedArgonAmountBaseUnits: TRANSFER_AMOUNT_BASE_UNITS,
      seedArgonRecipient: account.address,
    });
    const bob = new Keyring({ type: 'sr25519' }).createFromUri('//Bob');
    const argonDestination = toHex(bob.publicKey, { size: 32 });
    const burnForTransferTopic = encodeEventTopics({
      abi: mintingGatewayArtifact.abi,
      eventName: 'BurnForTransfer',
    })[0];
    if (!burnForTransferTopic) {
      throw new Error('Missing BurnForTransfer topic');
    }

    const approveHash = await walletClient.sendTransaction({
      to: gatewayDeployment.argonTokenAddress,
      data: encodeFunctionData({
        abi: argonTokenArtifact.abi,
        functionName: 'approve',
        args: [
          gatewayDeployment.gatewayAddress,
          TRANSFER_AMOUNT_BASE_UNITS * RUNTIME_TO_ERC20_SCALE,
        ],
      }),
    });
    await waitForExecutionReceipt(ethereum, approveHash);

    const burnHash = await walletClient.sendTransaction({
      to: gatewayDeployment.gatewayAddress,
      data: encodeFunctionData({
        abi: mintingGatewayArtifact.abi,
        functionName: 'burnForTransfer',
        args: [gatewayDeployment.argonTokenAddress, TRANSFER_AMOUNT_BASE_UNITS, argonDestination],
      }),
    });
    const burnReceipt = await waitForExecutionReceipt(ethereum, burnHash);
    const burnBlockNumber = BigInt(burnReceipt.blockNumber);
    const burnLogIndex = burnReceipt.logs.findIndex(
      log =>
        log.address.toLowerCase() === gatewayDeployment.gatewayAddress.toLowerCase() &&
        log.topics[0]?.toLowerCase() === burnForTransferTopic.toLowerCase(),
    );

    expect(burnReceipt.status).toBe('0x1');
    expect(burnReceipt.blockHash).toBeTruthy();
    expect(burnLogIndex).toBeGreaterThanOrEqual(0);

    const laterReceipt = await mineLaterExecutionAnchorReceipt(
      walletClient,
      chain,
      ethereum,
      account,
      burnBlockNumber,
    );
    await waitForFinalizedBeaconExecutionAtOrAbove(ethereum, BigInt(laterReceipt.blockNumber));

    await mainchainReady;

    const mainchainClient = await mainchain.client();
    const relayer = sudo();

    const checkpointTx = await getEthereumBeaconSyncBootstrapTx(
      mainchainClient,
      endpoints.beaconApiUrl,
    );
    const checkpointResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.sudo.sudo(checkpointTx),
      relayer,
    ).submit();
    await checkpointResult.waitForInFirstBlock;

    const checkpointSudoEvent = checkpointResult.events.find(event =>
      mainchainClient.events.sudo.Sudid.is(event),
    );
    if (!checkpointSudoEvent || !mainchainClient.events.sudo.Sudid.is(checkpointSudoEvent)) {
      throw new Error('forceCheckpoint did not emit sudo.Sudid');
    }
    if (checkpointSudoEvent.data.sudoResult.isErr) {
      throw new Error(
        `forceCheckpoint failed: ${checkpointSudoEvent.data.sudoResult.asErr.toString()}`,
      );
    }

    const maintenanceTxs = await getNextEthereumBeaconSyncTxs(
      mainchainClient,
      endpoints.beaconApiUrl,
    );
    if (maintenanceTxs.length === 0) {
      throw new Error('missing ethereum verifier maintenance txs');
    }
    for (const tx of maintenanceTxs) {
      const result = await new TxSubmitter(mainchainClient, tx, relayer).submit();
      await result.waitForInFirstBlock;
    }

    const finalizedState = await mainchainClient.query.ethereumVerifier.finalizedBeaconState(
      await mainchainClient.query.ethereumVerifier.latestFinalizedBlockRoot(),
    );

    expect(finalizedState.isSome).toBe(true);
    expect(
      (await mainchainClient.query.ethereumVerifier.latestExecutionHeaderAnchorBlockHash()).isSome,
    ).toBe(true);

    const setConfigResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.sudo.sudo(
        mainchainClient.tx.crosschainTransfer.setChainConfig({
          Ethereum: {
            gateway: gatewayDeployment.gatewayAddress,
            argonToken: gatewayDeployment.argonTokenAddress,
            argonotToken: gatewayDeployment.argonotTokenAddress,
            previousGateway: null,
            previousReleaseExpiration: null,
          },
        }),
      ),
      relayer,
    ).submit();
    await setConfigResult.waitForInFirstBlock;

    const burnAccount = mainchainClient.consts.crosschainTransfer.ethereumBurnAccount.toString();
    const burnAccountFunding =
      TRANSFER_AMOUNT_BASE_UNITS + mainchainClient.consts.balances.existentialDeposit.toBigInt();

    const fundBurnAccountResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.balances.transferAllowDeath(burnAccount, burnAccountFunding),
      relayer,
    ).submit();
    await fundBurnAccountResult.waitForInFirstBlock;

    const eventProof = await buildEthereumEventProof(mainchainClient, {
      executionRpcUrl: endpoints.executionRpcUrl,
      txHash: burnHash,
      logIndex: burnLogIndex,
    });
    const verifyResult = await mainchainClient.call.ethereumApis.verifyEventLog(
      eventProof.eventLog,
      eventProof.proof,
    );
    const executionBlockProof = eventProof.proof.executionBlockProof;
    const recipientBefore = await mainchainClient.query.system.account(bob.address);

    const proveTransferResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.crosschainTransfer.proveTransfer({
        Ethereum: {
          sourceChain: 'Ethereum',
          eventLog: eventProof.eventLog,
          proof: eventProof.proof,
        },
      }),
      relayer,
    ).submit();
    await proveTransferResult.waitForInFirstBlock;

    expect(eventProof.proof.receiptProof.nodes.length).toBeGreaterThan(0);
    expect(eventProof.proof.receiptProof.transactionIndex).toBe(
      Number(BigInt(burnReceipt.transactionIndex)),
    );
    expect(executionBlockProof.anchorBlockHash).toBeTruthy();
    expect(verifyResult.isOk).toBe(true);
    expect(
      proveTransferResult.events.some(event =>
        mainchainClient.events.crosschainTransfer.BurnNoticeAccepted.is(event),
      ),
    ).toBe(true);

    const recipientAfter = await mainchainClient.query.system.account(bob.address);
    expect(recipientAfter.data.free.toBigInt() - recipientBefore.data.free.toBigInt()).toBe(
      TRANSFER_AMOUNT_BASE_UNITS - (proveTransferResult.finalFee ?? 0n),
    );
  }, 420_000);
});

async function waitForFinalizedBeaconExecutionAtOrAbove(
  ethereum: TestEthereum,
  minimumExecutionBlockNumber: bigint,
) {
  const startedAt = Date.now();

  let lastSeenExecutionBlockNumber = 0n;
  let lastSeenHeadSlot = 0n;
  let lastSeenFinalizedSlot = 0n;
  let lastError: Error | undefined;

  while (Date.now() - startedAt < 300_000) {
    try {
      const [headHeader, finalizedHeader] = await Promise.all([
        ethereum.getBeacon<EthereumBeaconHeaderDetailsResponse>('/eth/v1/beacon/headers/head'),
        ethereum.getBeacon<EthereumBeaconHeaderDetailsResponse>('/eth/v1/beacon/headers/finalized'),
      ]);
      lastSeenHeadSlot = BigInt(headHeader.data.header.message.slot);
      lastSeenFinalizedSlot = BigInt(finalizedHeader.data.header.message.slot);
      const block = await ethereum.getBeacon<EthereumBeaconBlockResponse>(
        `/eth/v2/beacon/blocks/${finalizedHeader.data.root}`,
      );
      const executionBlockNumber = BigInt(block.data.message.body.execution_payload.block_number);
      lastSeenExecutionBlockNumber = executionBlockNumber;
      lastError = undefined;

      if (executionBlockNumber >= minimumExecutionBlockNumber) {
        return { header: finalizedHeader, block };
      }
    } catch (error) {
      if (!(error instanceof Error)) {
        throw error;
      }
      lastError = error;
    }

    await delay(1_000);
  }

  const lastErrorSuffix = lastError ? `; last beacon error was: ${lastError.message}` : '';
  throw new Error(
    `Timed out waiting for finalized beacon execution block at or above ${minimumExecutionBlockNumber}; last seen head slot was ${lastSeenHeadSlot}, finalized slot was ${lastSeenFinalizedSlot}, and finalized execution block was ${lastSeenExecutionBlockNumber}${lastErrorSuffix}`,
  );
}

async function mineLaterExecutionAnchorReceipt(
  walletClient: ReturnType<typeof createWalletClient>,
  chain: ReturnType<typeof defineChain>,
  ethereum: TestEthereum,
  account: ReturnType<typeof privateKeyToAccount>,
  minimumBlockNumber: bigint,
) {
  while (true) {
    const transactionHash = await walletClient.sendTransaction({
      account,
      chain,
      to: account.address,
      value: 0n,
    });
    const receipt = await waitForExecutionReceipt(ethereum, transactionHash);

    if (BigInt(receipt.blockNumber) > minimumBlockNumber) {
      return receipt;
    }
  }
}

async function waitForExecutionReceipt(
  ethereum: TestEthereum,
  transactionHash: Hex,
): Promise<RpcTransactionReceipt> {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 120_000) {
    try {
      const receipt = await ethereum.callExecution<RpcTransactionReceipt | null>(
        'eth_getTransactionReceipt',
        [transactionHash],
      );

      if (receipt) {
        return receipt;
      }
    } catch (error) {
      const errorText =
        error instanceof Error
          ? [
              error.message,
              'details' in error && typeof error.details === 'string' ? error.details : undefined,
            ]
              .filter(Boolean)
              .join(' ')
          : String(error);

      if (!errorText.includes('indexing is in progress')) {
        throw error;
      }
    }

    await delay(500);
  }

  throw new Error(`Timed out waiting for execution receipt ${transactionHash}`);
}

async function delay(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}
