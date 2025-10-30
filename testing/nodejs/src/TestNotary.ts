import { customAlphabet } from 'nanoid';
import type { Client } from 'pg';
import pg from 'pg';
import * as child_process from 'node:child_process';
import { ArgonClient, Keyring, KeyringPair, TxSubmitter } from '@argonprotocol/mainchain';
import * as fs from 'node:fs';
import * as readline from 'node:readline';
import {
  addTeardown,
  cleanHostForDocker,
  getDockerPortMapping,
  getProxy,
  ITeardownable,
  projectRoot,
} from './index';
import * as process from 'node:process';
import { Readable } from 'node:stream';
import * as Path from 'node:path';

const { Client: PgClient } = pg;

const nanoid = customAlphabet('0123456789abcdefghijklmnopqrstuvwxyz', 4);

export function createUid(): string {
  return nanoid();
}

export default class TestNotary implements ITeardownable {
  public operator?: KeyringPair;
  public ip = '127.0.0.1';
  public registeredPublicKey?: Uint8Array;
  public port?: string;
  public containerName?: string;
  public proxy?: string;
  #dbName?: string;
  #dbConnectionString: string;
  #childProcess?: child_process.ChildProcessByStdio<null, Readable, Readable>;
  #stdioInterface?: readline.Interface;

  public get address(): string {
    if (this.proxy) {
      const url = new URL(this.proxy);
      url.searchParams.set('target', `ws://${this.ip}:${this.port}`);
      return url.href;
    }
    return `ws://${this.ip}:${this.port}`;
  }

  constructor(dbConnectionString?: string) {
    this.#dbConnectionString =
      dbConnectionString ??
      process.env.NOTARY_DB_URL ??
      'postgres://postgres:postgres@localhost:5432';
    addTeardown(this);
  }

  /**
   * Returns the localhost address of the notary (NOTE: not accessible from containers)
   */
  public async start(options: {
    mainchainUrl: string;
    uuid: string;
    pathToNotaryBin?: string;
  }): Promise<string> {
    const { pathToNotaryBin, uuid, mainchainUrl } = options;
    this.operator = new Keyring({ type: 'sr25519' }).createFromUri('//Bob');
    this.registeredPublicKey = new Keyring({ type: 'ed25519' }).createFromUri(
      '//Ferdie//notary',
    ).publicKey;

    let notaryPath = pathToNotaryBin ?? Path.join(projectRoot(), 'target/debug/argon-notary');
    if (process.env.ARGON_USE_DOCKER_BINS) {
      this.containerName = 'notary_' + uuid;
      const addHost = process.env.ADD_DOCKER_HOST
        ? ` --add-host=host.docker.internal:host-gateway`
        : '';

      notaryPath = `docker run --rm -p=0:9925${addHost} --name=${this.containerName} -e RUST_LOG=warn ghcr.io/argonprotocol/argon-notary:dev`;

      this.#dbConnectionString = cleanHostForDocker(this.#dbConnectionString);
    } else if (!fs.existsSync(notaryPath)) {
      throw new Error(`Notary binary not found at ${notaryPath}`);
    }

    const client = await this.connect();
    let dbName = '';
    try {
      let tries = 10;
      while (tries > 0) {
        dbName = `notary_${uuid}`;
        // check if the db path  notary_{id} exists
        const result = await client.query('SELECT 1 FROM pg_database WHERE datname = $1', [dbName]);
        if (result.rowCount === 0) {
          break;
        }
        tries -= 1;
      }
      this.#dbName = dbName;
      await client.query(`CREATE DATABASE "${dbName}"`);
    } finally {
      await client.end();
    }

    const result = child_process.execSync(
      `${notaryPath} migrate --db-url ${this.#dbConnectionString}/${this.#dbName}`,
      {
        encoding: 'utf-8',
      },
    );
    if (result.trim().length) {
      console.log(result.trim());
    }
    console.log(
      "Notary >> connecting to mainchain '%s', db %s",
      mainchainUrl,
      `${this.#dbConnectionString}/${this.#dbName}`,
    );

    const bucketName = `notary-${uuid}`;
    const execArgs = [
      'run',
      `--db-url=${this.#dbConnectionString}/${this.#dbName}`,
      `--dev`,
      `-t ${mainchainUrl}`,
      `--archive-bucket=${bucketName}`,
      `--operator-address=${this.operator.address}`,
    ];
    if (process.env.ARGON_USE_DOCKER_BINS) {
      process.env.AWS_S3_ENDPOINT = 'http://host.docker.internal:9000';
      execArgs.unshift(...notaryPath.replace('docker run', 'run').split(' '));
      execArgs.push('-b=0.0.0.0:9925');

      notaryPath = 'docker';
    }
    if (process.env.AWS_S3_ENDPOINT) {
      execArgs.push(`--archive-endpoint=${process.env.AWS_S3_ENDPOINT}`);
    }
    this.#childProcess = child_process.spawn(notaryPath, execArgs, {
      stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env, RUST_LOG: 'warn' },
    });
    this.#childProcess.stdout.setEncoding('utf8');
    this.#childProcess.stderr.setEncoding('utf8');
    this.port = await new Promise<string>((resolve, reject) => {
      const onProcessError = (err: Error): void => {
        console.warn('Error running notary', err);
        reject(err);
      };
      this.#childProcess!.once('error', onProcessError);
      this.#childProcess!.stderr.on('data', data => {
        console.warn('Notary >> %s', data);
        if (typeof data === 'string' && data.startsWith('WARNING')) return;
        this.#childProcess!.off('error', onProcessError);
        reject(data);
      });
      this.#stdioInterface = readline
        .createInterface({ input: this.#childProcess!.stdout })
        .on('line', line => {
          console.log('Notary >> %s', line);
          const match = line.match(/Listening on ([ws:/\d.]+)/);
          if (match?.length ?? 0 > 0) {
            resolve(match![1].split(':').pop()!);
          }
        });
    });
    this.#childProcess.on('error', err => {
      throw err;
    });
    if (this.containerName) {
      this.port = await getDockerPortMapping(this.containerName, 9925);
      this.proxy = cleanHostForDocker(await getProxy());
    }

    return this.address;
  }

  public async register(client: ArgonClient): Promise<void> {
    const address = new URL(this.address);
    const result = await new TxSubmitter(
      client,
      client.tx.notaries.propose({
        public: this.registeredPublicKey,
        hosts: [address.href],
        name: 'Test Notary',
      }),
      this.operator!,
    ).submit();
    await result.waitForInFirstBlock;
  }

  public async teardown(): Promise<void> {
    this.#childProcess?.kill();
    this.#stdioInterface?.close();
    const client = await this.connect();
    try {
      await client.query(`DROP DATABASE "${this.#dbName}" WITH (FORCE)`);
    } finally {
      await client.end();
    }
    if (this.containerName) {
      try {
        child_process.execSync(`docker rm -f ${this.containerName}`);
      } catch {}
    }
  }

  async connect(): Promise<Client> {
    const client = new PgClient({ connectionString: this.#dbConnectionString });
    try {
      await client.connect();
    } catch (err) {
      console.error('ERROR connecting to postgres client', err);
      throw err;
    }
    return client;
  }
}
