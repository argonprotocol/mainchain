import {checkForExtrinsicSuccess, KeyringPair, UlxClient} from "@ulixee/mainchain";
import {Localchain} from "../index";
import TestNotary from "./TestNotary";

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
                        for (const { event } of events) {
                            if (client.events.chainTransfer.TransferToLocalchain.is(event)) {
                                let nonce = event.data.accountNonce.toNumber();
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
        client.tx.sudo.sudo(client.tx.notaries.activate(notary.operator.publicKey)).signAndSend(sudo, ({events, status}) => {
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