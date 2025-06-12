import { Command } from '@commander-js/extra-typings';
import { getClient, type KeyringPair, MICROGONS_PER_ARGON } from '../index';
import { printTable } from 'console-table-printer';
import { MiningBids } from '../MiningBids';
import { formatArgons } from '../utils';
import { TxSubmitter } from '../TxSubmitter';
import { accountsetFromCli, globalOptions, saveKeyringPair } from './index';
import { CohortBidder } from '../CohortBidder';

export default function miningCli() {
  const program = new Command('mining').description('Watch mining seats or setup bidding');

  program
    .command('list', { isDefault: true })
    .description('Monitor all miners')
    .action(async () => {
      const accountset = await accountsetFromCli(program);
      const bids = new MiningBids(accountset.client);
      const api = await accountset.client;
      let lastMiners: {
        [frameId: string]: {
          miner: string;
          bid?: bigint;
          isLastDay?: boolean;
        };
      } = {};

      function print(blockNumber: number) {
        console.clear();
        const toPrint = Object.entries(lastMiners).map(([seat, miner]) => ({
          seat,
          ...miner,
        }));
        if (!toPrint.length) {
          console.log('No active miners');
        } else {
          console.log(`Miners at block ${blockNumber}`);
          printTable(
            toPrint.map(x => ({
              ...x,
              bid: x.bid ? formatArgons(x.bid) : '-',
              isLastDay: x.isLastDay ? 'Y' : '',
              miner: x.miner,
            })),
          );
        }
        if (!bids.nextCohort.length) {
          console.log('-------------------------------------\nNo bids for next cohort');
        } else {
          bids.print();
        }
      }

      const { unsubscribe } = await bids.watch(accountset.namedAccounts, undefined, print);
      console.log('Watching miners...');
      const minMiners = api.consts.miningSlot.minCohortSize.toNumber();

      const unsub = await api.query.miningSlot.nextFrameId(async nextFrameId => {
        const frames = new Array(nextFrameId.toNumber())
          .fill(0)
          .map((_, i) => nextFrameId.toNumber() - i)
          .sort();
        const unseenFrames = new Set(frames);
        const entries = await api.query.miningSlot.minersByCohort.entries();
        const block = await api.query.system.number();

        const sortedEntries = entries.sort((a, b) => {
          const aIndex = a[0].args[0].toNumber();
          const bIndex = b[0].args[0].toNumber();
          return aIndex - bIndex;
        });

        for (const [rawFrameId, miners] of sortedEntries) {
          const frameId = rawFrameId.args[0].toNumber();
          unseenFrames.delete(frameId);
          let i = 0;
          for (const miner of miners) {
            const address = miner.accountId.toHuman();
            const startingFrameId = miner.startingFrameId.toNumber();
            lastMiners[`${frameId}-${i}`] = {
              miner: accountset.namedAccounts.get(address) ?? address,
              bid: miner.bid.toBigInt(),
              isLastDay: nextFrameId.toNumber() - startingFrameId === 10,
            };
            i++;
          }
          while (i < minMiners) {
            lastMiners[`${frameId}-${i}`] = {
              miner: 'none',
            };
            i++;
          }
        }
        for (const frameId of unseenFrames) {
          for (let i = 0; i < minMiners; i++) {
            lastMiners[`${frameId}-${i}`] = {
              miner: 'none',
            };
          }
        }
        print(block.toNumber());
      });
      process.on('SIGINT', () => {
        unsubscribe();
        unsub();
        process.exit(0);
      });
    });

  program
    .command('bid')
    .description('Submit mining bids within a range of parameters')
    .option('--min-bid <amount>', 'The minimum bid amount to use', parseFloat)
    .option('--max-bid <amount>', 'The maximum bid amount to use', parseFloat)
    .option('--max-seats <n>', 'The maximum number of seats to bid on for the slot', parseInt)
    .option(
      '--max-balance <argons>',
      "Use a maximum amount of the user's balance for the slot. If this ends in a percent, it will be a percent of the funds",
    )
    .option('--bid-increment <argons>', 'The bid increment', parseFloat, 0.01)
    .option('--bid-delay <ticks>', 'Delay between bids in ticks', parseInt, 0)
    .option('--run-continuous', 'Keep running and rebid every day')
    .option(
      '--proxy-for-address <address>',
      'The seed account to proxy for (eg: 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty)',
    )
    .action(
      async ({
        maxSeats,
        runContinuous,
        maxBid,
        minBid,
        maxBalance,
        bidDelay,
        bidIncrement,
        proxyForAddress,
      }) => {
        const accountset = await accountsetFromCli(program, proxyForAddress);

        let cohortBidder: CohortBidder | undefined;
        const miningBids = new MiningBids(accountset.client, false);
        const maxCohortSize = await miningBids.maxCohortSize();

        const stopBidder = async (unsubscribe: () => void) => {
          if (cohortBidder) {
            const stats = await cohortBidder.stop();
            console.log('Final bidding result', {
              cohortStartingFrameId: cohortBidder.cohortStartingFrameId,
              ...stats,
            });
            cohortBidder = undefined;
            if (!runContinuous) {
              unsubscribe();
              process.exit();
            }
          }
        };
        const { unsubscribe } = await miningBids.onCohortChange({
          async onBiddingEnd(cohortStartingFrameId) {
            if (cohortBidder?.cohortStartingFrameId === cohortStartingFrameId) {
              await stopBidder(unsubscribe);
            }
          },
          async onBiddingStart(cohortStartingFrameId) {
            const seatsToWin = maxSeats ?? maxCohortSize;
            const balance = await accountset.balance();
            const feeWiggleRoom = BigInt(25e3);
            const amountAvailable = balance - feeWiggleRoom;
            let maxBidAmount = maxBid ? BigInt(maxBid * MICROGONS_PER_ARGON) : undefined;
            let maxBalanceToUse = amountAvailable;
            if (maxBalance !== undefined) {
              if (maxBalance!.endsWith('%')) {
                let maxBalancePercent = parseInt(maxBalance);
                let amountToBid = (amountAvailable * BigInt(maxBalancePercent)) / 100n;
                if (amountToBid > balance) {
                  amountToBid = balance;
                }
                maxBalanceToUse = amountToBid;
              } else {
                maxBalanceToUse = BigInt(Math.floor(parseFloat(maxBalance) * MICROGONS_PER_ARGON));
              }

              maxBidAmount ??= maxBalanceToUse / BigInt(seatsToWin);
            }
            if (maxBalanceToUse > amountAvailable) {
              maxBalanceToUse = amountAvailable;
            }
            if (!maxBidAmount) {
              console.error('No max bid amount set');
              process.exit(1);
            }
            const subaccountRange = await accountset.getAvailableMinerAccounts(seatsToWin);

            if (cohortBidder && cohortBidder?.cohortStartingFrameId !== cohortStartingFrameId) {
              await stopBidder(unsubscribe);
            }
            cohortBidder = new CohortBidder(accountset, cohortStartingFrameId, subaccountRange, {
              maxBid: maxBidAmount,
              minBid: BigInt((minBid ?? 0) * MICROGONS_PER_ARGON),
              bidIncrement: BigInt(Math.floor(bidIncrement * MICROGONS_PER_ARGON)),
              maxBudget: maxBalanceToUse,
              bidDelay,
            });
            await cohortBidder.start();
          },
        });
      },
    );

  program
    .command('create-bid-proxy')
    .description('Create a mining-bid proxy account for your main account')
    .requiredOption(
      '--outfile <path>',
      'The file to use to store the proxy account json (eg: proxy.json)',
    )
    .requiredOption(
      '--fee-argons <argons>',
      'How many argons should be sent to the proxy account for fees (proxies must pay fees)',
      parseFloat,
    )
    .option('--proxy-passphrase <passphrase>', 'The passphrase for your proxy account')
    .action(async ({ outfile, proxyPassphrase, feeArgons }) => {
      const { mainchainUrl } = globalOptions(program);
      const client = await getClient(mainchainUrl);

      const keyringPair = await saveKeyringPair({
        filePath: outfile,
        passphrase: proxyPassphrase,
      });
      const address = keyringPair.address;
      console.log(`✅ Created proxy account at "${outfile}" with address ${address}`);
      const tx = client.tx.utility.batchAll([
        client.tx.proxy.addProxy(address, 'MiningBid', 0),
        client.tx.balances.transferAllowDeath(address, BigInt(feeArgons * MICROGONS_PER_ARGON)),
      ]);
      let keypair: KeyringPair;
      try {
        const accountset = await accountsetFromCli(program);
        keypair = accountset.txSubmitterPair;
      } catch (e) {
        const polkadotLink = `https://polkadot.js.org/apps/?rpc=${mainchainUrl}#/extrinsics/decode/${tx.toHex()}`;
        console.log(`Complete the registration at this link:`, polkadotLink);
        process.exit(0);
      }
      try {
        await new TxSubmitter(client, tx, keypair).submit({
          waitForBlock: true,
        });

        console.log('Mining bid proxy added and funded.');
        process.exit();
      } catch (error) {
        console.error('Error adding mining proxy', error);
        process.exit(1);
      }
    });
  return program;
}
