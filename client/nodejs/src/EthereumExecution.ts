import { createPublicClient, http } from 'viem';

export type EthereumReceipt = Awaited<
  ReturnType<ReturnType<typeof createPublicClient>['getTransactionReceipt']>
>;
export type EthereumExecutionClient = ReturnType<typeof createPublicClient>;

export type EthereumExecutionSource = {
  executionRpcUrl?: string;
  executionClient?: EthereumExecutionClient;
};

const executionClientsByUrl = new Map<string, EthereumExecutionClient>();

export function getExecutionClient(source: EthereumExecutionSource): EthereumExecutionClient {
  if (source.executionClient) {
    return source.executionClient;
  }
  if (source.executionRpcUrl) {
    const cached = executionClientsByUrl.get(source.executionRpcUrl);
    if (cached) {
      return cached;
    }

    const client = createPublicClient({ transport: http(source.executionRpcUrl) });
    executionClientsByUrl.set(source.executionRpcUrl, client);
    return client;
  }

  throw new Error('Ethereum event proof requires an execution client or execution RPC URL');
}

function getExecutionRpcErrorText(error: unknown): string {
  if (error instanceof Error) {
    return [
      error.message,
      'details' in error && typeof error.details === 'string' ? error.details : undefined,
    ]
      .filter(Boolean)
      .join(' ');
  }

  return String(error);
}

// Dev/test execution RPCs can expose a fresh block before their receipt/log indexes catch up.
// When that happens, recent reads may fail transiently with "indexing is in progress".
export async function retryWhileExecutionRpcIndexing<TResult>(
  request: () => Promise<TResult>,
): Promise<TResult> {
  const startedAt = Date.now();
  let lastError: Error | undefined;

  while (Date.now() - startedAt < 30_000) {
    try {
      return await request();
    } catch (error) {
      const errorText = getExecutionRpcErrorText(error);
      if (!errorText.includes('indexing is in progress')) {
        throw error;
      }

      lastError = error instanceof Error ? error : new Error(errorText);
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  }

  throw lastError ?? new Error('Timed out waiting for execution RPC indexing');
}
