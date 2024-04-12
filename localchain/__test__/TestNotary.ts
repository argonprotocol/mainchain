import {customAlphabet} from "nanoid";
import {Client} from 'pg';
import * as child_process from "node:child_process";
import {checkForExtrinsicSuccess, Keyring, KeyringPair, UlxClient} from "@ulixee/mainchain";
import fs from "node:fs";
import * as readline from "node:readline";
import {addTeardown, cleanHostForDocker, ITeardownable,} from "./testHelpers";
import * as process from "node:process";

const nanoid = customAlphabet('0123456789abcdefghijklmnopqrstuvwxyz', 4);
export default class TestNotary implements ITeardownable {
    public operator: KeyringPair;
    public registeredPublicKey: Uint8Array;
    public port: string;
    public containerName?: string;
    #dbName: string;
    #dbConnectionString: string;
    #childProcess: child_process.ChildProcessWithoutNullStreams;
    #stdioInterface: readline.Interface;

    public get containerSafeAddress(): string {
        return `ws://${this.containerName ?? '127.0.0.1'}:${this.port}`;
    }

    constructor(dbConnectionString?: string) {
        this.#dbConnectionString = dbConnectionString ?? process.env.NOTARY_DB_URL ?? "postgres://postgres:postgres@localhost:5432";
        addTeardown(this);
        this.operator = new Keyring({type: 'sr25519'}).createFromUri('//Bob');
        this.registeredPublicKey = new Keyring({type: 'ed25519'}).createFromUri('//Ferdie//notary').publicKey;
    }

    /**
     * Returns the localhost address of the notary (NOTE: not accessible from containers)
     */
    public async start(mainchainUrl: string, pathToNotaryBin?: string): Promise<string> {
        const client = await this.connect();
        try {

            let tries = 10;
            let db_name = '';
            while (tries > 0) {
                const uid = nanoid();
                db_name = `notary_${uid}`
                // check if the db path  notary_{id} exists
                const result = await client.query("SELECT 1 FROM pg_database WHERE datname = $1", [db_name]);
                if (result.rowCount == 0) {
                    break;
                }
                tries -= 1;
            }
            this.#dbName = db_name;
            await client.query(`CREATE DATABASE "${db_name}"`);
        } finally {
            await client.end();
        }

        let notaryPath = pathToNotaryBin ?? `${__dirname}/../../target/debug/ulx-notary`;
        let containerName: string;
        const execArgs = ['run', `--db-url=${this.#dbConnectionString}/${this.#dbName}`, `--dev`, `-t ${mainchainUrl}`];
        if (process.env.ULX_USE_DOCKER_BINS1) {
            try {
                child_process.execSync("docker network create ulx_test", {stdio: 'ignore'});
            } catch {
            }
            containerName = "notary_" + nanoid();
            this.containerName = containerName;
            notaryPath = `docker run --rm --network=ulx_test --name=${containerName} -p=9925 -e RUST_LOG=warn ghcr.io/ulixee/ulixee-notary:edge`;

            if (process.platform === "darwin") {
                this.#dbConnectionString = cleanHostForDocker(this.#dbConnectionString);
            }

        } else if (!fs.existsSync(notaryPath)) {
            throw new Error(`Notary binary not found at ${notaryPath}`);
        }

        let result = child_process.execSync(`${notaryPath} migrate --db-url ${this.#dbConnectionString}/${this.#dbName}`, {
            encoding: 'utf-8'
        });
        if (result.trim().length) {
            console.log(result.trim());
        }
        console.log("Notary >> connecting to mainchain '%s', db %s", mainchainUrl, `${this.#dbConnectionString}/${this.#dbName}`);


        if (process.env.ULX_USE_DOCKER_BINS1) {
            execArgs.unshift(...notaryPath.replace("docker run", "run").split(' '));
            execArgs.push('-b=0.0.0.0:9925')
            notaryPath = 'docker';
        }
        console.log('launching ulx-notary from', notaryPath, execArgs);
        this.#childProcess = child_process.spawn(notaryPath, execArgs, {
            stdio: ['ignore', 'pipe', 'pipe'],
            env: {...process.env, RUST_LOG: "warn"}
        });
        this.#childProcess.stdout.setEncoding('utf8');
        this.#childProcess.stderr.setEncoding('utf8');
        this.port = await new Promise<string>((resolve, reject) => {
            const onProcessError = (err: Error) => {
                console.warn("Error running notary", err);
                reject(err);
            };
            this.#childProcess.once('error', onProcessError);
            this.#childProcess.stderr.on('data', (data) => {
                console.warn('Notary >> %s', data);
                this.#childProcess.off('error', onProcessError);
                reject(data);
            });
            this.#stdioInterface = readline.createInterface({input: this.#childProcess.stdout}).on('line', line => {
                console.log('Notary >> %s', line);
                let match = line.match(/Listening on ([ws:/\d.]+)/);
                if (match) {
                    resolve(match[1].split(':').pop());
                }
            });
        });
        this.#childProcess.on('error', (err) => {
            throw err;
        });
        if (this.containerName) {
            this.port = child_process.execSync(`docker port ${this.containerName}`).toString().trim();
        }

        console.log('Notary listening on port', this.port);

        return `ws://127.0.0.1:${this.port}`;
    }

    public async register(client: UlxClient): Promise<void> {
        let address = new URL(this.containerSafeAddress);

        await new Promise<void>(async (resolve, reject) => {
            await client.tx.notaries.propose({
                public: this.registeredPublicKey,
                hosts: [address.href]
            }).signAndSend(this.operator, ({events, status}) => {
                if (status.isInBlock) {
                    checkForExtrinsicSuccess(events, client).then(() => {
                        console.log('Successful proposal of notary in block ' + status.asInBlock.toHex(), status.type);
                        resolve();
                    }, reject);
                } else {
                    console.log('Status of notary proposal: ' + status.type);
                }
            });
        });
    }

    public async teardown(): Promise<void> {
        this.#childProcess?.kill();
        this.#stdioInterface?.close();
        const client = await this.connect();
        try {
            await client.query(`DROP DATABASE "${this.#dbName}"`);
        } finally {
            await client.end();
        }
    }

    async connect(): Promise<Client> {
        const client = new Client({connectionString: this.#dbConnectionString});
        await client.connect();
        return client;
    }

}