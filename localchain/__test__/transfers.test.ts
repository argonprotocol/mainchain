import { AccountType, CryptoScheme, NotarizationTracker } from '../index';
import { getClient, Keyring } from '@argonprotocol/mainchain';
import {
  activateNotary,
  createLocalchain,
  describeIntegration,
  disconnectOnTeardown,
  getMainchainBalance,
  TestMainchain,
  TestNotary,
  KeyringSigner,
  teardown,
  transferToLocalchain,
} from './testHelpers';
import { afterAll, afterEach, expect, it } from 'vitest';

afterEach(teardown);
afterAll(teardown);

describeIntegration('Transfer Localchain <-> Mainchain', () => {
  it('can transfer from mainchain to local', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    await notary.start({ uuid: mainchain.uuid, mainchainUrl });

    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const alice = new Keyring({ type: 'sr25519' }).createFromUri('//Alice');
    const bob = await KeyringSigner.load('//Bob');
    await activateNotary(alice, mainchainClient, notary);
    const transferId = await transferToLocalchain(
      bob.defaultPair,
      5_000_000,
      1,
      mainchainClient,
    );
    const bobchain = await createLocalchain(mainchainUrl);
    await bobchain.keystore.importSuri('//Bob', CryptoScheme.Sr25519);
    const bobMainchainClient = await bobchain.mainchainClient;
    const transfer =
      await bobMainchainClient.waitForLocalchainTransfer(transferId);

    expect(transfer).toBeTruthy();
    expect(transfer?.amount).toBe(5_000_000n);
    const notarization = bobchain.beginChange();
    const balanceChange = await notarization.addAccount(
      bob.address,
      AccountType.Deposit,
      1,
    );
    await balanceChange.claimFromMainchain(transfer);

    const tracker = await notarization.notarizeAndWaitForNotebook();
    const proof = await tracker.getNotebookProof();
    expect(proof).toHaveLength(1);
    expect(proof[0].proof).toHaveLength(0);
    expect(proof[0].address).toBe(bob.address);
    expect(proof[0].balance).toBe(5_000_000n);
    expect(proof[0].changeNumber).toBe(1);
    expect(proof[0].numberOfLeaves).toBe(1);
  }, 60e3);

  it('can transfer from mainchain to local to mainchain', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    await notary.start({ uuid: mainchain.uuid, mainchainUrl });

    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const alice = new Keyring({ type: 'sr25519' }).createFromUri('//Alice');

    await activateNotary(alice, mainchainClient, notary);

    const bobchain = await createLocalchain(mainchainUrl);

    const ferdie = new Keyring().createFromUri('//Ferdie//1', {}, 'sr25519');

    // TODO: convert this to export pkcs8 and then import it
    await bobchain.keystore.importSuri('//Bob', CryptoScheme.Sr25519);
    {
      const transfer = await bobchain.mainchainTransfers.sendToLocalchain(
        5_000_000n,
        1,
      );
      console.log('Transfer', transfer);
      expect(transfer.transferId).toBe(1);

      let tracker: NotarizationTracker = null;
      while (!tracker) {
        const result = await bobchain.balanceSync.sync();
        tracker = result.mainchainTransfers[0];
        if (!tracker) {
          await new Promise(resolve => setTimeout(resolve, 1000));
        }
      }
      await tracker.getNotebookProof();
    }

    const bobSendArgonFile = await bobchain.transactions.send(5_000_000n, [
      ferdie.address,
    ]);

    let expectedAliceBalance = 0n;
    {
      const ferdiechain = await createLocalchain(mainchainUrl);
      await ferdiechain.keystore.importSuri(
        '//Ferdie//1',
        CryptoScheme.Sr25519,
      );
      const notarization = ferdiechain.beginChange();
      await notarization.importArgonFile(bobSendArgonFile);
      const balanceChange = (await notarization.balanceChangeBuilders).find(
        x =>
          x.address == ferdie.address && x.accountType == AccountType.Deposit,
      );
      const balance = await balanceChange?.balance;
      expectedAliceBalance = balance;
      await balanceChange.sendToMainchain(balance);
      const tracker = await notarization.notarizeAndWaitForNotebook();
      console.log(
        'Notarized. Waiting for notebook %s at %s',
        tracker.notebookNumber,
        tracker.notaryId,
      );
      const mainchainClient = await ferdiechain.mainchainClient;
      const finalized = await mainchainClient.waitForNotebookImmortalized(
        tracker.notaryId,
        tracker.notebookNumber,
      );
      console.log(
        'Finalized notebook %s at %s',
        tracker.notebookNumber,
        finalized,
      );
      const changesRoot = await mainchainClient.getAccountChangesRoot(
        tracker.notaryId,
        tracker.notebookNumber,
      );
      console.log('Changes root', changesRoot);
      expect(changesRoot).toBeTruthy();
      try {
        const finalizedBlock =
          await tracker.waitForImmortalized(mainchainClient);
        console.log('Finalized block %s', finalizedBlock);
      } catch (err) {
        console.error(err);
      }
      const proof = await tracker.getNotebookProof();
      console.log('Got proof', proof);
    }

    const ferdieMainchainBalance = await getMainchainBalance(
      mainchainClient,
      ferdie.address,
    );
    expect(ferdieMainchainBalance).toBeGreaterThanOrEqual(expectedAliceBalance);
  }, 120e3);
});
