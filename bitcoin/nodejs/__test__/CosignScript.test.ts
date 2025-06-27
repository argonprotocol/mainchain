import {
  describeIntegration,
  stringifyExt,
  sudo,
  teardown,
  TestMainchain,
  TestOracle,
} from '@argonprotocol/testing';
import {
  Accountset,
  ArgonClient,
  BitcoinLocks,
  IBitcoinLock,
  IBitcoinLockConfig,
  Keyring,
  SATS_PER_BTC,
  toFixedNumber,
  TxSubmitter,
  Vault,
} from '@argonprotocol/mainchain';
import { afterAll, beforeAll, expect, test } from 'vitest';
import { generateMnemonic } from 'bip39';
import {
  addressBytesHex,
  getBip39Seed,
  getChildXpriv,
  getCompressedPubkey,
  getXpubFromXpriv,
  keyToBuffer,
} from '../ts';
import CosignScript from '../ts/CosignScript';
import { address, networks } from 'bitcoinjs-lib';
import { BIP32Interface } from 'bip32';

afterAll(teardown);

describeIntegration(
  'Bitcoin Bindings test',
  () => {
    let vaulterchain: TestMainchain;
    let vaulterClient: ArgonClient;
    let vaulter: Accountset;
    let lock: IBitcoinLock;
    let bitcoinLocker: Accountset;
    const vaulterMnemonic = generateMnemonic();
    const bitcoinMnemonic = generateMnemonic();
    const devSeed = getBip39Seed(vaulterMnemonic);
    const vaulterHdPath = "m/84'/0'/0'";

    let bitcoinLocks: BitcoinLocks;
    let config: IBitcoinLockConfig;
    let bitcoinNetwork: networks.Network;
    let vaultXpriv: BIP32Interface;
    let vaultMasterXpub: string;
    let vault: Vault;
    beforeAll(async () => {
      console.log('Starting vaulterchain with bitcoin...');
      vaulterchain = new TestMainchain();
      await vaulterchain.launch({
        miningThreads: 1,
        launchBitcoin: true,
        author: 'alice',
      });
      vaulterClient = await vaulterchain.client();

      console.log('Vaulterchain started at', vaulterchain.address);
      const bitcoinOracle = new TestOracle();
      await bitcoinOracle.start('bitcoin', {
        mainchainUrl: vaulterchain.address,
        bitcoinRpcUrl: `http://bitcoin:bitcoin@localhost:${vaulterchain.bitcoinPort!}`,
      });

      vaulter = new Accountset({
        client: Promise.resolve(vaulterClient),
        seedAccount: sudo(),
        sessionKeyMnemonic: generateMnemonic(),
      });
      bitcoinLocker = new Accountset({
        seedAccount: new Keyring({ type: 'sr25519' }).addFromUri('//Bob'),
        client: Promise.resolve(vaulterClient),
        sessionKeyMnemonic: generateMnemonic(),
      });
      bitcoinLocks = new BitcoinLocks(Promise.resolve(vaulterClient));
      config = await bitcoinLocks.getConfig();
      console.log('Bitcoin Locks config:', stringifyExt(config));
      bitcoinNetwork = CosignScript.getBitcoinJsNetwork(config.bitcoinNetwork);

      vaultXpriv = getChildXpriv(devSeed, vaulterHdPath, bitcoinNetwork);
      // get the xpub from the xpriv
      vaultMasterXpub = getXpubFromXpriv(vaultXpriv);
    }, 60e3);

    test.sequential('Test price apis', async () => {
      // wait for first block to be mined
      await new Promise<void>(resolve => {
        const sub = vaulterClient.rpc.chain.subscribeAllHeads(h => {
          if (h.number.toNumber() > 1) {
            resolve();
            void sub.then(x => x());
          }
        });
      });

      const currentTick = await vaulterClient.query.ticks.currentTick();
      await new TxSubmitter(
        vaulterClient,
        vaulterClient.tx.priceIndex.submit({
          btcUsdPrice: toFixedNumber(60_000.5, 18),
          argonUsdPrice: toFixedNumber(1.0, 18),
          argonotUsdPrice: toFixedNumber(12.0, 18),
          argonUsdTargetPrice: toFixedNumber(1.0, 18),
          argonTimeWeightedAverageLiquidity: toFixedNumber(1_000, 18),
          tick: currentTick.toBigInt(),
        }),
        new Keyring({ type: 'sr25519' }).addFromUri(TestOracle.PriceIndexOperator),
      ).submit({ waitForBlock: true });
      await new Promise(resolve => setTimeout(resolve, 0));
      const priceIndex = await vaulterClient.query.priceIndex.current();
      expect(priceIndex.isSome).toBe(true);
      const bitcoinLocks = new BitcoinLocks(Promise.resolve(vaulterClient));
      await expect(bitcoinLocks.getMarketRate(100n)).resolves.toStrictEqual(60_000n);
      await expect(bitcoinLocks.getRedemptionRate(100n)).resolves.toStrictEqual(60_000n);
    });

    test.sequential('it can lock a bitcoin', async () => {
      vault = await Vault.create(vaulterClient, vaulter.txSubmitterPair, {
        securitization: 10_000_000n,
        securitizationRatio: 1,
        annualPercentRate: 0.05,
        baseFee: 500_000n,
        bitcoinXpub: vaultMasterXpub,
        liquidityPoolProfitSharing: 0.5,
      });

      const btcClient = vaulterchain.getBitcoinClient();
      await btcClient.command('createwallet', 'default').catch(() => null);
      await btcClient.command('loadwallet', 'default').catch(() => null);
      const newAddress = await btcClient.command('getnewaddress');
      expect(newAddress).toBeTypeOf('string');
      console.log(`Mining to ${newAddress}`);
      await btcClient.command('generatetoaddress', 101, newAddress);

      const ownerBitcoinXpriv = getChildXpriv(getBip39Seed(bitcoinMnemonic), "m/84'/0'/0'/0/0'");
      const ownerBitcoinPubkey = getCompressedPubkey(ownerBitcoinXpriv.publicKey);
      console.log(
        'Owner Bitcoin Pubkey:',
        ownerBitcoinPubkey.toString('hex'),
        ownerBitcoinPubkey.length,
      );
      const result = await bitcoinLocks.initializeLock({
        vault,
        satoshis: 2000n,
        ownerBitcoinPubkey,
        argonKeyring: bitcoinLocker.txSubmitterPair,
      });
      console.log('Locked bitcoin', result.lock);
      lock = result.lock;
      expect(lock.vaultId).toBe(1);
      expect(lock.satoshis).toBeGreaterThan(1000);
      expect(lock.utxoId).toBe(1);
      const cosignScript = new CosignScript(lock, bitcoinNetwork);
      const scriptPubkey = cosignScript.calculateScriptPubkey();
      console.log(`ScriptPubkey: ${scriptPubkey} vs calculated ${lock.p2wshScriptHashHex}`);
      expect(scriptPubkey).toBe(lock.p2wshScriptHashHex);

      const walletBalance = await btcClient.command('getbalances');
      const btc = Number(lock.satoshis) / Number(SATS_PER_BTC);
      console.log('Wallet balance:', walletBalance, 'Needed:', btc);
      const scriptAddress = address.fromOutputScript(keyToBuffer(scriptPubkey), bitcoinNetwork);

      const { psbt: fundingPsbt } = await btcClient.command(
        'walletcreatefundedpsbt',
        [],
        { [scriptAddress]: btc },
        0,
        {
          lockUnspents: true,
          feeRate: 0.00001,
        },
      );
      console.log('Created PSBT:', fundingPsbt);
      // Ensure we process the PSBT returned from walletcreatefundedpsbt
      const processed = await btcClient.command('walletprocesspsbt', fundingPsbt);
      console.log('Processed PSBT:', processed);
      if (!processed.complete) {
        const decoded = await btcClient.command('decodepsbt', processed.psbt);
        console.dir(decoded.inputs, { depth: null });
        throw new Error('PSBT could not be finalized: incomplete signing');
      }
      // Pass processed.psbt into finalizepsbt
      const finalizedPsbt = await btcClient.command('finalizepsbt', processed.psbt);
      console.log('Finalized PSBT:', finalizedPsbt);
      // Diagnostic logging: log txid from finalizepsbt (if present)
      if (finalizedPsbt.txid) {
        console.log('Finalized TXID (from finalizepsbt):', finalizedPsbt.txid);
      }
      const txid = await btcClient.command('sendrawtransaction', finalizedPsbt.hex);
      console.log('Broadcast TXID (from sendrawtransaction):', txid);
      console.log('TXID normalized:', txid.split('').reverse().join(''));
      // Fetch decoded transaction and log its TXID
      const decoded = await btcClient.command('getrawtransaction', txid, true);
      console.log('Decoded TXID from getrawtransaction:', decoded.txid);

      await btcClient.command('generatetoaddress', 7, newAddress);

      // wait for the bitcoin to verify
      await new Promise<void>(async (resolve, reject) => {
        const unsub = await vaulterClient.query.bitcoinLocks.locksByUtxoId(1, y => {
          if (!y.isSome) reject('No lock found');
          const lock = y.unwrap();
          if (lock.isVerified.isTrue) {
            resolve();
            unsub();
          }
        });
      });

      await expect(bitcoinLocks.getUtxoRef(1)).resolves.toEqual({
        bitcoinTxid: `0x${txid}`, // this is the little-endian representation.
        vout: expect.any(Number),
        txid: expect.any(String),
      });

      const pendingMints = await bitcoinLocks.findPendingMints(lock.utxoId);
      expect(pendingMints).toHaveLength(1);
    });

    test.sequential('it can release a bitcoin lock', async () => {
      lock = await bitcoinLocks.getBitcoinLock(1);
      expect(lock.isVerified).toBe(true);

      await expect(bitcoinLocks.releasePrice(lock.satoshis, lock.lockPrice)).resolves.toEqual(
        lock.lockPrice,
      );

      const btcClient = vaulterchain.getBitcoinClient();
      const nextAddress = await btcClient.command('getnewaddress');
      console.log(`Next address for release: ${nextAddress}`);
      const toScriptPubkey = addressBytesHex(nextAddress, bitcoinNetwork);
      const cosignScript = new CosignScript(lock, bitcoinNetwork);
      const networkFee = cosignScript.calculateFee(5n, toScriptPubkey);
      expect(networkFee).toBeGreaterThan(5n);
      console.log(`Network fee for release: ${networkFee} satoshis, next address: ${nextAddress}`);
      const result = await bitcoinLocks.requestRelease({
        lock,
        releaseRequest: { bitcoinNetworkFee: networkFee, toScriptPubkey },
        argonKeyring: bitcoinLocker.txSubmitterPair,
      });
      console.log('Release request result:', result);
      expect(result.blockHeight).toBeGreaterThan(1);
    });

    test.sequential('it can cosign as vault', async () => {
      lock = await bitcoinLocks.getBitcoinLock(1);
      const cosignScript = new CosignScript(lock, bitcoinNetwork);
      const releaseRequest = await bitcoinLocks.getReleaseRequest(lock.utxoId);
      expect(releaseRequest).toBeTruthy();
      const utxoRef = await bitcoinLocks.getUtxoRef(lock.utxoId);
      expect(utxoRef).toBeTruthy();
      const psbt = cosignScript.getCosignPsbt({ releaseRequest, utxoRef });
      const signedPsbt = cosignScript.vaultCosignPsbt(psbt, lock, vaultXpriv);
      expect(signedPsbt.data.inputs[0].partialSig).toHaveLength(1);
      const { signature } = signedPsbt.data.inputs[0].partialSig[0];
      expect(signature).toBeDefined();

      const tx = await bitcoinLocks.submitVaultSignature({
        utxoId: lock.utxoId,
        vaultSignature: signature,
        argonKeyring: vaulter.txSubmitterPair,
      });
      expect(tx.includedInBlock).toBeTruthy();
      console.log('Cosign transaction included in block:', tx.includedInBlock);
      const blockHeight = await vaulterClient
        .at(tx.includedInBlock!)
        .then(x => x.query.system.number())
        .then(x => x.toNumber());
      console.log('Cosign transaction block height:', blockHeight);

      const cosign = await bitcoinLocks.findVaultCosignSignature(lock.utxoId);
      expect(cosign).toBeDefined();
    });

    test.sequential('user can cosign a bitcoin lock', async () => {
      const ownerBitcoinXpriv = getChildXpriv(getBip39Seed(bitcoinMnemonic), "m/84'/0'/0'/0/0'");
      // can't load the lock now, as it is removed from the locksByUtxoId map
      const cosignScript = new CosignScript(lock, bitcoinNetwork);
      const cosign = await bitcoinLocks.findVaultCosignSignature(lock.utxoId);
      expect(cosign).toBeDefined();

      const dataStillAvailableHeight = cosign.blockHeight - 1;

      const releaseRequest = await bitcoinLocks.getReleaseRequest(
        lock.utxoId,
        dataStillAvailableHeight,
      );
      const utxoRef = await bitcoinLocks.getUtxoRef(lock.utxoId, dataStillAvailableHeight);
      expect(utxoRef).toBeTruthy();
      console.log('Got release request:', releaseRequest, utxoRef, cosign);
      const cosignedTx = cosignScript.cosignAndGenerateTx({
        releaseRequest: releaseRequest!,
        vaultCosignature: cosign.signature,
        utxoRef,
        ownerXpriv: ownerBitcoinXpriv,
      });
      console.log('Cosigned Tx:', stringifyExt(cosignedTx));
      const btcClient = vaulterchain.getBitcoinClient();
      const txHex = cosignedTx.toHex();
      const txid = await btcClient.command('sendrawtransaction', txHex);
      console.log('Broadcasted cosigned transaction with TXID:', txid);
      expect(txid).toBeDefined();
      // Wait for the transaction to be included in the bitcoin wallet

      const tx = await btcClient.command('gettransaction', txid);
      // If no error, the tx is in your wallet
      expect(tx).toBeDefined();
      console.log('Transaction is in wallet:', tx);
    });
  },
  { retry: 0, timeout: 60e3 },
);
