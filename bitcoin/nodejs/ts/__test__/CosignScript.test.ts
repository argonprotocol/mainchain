import {
  SKIP_E2E,
  stringifyExt,
  submitTx,
  sudo,
  teardown,
  TestMainchain,
  TestOracle,
} from '@argonprotocol/testing';
import {
  type ArgonClient,
  FIXED_U128_DECIMALS,
  Keyring,
  type KeyringPair,
  PERMILL_DECIMALS,
  PriceIndex,
  toFixedNumber,
  u8aToHex,
} from '@argonprotocol/mainchain';
import { afterAll, beforeAll, describe, expect, test } from 'vitest';
import {
  addressBytesHex,
  bip39,
  type IBitcoinReleaseRequest,
  BitcoinNetwork,
  CosignScript,
  type ICosignScriptLock,
  getBitcoinNetworkFromApi,
  getChildXpriv,
  getCompressedPubkey,
  getXpubBytes,
  getXpubFromXpriv,
  type HDKey,
  p2wshScriptHexToAddress,
} from '@argonprotocol/bitcoin';
import { wordlist as english } from '@scure/bip39/wordlists/english';

const { generateMnemonic, mnemonicToSeedSync } = bip39;
const SATOSHIS_PER_BITCOIN = 100_000_000n;

afterAll(teardown);

describe.skipIf(SKIP_E2E)('Bitcoin Bindings test', { retry: 0, timeout: 60e3 }, () => {
  let vaulterchain: TestMainchain;
  let vaulterClient: ArgonClient;
  let vaulter: KeyringPair;
  let bitcoinLocker: KeyringPair;
  let bitcoinNetwork: BitcoinNetwork;
  let vaultXpriv: HDKey;
  let lock: ICosignScriptLock;
  let vaultId: number;
  let lockId: number;
  let releaseRequest: IBitcoinReleaseRequest;
  let fundingUtxoRef: { txid: string; vout: number };
  let vaultCosignature: Uint8Array;

  const vaulterMnemonic = generateMnemonic(english);
  const bitcoinMnemonic = generateMnemonic(english);
  const devSeed = mnemonicToSeedSync(vaulterMnemonic);
  const vaulterHdPath = "m/84'/0'/0'";

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
    bitcoinNetwork = getBitcoinNetworkFromApi(
      await vaulterClient.query.bitcoinUtxos.bitcoinNetwork(),
    );
    console.log('Bitcoin network:', bitcoinNetwork);

    vaultXpriv = getChildXpriv(devSeed, vaulterHdPath, bitcoinNetwork);
  }, 60e3);

  test.sequential('Test price apis', async () => {
    await new Promise<void>(resolve => {
      const subscription = vaulterClient.rpc.chain.subscribeAllHeads(header => {
        if (header.number.toNumber() > 1) {
          resolve();
          void subscription.then(unsubscribe => unsubscribe());
        }
      });
    });

    const currentTick = await vaulterClient.query.ticks.currentTick();
    await submitTx(
      vaulterClient,
      vaulterClient.tx.priceIndex.submit(
        {
          btcUsdPrice: toFixedNumber(60_000.5, FIXED_U128_DECIMALS),
          argonUsdPrice: toFixedNumber(1, FIXED_U128_DECIMALS),
          argonotUsdPrice: toFixedNumber(12, FIXED_U128_DECIMALS),
          argonUsdTargetPrice: toFixedNumber(1, FIXED_U128_DECIMALS),
          argonTimeWeightedAverageLiquidity: toFixedNumber(1_000, FIXED_U128_DECIMALS),
          tick: currentTick.toBigInt(),
        },
        null,
      ),
      new Keyring({ type: 'sr25519' }).addFromUri(TestOracle.PriceIndexOperator),
    );

    const priceIndex = new PriceIndex();
    await priceIndex.load(vaulterClient);
    expect(priceIndex.argonotUsdPrice).toBeDefined();
    expect(priceIndex.getSatoshiPriceInTargetMicrogons(100n)).toStrictEqual(60_000n);
  });

  test.sequential('it can create and fund a bitcoin lock', async () => {
    const vaultResult = await submitTx(
      vaulterClient,
      vaulterClient.tx.vaults.create({
        terms: {
          bitcoinAnnualPercentRate: toFixedNumber(0.05, FIXED_U128_DECIMALS),
          bitcoinBaseFee: 500_000n,
          treasuryProfitSharing: toFixedNumber(0.5, PERMILL_DECIMALS),
          treasuryBonusProfitSharing: toFixedNumber(0, PERMILL_DECIMALS),
        },
        securitizationRatio: toFixedNumber(1, FIXED_U128_DECIMALS),
        securitization: 200_000_000n,
        bitcoinXpubkey: getXpubBytes(getXpubFromXpriv(vaultXpriv)),
        delegateAccountId: null,
      }),
      vaulter,
    );
    const vaultCreated = vaultResult.events.find(event =>
      vaulterClient.events.vaults.VaultCreated.is(event),
    );
    if (!vaultCreated || !vaulterClient.events.vaults.VaultCreated.is(vaultCreated)) {
      throw new Error('Vault creation event not found');
    }
    vaultId = vaultCreated.data.vaultId.toNumber();

    const btcClient = vaulterchain.getBitcoinClient();
    await btcClient.command('createwallet', 'default').catch(() => null);
    await btcClient.command('loadwallet', 'default').catch(() => null);
    const newAddress = await btcClient.command('getnewaddress');
    expect(newAddress).toBeTypeOf('string');

    console.log(`Mining initial Bitcoin blocks to ${newAddress}`);
    await btcClient.command('generatetoaddress', 101, newAddress);

    const ownerBitcoinXpriv = getChildXpriv(
      mnemonicToSeedSync(bitcoinMnemonic),
      "m/84'/0'/0'/0/0'",
    );
    const ownerBitcoinPubkey = getCompressedPubkey(ownerBitcoinXpriv.publicKey!);
    console.log(
      'Owner Bitcoin public key:',
      u8aToHex(ownerBitcoinPubkey),
      `(${ownerBitcoinPubkey.length} bytes)`,
    );

    const lockResult = await submitTx(
      vaulterClient,
      vaulterClient.tx.bitcoinLocks.createReceiveAddress(
        vaultId,
        200_000n,
        ownerBitcoinPubkey,
        null,
      ),
      bitcoinLocker,
    );
    const lockCreated = lockResult.events.find(event =>
      vaulterClient.events.bitcoinLocks.BitcoinLockCreated.is(event),
    );
    if (!lockCreated || !vaulterClient.events.bitcoinLocks.BitcoinLockCreated.is(lockCreated)) {
      throw new Error('Bitcoin Lock creation event not found');
    }
    lockId = lockCreated.data.lockId.toNumber();
    lock = await loadCosignScriptLock(vaulterClient, lockId);
    console.log('Created Bitcoin Lock:', stringifyExt(lock));

    expect(lock.securitizedSatoshis).toBe(200_000n);
    const cosignScript = new CosignScript(lock, bitcoinNetwork);
    const calculatedScriptPubkey = cosignScript.calculateScriptPubkey();
    console.log('Lock script pubkey:', {
      stored: lock.p2wshScriptHashHex,
      calculated: calculatedScriptPubkey,
    });
    expect(calculatedScriptPubkey).toBe(lock.p2wshScriptHashHex);

    const btc = Number(lock.securitizedSatoshis) / Number(SATOSHIS_PER_BITCOIN);
    const paytoScriptAddress = p2wshScriptHexToAddress(lock.p2wshScriptHashHex, bitcoinNetwork);
    const walletBalance = await btcClient.command('getbalance');
    console.log('Funding Bitcoin Lock:', {
      walletBalance,
      amountBtc: btc,
      address: paytoScriptAddress,
    });

    const { psbt: fundingPsbt } = await btcClient.command(
      'walletcreatefundedpsbt',
      [],
      { [paytoScriptAddress]: btc },
      0,
      { lockUnspents: true, feeRate: 0.00001 },
    );
    console.log('Created funding PSBT:', fundingPsbt);

    const processed = await btcClient.command('walletprocesspsbt', fundingPsbt);
    console.log('Processed funding PSBT:', processed);
    if (!processed.complete) {
      const decoded = await btcClient.command('decodepsbt', processed.psbt);
      console.dir(decoded.inputs, { depth: null });
      throw new Error('PSBT could not be finalized: incomplete signing');
    }

    const finalizedPsbt = await btcClient.command('finalizepsbt', processed.psbt);
    console.log('Finalized funding PSBT:', finalizedPsbt);

    const txid: string = await btcClient.command('sendrawtransaction', finalizedPsbt.hex);
    console.log('Broadcast funding transaction:', txid);

    const decodedFundingTx = await btcClient.command('getrawtransaction', txid, true);
    console.log('Bitcoin node decoded funding transaction:', decodedFundingTx.txid);

    console.log(`Mining confirmations to ${newAddress}`);
    await btcClient.command('generatetoaddress', 7, newAddress);
    for (let attempt = 0; attempt < 60; attempt += 1) {
      lock = await loadCosignScriptLock(vaulterClient, lockId);
      if (lock.fundedSatoshis > 0n) break;
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    expect(lock.fundedSatoshis).toBe(200_000n);
    console.log('Bitcoin Lock funding detected:', stringifyExt(lock));

    const runtimeLock = await vaulterClient.query.bitcoinLocks.locksById(lockId);
    if (runtimeLock.isNone) throw new Error('Funded Bitcoin Lock not found');
    const [[reference, satoshis]] = [...runtimeLock.unwrap().fundingUtxos.entries()];
    if (!reference || !satoshis) throw new Error('Funding UTXO reference not found');
    fundingUtxoRef = {
      txid: u8aToHex(reference.txid),
      vout: reference.outputIndex.toNumber(),
    };
    expect(satoshis.toBigInt()).toBe(lock.fundedSatoshis);
    console.log('Runtime funding UTXO reference:', fundingUtxoRef);
    expect(u8aToHex(reference.txid.slice().reverse())).toBe(`0x${txid}`);
  });

  test.sequential('it can release a bitcoin lock', async () => {
    const btcClient = vaulterchain.getBitcoinClient();
    const nextAddress = await btcClient.command('getnewaddress');
    console.log('Bitcoin release address:', nextAddress);

    const toScriptPubkey = addressBytesHex(nextAddress, bitcoinNetwork);
    const networkFee = new CosignScript(lock, bitcoinNetwork).calculateFee(
      5n,
      1,
      toScriptPubkey,
      true,
    );
    const destinationSatoshis = 50_000n;
    const changeSatoshis = lock.fundedSatoshis - destinationSatoshis - networkFee;
    console.log('Bitcoin release network fee:', `${networkFee} satoshis`);
    expect(networkFee).toBeGreaterThan(5n);

    const result = await submitTx(
      vaulterClient,
      vaulterClient.tx.bitcoinLocks.requestRelease(
        lockId,
        toScriptPubkey,
        destinationSatoshis,
        networkFee,
      ),
      bitcoinLocker,
    );
    console.log('Release request included in block:', result.blockHash);

    const request = await vaulterClient.query.bitcoinLocks.lockReleaseRequestsById(lockId);
    if (request.isNone) throw new Error('Release request not found');
    const value = request.unwrap();
    releaseRequest = {
      toScriptPubkey: value.toScriptPubkey.toHex(),
      bitcoinNetworkFee: value.bitcoinNetworkFee.toBigInt(),
      destinationSatoshis: value.destinationSatoshis.toBigInt(),
      changeSatoshis: value.changeSatoshis.toBigInt(),
    };
    expect(releaseRequest.changeSatoshis).toBe(changeSatoshis);
    console.log('Stored release request:', stringifyExt(releaseRequest));
  });

  test.sequential('it can cosign as vault', async () => {
    const cosignScript = new CosignScript(lock, bitcoinNetwork);
    const psbt = cosignScript.getCosignPsbt({
      releaseRequest,
      utxos: [{ utxoRef: fundingUtxoRef, satoshis: lock.fundedSatoshis }],
    });
    expect(psbt.outputsLength).toBe(2);
    expect(psbt.getOutput(0).amount).toBe(releaseRequest.destinationSatoshis);
    expect(psbt.getOutput(1).amount).toBe(releaseRequest.changeSatoshis);
    const signedPsbt = cosignScript.vaultCosignPsbt(psbt, lock, vaultXpriv);
    expect(signedPsbt.getInput(0).partialSig).toHaveLength(1);
    const signature = signedPsbt.getInput(0).partialSig?.[0]?.[1];
    if (!signature) throw new Error('Signature not found in PSBT');

    const result = await submitTx(
      vaulterClient,
      vaulterClient.tx.bitcoinLocks.cosignRelease(lockId, [u8aToHex(signature)]),
      vaulter,
    );
    const blockHeight = await vaulterClient
      .at(result.blockHash)
      .then(client => client.query.system.number())
      .then(number => number.toNumber());
    console.log('Vault cosign included:', { blockHash: result.blockHash, blockHeight });

    const cosigned = result.events.find(event =>
      vaulterClient.events.bitcoinLocks.BitcoinUtxoCosigned.is(event),
    );
    if (!cosigned || !vaulterClient.events.bitcoinLocks.BitcoinUtxoCosigned.is(cosigned)) {
      throw new Error('Bitcoin cosign event not found');
    }
    vaultCosignature = new Uint8Array(cosigned.data.signatures[0]);
  });

  test.sequential('user can cosign a bitcoin lock', async () => {
    const ownerBitcoinXpriv = getChildXpriv(
      mnemonicToSeedSync(bitcoinMnemonic),
      "m/84'/0'/0'/0/0'",
    );
    const cosignedTx = new CosignScript(lock, bitcoinNetwork).cosignAndGenerateTx({
      releaseRequest,
      vaultCosignatures: [vaultCosignature],
      utxos: [{ utxoRef: fundingUtxoRef, satoshis: lock.fundedSatoshis }],
      ownerXpriv: ownerBitcoinXpriv,
    });
    console.log('Cosigned Tx:', stringifyExt(cosignedTx));

    const btcClient = vaulterchain.getBitcoinClient();
    const txHex = u8aToHex(cosignedTx.toBytes(true, true), undefined, false);
    const txid = await btcClient.command('sendrawtransaction', txHex);
    console.log('Broadcast cosigned release transaction:', txid);

    const transaction = await btcClient.command('gettransaction', txid);
    console.log('Release transaction found in wallet:', stringifyExt(transaction));
    expect(transaction).toBeDefined();
  });
});

async function loadCosignScriptLock(
  client: ArgonClient,
  lockId: number,
): Promise<ICosignScriptLock> {
  const lock = await client.query.bitcoinLocks.locksById(lockId);
  if (lock.isNone) throw new Error(`Bitcoin Lock ${lockId} not found`);

  const value = lock.unwrap();
  const [parentFingerprint, cosignHdIndex] = value.vaultXpubSources;
  const wscriptHash = value.utxoScriptPubkey.asP2wsh.wscriptHash.toHex().replace('0x', '');

  return {
    createdAtHeight: value.createdAtHeight.toNumber(),
    fundedSatoshis: value.fundedSatoshis.toBigInt(),
    openClaimHeight: value.openClaimHeight.toNumber(),
    ownerPubkey: value.ownerPubkey.toHex(),
    p2wshScriptHashHex: `0x0020${wscriptHash}`,
    securitizedSatoshis: value.securitizationBasis.satoshis.toBigInt(),
    vaultClaimHeight: value.vaultClaimHeight.toNumber(),
    vaultClaimPubkey: value.vaultClaimPubkey.toHex(),
    vaultPubkey: value.vaultPubkey.toHex(),
    vaultXpubSources: {
      parentFingerprint: new Uint8Array(parentFingerprint),
      cosignHdIndex: cosignHdIndex.toNumber(),
    },
  };
}
