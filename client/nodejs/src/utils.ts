import BigNumber, * as BN from 'bignumber.js';
import type { ArgonClient } from './index';
import type { DispatchError } from '@polkadot/types/interfaces';
import { EventRecord } from '@polkadot/types/interfaces/system';

const { ROUND_FLOOR } = BN;

export const MICROGONS_PER_ARGON = 1_000_000;

export function formatArgons(microgons: bigint | number): string {
  if (microgons === undefined || microgons === null) return 'na';
  const isNegative = microgons < 0;
  let format = BigNumber(microgons.toString())
    .abs()
    .div(MICROGONS_PER_ARGON)
    .toFormat(2, ROUND_FLOOR);
  if (format.endsWith('.00')) {
    format = format.slice(0, -3);
  }
  return `${isNegative ? '-' : ''}₳${format}`;
}

export async function gettersToObject<T>(obj: T): Promise<T> {
  if (obj === null || obj === undefined || typeof obj !== 'object') return obj;

  const keys = [];

  for (const key in obj) {
    keys.push(key);
  }

  if (Symbol.iterator in obj) {
    const iterableToArray = [];
    // @ts-expect-error - it should iterate by virtue of Symbol.iterator
    for (const item of obj) {
      iterableToArray.push(await gettersToObject(item));
    }
    return iterableToArray as T;
  }

  const result = {} as any;
  for (const key of keys) {
    const descriptor = Object.getOwnPropertyDescriptor(obj, key);
    // Skip functions
    if (descriptor && typeof descriptor.value === 'function') {
      continue;
    }
    const value = descriptor && descriptor.get ? descriptor.get.call(obj) : obj[key as keyof T];
    if (typeof value === 'function') continue;

    result[key] = await gettersToObject(value);
  }
  return result as T;
}

export function dispatchErrorToString(client: ArgonClient, error: DispatchError) {
  let message = error.toString();
  if (error.isModule) {
    const decoded = client.registry.findMetaError(error.asModule);
    const { docs, name, section } = decoded;
    message = `${section}.${name}: ${docs.join(' ')}`;
  }
  return message;
}

// ExtrinsicError
export class ExtrinsicError extends Error {
  constructor(
    public readonly errorCode: string,
    public readonly details?: string,
    public readonly batchInterruptedIndex?: number,
    public readonly txFee: bigint = 0n,
  ) {
    super(errorCode);
  }

  public override toString() {
    if (this.batchInterruptedIndex !== undefined) {
      return `${this.errorCode} ${this.details ?? ''} (Batch interrupted at index ${this.batchInterruptedIndex})`;
    }
    return `${this.errorCode} ${this.details ?? ''}`;
  }
}

export function dispatchErrorToExtrinsicError(
  client: ArgonClient,
  error: DispatchError,
  batchInterruptedIndex?: number,
  txFee?: bigint,
) {
  if (error.isModule) {
    const decoded = client.registry.findMetaError(error.asModule);
    const { docs, name, section } = decoded;
    return new ExtrinsicError(`${section}.${name}`, docs.join(' '), batchInterruptedIndex, txFee);
  }
  return new ExtrinsicError(error.toString(), undefined, batchInterruptedIndex, txFee);
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

        reject(new Error(`${event.section}.${event.method}:: ExtrinsicFailed:: ${errorInfo}`));
      }
    }
  });
}
