import './interfaces/augment-api';
import './interfaces/augment-types';
import './interfaces/types-lookup';
import type { KeyringPair, KeyringPair$Json } from '@polkadot/keyring/types';
import { ApiPromise, HttpProvider, Keyring, WsProvider } from '@polkadot/api';
import { cryptoWaitReady, decodeAddress, mnemonicGenerate } from '@polkadot/util-crypto';
import type { InterfaceTypes } from '@polkadot/types/types/registry';
import type { KeypairType } from '@polkadot/util-crypto/types';
import type { ProviderInterface } from '@polkadot/rpc-provider/types';

export { WageProtector } from './WageProtector';
export { TxSubmitter, TxResult, ITxProgressCallback } from './TxSubmitter';
export { Accountset, type ISubaccountMiner, type IMiningIndex } from './Accountset';
export { MiningBids } from './MiningBids';
export { AccountMiners } from './AccountMiners';
export { FrameCalculator } from './FrameCalculator';
export { BlockWatch, getAuthorFromHeader, getTickFromHeader } from './BlockWatch';
export * from './utils';
export { AccountRegistry } from './AccountRegistry';
export { Vault } from './Vault';
export { VaultMonitor } from './VaultMonitor';
export { CohortBidder } from './CohortBidder';
export { BidPool } from './BidPool';
export * from './BitcoinLocks';
export * from './keyringUtils';
export { Keyring, KeyringPair, KeyringPair$Json, KeypairType, mnemonicGenerate, decodeAddress };
export { setConfig, getConfig, type ArgonClientConfig } from './config';

export { u8aToHex, hexToU8a, u8aEq } from '@polkadot/util';

export * from '@polkadot/types/lookup';
export { GenericEvent, GenericBlock, GenericAddress } from '@polkadot/types/generic';
export {
  BTreeMap,
  Bytes,
  Compact,
  Enum,
  Null,
  Option,
  Result,
  Bool,
  Tuple,
  Range,
  Struct,
  Text,
  U256,
  U8aFixed,
  Vec,
  bool,
  i128,
  u128,
  u16,
  u32,
  u64,
  u8,
} from '@polkadot/types-codec';
export type {
  ISubmittableResult,
  IExtrinsic,
  SignerResult,
  Signer,
  ISignerPayload,
} from '@polkadot/types/types/extrinsic';
export { type ITuple, type Codec } from '@polkadot/types-codec/types';
export {
  type AccountId32,
  type Call,
  type H160,
  type H256,
  type MultiAddress,
  type Permill,
  type AccountId,
  type Header,
  type Block,
} from '@polkadot/types/interfaces/runtime';
export type { ExtrinsicOrHash, ExtrinsicStatus } from '@polkadot/types/interfaces/author';
export { type BlockHash } from '@polkadot/types/interfaces/chain';
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
  if (host.startsWith('http')) {
    provider = new HttpProvider(host);
  } else {
    provider = new WsProvider(host);
  }
  return await ApiPromise.create({ provider, noInitWarn: true });
}
