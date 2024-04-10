import * as fs from "node:fs";
import {ChildProcess, spawn} from "node:child_process";
import * as path from "node:path";
import * as readline from "node:readline";
import {addTeardown, ITeardownable,} from "./testHelpers";
import process from "node:process";
import child_process from "node:child_process";
import {customAlphabet} from "nanoid";

const nanoid = customAlphabet('0123456789abcdefghijklmnopqrstuvwxyz', 4);

export default class TestMainchain implements ITeardownable {
    public containerName?: string;
    public port: string;
    public loglevel = 'warn';
    #binPath: string;
    #process: ChildProcess;
    #interfaces: readline.Interface[] = [];

    public get containerSafeAddress(): string {
        return `ws://${this.containerName ?? '127.0.0.1'}:${this.port}`;
    }

    constructor(binPath?: string) {
        this.#binPath = binPath ?? `${__dirname}/../../target/debug/ulx-node`;
        this.#binPath = path.resolve(this.#binPath);
        if (!process.env.ULX_USE_DOCKER_BINS && !fs.existsSync(this.#binPath)) {
            throw new Error(`Mainchain binary not found at ${this.#binPath}`);
        }
        addTeardown(this);
    }

    /**
     * Launch and return the localhost url. NOTE: this url will not work cross-docker. You need to use the containerAddress property
     * @param miningThreads
     */
    public async launch(miningThreads = 4): Promise<string> {
        let execArgs = ['--dev', '--alice', `--miners=${miningThreads}`, '--port=0', '--rpc-port=0', '--rpc-external'];
        let containerName: string;
        if (process.env.ULX_USE_DOCKER_BINS) {
            try {
                child_process.execSync("docker network create ulx_test", {stdio: 'ignore'});
            } catch {
            }
            containerName = "miner__" + nanoid();
            this.containerName = containerName;
            this.#binPath = 'docker';
            execArgs = ['run', '--rm', `--name=${containerName}`, '-p=9945', '-p=33344', '--network=ulx_test', '-e', `RUST_LOG=info,sc_rpc_server=info`,
                'ulixee/ulixee-miner:edge', '--dev', '--alice', `--miners=${miningThreads}`, '--port=33344', '--rpc-port=9945', '--rpc-external'];
        }
        console.log('launching ulx-node from', this.#binPath, execArgs);
        this.#process = spawn(this.#binPath, execArgs, {
            stdio: ['ignore', 'pipe', 'pipe', "ignore"],
            env: {...process.env, RUST_LOG: 'warn,sc_rpc_server=info'}
        });

        this.#process.stderr.setEncoding('utf8');
        this.#process.stdout.setEncoding('utf8');
        this.#process.stdout.on('data', (data) => {
            console.log('Main >> %s', data);
        });

        const i = readline.createInterface({input: this.#process.stdout}).on('line', line => {
            if (line) console.log('Main >> %s', line);
        });
        this.#interfaces.push(i);

        let isReady = false;
        this.port = await new Promise<string>((resolve, reject) => {
            this.#process.on('error', (err) => {
                console.warn("Error running mainchain", err);
                reject(err);
            });

            const i = readline.createInterface({input: this.#process.stderr}).on('line', line => {
                if (isReady && (this.loglevel === 'warn' || this.loglevel === 'error')) {
                    const showWarn = line.includes('WARN') && this.loglevel !== 'error';
                    if (showWarn || this.loglevel.includes('ERROR')) {
                        console.log('Main >> %s', line);
                    }
                    return;
                }
                console.log('Main >> %s', line);
                let match = line.match(/Running JSON-RPC server: addr=([\d.:]+)/);
                if (match) {
                    isReady = true;
                    resolve(match[1].split(':').pop());
                }
            });
            this.#interfaces.push(i);
        });
        if (this.containerName) {
            this.port = child_process.execSync(`docker port ${this.containerName} 9945`).toString().trim().split(':').pop();
        }

        console.log('Node listening on port', this.port);
        return `ws://127.0.0.1:${this.port}`;
    }

    public async teardown(): Promise<void> {
        this.#process?.kill();
        for (const i of this.#interfaces) {
            i.close();
        }
    }
}