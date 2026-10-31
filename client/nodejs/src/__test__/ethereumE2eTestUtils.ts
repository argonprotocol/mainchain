import { hexToU8a, u8aToHex } from '@polkadot/util';
import type { EthereumReceipt } from '../index';
import { privateKeyToAccount } from 'viem/accounts';
import type { Hex, RpcTransactionReceipt } from 'viem';
import { createPublicClient, createWalletClient, defineChain, parseSignature } from 'viem';
import { TestEthereum } from '@argonprotocol/testing';

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

async function delay(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}
