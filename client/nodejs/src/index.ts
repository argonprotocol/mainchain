import "./interfaces/augment-api.js";
import "./interfaces/augment-types.js";
import "./interfaces/types-lookup.js";
import {KeyringPair} from "@polkadot/keyring/types";
import {ApiPromise, Keyring, WsProvider} from '@polkadot/api';
import {EventRecord} from "@polkadot/types/interfaces/system";
import {InterfaceTypes} from '@polkadot/types/types/registry';

export {Keyring, KeyringPair};

export * from "@polkadot/types";

export * from '@polkadot/types/lookup';
export {InterfaceTypes as interfaces};

export type ArgonClient = ApiPromise;


export async function getClient(host: string): Promise<ArgonClient> {
    const provider = new WsProvider(host);
    return await ApiPromise.create({provider, noInitWarn: true});
}

export function checkForExtrinsicSuccess(events: EventRecord[], client: ArgonClient): Promise<void> {
    return new Promise((resolve, reject) => {
        for (const {event} of events) {
            if (client.events.system.ExtrinsicSuccess.is(event)) {
                resolve();
            } else if (client.events.system.ExtrinsicFailed.is(event)) {
                // extract the data for this event
                const [dispatchError] = event.data;
                let errorInfo = dispatchError.toString();

                if (dispatchError.isModule) {
                    const decoded = client.registry.findMetaError(dispatchError.asModule);
                    errorInfo = `${decoded.section}.${decoded.name}`;
                }

                reject(new Error(`${event.section}.${event.method}:: ExtrinsicFailed:: ${errorInfo}`));
            }
        }
    });
}
