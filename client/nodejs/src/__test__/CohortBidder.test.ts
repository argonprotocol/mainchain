import { Accountset, CohortBidder, MiningBids } from '../index';
import {
  activateNotary,
  describeIntegration,
  sudo,
  teardown,
  TestMainchain,
  TestNotary,
} from '@argonprotocol/testing';
import { parseSubaccountRange } from '../Accountset';
import { Keyring } from '@polkadot/api';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import { afterAll, afterEach, it, expect } from 'vitest';

afterEach(teardown);
afterAll(teardown);

describeIntegration('Cohort Bidder tests', () => {
  it('can compete on bids', async () => {
    const alicechain = new TestMainchain();
    await alicechain.launch({ miningThreads: 1 });
    const notary = new TestNotary();
    await notary.start({
      uuid: alicechain.uuid,
      mainchainUrl: alicechain.address,
    });
    const aliceClientPromise = alicechain.client();
    const aliceClient = await aliceClientPromise;
    await activateNotary(sudo(), aliceClient, notary);

    const alice = new Accountset({
      client: aliceClientPromise,
      seedAccount: sudo(),
      subaccountRange: parseSubaccountRange('0-49'),
      sessionKeyMnemonic: mnemonicGenerate(),
      name: 'alice',
    });
    await alice.registerKeys(alicechain.address);

    const bobchain = new TestMainchain();
    await bobchain.launch({
      miningThreads: 1,
      bootnodes: await alicechain.bootAddress(),
      author: 'bob',
    });
    const bob = new Accountset({
      client: bobchain.client(),
      seedAccount: new Keyring({ type: 'sr25519' }).addFromUri('//Bob'),
      subaccountRange: parseSubaccountRange('0-49'),
      sessionKeyMnemonic: mnemonicGenerate(),
      name: 'bob',
    });
    await bob.registerKeys(bobchain.address);

    // wait for bob to have ownership tokens
    await new Promise(async resolve => {
      const unsub = await (
        await bob.client
      ).query.ownership.account(bob.seedAddress, x => {
        if (x.free.toBigInt() > 100_000n) {
          resolve(true);
          unsub();
        } else {
          console.log(`Waiting for bob to have ownership tokens`);
        }
      });
    });

    const miningBids = new MiningBids(aliceClientPromise);
    let bobBidder: CohortBidder;
    let aliceBidder: CohortBidder;
    // wait for the cohort to change so we have enough time
    const startingCohort = await aliceClient.query.miningSlot.nextCohortId();
    await new Promise(resolve => {
      const unsub = aliceClient.query.miningSlot.nextCohortId(x => {
        if (x.toNumber() > startingCohort.toNumber()) {
          resolve(true);
          unsub.then();
        }
      });
    });

    let waitForStopPromise: () => void;
    const waitForStop = new Promise<void>(resolve => {
      waitForStopPromise = resolve;
    });
    const { unsubscribe } = await miningBids.onCohortChange({
      async onBiddingStart(cohortId) {
        if (bobBidder) return;
        console.log(`Cohort ${cohortId} started bidding`);
        bobBidder = new CohortBidder(
          bob,
          cohortId,
          await bob.getAvailableMinerAccounts(10),
          {
            minBid: 10_000n,
            maxBid: 5_000_000n,
            maxBudget: 25_000_000n,
            bidIncrement: 1_000_000n,
            bidDelay: 0,
          },
        );
        aliceBidder = new CohortBidder(
          alice,
          cohortId,
          await alice.getAvailableMinerAccounts(10),
          {
            minBid: 10_000n,
            maxBid: 4_000_000n,
            maxBudget: 40_000_000n,
            bidIncrement: 1_000_000n,
            bidDelay: 0,
          },
        );
        await bobBidder.start();
        await aliceBidder.start();
      },
      async onBiddingEnd(cohortId) {
        console.log(`Cohort ${cohortId} ended bidding`);
        await aliceBidder.stop();
        await bobBidder.stop();
        waitForStopPromise();
      },
    });
    await waitForStop;
    unsubscribe();

    expect(aliceBidder!).toBeTruthy();
    expect(bobBidder!).toBeTruthy();

    const bobWatch = await bob.watchBlocks(true);
    const bobMinePromise = new Promise<{ argons: bigint }>(resolve => {
      bobWatch.events.on('mined', (_blockHash, mined) => {
        resolve(mined);
      });
    });
    const aliceWatch = await alice.watchBlocks(true);
    const aliceMinePromise = new Promise<{ argons: bigint }>(resolve => {
      aliceWatch.events.on('mined', (_blockHash, mined) => {
        resolve(mined);
      });
    });
    // wait for the slot to fully complete
    await new Promise(resolve =>
      aliceClient.query.miningSlot.nextCohortId(y => {
        if (y.toNumber() >= bobBidder!.cohortId) {
          resolve(true);
        }
      }),
    );

    const aliceMiners = await alice.miningSeats();
    const aliceStats = aliceBidder!.stats;
    const cohortId = aliceBidder!.cohortId;
    const bobMiners = await bob.miningSeats();
    const bobStats = bobBidder!.stats;

    console.log({ cohortId, bobStats, aliceStats });

    const bobActive = bobMiners.filter(x => x.seat !== undefined);
    const aliceActive = aliceMiners.filter(x => x.seat !== undefined);

    expect(bobActive.length).toBe(bobStats.seats);
    expect(bobStats.bids).toBeGreaterThanOrEqual(20);
    expect(bobActive.reduce((acc, x) => acc + x.bidAmount!, 0n)).toBe(
      bobStats.totalArgonsBid,
    );
    // expect 5 rounds of bidding
    expect(bobStats.fees).toBeGreaterThanOrEqual(22_000n * 4n);

    expect(aliceStats.bids).toBeGreaterThanOrEqual(20);
    expect(aliceActive.length).toBe(aliceStats.seats);
    expect(aliceActive.reduce((acc, x) => acc + x.bidAmount!, 0n)).toBe(
      aliceStats.totalArgonsBid,
    );
    console.log('Waiting for each bidder to mine');
    await Promise.all([bobMinePromise, aliceMinePromise]);
  }, 180e3);
});
