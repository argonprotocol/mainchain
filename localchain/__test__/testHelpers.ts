import {checkForExtrinsicSuccess, Keyring, KeyringPair, UlxClient} from "@ulixee/mainchain";
import {Localchain} from "../index";
import TestNotary from "./TestNotary";
import type {KeypairType} from "@polkadot/util-crypto/types";

export interface ITeardownable {
    teardown(): Promise<void>;
}

const toTeardown: ITeardownable[] = [];

export async function teardown() {
    for (const t of toTeardown) {
        await t.teardown();
    }
    toTeardown.length = 0;
}

export class KeyringSigner {
    readonly keyring: Keyring;
    readonly defaultPair: KeyringPair;

    get address(): string {
        return this.defaultPair.address;
    }

    constructor(mainSuri: string, type: KeypairType = "sr25519") {
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
        dbPath: ':memory:',
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
                                let nonce = event.data.accountNonce.toPrimitive() as number;
                                resolve(nonce)
                            }
                        }

                    })
                    .catch(reject);
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

export function ipToInt32(ipAddress: string): number {
    let ip = ipAddress.split('.').reduce((ip, octet) => (ip << 8) + parseInt(octet, 10), 0);
    return ip >>> 0;
}