import * as fs from 'node:fs';
import { ChildProcess, execSync, spawn } from 'node:child_process';
import * as Path from 'node:path';
import * as readline from 'node:readline';
import {
  addTeardown,
  cleanHostForDocker,
  disconnectOnTeardown,
  getDockerPortMapping,
  getProxy,
  ITeardownable,
  projectRoot,
} from './index';
import { detectPort } from 'detect-port';
import { customAlphabet } from 'nanoid';
import * as lockfile from 'proper-lockfile';
import { createUid } from './TestNotary';
import { type ArgonClient, getClient } from '@argonprotocol/mainchain';

const nanoid = customAlphabet('0123456789abcdefghijklmnopqrstuvwxyz', 4);

const lockPath = Path.join(process.cwd(), '.port-lock');

export default class TestMainchain implements ITeardownable {
  public ip = '127.0.0.1';
  public port?: string;
  public loglevel = 'warn';
  public uuid: string;
  #binPath: string;
  #process?: ChildProcess;
  #interfaces: readline.Interface[] = [];
  containerName?: string;
  proxy?: string;
  #bitcoind?: ChildProcess;
  bitcoinPort?: number;
  #bitcoinDir?: string;

  public get address(): string {
    if (this.proxy) {
      const url = new URL(this.proxy);
      url.searchParams.set('target', `ws://${this.ip}:${this.port}`);
      return url.href;
    }
    return `ws://${this.ip}:${this.port}`;
  }

  constructor(binPath?: string) {
    this.#binPath = binPath ?? Path.join(projectRoot(), `target/debug/argon-node`);
    this.#binPath = Path.resolve(this.#binPath);
    if (!process.env.ARGON_USE_DOCKER_BINS && !fs.existsSync(this.#binPath)) {
      throw new Error(`Mainchain binary not found at ${this.#binPath}`);
    }
    this.uuid = createUid();
    addTeardown(this);
  }

  public getBitcoinClient(): BitcoinRpcClient {
    return new BitcoinRpcClient(`http://localhost:${this.bitcoinPort}`, 'bitcoin', 'bitcoin');
  }

  /**
   * Launch and return the localhost url. NOTE: this url will not work cross-docker. You need to use the containerAddress property
   * @param options
   * @param options.miningThreads - number of threads to use for mining
   * @param options.bootnodes - bootnodes to use for the mainchain
   */
  public async launch(options?: {
    miningThreads?: number;
    bootnodes?: string;
    author?: string;
    launchBitcoin?: boolean;
  }): Promise<string> {
    const { miningThreads = 1, bootnodes, author = 'alice', launchBitcoin = false } = options ?? {};
    let port = 0;
    let rpcPort = 0;
    let execArgs: string[] = [];
    let containerName: string;
    if (process.env.ARGON_USE_DOCKER_BINS) {
      containerName = 'miner_' + nanoid();
      this.containerName = containerName;
      this.#binPath = 'docker';
      port = 33344;
      rpcPort = 9944;
      execArgs = [
        'run',
        '--rm',
        `--name=${containerName}`,
        `-p=0:${port}`,
        `-p=0:${rpcPort}`,
        '-e',
        `RUST_LOG=${this.loglevel},sc_rpc_server=info`,
        'ghcr.io/argonprotocol/argon-miner:dev',
      ];

      if (process.env.ADD_DOCKER_HOST) {
        execArgs.splice(2, 0, `--add-host=host.docker.internal:host-gateway`);
      }
    }

    const bitcoinRpcUrl = await this.startBitcoin(launchBitcoin);
    execArgs.push(
      '--dev',
      '--validator',
      `--${author}`,
      `--compute-miners=${miningThreads}`,
      `--port=${port}`,
      `--rpc-port=${rpcPort}`,
      '--rpc-external',
      '--no-mdns',
      '--no-telemetry',
      '--no-prometheus',
      '--unsafe-rpc-external',
      '--rpc-methods=unsafe',
      `--bitcoin-rpc-url=${bitcoinRpcUrl}`,
      `--notebook-archive-hosts=http://127.0.0.1:9000/${this.uuid}`,
    );
    if (bootnodes) {
      execArgs.push(`--bootnodes=${bootnodes}`);
    }
    this.#process = spawn(this.#binPath, execArgs, {
      stdio: ['ignore', 'pipe', 'pipe', 'ignore'],
      env: { ...process.env, RUST_LOG: `${this.loglevel},sc_rpc_server=info` },
    });

    this.#process.stderr!.setEncoding('utf8');
    this.#process.stdout!.setEncoding('utf8');
    this.#process.stdout!.on('data', data => {
      console.log('Main >> %s', data);
    });

    const int1 = readline.createInterface({ input: this.#process.stdout! }).on('line', line => {
      if (line) console.log('Main >> %s', line);
    });
    this.#interfaces.push(int1);

    this.port = await new Promise<string>((resolve, reject) => {
      this.#process!.on('error', err => {
        console.warn('Error running mainchain', err);
        reject(err);
      });

      const int2 = readline.createInterface({ input: this.#process!.stderr! }).on('line', line => {
        console.log('Main >> %s', line);
        const match = line.match(/Running JSON-RPC server: addr=([\d.:]+)/);
        if (match) {
          const ipv4 = match[1].split(',').at(0);
          resolve(ipv4!.split(':').pop()!);
        }
      });
      this.#interfaces.push(int2);
    });
    if (this.containerName) {
      this.port = await getDockerPortMapping(this.containerName, rpcPort);
      this.proxy = cleanHostForDocker(await getProxy());
    }

    console.log(`argon Node listening at ${this.address}`);
    return this.address;
  }

  public async client(): Promise<ArgonClient> {
    const client = await getClient(this.address);
    disconnectOnTeardown(client);
    return client;
  }

  public async bootAddress(): Promise<string | undefined> {
    const client = await this.client();
    const bootAddress = await client.rpc.system.localListenAddresses();

    for (const address of bootAddress) {
      const addr = address.toString();
      if (addr.includes('127.0.0.1')) {
        return addr;
      }
    }
    return undefined;
  }

  public async teardown(): Promise<void> {
    if (process.env.ARGON_USE_DOCKER_BINS) {
      try {
        execSync(`docker rm -f ${this.containerName}`);
      } catch {}
    }
    const launchedProcess = this.#process;
    if (launchedProcess) {
      launchedProcess?.kill();
      try {
        launchedProcess.stdio.forEach(io => io?.destroy());
      } catch {}
      launchedProcess.unref();
    }

    this.#process?.kill();
    this.#process?.unref();
    this.#bitcoind?.kill();
    this.#bitcoind?.unref();
    if (this.#bitcoinDir) {
      await fs.promises.rm(this.#bitcoinDir, {
        recursive: true,
        force: true,
      });
    }
    for (const i of this.#interfaces) {
      i.close();
    }
  }

  private async startBitcoin(launchBitcoin: boolean): Promise<string> {
    let rpcPort = 14338;
    if (launchBitcoin) {
      // Ensure lock file exists
      fs.closeSync(fs.openSync(lockPath, 'w'));
      const release = await lockfile.lock(lockPath, { retries: 10 });
      try {
        rpcPort = await detectPort();
        const path = execSync(Path.join(projectRoot(), `target/debug/argon-testing-bitcoin`), {
          encoding: 'utf8',
        }).trim();

        const tmpDir = fs.mkdtempSync('/tmp/argon-bitcoin-' + this.uuid);

        console.log('Starting bitcoin node at %s. Data %s', path, tmpDir);
        this.#bitcoind = spawn(
          path,
          [
            '-regtest',
            '-fallbackfee=0.0001',
            '-listen=0',
            `-datadir=${tmpDir}`,
            '-blockfilterindex',
            '-txindex',
            `-rpcport=${rpcPort}`,
            '-rpcbind=0.0.0.0',
            '-rpcallowip=0.0.0.0/0',
            '-rpcuser=bitcoin',
            '-rpcpassword=bitcoin',
          ],
          {
            stdio: ['ignore', 'pipe', 'pipe'],
          },
        );
        this.#bitcoind.stderr!.setEncoding('utf8');
        this.#bitcoind.stdout!.setEncoding('utf8');
        this.#bitcoind.stdout!.on('data', data => {
          console.log('Bitcoin >> %s', data);
        });
        this.#bitcoind.stderr!.on('data', data => {
          console.error('Bitcoin >> %s', data);
        });
        this.#bitcoinDir = tmpDir;
      } finally {
        // Release the lock file
        await release();
      }
    }
    this.bitcoinPort = rpcPort;
    return cleanHostForDocker(`http://bitcoin:bitcoin@localhost:${rpcPort}`);
  }
}

class BitcoinRpcClient {
  #rpcUrl: string;
  #authorization: string;

  constructor(rpcUrl: string, username: string, password: string) {
    this.#rpcUrl = rpcUrl;
    this.#authorization = `Basic ${Buffer.from(`${username}:${password}`).toString('base64')}`;
  }

  public async command<TMethod extends BitcoinRpcMethod>(
    method: TMethod,
    ...params: Parameters<BitcoinRpcMethods[TMethod]>
  ): Promise<ReturnType<BitcoinRpcMethods[TMethod]>>;

  public async command<TResult = unknown>(method: string, ...params: unknown[]): Promise<TResult>;

  public async command(method: string, ...params: unknown[]): Promise<unknown> {
    const response = await fetch(this.#rpcUrl, {
      method: 'POST',
      headers: {
        authorization: this.#authorization,
        'content-type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '1.0',
        id: `${method}-${Date.now()}`,
        method,
        params,
      }),
    });

    const body = await response.text();

    let payload: BitcoinRpcPayload<unknown> | undefined;
    if (body) {
      try {
        payload = JSON.parse(body) as BitcoinRpcPayload<unknown>;
      } catch {
        payload = undefined;
      }
    }

    if (payload?.error) {
      const httpStatus = response.ok ? '' : ` with HTTP ${response.status}`;
      throw new Error(
        `Bitcoin RPC ${method} failed${httpStatus} (${payload.error.code}): ${payload.error.message}`,
      );
    }

    if (!response.ok) {
      throw new Error(`Bitcoin RPC ${method} failed with HTTP ${response.status}`);
    }

    if (!payload) {
      throw new Error(`Bitcoin RPC ${method} returned an invalid JSON response`);
    }

    return payload.result;
  }
}

type BitcoinRpcMethod = keyof BitcoinRpcMethods;

// bitcoin-core@5.0.0 only types `command()` as generic `any`, so we keep a
// narrow local signature map for the handful of Bitcoin Core RPC fields our tests
// actually read. Uninspected responses stay `unknown`.
type BitcoinRpcMethods = {
  createwallet(walletName: string): unknown;
  loadwallet(walletName: string): unknown;
  getnewaddress(): string;
  generatetoaddress(blockCount: number, address: string): unknown;
  getbalances(): unknown;
  walletcreatefundedpsbt(
    inputs: unknown[],
    outputs: Record<string, number>,
    locktime: number,
    options: BitcoinWalletCreateFundedPsbtOptions,
  ): BitcoinFundedPsbtResult;
  walletprocesspsbt(psbt: string): BitcoinProcessedPsbtResult;
  decodepsbt(psbt: string): BitcoinDecodedPsbtResult;
  finalizepsbt(psbt: string): BitcoinFinalizedPsbtResult;
  sendrawtransaction(transactionHex: string): string;
  getrawtransaction(txid: string, verbose: true): BitcoinRawTransactionResult;
  gettransaction(txid: string): unknown;
};

type BitcoinRpcPayload<TResult> = {
  result: TResult;
  error?: { code: number; message: string } | null;
};

type BitcoinWalletCreateFundedPsbtOptions = {
  lockUnspents?: boolean;
  feeRate?: number;
};

type BitcoinFundedPsbtResult = {
  psbt: string;
};

type BitcoinProcessedPsbtResult = {
  psbt: string;
  complete: boolean;
};

type BitcoinDecodedPsbtResult = {
  inputs?: unknown[];
};

type BitcoinFinalizedPsbtResult = {
  hex: string;
  txid?: string;
};

type BitcoinRawTransactionResult = {
  txid: string;
};
