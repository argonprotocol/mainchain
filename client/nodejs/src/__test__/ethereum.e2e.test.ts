import { Keyring } from '@polkadot/keyring';
import { afterEach, describe, expect, it } from 'vitest';
import {
  argonTokenAbi,
  MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
  mintingGatewayAbi,
  TxSubmitter,
  buildGatewayActivityProofPayload,
  dispatchErrorToString,
  type EthereumReceipt,
  getEthereumBeaconSyncBootstrapTx,
  getLatestArgonFinalizedExecutionHeader,
  getNextEthereumBeaconSyncTxs,
  isOutdatedTransactionError,
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
  parseSignature,
  toHex,
} from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { SKIP_E2E, sudo, teardown, TestEthereum, TestMainchain } from '@argonprotocol/testing';

const TEST_ACCOUNT = {
  address: '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
  balance: '100ETH',
  privateKey: '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
} as const;

const TRANSFER_AMOUNT_RUNTIME_UNITS = 10_000n;
const TRANSFER_AMOUNT_BASE_UNITS =
  TRANSFER_AMOUNT_RUNTIME_UNITS * MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE;
const PROOF_RELAYER_URI = '//Charlie';

afterEach(async () => {
  await teardown();
});

async function signGatewayPermit(args: {
  account: ReturnType<typeof privateKeyToAccount>;
  chainId: number;
  tokenAddress: Hex;
  gatewayAddress: Hex;
  owner: Hex;
  value: bigint;
  nonce: bigint;
  deadline: bigint;
}) {
  const signature = parseSignature(
    await args.account.signTypedData({
      domain: {
        name: 'Argon',
        version: '1',
        chainId: args.chainId,
        verifyingContract: args.tokenAddress,
      },
      types: {
        Permit: [
          { name: 'owner', type: 'address' },
          { name: 'spender', type: 'address' },
          { name: 'value', type: 'uint256' },
          { name: 'nonce', type: 'uint256' },
          { name: 'deadline', type: 'uint256' },
        ],
      },
      primaryType: 'Permit',
      message: {
        owner: args.owner,
        spender: args.gatewayAddress,
        value: args.value,
        nonce: args.nonce,
        deadline: args.deadline,
      },
    }),
  );
  return { v: Number(signature.v), r: signature.r, s: signature.s };
}

describe.skipIf(SKIP_E2E || !TestEthereum.isInstalled())('Ethereum proof e2e', () => {
  it('boots a real ethereum devnet and proves a MintingGateway burn into CrosschainTransfer', async () => {
    const ethereum = new TestEthereum();
    const endpoints = await ethereum.launch({
      consensusClient: 'lodestar',
      preset: 'minimal',
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

    const permitDeadline = (await publicClient.getBlock()).timestamp + 3600n;
    const permitNonce = await publicClient.readContract({
      address: gatewayDeployment.argonTokenAddress,
      abi: argonTokenAbi,
      functionName: 'nonces',
      args: [account.address],
    });
    const permitSignature = await signGatewayPermit({
      account,
      chainId: chain.id,
      tokenAddress: gatewayDeployment.argonTokenAddress,
      gatewayAddress: gatewayDeployment.gatewayAddress,
      owner: account.address,
      value: TRANSFER_AMOUNT_BASE_UNITS,
      nonce: permitNonce,
      deadline: permitDeadline,
    });

    const burnHash = await walletClient.sendTransaction({
      to: gatewayDeployment.gatewayAddress,
      data: encodeFunctionData({
        abi: mintingGatewayAbi,
        functionName: 'startTransferToArgon',
        args: [
          gatewayDeployment.argonTokenAddress,
          TRANSFER_AMOUNT_RUNTIME_UNITS,
          argonDestination,
          permitDeadline,
          permitSignature.v,
          permitSignature.r,
          permitSignature.s,
        ],
      }),
    });
    const burnReceipt = await waitForExecutionReceipt(ethereum, burnHash);
    const burnBlockNumber = BigInt(burnReceipt.blockNumber);

    expect(burnReceipt.status).toBe('0x1');
    expect(burnReceipt.blockHash).toBeTruthy();

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
    const sudoSigner = sudo();
    const proofRelayer = new Keyring({ type: 'sr25519' }).createFromUri(PROOF_RELAYER_URI);

    const checkpointTx = await getEthereumBeaconSyncBootstrapTx(
      mainchainClient,
      endpoints.beaconApiUrl,
    );
    const checkpointResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.sudo.sudo(checkpointTx),
      sudoSigner,
    ).submit();
    await checkpointResult.waitForInFirstBlock;

    const checkpointSudoEvent = checkpointResult.events.find(event =>
      mainchainClient.events.sudo.Sudid.is(event),
    );
    if (!checkpointSudoEvent || !mainchainClient.events.sudo.Sudid.is(checkpointSudoEvent)) {
      throw new Error('forceCheckpoint did not emit sudo.Sudid');
    }
    if (checkpointSudoEvent.data.sudoResult.isErr) {
      const dispatchError = checkpointSudoEvent.data.sudoResult.asErr;
      throw new Error(
        `forceCheckpoint failed: ${dispatchErrorToString(mainchainClient, dispatchError)}`,
      );
    }

    await syncEthereumVerifierUntilAnchorCovers(
      mainchainClient,
      sudoSigner,
      endpoints.beaconApiUrl,
      burnBlockNumber,
    );

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
      sudoSigner,
    ).submit();
    await setConfigResult.waitForInFirstBlock;

    const burnAccount = mainchainClient.consts.crosschainTransfer.ethereumBurnAccount.toString();
    const burnAccountFunding =
      TRANSFER_AMOUNT_RUNTIME_UNITS + mainchainClient.consts.balances.existentialDeposit.toBigInt();
    const proofRelayerFunding =
      mainchainClient.consts.balances.existentialDeposit.toBigInt() + 1_000_000n;

    const fundBurnAccountResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.balances.transferAllowDeath(burnAccount, burnAccountFunding),
      sudoSigner,
    ).submit();
    await fundBurnAccountResult.waitForInFirstBlock;

    const fundProofRelayerResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.balances.transferAllowDeath(proofRelayer.address, proofRelayerFunding),
      sudoSigner,
    ).submit();
    await fundProofRelayerResult.waitForInFirstBlock;

    const proofPayload = await buildGatewayActivityProofPayload(mainchainClient, {
      executionRpcUrl: endpoints.executionRpcUrl,
      gatewayAddress: gatewayDeployment.gatewayAddress,
      throughExecutionBlockNumber: burnBlockNumber,
    });
    if (!proofPayload) {
      throw new Error('Expected uncovered gateway activity to prove');
    }

    const { previousGatewayActivityNonce, proof: eventProof } = proofPayload;
    const proofBlock = eventProof.blocks[0];
    const recipientBefore = await mainchainClient.query.system.account(bob.address);
    const relayerBefore = await mainchainClient.query.system.account(proofRelayer.address);

    const proveGatewayActivityResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.crosschainTransfer.proveGatewayActivity(
        'Ethereum',
        previousGatewayActivityNonce,
        eventProof,
      ),
      proofRelayer,
    ).submit();
    await proveGatewayActivityResult.waitForInFirstBlock;

    expect(previousGatewayActivityNonce).toBe(0n);
    expect(proofPayload.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
    expect(proofPayload.executionBlockNumberRange).toEqual({
      start: burnBlockNumber,
      end: burnBlockNumber,
    });
    expect(eventProof.executionBlockProof.anchorBlockHash).toBeTruthy();
    expect(proofBlock.receiptProof.nodes.length).toBeGreaterThan(0);
    expect(proofBlock.receiptLogs[0]?.transactionIndex).toBe(
      Number(BigInt(burnReceipt.transactionIndex)),
    );
    expect(proofBlock.receiptProof.receipts[0]?.transactionIndex).toBe(
      Number(BigInt(burnReceipt.transactionIndex)),
    );
    expect(
      proveGatewayActivityResult.events.some(event =>
        mainchainClient.events.crosschainTransfer.TransferToArgonSettled.is(event),
      ),
    ).toBe(true);
    expect(
      proveGatewayActivityResult.events.some(event =>
        mainchainClient.events.crosschainTransfer.GatewayStateAdvanced.is(event),
      ),
    ).toBe(true);
    expect(proveGatewayActivityResult.finalFee ?? 0n).toBe(0n);

    const recipientAfter = await mainchainClient.query.system.account(bob.address);
    const relayerAfter = await mainchainClient.query.system.account(proofRelayer.address);
    expect(recipientAfter.data.free.toBigInt() - recipientBefore.data.free.toBigInt()).toBe(
      TRANSFER_AMOUNT_RUNTIME_UNITS,
    );
    expect(relayerAfter.data.free.toBigInt()).toBe(relayerBefore.data.free.toBigInt());
  }, 420_000);

  it('proves a burn after the first minimal sync committee transition', async () => {
    const ethereum = new TestEthereum();
    const endpoints = await ethereum.launch({
      consensusClient: 'lodestar',
      preset: 'minimal',
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

    const gatewayDeployment = await ethereum.deployMintingGatewayFixture({
      deployerPrivateKey: TEST_ACCOUNT.privateKey,
      seedArgonAmountBaseUnits: TRANSFER_AMOUNT_BASE_UNITS,
      seedArgonRecipient: account.address,
    });
    const bob = new Keyring({ type: 'sr25519' }).createFromUri('//Bob');
    const argonDestination = toHex(bob.publicKey, { size: 32 });
    const permitDeadline = (await publicClient.getBlock()).timestamp + 3600n;
    const permitNonce = await publicClient.readContract({
      address: gatewayDeployment.argonTokenAddress,
      abi: argonTokenAbi,
      functionName: 'nonces',
      args: [account.address],
    });
    const permitSignature = await signGatewayPermit({
      account,
      chainId: chain.id,
      tokenAddress: gatewayDeployment.argonTokenAddress,
      gatewayAddress: gatewayDeployment.gatewayAddress,
      owner: account.address,
      value: TRANSFER_AMOUNT_RUNTIME_UNITS * MINTING_GATEWAY_RUNTIME_TO_ERC20_SCALE,
      nonce: permitNonce,
      deadline: permitDeadline,
    });
    await waitForExecutionBlockAtOrAbove(publicClient, 73n);

    const burnHash = await walletClient.sendTransaction({
      to: gatewayDeployment.gatewayAddress,
      data: encodeFunctionData({
        abi: mintingGatewayAbi,
        functionName: 'startTransferToArgon',
        args: [
          gatewayDeployment.argonTokenAddress,
          TRANSFER_AMOUNT_RUNTIME_UNITS,
          argonDestination,
          permitDeadline,
          permitSignature.v,
          permitSignature.r,
          permitSignature.s,
        ],
      }),
    });
    const burnReceipt = await waitForExecutionReceipt(ethereum, burnHash);
    const burnBlockNumber = BigInt(burnReceipt.blockNumber);

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
    const sudoSigner = sudo();
    const proofRelayer = new Keyring({ type: 'sr25519' }).createFromUri(PROOF_RELAYER_URI);

    const checkpointTx = await getEthereumBeaconSyncBootstrapTx(
      mainchainClient,
      endpoints.beaconApiUrl,
    );
    const checkpointResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.sudo.sudo(checkpointTx),
      sudoSigner,
    ).submit();
    await checkpointResult.waitForInFirstBlock;

    const checkpointSudoEvent = checkpointResult.events.find(event =>
      mainchainClient.events.sudo.Sudid.is(event),
    );
    if (!checkpointSudoEvent || !mainchainClient.events.sudo.Sudid.is(checkpointSudoEvent)) {
      throw new Error('forceCheckpoint did not emit sudo.Sudid');
    }
    if (checkpointSudoEvent.data.sudoResult.isErr) {
      const dispatchError = checkpointSudoEvent.data.sudoResult.asErr;
      throw new Error(
        `forceCheckpoint failed: ${dispatchErrorToString(mainchainClient, dispatchError)}`,
      );
    }

    await syncEthereumVerifierUntilAnchorCovers(
      mainchainClient,
      sudoSigner,
      endpoints.beaconApiUrl,
      burnBlockNumber,
    );

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
      sudoSigner,
    ).submit();
    await setConfigResult.waitForInFirstBlock;

    const burnAccount = mainchainClient.consts.crosschainTransfer.ethereumBurnAccount.toString();
    const burnAccountFunding =
      TRANSFER_AMOUNT_RUNTIME_UNITS + mainchainClient.consts.balances.existentialDeposit.toBigInt();
    const proofRelayerFunding =
      mainchainClient.consts.balances.existentialDeposit.toBigInt() + 1_000_000n;

    const fundBurnAccountResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.balances.transferAllowDeath(burnAccount, burnAccountFunding),
      sudoSigner,
    ).submit();
    await fundBurnAccountResult.waitForInFirstBlock;

    const fundProofRelayerResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.balances.transferAllowDeath(proofRelayer.address, proofRelayerFunding),
      sudoSigner,
    ).submit();
    await fundProofRelayerResult.waitForInFirstBlock;

    const proofPayload = await buildGatewayActivityProofPayload(mainchainClient, {
      executionRpcUrl: endpoints.executionRpcUrl,
      gatewayAddress: gatewayDeployment.gatewayAddress,
      throughExecutionBlockNumber: burnBlockNumber,
    });
    if (!proofPayload) {
      throw new Error('Expected uncovered gateway activity to prove');
    }

    const { previousGatewayActivityNonce, proof: eventProof } = proofPayload;
    const relayerBefore = await mainchainClient.query.system.account(proofRelayer.address);
    const proveGatewayActivityResult = await new TxSubmitter(
      mainchainClient,
      mainchainClient.tx.crosschainTransfer.proveGatewayActivity(
        'Ethereum',
        previousGatewayActivityNonce,
        eventProof,
      ),
      proofRelayer,
    ).submit();
    await proveGatewayActivityResult.waitForInFirstBlock;

    expect(previousGatewayActivityNonce).toBe(0n);
    expect(proofPayload.gatewayActivityNonceRange).toEqual({ start: 1n, end: 1n });
    expect(proofPayload.executionBlockNumberRange).toEqual({
      start: burnBlockNumber,
      end: burnBlockNumber,
    });
    expect(
      proveGatewayActivityResult.events.some(event =>
        mainchainClient.events.crosschainTransfer.TransferToArgonSettled.is(event),
      ),
    ).toBe(true);
    expect(
      proveGatewayActivityResult.events.some(event =>
        mainchainClient.events.crosschainTransfer.GatewayStateAdvanced.is(event),
      ),
    ).toBe(true);
    expect(proveGatewayActivityResult.finalFee ?? 0n).toBe(0n);

    const relayerAfter = await mainchainClient.query.system.account(proofRelayer.address);
    expect(relayerAfter.data.free.toBigInt()).toBe(relayerBefore.data.free.toBigInt());
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

async function waitForExecutionBlockAtOrAbove(
  publicClient: ReturnType<typeof createPublicClient>,
  minimumExecutionBlockNumber: bigint,
) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 120_000) {
    const blockNumber = await publicClient.getBlockNumber();
    if (blockNumber >= minimumExecutionBlockNumber) {
      return blockNumber;
    }

    await delay(500);
  }

  throw new Error(`Timed out waiting for execution block ${minimumExecutionBlockNumber}`);
}

async function waitForExecutionReceipt(
  ethereum: TestEthereum,
  transactionHash: Hex,
): Promise<EthereumReceipt> {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 120_000) {
    try {
      const receipt = await ethereum.callExecution<RpcTransactionReceipt | null>(
        'eth_getTransactionReceipt',
        [transactionHash],
      );

      if (receipt) {
        return receipt as unknown as EthereumReceipt;
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

async function syncEthereumVerifierUntilAnchorCovers(
  mainchainClient: Awaited<ReturnType<TestMainchain['client']>>,
  relayer: ReturnType<typeof sudo>,
  beaconApiUrl: string,
  minimumExecutionBlockNumber: bigint,
) {
  const startedAt = Date.now();
  let sawAnyTx = false;
  let lastRetryableError: Error | undefined;

  while (Date.now() - startedAt < 120_000) {
    try {
      const anchor = await getLatestArgonFinalizedExecutionHeader(mainchainClient);
      if (anchor.blockNumber >= minimumExecutionBlockNumber) {
        return;
      }
    } catch {}

    const txs = await getNextEthereumBeaconSyncTxs(mainchainClient, beaconApiUrl);
    if (txs.length === 0) {
      if (sawAnyTx && !lastRetryableError) {
        break;
      }

      await delay(500);
      continue;
    }

    sawAnyTx = true;
    let shouldRetry = false;
    for (const tx of txs) {
      try {
        const result = await new TxSubmitter(mainchainClient, tx, relayer).submit();
        await result.waitForInFirstBlock;
        lastRetryableError = undefined;
      } catch (error) {
        if (isRetryableEthereumVerifierSyncError(error)) {
          lastRetryableError = error instanceof Error ? error : new Error(String(error));
          shouldRetry = true;
          break;
        }
        throw error;
      }
    }

    if (shouldRetry) {
      await delay(500);
    }
  }

  throw (
    lastRetryableError ??
    new Error(
      `Ethereum verifier did not retain an anchor at or above execution block ${minimumExecutionBlockNumber} within the retry window`,
    )
  );
}

function isRetryableEthereumVerifierSyncError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);

  return (
    isOutdatedTransactionError(error) ||
    message.includes('ethereumVerifier.InvalidHeaderMerkleProof')
  );
}

async function delay(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}
