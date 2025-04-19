import * as child_process from 'node:child_process';
import { Keyring, KeyringPair } from '@argonprotocol/mainchain';
import * as fs from 'node:fs';
import * as readline from 'node:readline';
import { addTeardown, ITeardownable, projectRoot } from './index';
import * as process from 'node:process';
import * as Path from 'node:path';
import { Readable } from 'node:stream';

export default class TestOracle implements ITeardownable {
  public static BitcoinOperator = '//Dave';
  public static PriceIndexOperator = '//Eve';
  public operator?: KeyringPair;
  public port?: string;
  #childProcess?: child_process.ChildProcessByStdio<null, Readable, Readable>;
  #stdioInterface?: readline.Interface;

  constructor() {
    addTeardown(this);
  }

  public async start(
    service: 'price-index' | 'bitcoin',
    options: {
      mainchainUrl: string;
      bitcoinRpcUrl?: string;
      pathToBin?: string;
      env?: Record<string, string>;
    },
  ) {
    const { pathToBin, mainchainUrl, bitcoinRpcUrl } = options;
    const operatorSuri =
      service == 'bitcoin'
        ? TestOracle.BitcoinOperator
        : TestOracle.PriceIndexOperator;
    this.operator = new Keyring({ type: 'sr25519' }).createFromUri(
      operatorSuri,
    );
    const binPath =
      pathToBin ?? Path.join(projectRoot(), 'target/debug/argon-oracle');
    if (!fs.existsSync(binPath)) {
      throw new Error(`Oracle binary not found at ${binPath}`);
    }
    console.log(`Starting ${service} oracle`);

    const execArgs: string[] = ['--dev', '-t', mainchainUrl, service];
    if (service == 'bitcoin') {
      if (!bitcoinRpcUrl) {
        throw new Error('Bitcoin RPC URL is required for bitcoin oracle');
      }
      execArgs.push('--bitcoin-rpc-url', bitcoinRpcUrl);
    } else {
      execArgs.push('--simulate-prices');
    }
    this.#childProcess = child_process.spawn(binPath, execArgs, {
      stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env, RUST_LOG: 'info', ...options.env },
    });
    this.#childProcess.stdout.setEncoding('utf8');
    this.#childProcess.stderr.setEncoding('utf8');
    this.#childProcess!.stderr.on('data', data => {
      console.warn('%sOracle >> %s', service, data);
    });
    this.#stdioInterface = readline
      .createInterface({ input: this.#childProcess!.stdout })
      .on('line', line => {
        console.log('%sOracle >> %s', service, line);
      });

    this.#childProcess.on('error', err => {
      throw err;
    });
  }

  public async teardown(): Promise<void> {
    this.#childProcess?.kill();
    this.#stdioInterface?.close();
  }
}
