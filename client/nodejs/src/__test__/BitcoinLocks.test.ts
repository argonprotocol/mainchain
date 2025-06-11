import {
  addTeardown,
  describeIntegration, runOnTeardown,
  sudo,
  teardown,
  TestBitcoinCli,
  TestMainchain,
  TestOracle,
} from '@argonprotocol/testing';
import {
  Accountset,
  ArgonClient,
  BitcoinLocks,
  TxSubmitter,
  VaultMonitor,
} from '../index';
import { parseSubaccountRange } from '../Accountset';
import { afterAll, beforeAll, expect, test } from 'vitest';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import * as fs from 'node:fs';
import { Keyring } from '@polkadot/api';

afterAll(teardown);

describeIntegration(
  'BitcoinLock tests',
  () => {
    let alicechain: TestMainchain;
    let aliceClient: ArgonClient;
    let alice: Accountset;
    beforeAll(async () => {
      alicechain = new TestMainchain();
      await alicechain.launch({
        miningThreads: 1,
        launchBitcoin: true,
        author: 'alice',
      });
      aliceClient = await alicechain.client();

      const bitcoinOracle = new TestOracle();
      await bitcoinOracle.start('bitcoin', {
        mainchainUrl: alicechain.address,
        bitcoinRpcUrl: `http://bitcoin:bitcoin@localhost:${alicechain.bitcoinPort!}`,
      });

      alice = new Accountset({
        client: Promise.resolve(aliceClient),
        seedAccount: sudo(),
        subaccountRange: parseSubaccountRange('0-49'),
        sessionKeyMnemonic: mnemonicGenerate(),
      });
    });

    test.sequential(
      'it can monitor vaults for bitcoin space and lock once space is available',
      async () => {
        const vaultMonitor = new VaultMonitor(alice, {
          bitcoinSpaceAvailable: 5_000n,
        });
        const didRegisterVaultId = new Promise<number>(resolve =>
          vaultMonitor.events.on('bitcoin-space-above', resolve),
        );
        await vaultMonitor.monitor(false);

        const path = fs.mkdtempSync('/tmp/argon-bitcoin-locks-test-');
        runOnTeardown(() => fs.promises.rm(path, { recursive: true }));
        TestBitcoinCli.run(
          `xpriv master --xpriv-path="${path}/xpriv.key" --xpriv-password=1234 --bitcoin-network=regtest`,
        );
        const xpub = TestBitcoinCli.run(
          `xpriv derive-xpub --xpriv-path="${path}/xpriv.key"  --xpriv-password=1234 --hd-path="m/84'/0'/0'"`,
        );

        const vaultCreate = TestBitcoinCli.run(
          `vault create --argons=₳50 --securitization-ratio=1x --bitcoin-apr=0.5% --bitcoin-base-fee=₳0.50 --liquidity-pool-profit-sharing=50% --bitcoin-xpub=${xpub} -t ${alicechain.address}`,
        );
        const txHex = vaultCreate.match(`extrinsics/decode/(.+)`)?.[1];
        const call = aliceClient.registry.createType('Call', txHex);
        const ext = aliceClient.tx(call);
        const tx = await alice.tx(ext);
        await tx.submit({ waitForBlock: true });

        await expect(didRegisterVaultId).resolves.toBe(1);
        vaultMonitor.stop();
      },
    );

    test.sequential('a user can wait for bitcoin space', async () => {
      const currentTick = await aliceClient.query.ticks.currentTick();
      const tx = new TxSubmitter(
        aliceClient,
        aliceClient.tx.priceIndex.submit({
          btcUsdPrice: BigInt(60_000.5e18),
          argonUsdPrice: BigInt(1.0e18),
          argonotUsdPrice: BigInt(12.0e18),
          argonUsdTargetPrice: BigInt(1.0e18),
          argonTimeWeightedAverageLiquidity: BigInt(1_000e18),
          tick: currentTick.toNumber(),
        }),
        new Keyring({ type: 'sr25519' }).addFromUri(
          TestOracle.PriceIndexOperator,
        ),
      );
      await tx.submit({ waitForBlock: true });

      const btcClient = alicechain.getBitcoinClient();
      await btcClient.command('createwallet', 'default').catch(() => null);
      await btcClient.command('loadwallet', 'default').catch(() => null);
      const newAddress = await btcClient.command('getnewaddress');
      expect(newAddress).toBeTypeOf('string');
      console.log(newAddress);
      await btcClient.command('generatetoaddress', 101, newAddress);

      const addressinfo = await btcClient.command('getaddressinfo', newAddress);
      console.log(addressinfo);
      const scriptPubKey = addressinfo.pubkey;

      const bobaccount = new Accountset({
        seedAccount: new Keyring({ type: 'sr25519' }).addFromUri('//Bob'),
        client: Promise.resolve(aliceClient),
        sessionKeyMnemonic: mnemonicGenerate(),
      });
      const result = await BitcoinLocks.waitForSpace(bobaccount, {
        argonAmount: 10_000_000n,
        bitcoinXpub: `0x${scriptPubKey}`,
      });
      console.log('Locked bitcoin space', result);
      expect(result.vaultId).toBe(1);
      expect(result.satoshis).toBeGreaterThan(1000);
      expect(result.utxoId).toBe(1);
      await expect(result.finalizedPromise).resolves.toBeTruthy();

      // now complete bitcoin lock
      const lockSendOutput = TestBitcoinCli.run(
        `lock send-to-address --utxo-id 1 -t ${alicechain.address}`,
      );
      const vaultScriptpub = lockSendOutput.match(
        /satoshis to ([a-z0-9]+1[a-z0-9]+)/,
      )?.[1];

      console.log('Sending to vault scriptpub', vaultScriptpub, lockSendOutput);

      const walletBalance = await btcClient.command('getbalances');
      const btc = Number(result.satoshis) / 100_000_000;
      console.log('Wallet balance:', walletBalance, 'Needed:', btc);
      console.log(`sendtoaddress ${vaultScriptpub} ${btc}`);
      await btcClient.command('sendtoaddress', vaultScriptpub, btc);
      await btcClient.command('generatetoaddress', 7, newAddress);

      // wait for the bitcoin to verify
      await new Promise<void>(async (resolve, reject) => {
        const unsub = await aliceClient.query.bitcoinLocks.locksByUtxoId(
          1,
          y => {
            if (!y.isSome) reject('No lock found');
            const lock = y.unwrap();
            if (lock.isVerified) {
              resolve();
              unsub();
            }
          },
        );
      });
    });
  },
  { retry: 0 },
);
