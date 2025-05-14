import './interfaces/augment-api';
import './interfaces/augment-types';
import './interfaces/types-lookup';
import type { KeyringPair, KeyringPair$Json } from '@polkadot/keyring/types';
import { ApiPromise, HttpProvider, Keyring, WsProvider } from '@polkadot/api';
import {
  cryptoWaitReady,
  decodeAddress,
  mnemonicGenerate,
} from '@polkadot/util-crypto';
import type { InterfaceTypes } from '@polkadot/types/types/registry';
import type { KeypairType } from '@polkadot/util-crypto/types';
import type { ProviderInterface } from '@polkadot/rpc-provider/types';

export * from '@polkadot/types-codec/types';
export { WageProtector } from './WageProtector';
export { TxSubmitter } from './TxSubmitter';
export { Accountset } from './Accountset';
export { MiningBids } from './MiningBids';
export { AccountMiners } from './AccountMiners';
export { FrameCalculator } from './FrameCalculator';
export {
  BlockWatch,
  getAuthorFromHeader,
  getTickFromHeader,
} from './BlockWatch';
export * from './utils';
export { AccountRegistry } from './AccountRegistry';
export { Vault } from './Vault';
export { VaultMonitor } from './VaultMonitor';
export { CohortBidder } from './CohortBidder';
export { CohortBidderHistory } from './CohortBidderHistory';
export { BidPool } from './BidPool';
export { BitcoinLocks } from './BitcoinLocks';
export { keyringFromSuri, createKeyringPair } from './keyringUtils';
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
export * from '@polkadot/types/interfaces';
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
  let provider: ProviderInterface;
  if (host.startsWith('http:')) {
    provider = new HttpProvider(host);
  } else {
    provider = new WsProvider(host);
  }
  return await ApiPromise.create({ provider, noInitWarn: true });
}
