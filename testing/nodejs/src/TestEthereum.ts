import * as fs from 'node:fs/promises';
import * as os from 'node:os';
import * as Path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { detectPort } from 'detect-port';
import { EvmContracts } from '@argonprotocol/mainchain';
import { privateKeyToAccount } from 'viem/accounts';
import {
  type Abi,
  createPublicClient,
  createWalletClient,
  defineChain,
  encodeFunctionData,
  getAddress,
  http,
  type Hex,
  zeroAddress,
} from 'viem';
import { addTeardown, type ITeardownable } from './support';

const {
  argonTokenArtifact,
  argonotTokenArtifact,
  hashMintingGatewayGlobalIssuanceCouncil,
  mintingGatewayArtifact,
  proxyAdminArtifact,
  transparentUpgradeableProxyArtifact,
} = EvmContracts;

const DEFAULT_KURTOSIS_BIN = 'kurtosis';
const DEFAULT_ETHEREUM_PACKAGE = 'github.com/ethpandaops/ethereum-package';
const DEFAULT_EL_PORT_START = 32000;
const DEFAULT_CL_PORT_START = 33000;
const PORT_RANGE_SIZE = 32;
const ENCLAVE_NAME_PREFIX = 'argon-eth-';
const PROBE_INTERVAL_MS = 1_000;
const PROBE_TIMEOUT_MS = 60_000;
const LIGHT_CLIENT_READY_TIMEOUT_MS = 5 * 60_000;
const KURTOSIS_RUN_TIMEOUT_MS = 20 * 60_000;
const DEFAULT_SEED_ARGON_AMOUNT_BASE_UNITS = 1_000_000_000n;
const DEFAULT_INITIAL_MICROGONS_PER_ARGONOT = 1_000_000n;
const ERC1967_ADMIN_SLOT =
  '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103' as const;

export interface EthereumEndpoints {
  executionRpcUrl: string;
  beaconApiUrl: string;
  chainId: string;
}

export interface EthereumMintingGatewayFixture {
  argonTokenAddress: Hex;
  argonotTokenAddress: Hex;
  gatewayAddress: Hex;
}

export interface EthereumPrefundedAccount {
  balance: string;
}

export type EthereumConsensusClient = 'lighthouse' | 'lodestar';
export type EthereumBeaconPreset = 'mainnet' | 'minimal';

interface ProbeResult {
  chainId?: string;
  url: string;
}

export default class TestEthereum implements ITeardownable {
  public readonly enclaveName: string;

  public readonly kurtosisBin: string;

  public readonly packageRef: string;

  public executionRpcUrl?: string;

  public beaconApiUrl?: string;

  public chainId?: string;

  #argsDir?: string;

  constructor(
    enclaveName = `${ENCLAVE_NAME_PREFIX}${Math.random().toString(36).slice(2, 8)}`,
    kurtosisBin = DEFAULT_KURTOSIS_BIN,
    packageRef = DEFAULT_ETHEREUM_PACKAGE,
  ) {
    this.enclaveName = enclaveName;
    this.kurtosisBin = kurtosisBin;
    this.packageRef = packageRef;
    addTeardown(this);
  }

  public static isInstalled(kurtosisBin = DEFAULT_KURTOSIS_BIN): boolean {
    return spawnSync(kurtosisBin, ['version'], { stdio: 'ignore' }).status === 0;
  }

  public async launch(options?: {
    consensusClient?: EthereumConsensusClient;
    preset?: EthereumBeaconPreset;
    secondsPerSlot?: number;
    waitForFinalization?: boolean;
    prefundedAccounts?: Record<string, EthereumPrefundedAccount>;
  }): Promise<EthereumEndpoints> {
    const {
      consensusClient = 'lighthouse',
      preset = 'mainnet',
      secondsPerSlot,
      waitForFinalization = true,
      prefundedAccounts,
    } = options ?? {};
    const elPublicPortStart = await findFreePortRange(DEFAULT_EL_PORT_START, PORT_RANGE_SIZE);
    const clPublicPortStart = await findFreePortRange(DEFAULT_CL_PORT_START, PORT_RANGE_SIZE);

    this.#argsDir = await fs.mkdtemp(Path.join(os.tmpdir(), 'argon-ethereum-devnet-'));
    const argsFile = Path.join(this.#argsDir, 'network-params.yaml');
    await fs.writeFile(
      argsFile,
      renderEthereumArgs(
        elPublicPortStart,
        clPublicPortStart,
        consensusClient,
        preset,
        secondsPerSlot,
        waitForFinalization,
        prefundedAccounts,
      ),
    );

    await runCommand(
      this.kurtosisBin,
      ['run', '--enclave', this.enclaveName, this.packageRef, '--args-file', argsFile],
      KURTOSIS_RUN_TIMEOUT_MS,
    );

    const executionRpc = await waitForProbe(
      () => findExecutionRpcUrl(elPublicPortStart, PORT_RANGE_SIZE),
      PROBE_TIMEOUT_MS,
    );
    const beaconApi = await waitForProbe(
      () => findBeaconApiUrl(clPublicPortStart, PORT_RANGE_SIZE),
      PROBE_TIMEOUT_MS,
    );

    this.executionRpcUrl = executionRpc.url;
    this.beaconApiUrl = beaconApi.url;
    this.chainId = executionRpc.chainId;

    await waitForProbe(
      () => this.getBeacon('/eth/v1/beacon/genesis'),
      LIGHT_CLIENT_READY_TIMEOUT_MS,
    );

    return {
      executionRpcUrl: this.executionRpcUrl,
      beaconApiUrl: this.beaconApiUrl,
      chainId: this.chainId!,
    };
  }

  public async callExecution<TResult>(method: string, params: unknown[] = []): Promise<TResult> {
    const executionRpcUrl = this.executionRpcUrl;
    if (!executionRpcUrl) {
      throw new Error('Execution RPC URL is not available before launch');
    }

    const response = await fetch(executionRpcUrl, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        id: 1,
        jsonrpc: '2.0',
        method,
        params,
      }),
      signal: AbortSignal.timeout(10_000),
    });

    if (!response.ok) {
      throw new Error(`Execution RPC request failed for ${method}: ${response.status}`);
    }

    const body = (await response.json()) as {
      error?: { code?: number; message?: string };
      result?: TResult;
    };

    if (body.error) {
      throw new Error(
        `Execution RPC ${method} failed (${body.error.code ?? 'unknown'}): ${body.error.message ?? 'unknown error'}`,
      );
    }

    return body.result as TResult;
  }

  public async getBeacon<TResult>(path: string): Promise<TResult> {
    const beaconApiUrl = this.beaconApiUrl;
    if (!beaconApiUrl) {
      throw new Error('Beacon API URL is not available before launch');
    }

    const response = await fetch(new URL(path, `${beaconApiUrl}/`), {
      signal: AbortSignal.timeout(10_000),
    });

    if (!response.ok) {
      throw new Error(`Beacon API request failed for ${path}: ${response.status}`);
    }

    return (await response.json()) as TResult;
  }

  public async deployMintingGatewayFixture(options: {
    deployerPrivateKey: Hex;
    adminSafe?: Hex;
    guardianSafe?: Hex;
    initialMicrogonsPerArgonot?: bigint;
    seedArgonRecipient?: Hex;
    seedArgonAmountBaseUnits?: bigint;
  }): Promise<EthereumMintingGatewayFixture> {
    const { executionRpcUrl, chainId } = this;
    if (!executionRpcUrl || !chainId) {
      throw new Error('Ethereum devnet must be launched before deploying MintingGateway fixtures');
    }

    const account = privateKeyToAccount(options.deployerPrivateKey);
    const adminSafe = options.adminSafe ?? account.address;
    const guardianSafe = options.guardianSafe ?? adminSafe;
    const chain = createExecutionChain(chainId, executionRpcUrl);
    const publicClient = createPublicClient({
      chain,
      transport: http(executionRpcUrl),
    });
    const walletClient = createWalletClient({
      account,
      chain,
      transport: http(executionRpcUrl),
    });
    const bootstrapCouncil = {
      signers: [adminSafe],
      weights: [1n],
    } as const;
    const initialMicrogonsPerArgonot =
      options.initialMicrogonsPerArgonot ?? DEFAULT_INITIAL_MICROGONS_PER_ARGONOT;
    const bootstrapCouncilHash = hashMintingGatewayGlobalIssuanceCouncil({
      ...bootstrapCouncil,
      epochMicrogonsPerArgonot: initialMicrogonsPerArgonot,
    });

    const bootstrapImplementationAddress = await deployContract(walletClient, publicClient, {
      abi: mintingGatewayArtifact.abi,
      bytecode: mintingGatewayArtifact.bytecode,
      args: [zeroAddress, zeroAddress],
    });
    const initializeData = encodeFunctionData({
      abi: mintingGatewayArtifact.abi,
      functionName: 'initialize',
      args: [
        adminSafe,
        guardianSafe,
        bootstrapCouncilHash,
        BigInt(bootstrapCouncil.signers.length),
        1n,
        initialMicrogonsPerArgonot,
      ],
    });
    const gatewayAddress = await deployContract(walletClient, publicClient, {
      abi: transparentUpgradeableProxyArtifact.abi,
      bytecode: transparentUpgradeableProxyArtifact.bytecode,
      args: [bootstrapImplementationAddress, adminSafe, initializeData],
    });
    const proxyAdminAddress = getAddressFromStorage(
      await publicClient.getStorageAt({
        address: gatewayAddress,
        slot: ERC1967_ADMIN_SLOT,
      }),
    );
    const argonTokenAddress = await deployContract(walletClient, publicClient, {
      abi: argonTokenArtifact.abi,
      bytecode: argonTokenArtifact.bytecode,
      args: [gatewayAddress],
    });
    const argonotTokenAddress = await deployContract(walletClient, publicClient, {
      abi: argonotTokenArtifact.abi,
      bytecode: argonotTokenArtifact.bytecode,
      args: [gatewayAddress],
    });
    const finalImplementationAddress = await deployContract(walletClient, publicClient, {
      abi: mintingGatewayArtifact.abi,
      bytecode: mintingGatewayArtifact.bytecode,
      args: [argonTokenAddress, argonotTokenAddress],
    });
    const upgradeHash = await walletClient.sendTransaction({
      to: proxyAdminAddress,
      data: encodeFunctionData({
        abi: proxyAdminArtifact.abi,
        functionName: 'upgradeAndCall',
        args: [gatewayAddress, finalImplementationAddress, '0x'],
      }),
    });
    const upgradeReceipt = await waitForExecutionReceipt(publicClient, upgradeHash);
    if (upgradeReceipt.status !== 'success') {
      throw new Error('MintingGateway proxy upgrade failed');
    }

    if (options.seedArgonRecipient) {
      const mintHash = await walletClient.sendTransaction({
        to: gatewayAddress,
        data: encodeFunctionData({
          abi: mintingGatewayArtifact.abi,
          functionName: 'migrate',
          args: [
            {
              recipients: [options.seedArgonRecipient],
              amounts: [options.seedArgonAmountBaseUnits ?? DEFAULT_SEED_ARGON_AMOUNT_BASE_UNITS],
            },
            {
              recipients: [],
              amounts: [],
            },
          ],
        }),
      });
      const mintReceipt = await waitForExecutionReceipt(publicClient, mintHash);
      if (mintReceipt.status !== 'success') {
        throw new Error('MintingGateway migrate failed');
      }
    }

    return {
      argonTokenAddress,
      argonotTokenAddress,
      gatewayAddress,
    };
  }

  public async teardown(): Promise<void> {
    if (this.#argsDir) {
      await fs.rm(this.#argsDir, { recursive: true, force: true });
      this.#argsDir = undefined;
    }

    await runCommand(this.kurtosisBin, ['enclave', 'rm', '-f', this.enclaveName], 60_000, true);
  }
}

async function deployContract(
  walletClient: ReturnType<typeof createWalletClient>,
  publicClient: ReturnType<typeof createPublicClient>,
  request: {
    abi: Abi;
    bytecode: Hex;
    args: readonly unknown[];
  },
) {
  const hash = await walletClient.deployContract({
    ...request,
    account: walletClient.account!,
    chain: walletClient.chain,
  });
  const receipt = await waitForExecutionReceipt(publicClient, hash);

  if (receipt.status !== 'success' || !receipt.contractAddress) {
    throw new Error(`Contract deployment failed for ${request.bytecode.slice(0, 10)}`);
  }

  return receipt.contractAddress;
}

function getAddressFromStorage(value: Hex | undefined) {
  if (!value || value === '0x') {
    throw new Error('Missing proxy admin address in ERC1967 admin slot');
  }

  return getAddress(`0x${value.slice(-40)}`);
}

function renderEthereumArgs(
  elPublicPortStart: number,
  clPublicPortStart: number,
  consensusClient: EthereumConsensusClient,
  preset: EthereumBeaconPreset,
  secondsPerSlot: number | undefined,
  waitForFinalization: boolean,
  prefundedAccounts?: Record<string, EthereumPrefundedAccount>,
): string {
  const lines = [
    'participants:',
    '  - el_type: geth',
    `    cl_type: ${consensusClient}`,
    'network_params:',
    '  network: kurtosis',
    `  preset: ${preset}`,
    ...(secondsPerSlot ? [`  seconds_per_slot: ${secondsPerSlot}`] : []),
    ...(prefundedAccounts && Object.keys(prefundedAccounts).length > 0
      ? [`  prefunded_accounts: '${JSON.stringify(prefundedAccounts)}'`]
      : []),
    'additional_services: []',
    `wait_for_finalization: ${waitForFinalization ? 'true' : 'false'}`,
    'global_log_level: warn',
    'port_publisher:',
    '  el:',
    '    enabled: true',
    `    public_port_start: ${elPublicPortStart}`,
    '  cl:',
    '    enabled: true',
    `    public_port_start: ${clPublicPortStart}`,
  ];

  lines.push('');
  return lines.join('\n');
}

async function findExecutionRpcUrl(portStart: number, rangeSize: number): Promise<ProbeResult> {
  for (let port = portStart; port < portStart + rangeSize; port += 1) {
    const url = `http://127.0.0.1:${port}`;
    const response = await fetchJsonRpc(url, 'eth_chainId');
    if (typeof response === 'string') {
      return { url, chainId: response };
    }
  }

  throw new Error(
    `Unable to find an execution RPC endpoint in ${portStart}-${portStart + rangeSize - 1}`,
  );
}

async function findBeaconApiUrl(portStart: number, rangeSize: number): Promise<ProbeResult> {
  for (let port = portStart; port < portStart + rangeSize; port += 1) {
    const url = `http://127.0.0.1:${port}`;

    try {
      const response = await fetch(new URL('/eth/v1/node/version', `${url}/`), {
        signal: AbortSignal.timeout(2_000),
      });
      if (!response.ok) {
        continue;
      }

      const body = (await response.json()) as { data?: { version?: string } };
      if (body.data?.version) {
        return { url };
      }
    } catch {}
  }

  throw new Error(
    `Unable to find a Beacon API endpoint in ${portStart}-${portStart + rangeSize - 1}`,
  );
}

async function fetchJsonRpc(url: string, method: string): Promise<unknown> {
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        id: 1,
        jsonrpc: '2.0',
        method,
        params: [],
      }),
      signal: AbortSignal.timeout(2_000),
    });

    if (!response.ok) {
      return null;
    }

    const body = (await response.json()) as { result?: unknown };
    return body.result ?? null;
  } catch {
    return null;
  }
}

function createExecutionChain(chainId: string, executionRpcUrl: string) {
  return defineChain({
    id: Number.parseInt(chainId, 16),
    name: 'argon-test-ethereum',
    nativeCurrency: {
      name: 'Ether',
      symbol: 'ETH',
      decimals: 18,
    },
    rpcUrls: {
      default: {
        http: [executionRpcUrl],
      },
    },
  });
}

async function findFreePortRange(start: number, size: number): Promise<number> {
  for (let candidate = start; candidate < start + 1_000; candidate += size) {
    const ports = Array.from({ length: size }, (_, index) => candidate + index);
    const results = await Promise.all(ports.map(port => detectPort(port)));
    if (results.every((resolvedPort, index) => resolvedPort === ports[index])) {
      return candidate;
    }
  }

  throw new Error(`Unable to find a free port range starting near ${start}`);
}

async function waitForProbe<T>(probe: () => Promise<T>, timeoutMs: number): Promise<T> {
  const start = Date.now();
  let lastError: unknown;

  while (Date.now() - start < timeoutMs) {
    try {
      return await probe();
    } catch (error) {
      lastError = error;
      await delay(PROBE_INTERVAL_MS);
    }
  }

  throw lastError instanceof Error ? lastError : new Error('Timed out waiting for probe');
}

async function waitForExecutionReceipt(
  publicClient: ReturnType<typeof createPublicClient>,
  hash: Hex,
) {
  const start = Date.now();
  let lastError: Error | undefined;

  while (Date.now() - start < 120_000) {
    try {
      const receipt = await publicClient.getTransactionReceipt({ hash });
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

      if (
        !errorText.includes('indexing is in progress') &&
        !errorText.includes('Transaction receipt with hash') &&
        !errorText.includes('could not be found')
      ) {
        throw error;
      }

      lastError = error instanceof Error ? error : new Error(errorText);
    }

    await delay(500);
  }

  throw lastError ?? new Error(`Timed out waiting for execution receipt ${hash}`);
}

async function runCommand(
  command: string,
  args: string[],
  timeoutMs: number,
  allowFailure = false,
): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    const timeout = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`Command timed out: ${command} ${args.join(' ')}`));
    }, timeoutMs);

    child.stdout?.setEncoding('utf8');
    child.stderr?.setEncoding('utf8');
    child.stdout?.on('data', chunk => {
      stdout += chunk;
    });
    child.stderr?.on('data', chunk => {
      stderr += chunk;
    });

    child.on('error', error => {
      clearTimeout(timeout);
      reject(error);
    });

    child.on('exit', code => {
      clearTimeout(timeout);

      if (code === 0 || allowFailure) {
        resolve();
        return;
      }

      reject(
        new Error(
          [`Command failed: ${command} ${args.join(' ')}`, stdout.trim(), stderr.trim()]
            .filter(Boolean)
            .join('\n'),
        ),
      );
    });
  });
}

async function delay(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}
