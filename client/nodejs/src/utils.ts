import BigNumber, * as BN from 'bignumber.js';
import type { ArgonClient, GenericEvent } from './index';
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

export function formatPercent(x: BigNumber | undefined): string {
  if (!x) return 'na';
  return `${x.times(100).decimalPlaces(3)}%`;
}

type NonNullableProps<T> = {
  [K in keyof T]-?: Exclude<T[K], undefined | null>;
};

export function filterUndefined<T extends Record<string, any>>(
  obj: Partial<T>,
): NonNullableProps<T> {
  return Object.fromEntries(
    Object.entries(obj).filter(([_, value]) => value !== undefined && value !== null),
  ) as NonNullableProps<T>;
}

export async function gettersToObject<T>(obj: T): Promise<T> {
  if (obj === null || obj === undefined || typeof obj !== 'object') return obj;

  const keys = [];
  // eslint-disable-next-line guard-for-in
  for (const key in obj) {
    keys.push(key);
  }

  if (Symbol.iterator in obj) {
    const iterableToArray = [];
    // @ts-ignore
    for (const item of obj) {
      iterableToArray.push(await gettersToObject(item));
    }
    return iterableToArray as any;
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
  return result;
}

export function toFixedNumber(
  value: string | number | BigNumber, // accept string to avoid early precision loss
  decimals: number,
): bigint {
  const factor = new BigNumber(10).pow(decimals);
  const bn = new BigNumber(value);
  // truncate toward 0; use ROUND_FLOOR if you really need floor for positives
  const int = bn.times(factor).integerValue(BigNumber.ROUND_DOWN);
  return BigInt(int.toFixed(0));
}

export function convertNumberToFixedU128(value: number): bigint {
  return toFixedNumber(value, 18);
}

export function convertFixedU128ToBigNumber(fixedU128: bigint): BigNumber {
  const decimalFactor = new BigNumber(10).pow(new BigNumber(18)); // Fixed point precision (18 decimals)
  const rawValue = new BigNumber(fixedU128.toString()); // Parse the u128 string value into BN
  // Convert the value to fixed-point
  return rawValue.div(decimalFactor);
}

export function convertPermillToBigNumber(permill: bigint): BigNumber {
  const decimalFactor = new BigNumber(1_000_000);
  const rawValue = new BigNumber(permill.toString()); // Parse the u128 string value into BN
  // Convert the value to fixed-point
  return rawValue.div(decimalFactor);
}

export function convertNumberToPermill(value: number): bigint {
  return toFixedNumber(value, 6);
}

export function eventDataToJson(event: GenericEvent): any {
  const obj = {} as any;
  event.data.forEach((data, index) => {
    const name = event.data.names?.[index];
    obj[name ?? `${index}`] = data.toJSON();
  });
  return obj;
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
) {
  if (error.isModule) {
    const decoded = client.registry.findMetaError(error.asModule);
    const { docs, name, section } = decoded;
    return new ExtrinsicError(`${section}.${name}`, docs.join(' '), batchInterruptedIndex);
  }
  return new ExtrinsicError(error.toString(), undefined, batchInterruptedIndex);
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

/**
 * JSON with support for BigInt in JSON.stringify and JSON.parse
 */
export class JsonExt {
  public static stringify(obj: any, space?: number): string {
    return JSON.stringify(
      obj,
      (_, v) => {
        if (typeof v === 'bigint') {
          return `${v}n`; // Append 'n' to indicate BigInt
        }
        // convert Uint8Array objects to a JSON representation
        if (v instanceof Uint8Array) {
          return {
            type: 'Buffer',
            data: Array.from(v), // Convert Uint8Array to an array of numbers
          };
        }
        return v;
      },
      space,
    );
  }

  public static parse<T = any>(str: string): T {
    return JSON.parse(str, (_, v) => {
      if (typeof v === 'string' && v.match(/^\d+n$/)) {
        return BigInt(v.slice(0, -1));
      }
      // rehydrate Uint8Array objects
      if (typeof v === 'object' && v !== null && v.type === 'Buffer' && Array.isArray(v.data)) {
        return Uint8Array.from(v.data);
      }
      return v;
    });
  }
}

export function createNanoEvents<Events extends EventsMap = DefaultEvents>(): TypedEmitter<Events> {
  return new TypedEmitter<Events>();
}

export class TypedEmitter<Events extends EventsMap = DefaultEvents> {
  private events: Partial<{ [E in keyof Events]: Events[E][] }> = {};

  emit<K extends keyof Events>(this: this, event: K, ...args: Parameters<Events[K]>): void {
    for (const cb of this.events[event] || []) {
      cb(...args);
    }
  }

  on<K extends keyof Events>(this: this, event: K, cb: Events[K]): () => void {
    (this.events[event] ||= []).push(cb);
    return () => {
      this.events[event] = this.events[event]?.filter(i => cb !== i);
    };
  }
}

interface EventsMap {
  [event: string]: any;
}

interface DefaultEvents extends EventsMap {
  [event: string]: (...args: any) => void;
}
