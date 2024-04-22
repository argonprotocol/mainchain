import {checkForExtrinsicSuccess, Keyring, KeyringPair, UlxClient} from "@ulixee/mainchain";
import {Localchain} from "../index";
import TestNotary from "./TestNotary";
import type {KeypairType} from "@polkadot/util-crypto/types";
import process from "node:process";
import HttpProxy from "http-proxy";
import child_process from "node:child_process";
import * as http from "node:http";
import * as url from "node:url";
import * as net from "node:net";
import {cryptoWaitReady} from '@polkadot/util-crypto';

export interface ITeardownable {
    teardown(): Promise<void>;
}

const toTeardown: ITeardownable[] = [];

let proxy: HttpProxy;
let proxyServer: http.Server;
export const describeIntegration = (process.env.SKIP_E2E === "true" || process.env.SKIP_E2E === "1") ? describe.skip : describe;


export async function getProxy() {
    if (!proxy) {
        proxy = HttpProxy.createProxyServer({
            changeOrigin: true,
            ws: true,
            autoRewrite: true,
        });
        proxy.on('error', () => null);
        proxyServer = http.createServer(function (req, res) {
            //parse query string and get targetUrl
            const queryData = url.parse(req.url, true).query;
            if (!queryData.target) {
                res.writeHead(500, {'Content-Type': 'text/plain'});
                res.end('Target parameter is required');
                return;
            }
            console.log('Proxying http request', queryData.target);
            proxy.web(req, res, {target: queryData.target as string});
        });
        proxyServer.on('upgrade', function (req, clientSocket, head) {
            const queryData = url.parse(req.url, true).query;
            const target = url.parse(queryData.target as string);
            proxy.ws(req, clientSocket, head, {
                target: target.href,
                ws: true
            });
            clientSocket.on('error', console.error);
        })
        await new Promise<void>(resolve => proxyServer.listen(0, resolve));
        toTeardown.push({
            teardown: () => new Promise<void>(resolve => {
                proxy.close();
                proxyServer.close(_ => null);
                proxy = null;
                proxyServer = null;
                resolve()
            })
        })
    }
    const port = (proxyServer.address() as net.AddressInfo).port;
    return `ws://host.docker.internal:${port}`;

}

export async function getDockerPortMapping(containerName: string, port: number): Promise<string> {
    return child_process.execSync(`docker port ${containerName} ${port}`, {encoding: 'utf8'}).trim().split(':').pop();
}


export async function teardown() {
    for (const t of toTeardown) {
        try {
            await t.teardown().catch(console.error);
        } catch {
        }
    }
    toTeardown.length = 0;
}

export function cleanHostForDocker(host: string, replacer = 'host.docker.internal'): string {
    if (process.env.ULX_USE_DOCKER_BINS) {
        return host.replace('localhost', replacer).replace('127.0.0.1', replacer).replace('0.0.0.0', replacer);
    }
    return host
}

export class KeyringSigner {
    readonly keyring: Keyring;
    readonly defaultPair: KeyringPair;

    get address(): string {
        return this.defaultPair.address;
    }

    private constructor(mainSuri: string, type: KeypairType = "sr25519") {
        this.keyring = new Keyring();
        this.defaultPair = this.keyring.addFromUri(mainSuri, {}, type);
        this.sign = this.sign.bind(this);
        this.derive = this.derive.bind(this);
    }

    async sign(address: string, message: Uint8Array): Promise<Uint8Array> {
        return this.keyring.getPair(address)?.sign(message, {withType: true})
    }

    async derive(hdPath: string): Promise<string> {
        const pair = this.defaultPair.derive(hdPath);
        return this.keyring.addPair(pair).address;
    }

    static async load(mainSuri: string, type: KeypairType = "sr25519"): Promise<KeyringSigner> {
        await cryptoWaitReady();
        return new KeyringSigner(mainSuri, type)
    }

}

export function addTeardown(teardownable: ITeardownable) {
    toTeardown.push(teardownable);
}

export function closeOnTeardown<T extends { close(): Promise<void> }>(closeable: T): T {
    addTeardown({teardown: () => closeable.close()});
    return closeable;
}

export function disconnectOnTeardown<T extends { disconnect(): Promise<void> }>(closeable: T): T {
    addTeardown({teardown: () => closeable.disconnect()});
    return closeable;
}

export async function getMainchainBalance(client: UlxClient, address: string): Promise<bigint> {
    const {data} = await client.query.system.account(address);
    return data.free.toBigInt();
}

export async function createLocalchain(mainchainUrl: string): Promise<Localchain> {
    const localchain = await Localchain.load({
        mainchainUrl,
        path: ':memory:',
    });
    closeOnTeardown(localchain);
    return localchain;
}

export async function transferToLocalchain(account: KeyringPair, amount: number, viaNotaryId: number, client: UlxClient): Promise<number> {
    return new Promise<number>((resolve, reject) => {
        client.tx.chainTransfer.sendToLocalchain(amount, viaNotaryId).signAndSend(account, ({events, status}) => {
            if (status.isFinalized) {
                checkForExtrinsicSuccess(events, client)
                    .then(() => {
                        for (const {event} of events) {
                            if (client.events.chainTransfer.TransferToLocalchain.is(event)) {
                                let transferId = event.data.transferId.toPrimitive() as number;
                                resolve(transferId)
                            }
                        }

                    })
                    .catch(reject);
            }
            if (status.isInBlock) {
                checkForExtrinsicSuccess(events, client).catch(reject);
            }
        });
    });
}

export async function activateNotary(sudo: KeyringPair, client: UlxClient, notary: TestNotary) {
    await notary.register(client);
    await new Promise<void>((resolve, reject) => {
        client.tx.sudo.sudo(client.tx.notaries.activate(notary.operator.publicKey)).signAndSend(sudo, ({
                                                                                                           events,
                                                                                                           status
                                                                                                       }) => {
            if (status.isInBlock) {
                checkForExtrinsicSuccess(events, client).then(() => {
                    console.log('Successful activation of notary in block ' + status.asInBlock.toHex());
                    resolve();
                }, reject);
            } else {
                console.log('Status of notary activation: ' + status.type);
            }
        })
    });
}