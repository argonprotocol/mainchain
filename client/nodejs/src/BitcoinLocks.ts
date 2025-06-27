import {
  Accountset,
  type ArgonClient,
  ArgonPrimitivesBitcoinBitcoinNetwork,
  type KeyringPair,
  TxSubmitter,
  VaultMonitor,
} from './index';
import { formatArgons } from './utils';
import { Vault } from './Vault';
import { GenericEvent } from '@polkadot/types';
import { TxResult } from './TxSubmitter';

export const SATS_PER_BTC = 100_000_000n;

export class BitcoinLocks {
  constructor(readonly client: Promise<ArgonClient>) {}

  async getUtxoIdFromEvents(events: GenericEvent[]) {
    const client = await this.client;
    for (const event of events) {
      if (client.events.bitcoinLocks.BitcoinLockCreated.is(event)) {
        return event.data.utxoId.toNumber();
      }
    }
    return undefined;
  }

  async getMarketRate(satoshis: bigint): Promise<bigint> {
    const client = await this.client;
    const sats = client.createType('U64', satoshis.toString());
    const marketRate = await client.rpc.state.call('BitcoinApis_market_rate', sats.toHex(true));
    const rate = client.createType('Option<U128>', marketRate);
    if (!rate.isSome) {
      throw new Error('Market rate not available');
    }
    return rate.value.toBigInt();
  }

  async getRedemptionRate(satoshis: bigint): Promise<bigint> {
    const client = await this.client;
    const sats = client.createType('U64', satoshis.toString());
    const marketRate = await client.rpc.state.call('BitcoinApis_redemption_rate', sats.toHex(true));
    const rate = client.createType('Option<U128>', marketRate);
    if (!rate.isSome) {
      throw new Error('Redemption rate not available');
    }
    return rate.value.toBigInt();
  }

  async getConfig(): Promise<IBitcoinLockConfig> {
    const client = await this.client;
    const bitcoinNetwork = await client.query.bitcoinUtxos.bitcoinNetwork();
    return {
      releaseExpirationBlocks:
        client.consts.bitcoinLocks.lockReleaseCosignDeadlineBlocks.toNumber(),
      tickDurationMillis: await client.query.ticks
        .genesisTicker()
        .then(x => x.tickDurationMillis.toNumber()),
      bitcoinNetwork,
    };
  }

  async getBitcoinConfirmedBlockHeight(): Promise<number> {
    const client = await this.client;
    return await client.query.bitcoinUtxos
      .confirmedBitcoinBlockTip()
      .then(x => x.value?.blockHeight.toNumber() ?? 0);
  }

  /**
   * Gets the UTXO reference by ID.
   * @param utxoId - The UTXO ID to look up.
   * @param atHeight - Optional block height to query the UTXO reference at a specific point in time.
   * @return An object containing the transaction ID and output index, or undefined if not found.
   * @return.txid - The Bitcoin transaction ID of the UTXO.
   * @return.vout - The output index of the UTXO in the transaction.
   * @return.bitcoinTxid - The Bitcoin transaction ID of the UTXO formatted in little endian
   */
  async getUtxoRef(
    utxoId: number,
    atHeight?: number,
  ): Promise<{ txid: string; vout: number; bitcoinTxid: string } | undefined> {
    let client = await this.client;
    if (atHeight !== undefined) {
      const blockHash = await client.query.system.blockHash(atHeight);
      client = (await client.at(blockHash)) as ArgonClient;
    }
    const refRaw = await client.query.bitcoinUtxos.utxoIdToRef(utxoId);
    if (!refRaw) {
      return;
    }
    const ref = refRaw.unwrap();

    const txid = Buffer.from(ref.txid).toString('hex');
    const btcTxid = Buffer.from(ref.txid.reverse()).toString('hex');
    const vout = ref.outputIndex.toNumber();
    return { txid: `0x${txid}`, vout, bitcoinTxid: `0x${btcTxid}` };
  }

  async getReleaseRequest(
    utxoId: number,
    atHeight?: number,
  ): Promise<IReleaseRequestDetails | undefined> {
    let client = await this.client;
    if (atHeight !== undefined) {
      const blockHash = await client.query.system.blockHash(atHeight);
      client = (await client.at(blockHash)) as ArgonClient;
    }
    const locksPendingRelease = await client.query.bitcoinLocks.locksPendingReleaseByUtxoId();

    for (const [id, request] of locksPendingRelease.entries()) {
      if (id.toNumber() === utxoId) {
        return {
          toScriptPubkey: request.toScriptPubkey.toHex(),
          bitcoinNetworkFee: request.bitcoinNetworkFee.toBigInt(),
          dueBlockHeight: request.cosignDueBlock.toNumber(),
          vaultId: request.vaultId.toNumber(),
          redemptionPrice: request.redemptionPrice.toBigInt(),
        };
      }
    }
    return undefined;
  }

  async submitVaultSignature(args: {
    utxoId: number;
    vaultSignature: Buffer;
    argonKeyring: KeyringPair;
  }): Promise<TxResult> {
    const { utxoId, vaultSignature, argonKeyring } = args;
    const client = await this.client;
    if (!vaultSignature || vaultSignature.byteLength < 71 || vaultSignature.byteLength > 73) {
      throw new Error(
        `Invalid vault signature length: ${vaultSignature.byteLength}. Must be 71-73 bytes.`,
      );
    }
    const signature = `0x${vaultSignature.toString('hex')}`;
    const tx = client.tx.bitcoinLocks.cosignRelease(utxoId, signature);
    const submitter = new TxSubmitter(client, tx, argonKeyring);
    return await submitter.submit({ waitForBlock: true });
  }

  async getBitcoinLock(utxoId: number): Promise<IBitcoinLock | undefined> {
    const client = await this.client;
    const utxoRaw = await client.query.bitcoinLocks.locksByUtxoId(utxoId);
    if (!utxoRaw.isSome) {
      return;
    }
    const utxo = utxoRaw.unwrap();
    const p2shBytesPrefix = '0020';
    const wscriptHash = utxo.utxoScriptPubkey.asP2wsh.wscriptHash.toHex().replace('0x', '');
    const p2wshScriptHashHex = `0x${p2shBytesPrefix}${wscriptHash}`;
    const vaultId = utxo.vaultId.toNumber();
    const lockPrice = utxo.lockPrice.toBigInt();
    const ownerAccount = utxo.ownerAccount.toHuman();
    const satoshis = utxo.satoshis.toBigInt();
    const vaultPubkey = utxo.vaultPubkey.toHex();
    const vaultClaimPubkey = utxo.vaultClaimPubkey.toHex();
    const ownerPubkey = utxo.ownerPubkey.toHex();
    const [fingerprint, cosign_hd_index, claim_hd_index] = utxo.vaultXpubSources;
    const vaultXpubSources = {
      parentFingerprint: Buffer.from(fingerprint),
      cosignHdIndex: cosign_hd_index.toNumber(),
      claimHdIndex: claim_hd_index.toNumber(),
    };

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
      lockPrice,
      ownerAccount,
      satoshis,
      vaultPubkey,
      vaultClaimPubkey,
      ownerPubkey,
      vaultXpubSources,
      vaultClaimHeight,
      openClaimHeight,
      createdAtHeight,
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
    const client = await this.client;
    const releaseHeight = await client.query.bitcoinLocks.lockReleaseCosignHeightById(utxoId);
    if (releaseHeight.isSome) {
      const releaseHeightValue = releaseHeight.unwrap().toNumber();
      const signature = await this.getVaultCosignSignature(utxoId, releaseHeightValue);
      if (signature) {
        return { blockHeight: releaseHeightValue, signature };
      }
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
    const client = await this.client;

    for (let i = 0; i < 10; i++) {
      const currentHeight = await client.query.system.number().then(x => x.toNumber());
      if (atHeight > currentHeight) {
        console.warn(`Requested block height ${atHeight} is greater than current height ${currentHeight}. Retrying...`);
        await new Promise(resolve => setTimeout(resolve, 1000)); // wait 1 second before retrying
        continue;
      }
      const hash = await client.query.system.blockHash(atHeight).then(x => x.toHex());
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
    const client = await this.client;

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
          return Buffer.from(signature);
        }
      }
    }
    return undefined;
  }

  async findPendingMints(utxoId: number): Promise<bigint[]> {
    const client = await this.client;
    const pendingMint = await client.query.mint.pendingMintUtxos();
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
    ownerBitcoinPubkey: Buffer;
    satoshis: bigint;
    argonKeyring: KeyringPair;
    reducedBalanceBy?: bigint;
    tip?: bigint;
  }) {
    const { vault, argonKeyring, satoshis, tip = 0n, ownerBitcoinPubkey } = args;
    const client = await this.client;
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
    const marketPrice = await this.getMarketRate(BigInt(satoshis));
    const securityFee = vault.calculateBitcoinFee(marketPrice);

    const { canAfford, availableBalance, txFee } = await submitter.canAfford({
      tip,
      unavailableBalance: securityFee + (args.reducedBalanceBy ?? 0n),
      includeExistentialDeposit: true,
    });
    if (!canAfford) {
      throw new Error(
        `Insufficient funds to initialize lock. Available: ${formatArgons(availableBalance)}, Required: ${satoshis}`,
      );
    }
    return { tx, securityFee, txFee };
  }

  async initializeLock(args: {
    vault: Vault;
    ownerBitcoinPubkey: Buffer;
    argonKeyring: KeyringPair;
    satoshis: bigint;
    tip?: bigint;
  }): Promise<{
    lock: IBitcoinLock;
    createdAtHeight: number;
    txResult: TxResult;
    securityFee: bigint;
  }> {
    const { argonKeyring, tip = 0n } = args;
    const client = await this.client;

    const { tx, securityFee } = await this.createInitializeLockTx(args);
    const submitter = new TxSubmitter(client, tx, argonKeyring);
    const txResult = await submitter.submit({ waitForBlock: true, logResults: true, tip });
    const blockHash = await txResult.inBlockPromise;
    const blockHeight = await client
      .at(blockHash)
      .then(x => x.query.system.number())
      .then(x => x.toNumber());
    const utxoId = (await this.getUtxoIdFromEvents(txResult.events)) ?? 0;
    if (utxoId === 0) {
      throw new Error('Bitcoin lock creation failed, no UTXO ID found in transaction events');
    }
    const lock = await this.getBitcoinLock(utxoId);
    if (!lock) {
      throw new Error(`Lock with ID ${utxoId} not found after initialization`);
    }
    return { lock, createdAtHeight: blockHeight, txResult, securityFee };
  }

  async requiredSatoshisForArgonLiquidity(argonAmount: bigint): Promise<bigint> {
    /**
     * If 1_000_000 microgons are available, and the market rate is 100 microgons per satoshi, then
     * 1_000_000 / 100 = 10_000 satoshis needed
     */
    const marketRatePerBitcoin = await this.getMarketRate(SATS_PER_BTC);
    return (argonAmount * SATS_PER_BTC) / marketRatePerBitcoin;
  }

  async requestRelease(args: {
    lock: IBitcoinLock;
    releaseRequest: IReleaseRequest;
    argonKeyring: KeyringPair;
    tip?: bigint;
  }): Promise<{ blockHash: Uint8Array; blockHeight: number }> {
    const client = await this.client;
    const {
      lock,
      releaseRequest: { bitcoinNetworkFee, toScriptPubkey },
      argonKeyring,
      tip,
    } = args;

    if (!toScriptPubkey.startsWith('0x')) {
      throw new Error('toScriptPubkey must be a hex string starting with 0x');
    }

    const submitter = new TxSubmitter(
      client,
      client.tx.bitcoinLocks.requestRelease(lock.utxoId, toScriptPubkey, bitcoinNetworkFee),
      argonKeyring,
    );

    let redemptionPrice = await this.getRedemptionRate(lock.satoshis);
    if (redemptionPrice > lock.lockPrice) {
      redemptionPrice = lock.lockPrice;
    }

    const canAfford = await submitter.canAfford({
      tip,
      unavailableBalance: BigInt(redemptionPrice),
    });

    if (!canAfford.canAfford) {
      throw new Error(
        `Insufficient funds to release lock. Available: ${formatArgons(canAfford.availableBalance)}, Required: ${formatArgons(redemptionPrice)}`,
      );
    }
    const txResult = await submitter.submit({ waitForBlock: true, logResults: true, tip });
    const blockHash = await txResult.inBlockPromise;
    const blockHeight = await client
      .at(blockHash)
      .then(x => x.query.system.number())
      .then(x => x.toNumber());
    return {
      blockHash,
      blockHeight,
    };
  }

  async releasePrice(satoshis: bigint, lockPrice: bigint): Promise<bigint> {
    const client = await this.client;
    const redemptionRate = await this.getRedemptionRate(satoshis);
    if (redemptionRate > lockPrice) {
      return redemptionRate;
    }
    return lockPrice;
  }

  async getRatchetPrice(
    lock: IBitcoinLock,
    vault: Vault,
  ): Promise<{ burnAmount: bigint; ratchetingFee: bigint; marketRate: bigint }> {
    const { createdAtHeight, vaultClaimHeight, lockPrice, satoshis } = lock;
    const client = await this.client;
    const marketRate = await this.getMarketRate(BigInt(satoshis));

    let ratchetingFee = vault.terms.bitcoinBaseFee;
    let burnAmount = 0n;
    // ratchet up
    if (marketRate > lockPrice) {
      const lockFee = vault.calculateBitcoinFee(marketRate);
      const currentBitcoinHeight = await client.query.bitcoinUtxos
        .confirmedBitcoinBlockTip()
        .then(x => x.unwrap().blockHeight.toNumber());
      const blockLength = vaultClaimHeight - createdAtHeight;
      const elapsed = (currentBitcoinHeight - createdAtHeight) / blockLength;
      const remainingDuration = 1 - elapsed;
      ratchetingFee = BigInt(remainingDuration * Number(lockFee));
    } else {
      burnAmount = await this.releasePrice(lock.satoshis, lockPrice);
    }

    return {
      ratchetingFee,
      burnAmount,
      marketRate,
    };
  }

  async ratchet(args: {
    lock: IBitcoinLock;
    argonKeyring: KeyringPair;
    tip?: bigint;
    vault: Vault;
  }): Promise<{
    newLockPrice: bigint;
    pendingMint: bigint;
    burned: bigint;
    blockHeight: number;
    bitcoinBlockHeight: number;
  }> {
    const { lock, argonKeyring, tip = 0n, vault } = args;
    const client = await this.client;

    const ratchetPrice = await this.getRatchetPrice(lock, vault);
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

    const submission = await txSubmitter.submit({
      waitForBlock: true,
      tip,
    });
    const ratchetEvent = submission.events.find(x =>
      client.events.bitcoinLocks.BitcoinLockRatcheted.is(x),
    );
    if (!ratchetEvent) {
      throw new Error(`Ratchet event not found in transaction events`);
    }
    const blockHash = await submission.inBlockPromise;
    const api = await client.at(blockHash);
    const blockHeight = await api.query.system.number().then(x => x.toNumber());
    const bitcoinBlockHeight = await api.query.bitcoinUtxos
      .confirmedBitcoinBlockTip()
      .then(x => x.unwrap().blockHeight.toNumber());
    const { amountBurned, newLockPrice, originalLockPrice } = ratchetEvent.data;
    let mintAmount = newLockPrice.toBigInt();
    if (newLockPrice > originalLockPrice) {
      mintAmount -= originalLockPrice.toBigInt();
    }
    return {
      pendingMint: mintAmount,
      newLockPrice: newLockPrice.toBigInt(),
      burned: amountBurned.toBigInt(),
      blockHeight,
      bitcoinBlockHeight,
    };
  }

  static async waitForSpace(
    accountset: Accountset,
    options: {
      argonAmount: bigint;
      bitcoinXpub: string;
      maxLockFee?: bigint;
      tip?: bigint;
      satoshiWiggleRoomForDynamicPrice?: bigint;
    },
  ): Promise<{
    satoshis: bigint;
    argons: bigint;
    vaultId: number;
    txFee: bigint;
    securityFee: bigint;
    utxoId: number;
    finalizedPromise: Promise<Uint8Array>;
  }> {
    const { argonAmount, bitcoinXpub, maxLockFee, tip = 0n } = options;
    const vaults = new VaultMonitor(accountset, {
      bitcoinSpaceAvailable: argonAmount,
    });
    const bitcoinXpubBuffer = Buffer.from(bitcoinXpub.replace(/^0x(.+)/, '$1'), 'hex');

    return new Promise(async (resolve, reject) => {
      vaults.events.on('bitcoin-space-above', async (vaultId, amount) => {
        const vault = vaults.vaultsById[vaultId];
        const fee = vault.calculateBitcoinFee(amount);
        console.log(
          `Vault ${vaultId} has ${formatArgons(amount)} argons available for bitcoin. Lock fee is ${formatArgons(fee)}`,
        );
        if (maxLockFee !== undefined && fee > maxLockFee) {
          console.log(
            `Skipping vault ${vaultId} due to high lock fee: ${formatArgons(maxLockFee)}`,
          );
          return;
        }

        try {
          const bitcoinLock = new BitcoinLocks(accountset.client);
          let satoshis = await bitcoinLock.requiredSatoshisForArgonLiquidity(amount);
          satoshis -= options.satoshiWiggleRoomForDynamicPrice ?? 500n;
          const { txResult, lock, securityFee } = await bitcoinLock.initializeLock({
            vault,
            satoshis,
            argonKeyring: accountset.txSubmitterPair,
            ownerBitcoinPubkey: bitcoinXpubBuffer,
            tip,
          });

          resolve({
            satoshis,
            argons: argonAmount,
            vaultId,
            securityFee,
            txFee: txResult.finalFee!,
            finalizedPromise: txResult.finalizedPromise,
            utxoId: lock.utxoId,
          });
        } catch (err) {
          console.error('Error submitting bitcoin lock tx:', err);
          reject(err);
        } finally {
          vaults.stop();
        }
      });
      await vaults.monitor();
    });
  }
}

export interface IBitcoinLockConfig {
  releaseExpirationBlocks: number;
  tickDurationMillis: number;
  bitcoinNetwork: ArgonPrimitivesBitcoinBitcoinNetwork;
}
export interface IReleaseRequest {
  toScriptPubkey: string;
  bitcoinNetworkFee: bigint;
}

export interface IReleaseRequestDetails extends IReleaseRequest {
  dueBlockHeight: number;
  vaultId: number;
  redemptionPrice: bigint;
}

export interface IBitcoinLock {
  utxoId: number;
  p2wshScriptHashHex: string;
  vaultId: number;
  lockPrice: bigint;
  ownerAccount: string;
  satoshis: bigint;
  vaultPubkey: string;
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
