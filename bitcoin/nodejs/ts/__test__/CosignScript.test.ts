import {
  SKIP_E2E,
  stringifyExt,
  sudo,
  teardown,
  TestMainchain,
  TestOracle,
} from '@argonprotocol/testing';
import {
  ArgonClient,
  BitcoinLock,
  IBitcoinLock,
  IBitcoinLockConfig,
  Keyring,
  KeyringPair,
  PriceIndex,
  SATS_PER_BTC,
  toFixedNumber,
  TxSubmitter,
  u8aToHex,
  Vault,
} from '@argonprotocol/mainchain';
import { afterAll, beforeAll, describe, expect, test } from 'vitest';
import {
  addressBytesHex,
  bip39,
  BitcoinNetwork,
  CosignScript,
  getBitcoinNetworkFromApi,
  getChildXpriv,
  getCompressedPubkey,
  getXpubFromXpriv,
  HDKey,
  p2wshScriptHexToAddress,
} from '..';
import { wordlist as english } from '@scure/bip39/wordlists/english';

const { generateMnemonic, mnemonicToSeedSync } = bip39;

afterAll(teardown);

describe.skipIf(SKIP_E2E)('Bitcoin Bindings test', { retry: 0, timeout: 60e3 }, () => {
  let vaulterchain: TestMainchain;
  let vaulterClient: ArgonClient;
  let vaulter: KeyringPair;
  let lock: BitcoinLock;
  let bitcoinLocker: KeyringPair;
  const vaulterMnemonic = generateMnemonic(english);
  const bitcoinMnemonic = generateMnemonic(english);
  const devSeed = mnemonicToSeedSync(vaulterMnemonic);
  const vaulterHdPath = "m/84'/0'/0'";

  let config: IBitcoinLockConfig;
  let bitcoinNetwork: BitcoinNetwork;
  let vaultXpriv: HDKey;
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

    vaulter = sudo();
    bitcoinLocker = new Keyring({ type: 'sr25519' }).addFromUri('//Bob');
    config = await BitcoinLock.getConfig(vaulterClient);
    console.log('Bitcoin Locks config:', stringifyExt(config));
    bitcoinNetwork = getBitcoinNetworkFromApi(config.bitcoinNetwork);

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
    const txResult = await new TxSubmitter(
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
    ).submit();
    await txResult.waitForInFirstBlock;
    await new Promise(resolve => setTimeout(resolve, 0));
    const priceIndex = new PriceIndex();
    await priceIndex.load(vaulterClient);
    expect(priceIndex.argonotUsdPrice).toBeDefined();
    await expect(BitcoinLock.getMarketRate(priceIndex, 100n)).resolves.toStrictEqual(60_000n);
    await expect(
      BitcoinLock.getRedemptionRate(priceIndex, { peggedPrice: 60_000n, satoshis: 100n }),
    ).resolves.toStrictEqual(60_000n);
    await expect(
      BitcoinLock.getRedemptionRate(priceIndex, { peggedPrice: 50_000n, satoshis: 100n }),
    ).resolves.toStrictEqual(50_000n);
  });

  test.sequential('it can lock a bitcoin', async () => {
    const vaultResult = await Vault.create(vaulterClient, vaulter, {
      securitization: 10_000_000n,
      securitizationRatio: 1,
      annualPercentRate: 0.05,
      baseFee: 500_000n,
      bitcoinXpub: vaultMasterXpub,
      treasuryProfitSharing: 0.5,
    });
    vault = await vaultResult.getVault();

    const btcClient = vaulterchain.getBitcoinClient();
    await btcClient.command('createwallet', 'default').catch(() => null);
    await btcClient.command('loadwallet', 'default').catch(() => null);
    const newAddress = await btcClient.command('getnewaddress');
    expect(newAddress).toBeTypeOf('string');
    console.log(`Mining to ${newAddress}`);
    await btcClient.command('generatetoaddress', 101, newAddress);

    const ownerBitcoinXpriv = getChildXpriv(
      mnemonicToSeedSync(bitcoinMnemonic),
      "m/84'/0'/0'/0/0'",
    );
    const ownerBitcoinPubkey = getCompressedPubkey(ownerBitcoinXpriv.publicKey!);
    console.log('Owner Bitcoin Pubkey:', u8aToHex(ownerBitcoinPubkey), ownerBitcoinPubkey.length);
    const priceIndex = new PriceIndex();
    await priceIndex.load(vaulterClient);
    const result = await BitcoinLock.initialize({
      client: vaulterClient,
      vault,
      priceIndex,
      satoshis: 2000n,
      ownerBitcoinPubkey,
      argonKeyring: bitcoinLocker,
    });
    const { lock } = await result.getLock();
    console.log('Locked bitcoin', lock);
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

    const paytoScriptAddress = p2wshScriptHexToAddress(lock.p2wshScriptHashHex, bitcoinNetwork);

    const { psbt: fundingPsbt } = await btcClient.command(
      'walletcreatefundedpsbt',
      [],
      {
        [paytoScriptAddress]: btc,
      },
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
    const txid: string = await btcClient.command('sendrawtransaction', finalizedPsbt.hex);
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

    await expect(lock.getUtxoRef(vaulterClient)).resolves.toEqual({
      bitcoinTxid: `0x${txid}`, // this is the little-endian representation.
      vout: expect.any(Number),
      txid: expect.any(String),
    });

    const pendingMints = await lock.findPendingMints(vaulterClient);
    expect(pendingMints).toHaveLength(1);
  });

  test.sequential('it can release a bitcoin lock', async () => {
    const lookup = await BitcoinLock.get(vaulterClient, 1);
    if (!lookup) {
      throw new Error('Lock not found');
    }
    lock = lookup;
    expect(lock.isVerified).toBe(true);
    const priceIndex = new PriceIndex();
    await priceIndex.load(vaulterClient);

    await expect(lock.releasePrice(priceIndex)).resolves.toEqual(lock.peggedPrice);

    const btcClient = vaulterchain.getBitcoinClient();
    const nextAddress = await btcClient.command('getnewaddress');
    console.log(`Next address for release: ${nextAddress}`);
    const toScriptPubkey = addressBytesHex(nextAddress, bitcoinNetwork);
    const cosignScript = new CosignScript(lock, bitcoinNetwork);
    const networkFee = cosignScript.calculateFee(5n, toScriptPubkey);
    expect(networkFee).toBeGreaterThan(5n);
    console.log(`Network fee for release: ${networkFee} satoshis, next address: ${nextAddress}`);
    const result = await lock.requestRelease({
      client: vaulterClient,
      priceIndex,
      releaseRequest: { bitcoinNetworkFee: networkFee, toScriptPubkey },
      argonKeyring: bitcoinLocker,
    });
    await result.waitForFinalizedBlock;
    console.log('Release request result:', result);
    expect(result.blockNumber).toBeGreaterThan(1);
  });

  test.sequential('it can cosign as vault', async () => {
    const lookup = await BitcoinLock.get(vaulterClient, 1);
    if (!lookup) {
      throw new Error('Lock not found');
    }
    lock = lookup;
    const cosignScript = new CosignScript(lock, bitcoinNetwork);
    const releaseRequest = await lock.getReleaseRequest(vaulterClient);
    expect(releaseRequest).toBeTruthy();
    if (!releaseRequest) {
      throw new Error('Release request not found');
    }
    const utxoRef = await lock.getUtxoRef(vaulterClient);
    if (!utxoRef) {
      throw new Error('UTXO reference not found');
    }
    expect(utxoRef).toBeTruthy();
    const psbt = cosignScript.getCosignPsbt({ releaseRequest, utxoRef });
    const signedPsbt = cosignScript.vaultCosignPsbt(psbt, lock, vaultXpriv);
    expect(signedPsbt.getInput(0).partialSig).toHaveLength(1);
    const [_, signature] = signedPsbt.getInput(0).partialSig?.[0] ?? [];
    expect(signature).toBeDefined();
    if (!signature) throw new Error('Signature not found in PSBT');

    const tx = await BitcoinLock.submitVaultSignature({
      client: vaulterClient,
      utxoId: lock.utxoId,
      vaultSignature: signature,
      argonKeyring: vaulter,
    });
    await tx.waitForFinalizedBlock;
    expect(tx.blockHash).toBeTruthy();
    console.log('Cosign transaction included in block:', tx.blockHash);
    const blockHeight = await vaulterClient
      .at(tx.blockHash!)
      .then(x => x.query.system.number())
      .then(x => x.toNumber());
    console.log('Cosign transaction block height:', blockHeight);

    const cosign = await lock.findVaultCosignSignature(vaulterClient);
    expect(cosign).toBeDefined();
  });

  test.sequential('user can cosign a bitcoin lock', async () => {
    const ownerBitcoinXpriv = getChildXpriv(
      mnemonicToSeedSync(bitcoinMnemonic),
      "m/84'/0'/0'/0/0'",
    );
    // can't load the lock now, as it is removed from the locksByUtxoId map
    const cosignScript = new CosignScript(lock, bitcoinNetwork);
    const cosign = await lock.findVaultCosignSignature(vaulterClient);
    expect(cosign).toBeDefined();
    if (!cosign) throw new Error('Cosign not found');

    const dataStillAvailableHeight = cosign.blockHeight - 1;

    const blockHash = await vaulterClient.rpc.chain.getBlockHash(dataStillAvailableHeight);
    const clientAtHeight = await vaulterClient.at(blockHash);

    const releaseRequest = await lock.getReleaseRequest(clientAtHeight);
    const utxoRef = await lock.getUtxoRef(clientAtHeight);
    expect(utxoRef).toBeTruthy();
    console.log('Got release request:', releaseRequest, utxoRef, cosign);
    const cosignedTx = cosignScript.cosignAndGenerateTx({
      releaseRequest: releaseRequest!,
      vaultCosignature: cosign.signature,
      utxoRef: utxoRef!,
      ownerXpriv: ownerBitcoinXpriv,
    });
    console.log('Cosigned Tx:', stringifyExt(cosignedTx));
    const btcClient = vaulterchain.getBitcoinClient();
    const txHex = u8aToHex(cosignedTx.toBytes(true, true), undefined, false);
    const txid = await btcClient.command('sendrawtransaction', txHex);
    console.log('Broadcasted cosigned transaction with TXID:', txid);
    expect(txid).toBeDefined();
    // Wait for the transaction to be included in the bitcoin wallet

    const tx = await btcClient.command('gettransaction', txid);
    // If no error, the tx is in your wallet
    expect(tx).toBeDefined();
    console.log('Transaction is in wallet:', tx);
  });
});
