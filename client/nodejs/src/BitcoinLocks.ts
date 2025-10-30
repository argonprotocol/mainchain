import {
  type ArgonClient,
  type ArgonPrimitivesBitcoinBitcoinNetwork,
  formatArgons,
  type KeyringPair,
  MICROGONS_PER_ARGON,
  TxSubmitter,
  Vault,
} from './index';
import { GenericEvent } from '@polkadot/types';
import { ISubmittableOptions } from './TxSubmitter';
import { TxResult } from './TxResult';
import { u8aToHex } from '@polkadot/util';
import { ApiDecoration } from '@polkadot/api/types';
import { PriceIndex } from './PriceIndex';

export const SATS_PER_BTC = 100_000_000n;

export class BitcoinLocks {
  constructor(readonly client: ArgonClient) {}

  async getUtxoIdFromEvents(events: GenericEvent[]) {
    for (const event of events) {
      if (this.client.events.bitcoinLocks.BitcoinLockCreated.is(event)) {
        return event.data.utxoId.toNumber();
      }
    }
    return undefined;
  }

  async getMarketRate(priceIndex: PriceIndex, satoshis: number | bigint): Promise<bigint> {
    return priceIndex.getBtcMicrogonPrice(satoshis);
  }

  async getRedemptionRate(
    priceIndex: PriceIndex,
    details: { satoshis: bigint; peggedPrice?: bigint },
  ): Promise<bigint> {
    const { satoshis, peggedPrice } = details;
    // scale inputs
    const satsPerArgon = Number(SATS_PER_BTC) / MICROGONS_PER_ARGON;
    let price = Number(priceIndex.btcUsdPrice);
    price = (price / satsPerArgon) * Number(satoshis);

    if (peggedPrice !== undefined && peggedPrice < price) {
      price = Number(peggedPrice);
    }

    const r = Number(priceIndex.rValue);

    let multiplier: number;

    if (r >= 1) {
      // Case 1: no penalty
      multiplier = 1;
    } else if (r >= 0.9) {
      // Case 2: quadratic curve
      // Formula: 20r² - 38r + 19
      multiplier = 20 * (r * r) - 38 * r + 19;
    } else if (r >= 0.01) {
      // Case 3: rational linear formula
      // Formula: (0.5618r + 0.3944) / r
      multiplier = (0.5618 * r + 0.3944) / r;
    } else {
      // Case 4: extreme deviation
      // Formula: (1 / r) * (0.576r + 0.4)
      multiplier = (1 / r) * (0.576 * r + 0.4);
    }

    return BigInt(Math.floor(price * multiplier));
  }

  async getMarketRateApi(satoshis: bigint): Promise<bigint> {
    const client = this.client;
    const sats = client.createType('U64', satoshis.toString());
    const marketRate = await client.rpc.state.call('BitcoinApis_market_rate', sats.toHex(true));
    const rate = client.createType('Option<U128>', marketRate);
    if (!rate.isSome) {
      throw new Error('Market rate not available');
    }
    return rate.value.toBigInt();
  }

  async getRedemptionRateApi(satoshis: bigint): Promise<bigint> {
    const client = this.client;
    const sats = client.createType('U64', satoshis.toString());
    const marketRate = await client.rpc.state.call('BitcoinApis_redemption_rate', sats.toHex(true));
    const rate = client.createType('Option<U128>', marketRate);
    if (!rate.isSome) {
      throw new Error('Redemption rate not available');
    }
    return rate.value.toBigInt();
  }

  async getConfig(): Promise<IBitcoinLockConfig> {
    const client = this.client;
    const bitcoinNetwork = await client.query.bitcoinUtxos.bitcoinNetwork();
    return {
      lockReleaseCosignDeadlineFrames:
        client.consts.bitcoinLocks.lockReleaseCosignDeadlineFrames.toNumber(),
      pendingConfirmationExpirationBlocks:
        client.consts.bitcoinUtxos.maxPendingConfirmationBlocks.toNumber(),
      tickDurationMillis: await client.query.ticks
        .genesisTicker()
        .then(x => x.tickDurationMillis.toNumber()),
      bitcoinNetwork,
    };
  }

  async getBitcoinConfirmedBlockHeight(): Promise<number> {
    return await this.client.query.bitcoinUtxos
      .confirmedBitcoinBlockTip()
      .then(x => x.value?.blockHeight.toNumber() ?? 0);
  }

  /**
   * Gets the UTXO reference by ID.
   * @param utxoId - The UTXO ID to look up.
   * @param clientAtHeight - Optional client at the block height to query the UTXO reference at a specific point in time.
   * @return An object containing the transaction ID and output index, or undefined if not found.
   * @return.txid - The Bitcoin transaction ID of the UTXO.
   * @return.vout - The output index of the UTXO in the transaction.
   * @return.bitcoinTxid - The Bitcoin transaction ID of the UTXO formatted in little endian
   */
  async getUtxoRef(
    utxoId: number,
    clientAtHeight?: ApiDecoration<'promise'>,
  ): Promise<{ txid: string; vout: number; bitcoinTxid: string } | undefined> {
    const client = clientAtHeight ?? this.client;
    const refRaw = await client.query.bitcoinUtxos.utxoIdToRef(utxoId);
    if (!refRaw) {
      return;
    }
    const ref = refRaw.unwrap();

    const txid = u8aToHex(ref.txid);
    const bitcoinTxid = u8aToHex(ref.txid.reverse());
    const vout = ref.outputIndex.toNumber();
    return { txid, vout, bitcoinTxid };
  }

  async getReleaseRequest(
    utxoId: number,
    clientAtHeight?: ApiDecoration<'promise'>,
  ): Promise<IReleaseRequestDetails | undefined> {
    const client = clientAtHeight ?? this.client;
    const requestMaybe = await client.query.bitcoinLocks.lockReleaseRequestsByUtxoId(utxoId);
    if (!requestMaybe.isSome) {
      return undefined;
    }
    const request = requestMaybe.unwrap();
    return {
      toScriptPubkey: request.toScriptPubkey.toHex(),
      bitcoinNetworkFee: request.bitcoinNetworkFee.toBigInt(),
      dueFrame: request.cosignDueFrame.toNumber(),
      vaultId: request.vaultId.toNumber(),
      redemptionPrice: request.redemptionPrice.toBigInt(),
    };
  }

  async submitVaultSignature(
    args: {
      utxoId: number;
      vaultSignature: Uint8Array;
      argonKeyring: KeyringPair;
    } & ISubmittableOptions,
  ): Promise<TxResult> {
    const { utxoId, vaultSignature, argonKeyring, txProgressCallback } = args;
    const client = this.client;
    if (!vaultSignature || vaultSignature.byteLength < 70 || vaultSignature.byteLength > 73) {
      throw new Error(
        `Invalid vault signature length: ${vaultSignature.byteLength}. Must be 70-73 bytes.`,
      );
    }
    const signature = u8aToHex(vaultSignature);
    const tx = client.tx.bitcoinLocks.cosignRelease(utxoId, signature);
    const submitter = new TxSubmitter(client, tx, argonKeyring);

    return await submitter.submit(args);
  }

  async getBitcoinLock(utxoId: number): Promise<IBitcoinLock | undefined> {
    const utxoRaw = await this.client.query.bitcoinLocks.locksByUtxoId(utxoId);
    if (!utxoRaw.isSome) {
      return;
    }
    const utxo = utxoRaw.unwrap();
    const p2shBytesPrefix = '0020';
    const wscriptHash = utxo.utxoScriptPubkey.asP2wsh.wscriptHash.toHex().replace('0x', '');
    const p2wshScriptHashHex = `0x${p2shBytesPrefix}${wscriptHash}`;
    const vaultId = utxo.vaultId.toNumber();
    const peggedPrice = utxo.peggedPrice.toBigInt();
    const liquidityPromised = utxo.liquidityPromised.toBigInt();
    const ownerAccount = utxo.ownerAccount.toHuman();
    const satoshis = utxo.satoshis.toBigInt();
    const vaultPubkey = utxo.vaultPubkey.toHex();
    const vaultClaimPubkey = utxo.vaultClaimPubkey.toHex();
    const ownerPubkey = utxo.ownerPubkey.toHex();
    const [fingerprint, cosign_hd_index, claim_hd_index] = utxo.vaultXpubSources;
    const vaultXpubSources = {
      parentFingerprint: new Uint8Array(fingerprint),
      cosignHdIndex: cosign_hd_index.toNumber(),
      claimHdIndex: claim_hd_index.toNumber(),
    };

    const securityFees = utxo.securityFees.toBigInt();
    const vaultClaimHeight = utxo.vaultClaimHeight.toNumber();
    const openClaimHeight = utxo.openClaimHeight.toNumber();
    const createdAtHeight = utxo.createdAtHeight.toNumber();
    const isVerified = utxo.isVerified.toJSON();
    const isRejectedNeedsRelease = utxo.isRejectedNeedsRelease.toJSON();
    const fundHoldExtensionsByBitcoinExpirationHeight = Object.fromEntries(
      [...utxo.fundHoldExtensions.entries()].map(([x, y]) => [x.toNumber(), y.toBigInt()]),
    );

    return {
      utxoId,
      p2wshScriptHashHex,
      vaultId,
      peggedPrice,
      liquidityPromised,
      ownerAccount,
      satoshis,
      vaultPubkey,
      vaultClaimPubkey,
      ownerPubkey,
      vaultXpubSources,
      vaultClaimHeight,
      openClaimHeight,
      createdAtHeight,
      securityFees,
      isVerified,
      isRejectedNeedsRelease,
      fundHoldExtensionsByBitcoinExpirationHeight,
    };
  }

  /**
   * Finds the cosign signature for a vault lock by UTXO ID. Optionally waits for the signature
   * @param utxoId - The UTXO ID of the bitcoin lock
   * @param waitForSignatureMillis - Optional timeout in milliseconds to wait for the signature. If -1, waits indefinitely.
   */
  async findVaultCosignSignature(
    utxoId: number,
    waitForSignatureMillis?: number,
  ): Promise<{ blockHeight: number; signature: Uint8Array } | undefined> {
    const client = this.client;
    const releaseHeight = await client.query.bitcoinLocks.lockReleaseCosignHeightById(utxoId);
    if (releaseHeight.isSome) {
      const releaseHeightValue = releaseHeight.unwrap().toNumber();
      const signature = await this.getVaultCosignSignature(utxoId, releaseHeightValue);
      if (signature) {
        return { blockHeight: releaseHeightValue, signature };
      }
    }

    if (!waitForSignatureMillis) {
      return undefined;
    }

    return await new Promise(async (resolve, reject) => {
      let timeout: NodeJS.Timeout | undefined;
      const unsub = await client.rpc.chain.subscribeNewHeads(header => {
        const atHeight = header.number.toNumber();
        this.getVaultCosignSignature(utxoId, atHeight)
          .then(signature => {
            if (signature) {
              unsub?.();
              clearTimeout(timeout);
              resolve({ signature, blockHeight: atHeight });
            }
          })
          .catch(err => {
            console.error(`Error checking for cosign signature at height ${atHeight}:`, err);
          });
      });
      if (waitForSignatureMillis !== -1) {
        timeout = setTimeout(() => {
          unsub?.();
          reject(new Error(`Timeout waiting for cosign signature for UTXO ID ${utxoId}`));
        }, waitForSignatureMillis);
      }
    });
  }

  async blockHashAtHeight(atHeight: number): Promise<string | undefined> {
    const client = this.client;

    for (let i = 0; i < 10; i++) {
      const currentHeight = await client.query.system.number().then(x => x.toNumber());
      if (atHeight > currentHeight) {
        console.warn(
          `Requested block height ${atHeight} is greater than current height ${currentHeight}. Retrying...`,
        );
        await new Promise(resolve => setTimeout(resolve, 1000)); // wait 1 second before retrying
        continue;
      }
      const hash = await client.rpc.chain.getBlockHash(atHeight).then(x => x.toHex());
      if (hash === '0x0000000000000000000000000000000000000000000000000000000000000000') {
        console.warn(`Block hash not found for height ${atHeight}. Retrying...`);
        await new Promise(resolve => setTimeout(resolve, 1000)); // wait 1 second before retrying
        continue;
      }
      return hash;
    }
    return undefined;
  }

  async getVaultCosignSignature(utxoId: number, atHeight: number): Promise<Uint8Array | undefined> {
    const client = this.client;

    const blockHash = await this.blockHashAtHeight(atHeight);
    if (!blockHash) {
      console.warn(`Block hash not found for height ${atHeight}`);
      return undefined;
    }

    const blockEvents = await client.at(blockHash).then(api => api.query.system.events());
    for (const event of blockEvents) {
      if (client.events.bitcoinLocks.BitcoinUtxoCosigned.is(event.event)) {
        const { utxoId: id, signature } = event.event.data;
        if (id.toNumber() === utxoId) {
          return new Uint8Array(signature);
        }
      }
    }
    return undefined;
  }

  async findPendingMints(utxoId: number): Promise<bigint[]> {
    const pendingMint = await this.client.query.mint.pendingMintUtxos();
    const mintsPending: bigint[] = [];
    for (const [utxoIdRaw, _accountId, mintAmountRaw] of pendingMint) {
      if (utxoIdRaw.toNumber() === utxoId) {
        mintsPending.push(mintAmountRaw.toBigInt());
      }
    }
    return mintsPending;
  }

  async createInitializeLockTx(args: {
    vault: Vault;
    priceIndex: PriceIndex;
    ownerBitcoinPubkey: Uint8Array;
    satoshis: bigint;
    argonKeyring: KeyringPair;
    reducedBalanceBy?: bigint;
    tip?: bigint;
  }) {
    const { vault, priceIndex, argonKeyring, satoshis, tip = 0n, ownerBitcoinPubkey } = args;
    const client = this.client;
    if (ownerBitcoinPubkey.length !== 33) {
      throw new Error(
        `Invalid Bitcoin key length: ${ownerBitcoinPubkey.length}. Must be a compressed pukey (33 bytes).`,
      );
    }

    const tx = client.tx.bitcoinLocks.initialize(vault.vaultId, satoshis, ownerBitcoinPubkey);
    const submitter = new TxSubmitter(
      client,
      client.tx.bitcoinLocks.initialize(vault.vaultId, satoshis, ownerBitcoinPubkey),
      argonKeyring,
    );
    const marketPrice = await this.getMarketRate(priceIndex, satoshis);
    const isVaultOwner = argonKeyring.address === vault.operatorAccountId;
    const securityFee = isVaultOwner ? 0n : vault.calculateBitcoinFee(marketPrice);

    const { canAfford, availableBalance, txFee } = await submitter.canAfford({
      tip,
      unavailableBalance: securityFee + (args.reducedBalanceBy ?? 0n),
      includeExistentialDeposit: true,
    });
    return { tx, securityFee, txFee, canAfford, availableBalance, txFeePlusTip: txFee + tip };
  }

  async getBitcoinLockFromTxResult(txResult: TxResult): Promise<{
    lock: IBitcoinLock;
    createdAtHeight: number;
    txResult: TxResult;
  }> {
    await txResult.waitForFinalizedBlock;
    const blockHeight = txResult.blockNumber!;
    const utxoId = (await this.getUtxoIdFromEvents(txResult.events)) ?? 0;
    if (utxoId === 0) {
      throw new Error('Bitcoin lock creation failed, no UTXO ID found in transaction events');
    }
    const lock = await this.getBitcoinLock(utxoId);
    if (!lock) {
      throw new Error(`Lock with ID ${utxoId} not found after initialization`);
    }
    return { lock, createdAtHeight: blockHeight, txResult };
  }

  async initializeLock(
    args: {
      vault: Vault;
      priceIndex: PriceIndex;
      ownerBitcoinPubkey: Uint8Array;
      argonKeyring: KeyringPair;
      satoshis: bigint;
    } & ISubmittableOptions,
  ): Promise<{
    getLock(): Promise<{ lock: IBitcoinLock; createdAtHeight: number }>;
    txResult: TxResult;
    securityFee: bigint;
  }> {
    const { argonKeyring } = args;
    const client = this.client;

    const { tx, securityFee, canAfford, txFeePlusTip } = await this.createInitializeLockTx(args);
    if (!canAfford) {
      throw new Error(
        `Insufficient funds to initialize bitcoin lock. Required security fee: ${formatArgons(securityFee)}, Tx fee plus tip: ${formatArgons(txFeePlusTip)}`,
      );
    }
    const submitter = new TxSubmitter(client, tx, argonKeyring);
    const txResult = await submitter.submit({ logResults: true, ...args });

    return {
      getLock: () => this.getBitcoinLockFromTxResult(txResult),
      txResult,
      securityFee,
    };
  }

  async requiredSatoshisForArgonLiquidity(
    priceIndex: PriceIndex,
    argonAmount: bigint,
  ): Promise<bigint> {
    /**
     * If 1_000_000 microgons are available, and the market rate is 100 microgons per satoshi, then
     * 1_000_000 / 100 = 10_000 satoshis needed
     */
    const marketRatePerBitcoin = priceIndex.getBtcMicrogonPrice(SATS_PER_BTC);
    return (argonAmount * SATS_PER_BTC) / marketRatePerBitcoin;
  }

  async requestRelease(
    args: {
      lock: IBitcoinLock;
      priceIndex: PriceIndex;
      releaseRequest: IReleaseRequest;
      argonKeyring: KeyringPair;
    } & ISubmittableOptions,
  ): Promise<TxResult> {
    const client = this.client;
    const {
      lock,
      priceIndex,
      releaseRequest: { bitcoinNetworkFee, toScriptPubkey },
      argonKeyring,
      tip = 0n,
    } = args;

    if (!toScriptPubkey.startsWith('0x')) {
      throw new Error('toScriptPubkey must be a hex string starting with 0x');
    }

    const submitter = new TxSubmitter(
      client,
      client.tx.bitcoinLocks.requestRelease(lock.utxoId, toScriptPubkey, bitcoinNetworkFee),
      argonKeyring,
    );

    const redemptionPrice = await this.getRedemptionRate(priceIndex, lock);

    const canAfford = await submitter.canAfford({
      tip,
      unavailableBalance: BigInt(redemptionPrice),
    });

    if (!canAfford.canAfford) {
      throw new Error(
        `Insufficient funds to release lock. Available: ${formatArgons(canAfford.availableBalance)}, Required: ${formatArgons(redemptionPrice + canAfford.txFee + tip)}`,
      );
    }
    return submitter.submit({
      logResults: true,
      ...args,
    });
  }

  async releasePrice(
    priceIndex: PriceIndex,
    lock: { satoshis: bigint; peggedPrice: bigint },
  ): Promise<bigint> {
    return await this.getRedemptionRate(priceIndex, lock);
  }

  async getRatchetPrice(
    lock: IBitcoinLock,
    priceIndex: PriceIndex,
    vault: Vault,
  ): Promise<{ burnAmount: bigint; ratchetingFee: bigint; marketRate: bigint }> {
    const { createdAtHeight, vaultClaimHeight, peggedPrice, satoshis } = lock;
    const client = this.client;
    const marketRate = await this.getMarketRate(priceIndex, BigInt(satoshis));

    let ratchetingFee = vault.terms.bitcoinBaseFee;
    let burnAmount = 0n;
    // ratchet up
    if (marketRate > peggedPrice) {
      const lockFee = vault.calculateBitcoinFee(marketRate);
      const currentBitcoinHeight = await client.query.bitcoinUtxos
        .confirmedBitcoinBlockTip()
        .then(x => x.unwrap().blockHeight.toNumber());
      const blockLength = vaultClaimHeight - createdAtHeight;
      const elapsed = (currentBitcoinHeight - createdAtHeight) / blockLength;
      const remainingDuration = 1 - elapsed;
      ratchetingFee = BigInt(remainingDuration * Number(lockFee));
    } else {
      burnAmount = await this.releasePrice(priceIndex, lock);
    }

    return {
      ratchetingFee,
      burnAmount,
      marketRate,
    };
  }

  async ratchet(
    args: {
      lock: IBitcoinLock;
      priceIndex: PriceIndex;
      argonKeyring: KeyringPair;
      vault: Vault;
    } & ISubmittableOptions,
  ): Promise<{
    txResult: TxResult;
    getRatchetResult: () => Promise<{
      securityFee: bigint;
      txFee: bigint;
      newPeggedPrice: bigint;
      liquidityPromised: bigint;
      pendingMint: bigint;
      burned: bigint;
      blockHeight: number;
      bitcoinBlockHeight: number;
    }>;
  }> {
    const { lock, priceIndex, argonKeyring, tip = 0n, vault, txProgressCallback } = args;
    const client = this.client;

    const ratchetPrice = await this.getRatchetPrice(lock, priceIndex, vault);
    const txSubmitter = new TxSubmitter(
      client,
      client.tx.bitcoinLocks.ratchet(lock.utxoId),
      argonKeyring,
    );
    const canAfford = await txSubmitter.canAfford({
      tip,
      unavailableBalance: BigInt(ratchetPrice.burnAmount + ratchetPrice.ratchetingFee),
    });
    if (!canAfford.canAfford) {
      throw new Error(
        `Insufficient funds to ratchet lock. Available: ${formatArgons(canAfford.availableBalance)}, Required: ${formatArgons(
          ratchetPrice.burnAmount + ratchetPrice.ratchetingFee,
        )}`,
      );
    }

    const txResult = await txSubmitter.submit(args);
    const getRatchetResult = async () => {
      const blockHash = await txResult.waitForFinalizedBlock;
      const ratchetEvent = txResult.events.find(x =>
        client.events.bitcoinLocks.BitcoinLockRatcheted.is(x),
      );
      if (!ratchetEvent) {
        throw new Error(`Ratchet event not found in transaction events`);
      }
      const api = await client.at(blockHash);
      const bitcoinBlockHeight = await api.query.bitcoinUtxos
        .confirmedBitcoinBlockTip()
        .then(x => x.unwrap().blockHeight.toNumber());
      const {
        amountBurned,
        liquidityPromised: liquidityPromisedRaw,
        newPeggedPrice,
        originalPeggedPrice,
        securityFee,
      } = ratchetEvent.data;
      const liquidityPromised = liquidityPromisedRaw.toBigInt();
      let mintAmount = liquidityPromised;
      if (liquidityPromised > originalPeggedPrice.toBigInt()) {
        mintAmount -= originalPeggedPrice.toBigInt();
      }
      return {
        txFee: txResult.finalFee ?? 0n,
        blockHeight: txResult.blockNumber!,
        bitcoinBlockHeight,
        pendingMint: mintAmount,
        liquidityPromised,
        newPeggedPrice: newPeggedPrice.toBigInt(),
        burned: amountBurned.toBigInt(),
        securityFee: securityFee.toBigInt(),
      };
    };
    return {
      txResult,
      getRatchetResult,
    };
  }
}

export interface IBitcoinLockConfig {
  lockReleaseCosignDeadlineFrames: number;
  pendingConfirmationExpirationBlocks: number;
  tickDurationMillis: number;
  bitcoinNetwork: ArgonPrimitivesBitcoinBitcoinNetwork;
}
export interface IReleaseRequest {
  toScriptPubkey: string;
  bitcoinNetworkFee: bigint;
}

export interface IReleaseRequestDetails extends IReleaseRequest {
  dueFrame: number;
  vaultId: number;
  redemptionPrice: bigint;
}

export interface IBitcoinLock {
  utxoId: number;
  p2wshScriptHashHex: string;
  vaultId: number;
  peggedPrice: bigint;
  liquidityPromised: bigint;
  ownerAccount: string;
  satoshis: bigint;
  vaultPubkey: string;
  securityFees: bigint;
  vaultClaimPubkey: string;
  ownerPubkey: string;
  vaultXpubSources: {
    parentFingerprint: Uint8Array;
    cosignHdIndex: number;
    claimHdIndex: number;
  };
  vaultClaimHeight: number;
  openClaimHeight: number;
  createdAtHeight: number;
  isVerified: boolean;
  isRejectedNeedsRelease: boolean;
  fundHoldExtensionsByBitcoinExpirationHeight: Record<number, bigint>;
}
