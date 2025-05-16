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
import { inspect } from 'util';

// set the default log depth to 10
inspect.defaultOptions.depth = 10;
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
    const startingCohort = await aliceClient.query.miningSlot.nextFrameId();
    await new Promise(resolve => {
      const unsub = aliceClient.query.miningSlot.nextFrameId(x => {
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
      async onBiddingStart(cohortFrameId) {
        if (bobBidder) return;
        console.log(`Cohort ${cohortFrameId} started bidding`);
        bobBidder = new CohortBidder(
          bob,
          cohortFrameId,
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
          cohortFrameId,
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
      async onBiddingEnd(cohortFrameId) {
        console.log(`Cohort ${cohortFrameId} ended bidding`);
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
      aliceClient.query.miningSlot.nextFrameId(y => {
        if (y.toNumber() >= bobBidder!.cohortFrameId) {
          resolve(true);
        }
      }),
    );

    const aliceMiners = await alice.miningSeats();
    const aliceStats = aliceBidder!.stats;
    const cohortFrameId = aliceBidder!.cohortFrameId;
    const bobMiners = await bob.miningSeats();
    const bobStats = bobBidder!.stats;

    console.log({ cohortFrameId, bobStats, aliceStats });
    console.log('bob', bobBidder!.bidHistory);
    console.log('alice', aliceBidder!.bidHistory);

    const bobActive = bobMiners.filter(x => x.seat !== undefined);
    const aliceActive = aliceMiners.filter(x => x.seat !== undefined);

    expect(bobActive.length).toBe(bobStats.seatsWon);
    expect(bobStats.bidsAttempted).toBeGreaterThanOrEqual(20);
    expect(bobActive.reduce((acc, x) => acc + x.bidAmount!, 0n)).toBe(
      bobStats.totalArgonsBid,
    );
    // expect 5 rounds of bidding
    expect(bobStats.fees).toBeGreaterThanOrEqual(22_000n * 4n);

    expect(aliceStats.bidsAttempted).toBeGreaterThanOrEqual(20);
    expect(aliceActive.length).toBe(aliceStats.seatsWon);
    expect(aliceActive.reduce((acc, x) => acc + x.bidAmount!, 0n)).toBe(
      aliceStats.totalArgonsBid,
    );
    console.log('Waiting for each bidder to mine');
    await Promise.all([bobMinePromise, aliceMinePromise]);
  }, 180e3);
});
