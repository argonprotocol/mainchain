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
  encodeFunctionData,
  http,
  padHex,
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

const BURN_FOR_TRANSFER_TOPIC =
  '0x805e16cfddeedbafc2c9e510cc99d489152ee3d1179eb6a6ca444404a493af4b';
const RUNTIME_TO_ERC20_SCALE = 1_000_000_000_000n;
const TEST_BURN_AMOUNT = 123_456_789n;
const TEST_ARGON_DESTINATION = padHex('0x6172676f6e2d64657374696e6174696f6e', { size: 32 });

afterEach(async () => {
  await teardown();
});

describe.skipIf(SKIP_E2E || !TestEthereum.isInstalled())('Ethereum proof e2e', () => {
  it('boots a real ethereum devnet and preflights a MintingGateway burn using a retained finalized anchor', async () => {
    const ethereum = new TestEthereum();
    const endpoints = await ethereum.launch({
      secondsPerSlot: 1,
      waitForFinalization: false,
      prefundedAccounts: {
        [TEST_ACCOUNT.address]: {
          balance: TEST_ACCOUNT.balance,
        },
      },
    });

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
      seedArgonAmountBaseUnits: 1_000_000_000n,
      seedArgonRecipient: account.address,
    });

    expect(gatewayDeployment.gatewayAddress).toMatch(/^0x[a-fA-F0-9]{40}$/);
    expect(gatewayDeployment.argonTokenAddress).toMatch(/^0x[a-fA-F0-9]{40}$/);

    const approveHash = await walletClient.sendTransaction({
      to: gatewayDeployment.argonTokenAddress,
      data: encodeFunctionData({
        abi: argonTokenArtifact.abi,
        functionName: 'approve',
        args: [gatewayDeployment.gatewayAddress, TEST_BURN_AMOUNT * RUNTIME_TO_ERC20_SCALE],
      }),
    });
    const approveReceipt = await waitForExecutionReceipt(ethereum, approveHash);
    expect(approveReceipt.status).toBe('0x1');

    const burnHash = await walletClient.sendTransaction({
      to: gatewayDeployment.gatewayAddress,
      data: encodeFunctionData({
        abi: mintingGatewayArtifact.abi,
        functionName: 'burnForTransfer',
        args: [gatewayDeployment.argonTokenAddress, TEST_BURN_AMOUNT, TEST_ARGON_DESTINATION],
      }),
    });
    const burnReceipt = await waitForExecutionReceipt(ethereum, burnHash);
    const burnBlockNumber = BigInt(burnReceipt.blockNumber);
    const burnTransactionIndex = Number(BigInt(burnReceipt.transactionIndex));
    const burnLogIndex = burnReceipt.logs.findIndex(
      log =>
        log.address.toLowerCase() === gatewayDeployment.gatewayAddress.toLowerCase() &&
        log.topics[0]?.toLowerCase() === BURN_FOR_TRANSFER_TOPIC,
    );

    expect(burnReceipt.status).toBe('0x1');
    expect(burnReceipt.logs.length).toBeGreaterThan(1);
    expect(burnReceipt.blockHash).toBeTruthy();
    expect(burnLogIndex).toBeGreaterThanOrEqual(0);

    const burnLog = burnReceipt.logs[burnLogIndex];
    expect(burnLog.address.toLowerCase()).toBe(gatewayDeployment.gatewayAddress.toLowerCase());
    expect(burnLog.topics[0]?.toLowerCase()).toBe(BURN_FOR_TRANSFER_TOPIC);
    expect(burnLog.topics[1]?.toLowerCase()).toBe(
      padHex(account.address, { size: 32 }).toLowerCase(),
    );
    expect(burnLog.topics[2]?.toLowerCase()).toBe(
      padHex(gatewayDeployment.argonTokenAddress, { size: 32 }).toLowerCase(),
    );
    expect(burnLog.data.toLowerCase()).toContain(
      toHex(TEST_BURN_AMOUNT, { size: 32 }).slice(2).toLowerCase(),
    );
    expect(burnLog.data.toLowerCase()).toContain(TEST_ARGON_DESTINATION.slice(2).toLowerCase());

    await waitForFinalizedBeaconExecutionAtOrAbove(ethereum, burnBlockNumber);

    const mainchain = new TestMainchain();
    await mainchain.launch();

    const mainchainClient = await mainchain.client();
    const alice = sudo();

    const checkpointTx = await getEthereumBeaconSyncBootstrapTx(
      mainchainClient,
      endpoints.beaconApiUrl,
    );
    const checkpointResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.sudo.sudo(checkpointTx),
      alice,
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
      const result = await new TxSubmitter(mainchainClient, tx, alice).submit();
      await result.waitForInFirstBlock;
    }

    const finalizedState = await mainchainClient.query.ethereumVerifier.finalizedBeaconState(
      await mainchainClient.query.ethereumVerifier.latestFinalizedBlockRoot(),
    );

    expect(finalizedState.isSome).toBe(true);
    expect(
      (await mainchainClient.query.ethereumVerifier.latestExecutionHeaderAnchorBlockHash()).isSome,
    ).toBe(true);

    const eventProof = await buildEthereumEventProof(mainchainClient, {
      executionRpcUrl: endpoints.executionRpcUrl,
      logIndex: burnLogIndex,
      txHash: burnHash,
    });
    const verifyResult = await mainchainClient.call.ethereumApis.verifyEventLog(
      eventProof.eventLog,
      eventProof.proof,
    );
    const executionBlockProof = eventProof.proof.executionBlockProof;

    expect(eventProof.proof.receiptProof.nodes.length).toBeGreaterThan(0);
    expect(eventProof.proof.receiptProof.transactionIndex).toBe(burnTransactionIndex);
    expect(executionBlockProof.anchorBlockHash).toBeTruthy();
    expect(verifyResult.isOk).toBe(true);
  }, 420_000);
});

async function waitForFinalizedBeaconExecutionAtOrAbove(
  ethereum: TestEthereum,
  minimumExecutionBlockNumber: bigint,
) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 180_000) {
    const header = await ethereum.getBeacon<EthereumBeaconHeaderDetailsResponse>(
      '/eth/v1/beacon/headers/finalized',
    );
    const block = await ethereum.getBeacon<EthereumBeaconBlockResponse>(
      `/eth/v2/beacon/blocks/${header.data.root}`,
    );
    const executionBlockNumber = BigInt(block.data.message.body.execution_payload.block_number);

    if (executionBlockNumber >= minimumExecutionBlockNumber) {
      return { header, block };
    }

    await delay(1_000);
  }

  throw new Error(
    `Timed out waiting for finalized beacon execution block at or above ${minimumExecutionBlockNumber}`,
  );
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
