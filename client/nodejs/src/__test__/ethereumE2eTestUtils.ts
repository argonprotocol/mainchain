import type { KeyringPair } from '@polkadot/keyring/types';
import { hexToU8a, u8aToHex } from '@polkadot/util';
import {
  getLatestArgonFinalizedExecutionHeader,
  getNextEthereumBeaconSyncTxs,
  isOutdatedTransactionError,
  type ArgonClient,
  type EthereumReceipt,
  TxSubmitter,
} from '../index';
import type {
  EthereumBeaconBlockResponse,
  EthereumBeaconHeaderDetailsResponse,
} from '../EthereumBeaconTypes';
import { privateKeyToAccount } from 'viem/accounts';
import type { Hex, RpcTransactionReceipt } from 'viem';
import { createPublicClient, createWalletClient, defineChain, parseSignature } from 'viem';
import { TestEthereum } from '../../../../testing/nodejs/src/index';

export async function signGatewayPermit(args: {
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

  return {
    v: Number(signature.v),
    r: signature.r,
    s: signature.s,
  };
}

export async function waitForFinalizedBeaconExecutionAtOrAbove(
  ethereum: TestEthereum,
  minimumExecutionBlockNumber: bigint,
  options: {
    minimumFinalizedSlot?: bigint;
  } = {},
) {
  const startedAt = Date.now();
  const minimumFinalizedSlot = options.minimumFinalizedSlot ?? 0n;

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

      if (
        executionBlockNumber >= minimumExecutionBlockNumber &&
        lastSeenFinalizedSlot >= minimumFinalizedSlot
      ) {
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
    `Timed out waiting for finalized beacon execution block at or above ${minimumExecutionBlockNumber} and finalized slot at or above ${minimumFinalizedSlot}; last seen head slot was ${lastSeenHeadSlot}, finalized slot was ${lastSeenFinalizedSlot}, and finalized execution block was ${lastSeenExecutionBlockNumber}${lastErrorSuffix}`,
  );
}

export async function mineLaterExecutionAnchorReceipt(
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

export async function waitForExecutionBlockAtOrAbove(
  publicClient: Pick<ReturnType<typeof createPublicClient>, 'getBlockNumber'>,
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

export async function waitForExecutionReceipt(
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

export async function syncEthereumVerifierUntilAnchorCovers(
  mainchainClient: ArgonClient,
  relayer: KeyringPair,
  beaconApiUrl: string,
  minimumExecutionBlockNumber: bigint,
) {
  const startedAt = Date.now();
  const timeoutMs = 5 * 60_000;
  let sawAnyTx = false;
  let lastRetryableError: Error | undefined;
  let lastAnchorBlockNumber: bigint | undefined;

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const anchor = await getLatestArgonFinalizedExecutionHeader(mainchainClient);
      lastAnchorBlockNumber = anchor.blockNumber;
      if (anchor.blockNumber >= minimumExecutionBlockNumber) {
        return;
      }
    } catch {}

    const txs = await getNextEthereumBeaconSyncTxs(mainchainClient, beaconApiUrl);
    if (txs.length === 0) {
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
      `Ethereum verifier did not retain an anchor at or above execution block ${minimumExecutionBlockNumber} within ${Math.floor(timeoutMs / 1000)}s; last seen anchor was ${lastAnchorBlockNumber ?? 'unavailable'}`,
    )
  );
}

export function repeatByteHex(byte: string, size: number): Hex {
  return `0x${byte.repeat(size)}`;
}

export function toArgonKeccakSignature(signature: Hex): Hex {
  const bytes = hexToU8a(signature);
  if (bytes.length !== 65) {
    throw new Error(`Expected 65-byte ECDSA signature, received ${bytes.length} bytes`);
  }
  if (bytes[64] >= 27) {
    bytes[64] -= 27;
  }
  return u8aToHex(bytes);
}

export function toEvmRecoverableSignature(signature: Hex): Hex {
  const bytes = hexToU8a(signature);
  if (bytes.length !== 65) {
    throw new Error(`Expected 65-byte ECDSA signature, received ${bytes.length} bytes`);
  }
  if (bytes[64] <= 1) {
    bytes[64] += 27;
  }
  return u8aToHex(bytes);
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
