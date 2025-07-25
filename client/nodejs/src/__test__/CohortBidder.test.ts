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
      async onBiddingStart(cohortStartingFrameId) {
        if (bobBidder) return;
        console.log(`Cohort ${cohortStartingFrameId} started bidding`);
        bobBidder = new CohortBidder(
          bob,
          cohortStartingFrameId,
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
          cohortStartingFrameId,
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
      async onBiddingEnd(cohortStartingFrameId) {
        console.log(`Cohort ${cohortStartingFrameId} ended bidding`);
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
        if (y.toNumber() >= bobBidder!.cohortStartingFrameId) {
          resolve(true);
        }
      }),
    );

    const aliceMiners = await alice.miningSeats();
    const cohortStartingFrameId = aliceBidder!.cohortStartingFrameId;
    const bobMiners = await bob.miningSeats();

    const aliceStats = { seatsWon: aliceBidder!.winningBids.length, fees: aliceBidder!.txFees };
    const bobStats = { seatsWon: bobBidder!.winningBids.length, fees: bobBidder!.txFees };
    console.log({ cohortStartingFrameId, aliceMiners, bobStats });

    const bobActive = bobMiners.filter(x => x.seat !== undefined);
    const aliceActive = aliceMiners.filter(x => x.seat !== undefined);

    expect(bobActive.length).toBe(bobStats.seatsWon);
    expect(bobStats.fees).toBeGreaterThanOrEqual(6_000n * 4n);

    expect(aliceActive.length).toBe(aliceStats.seatsWon);
    console.log('Waiting for each bidder to mine');
    if (bobStats.seatsWon > 0) {
      await expect(bobMinePromise).resolves.toBeTruthy();
    }
    if (aliceStats.seatsWon > 0) {
      await expect(aliceMinePromise).resolves.toBeTruthy();
    }
  }, 180e3);
});
