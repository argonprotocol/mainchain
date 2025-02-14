import './interfaces/augment-api.js';
import './interfaces/augment-types.js';
import './interfaces/types-lookup.js';
import { KeyringPair, KeyringPair$Json } from '@polkadot/keyring/types';
import { ApiPromise, Keyring, WsProvider } from '@polkadot/api';
import {
  cryptoWaitReady,
  decodeAddress,
  mnemonicGenerate,
} from '@polkadot/util-crypto';
import { EventRecord } from '@polkadot/types/interfaces/system';
import { InterfaceTypes } from '@polkadot/types/types/registry';
import { KeypairType } from '@polkadot/util-crypto/types';

export { WageProtector } from './WageProtector';

export {
  Keyring,
  KeyringPair,
  KeyringPair$Json,
  KeypairType,
  mnemonicGenerate,
  decodeAddress,
};

export * from '@polkadot/types';
export * from '@polkadot/types/lookup';
export { InterfaceTypes as interfaces };

export type ArgonClient = ApiPromise;

/**
 * Wait for the crypto library to be ready (requires wasm, which needs async loading in commonjs)
 */
export async function waitForLoad(): Promise<void> {
  await cryptoWaitReady();
}

/**
 * Get a client for the given host
 * @param host The host to connect to
 * @returns The client
 */
export async function getClient(host: string): Promise<ArgonClient> {
  const provider = new WsProvider(host);
  return await ApiPromise.create({ provider, noInitWarn: true });
}

/**
 * Check for an extrinsic success event in the given events. Helpful to validate the result of an extrinsic inclusion in a block (it will be included even if it fails)
 * @param events The events to check
 * @param client The client to use
 * @returns A promise that resolves if the extrinsic was successful, and rejects if it failed
 */
export function checkForExtrinsicSuccess(
  events: EventRecord[],
  client: ArgonClient,
): Promise<void> {
  return new Promise((resolve, reject) => {
    for (const { event } of events) {
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

        reject(
          new Error(
            `${event.section}.${event.method}:: ExtrinsicFailed:: ${errorInfo}`,
          ),
        );
      }
    }
  });
}
