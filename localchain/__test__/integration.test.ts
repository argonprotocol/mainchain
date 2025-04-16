import { AccountType, MainchainClient, NotaryClient } from '../index';
import { Keyring } from '@argonprotocol/mainchain';
import {
  describeIntegration,
  teardown,
  TestMainchain,
  TestNotary,
} from './testHelpers';

import { afterAll, afterEach, expect, it } from 'vitest';

afterEach(teardown);
afterAll(teardown);

describeIntegration('Integration tests', () => {
  it('can start a mainchain', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const url = new URL(mainchainUrl);
    expect(url.protocol).toEqual('ws:');
    expect(url.port).toBeTruthy();
  });

  it('can get a ticker', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();

    const client = await MainchainClient.connect(mainchainUrl, 2000);

    await expect(client.getTicker()).resolves.toBeTruthy();
  });

  it('can start a notary', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    const notaryUrl = await notary.start({
      uuid: mainchain.uuid,
      mainchainUrl,
    });
    expect(notaryUrl).toBeTruthy();
    const url = new URL(notaryUrl);
    expect(url.protocol).toEqual('ws:');
    expect(url.port).toBeTruthy();

    const addressKeyring = new Keyring({ type: 'sr25519' });
    const bob = addressKeyring.createFromUri('//Bob', { type: 'ed25519' });
    const client = await NotaryClient.connect(
      1,
      Buffer.from(bob.publicKey),
      notaryUrl,
      false,
    );
    const metadata = await client.metadata;
    expect(metadata.finalizedNotebookNumber).toBeGreaterThanOrEqual(0);

    await expect(
      client.getBalanceTip(bob.address, AccountType.Deposit),
    ).rejects.toThrow();
  });
});
