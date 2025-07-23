import {
  Accountset,
  ArgonClient,
  createKeyringPair,
  getClient,
  TxSubmitter,
  mnemonicGenerate,
} from '../index';
import { describeIntegration, sudo, teardown, TestMainchain } from '@argonprotocol/testing';
import { parseSubaccountRange } from '../Accountset';
import { afterAll, beforeAll, expect, it } from 'vitest';

afterAll(teardown);

describeIntegration('Accountset tests', () => {
  let mainchain: TestMainchain;
  let client: Promise<ArgonClient>;
  beforeAll(async () => {
    mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    client = getClient(mainchainUrl);
  });

  it('can derive multiple accounts', async () => {
    const seedAccount = createKeyringPair({});
    const accountset = new Accountset({
      client,
      seedAccount,
      subaccountRange: parseSubaccountRange('0-49'),
      sessionKeyMnemonic: mnemonicGenerate(),
    });

    expect(Object.keys(accountset.subAccountsByAddress).length).toBe(50);
    expect(accountset.accountRegistry.getName(seedAccount.address)).toContain('//seed');
    for (const i of Object.keys(accountset.subAccountsByAddress)) {
      expect(accountset.accountRegistry.getName(i)).toBeTruthy();
    }
  });

  it('can submit bids', async () => {
    const seedAccount = sudo();
    const accountset = new Accountset({
      client,
      seedAccount,
      subaccountRange: parseSubaccountRange('0-49'),
      sessionKeyMnemonic: mnemonicGenerate(),
    });

    const nextSeats = await accountset.getAvailableMinerAccounts(5);
    expect(nextSeats).toHaveLength(5);
    const existingBids = await accountset.bids();
    expect(existingBids.filter(x => x.bidPlace !== undefined)).toHaveLength(0);
    {
      const result = await accountset.createMiningBids({
        bidAmount: 1000n,
        subaccountRange: [0, 1, 2, 3, 4],
        tip: 100n,
      });
      expect(result).toBeTruthy();
      expect(result.finalFee).toBeGreaterThan(6000);
      expect(result.successfulBids).toBe(0);
      expect(result.bidError?.stack).toMatch('InvalidBidAmount');
    }
    const result = await accountset.createMiningBids({
      bidAmount: 10_000n,
      subaccountRange: [0, 1, 2, 3, 4],
      tip: 100n,
    });
    expect(result).toBeTruthy();
    expect(result.finalFee).toBeGreaterThan(6000);
    expect(result.successfulBids).toBe(5);
    expect(result.bidError).toBeFalsy();
    const bids = await accountset.bids(result.blockHash);
    console.log(bids);
    expect(bids.filter(x => x.bidPlace !== undefined)).toHaveLength(5);
  });

  it('can will handle a subset of failed bids', async () => {
    const alice = sudo();
    const secondAccount = createKeyringPair({});

    const api = await client;
    const txSubmitter = new TxSubmitter(
      api,
      api.tx.utility.batchAll([
        api.tx.balances.transferKeepAlive(secondAccount.address, 5_000_000),
        api.tx.ownership.transferKeepAlive(secondAccount.address, 500_000),
      ]),
      alice,
    );
    await txSubmitter.submit({ waitForBlock: true });
    let startingCohortStartingFrameId = await api.query.miningSlot
      .nextFrameId()
      .then(x => x.toNumber());
    await new Promise(resolve =>
      api.query.miningSlot.nextFrameId(x => {
        if (x.toNumber() > startingCohortStartingFrameId) {
          resolve(true);
        }
      }),
    );

    const account = new Accountset({
      client,
      seedAccount: alice,
      subaccountRange: parseSubaccountRange('0-49'),
      sessionKeyMnemonic: mnemonicGenerate(),
    });
    const subaccounts = await account.getAvailableMinerAccounts(7);
    const alice_result = await account.createMiningBids({
      bidAmount: 2_000_000n,
      subaccountRange: subaccounts.map(x => x.index),
    });
    expect(alice_result.bidError).toBeFalsy();
    expect(alice_result.successfulBids).toBe(7);

    const second_bids = await new Accountset({
      client,
      seedAccount: secondAccount,
      subaccountRange: parseSubaccountRange('0-49'),
      sessionKeyMnemonic: mnemonicGenerate(),
    }).createMiningBids({
      bidAmount: 500_000n,
      subaccountRange: [0, 1, 2, 3, 4],
      tip: 100n,
    });
    expect(second_bids).toBeTruthy();
    expect(second_bids.finalFee).toBeGreaterThan(6000n);
    expect(second_bids.successfulBids).toBe(3);
    expect(second_bids.bidError?.stack).toMatch('BidTooLow');
  });
});
