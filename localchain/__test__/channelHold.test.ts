import {
  BalanceChangeBuilder,
  Chain,
  DomainStore,
  DomainTopLevel,
  Localchain,
  NotarizationBuilder,
} from '../index';
import {
  ArgonClient,
  ArgonPrimitivesDomainVersionHost,
  checkForExtrinsicSuccess,
  getClient,
  Keyring,
  KeyringPair,
} from '@argonprotocol/mainchain';
import {
  activateNotary,
  createLocalchain,
  disconnectOnTeardown,
  KeyringSigner,
  SKIP_E2E,
  teardown,
  TestMainchain,
  TestNotary,
  transferToLocalchain,
} from './testHelpers';
import * as Crypto from 'node:crypto';
import { afterAll, afterEach, describe, expect, it } from 'vitest';

afterEach(teardown);
afterAll(teardown);

describe.skipIf(SKIP_E2E)('ChannelHold integration', {}, () => {
  it('can create a zone record type', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const domainHash = Crypto.createHash('sha256').update('example.com').digest();
    const ferdieDomainAddress = new Keyring({ type: 'sr25519' }).createFromUri(
      '//Ferdie//domain//1',
    );
    const ferdie = new Keyring({ type: 'sr25519' }).createFromUri('//Ferdie');

    await expect(
      registerZoneRecord(mainchainClient, domainHash, ferdie, ferdieDomainAddress.publicKey, 1, {
        '1.0.0': mainchainClient.createType('ArgonPrimitivesDomainVersionHost', {
          datastoreId: mainchainClient.createType('Bytes', 'default'),
          host: 'ws://192.168.1.1:80',
        }),
      }),
    ).rejects.toThrow('ExtrinsicFailed:: domains.DomainNotRegistered');
  }, 120e3);

  it('can create an channelHold from a domain registration', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    await notary.start({ uuid: mainchain.uuid, mainchainUrl });

    const ferdiekeys = await KeyringSigner.load('//Ferdie');

    const sudo = new Keyring({ type: 'sr25519' }).createFromUri('//Alice');
    const bobkeys = new Keyring({ type: 'sr25519' });

    const bob = bobkeys.addFromUri('//Bob');
    const ferdieVotesAddress = new Keyring({ type: 'sr25519' }).createFromUri('//Ferdie//voter//1');

    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    await activateNotary(sudo, mainchainClient, notary);

    const bobchain = await createLocalchain(mainchainUrl);
    const overview = await bobchain.accountOverview();
    expect(overview.mainchainIdentity.chain).toBe(Chain.Devnet);

    await bobchain.keystore.useExternal(
      bob.address,
      async (address, signatureMessage) => {
        return bobkeys.getPair(address)?.sign(signatureMessage, { withType: true });
      },
      async hd_path => {
        return bobkeys.addPair(bob.derive(hd_path)).address;
      },
    );

    const ferdiechain = await createLocalchain(mainchainUrl);
    // eslint-disable-next-line @typescript-eslint/unbound-method
    await ferdiechain.keystore.useExternal(ferdiekeys.address, ferdiekeys.sign, ferdiekeys.derive);

    const domain = {
      name: 'example',
      topLevel: DomainTopLevel.Analytics,
    };
    const domainHash = DomainStore.getHash('example.analytics');
    {
      const [bobChange, ferdieChange] = await Promise.all([
        transferMainchainToLocalchain(mainchainClient, bobchain, bob, 5_200_000, 1),
        transferMainchainToLocalchain(
          mainchainClient,
          ferdiechain,
          ferdiekeys.defaultPair,
          5_000_000,
          1,
        ),
      ]);
      await ferdieChange.notarization.leaseDomain('example.Analytics', ferdiekeys.address);
      const [ferdieTracker] = await Promise.all([
        bobChange.notarization.notarizeAndWaitForNotebook(),
        ferdieChange.notarization.notarizeAndWaitForNotebook(),
      ]);

      const domains = await ferdiechain.domains.list;
      expect(domains[0].name).toBe(domain.name);
      expect(domains[0].topLevel).toBe('analytics');

      const ferdieMainchainClient = await ferdiechain.mainchainClient;
      await ferdieTracker.waitForImmortalized(ferdieMainchainClient);
      await expect(
        ferdieMainchainClient.getDomainRegistration(domain.name, domain.topLevel),
      ).resolves.toBeTruthy();
    }

    await registerZoneRecord(
      mainchainClient,
      domainHash,
      ferdiekeys.defaultPair,
      ferdiekeys.defaultPair.publicKey,
      1,
      {
        '1.0.0': mainchainClient.createType('ArgonPrimitivesDomainVersionHost', {
          datastoreId: mainchainClient.createType('Bytes', 'default'),
          host: 'ws://192.168.1.1:80',
        }),
      },
    );

    const mainchainClientBob = await bobchain.mainchainClient;
    const zoneRecord = await mainchainClientBob.getDomainZoneRecord(domain.name, domain.topLevel);
    expect(zoneRecord).toBeTruthy();
    expect(zoneRecord.notaryId).toBe(1);
    expect(zoneRecord.paymentAddress).toBe(ferdiekeys.address);
    const channelHoldFunding = bobchain.beginChange();
    const jumpAccount = await channelHoldFunding.fundJumpAccount(5_200_000n);
    await channelHoldFunding.notarize();

    const bobChannelHold = bobchain.beginChange();
    const change = await bobChannelHold.addAccountById(jumpAccount.localAccountId);
    await change.createChannelHold(5_000_000n, zoneRecord.paymentAddress, 'example.Analytics');
    const holdTracker = await bobChannelHold.notarizeAndWaitForNotebook();

    const clientChannelHold = await bobchain.openChannelHolds.openClientChannelHold(
      jumpAccount.localAccountId,
    );
    await clientChannelHold.sign(5_000_000n);
    const channelHoldJson = await clientChannelHold.exportForSend();
    {
      const parsed = JSON.parse(channelHoldJson.toString());
      console.log(parsed);
      expect(parsed).toBeTruthy();
      expect(parsed.channelHoldNote).toBeTruthy();
      expect(parsed.channelHoldNote.microgons).toBe(5_000_000);
      expect(parsed.notes[0].microgons).toBe(5_000_000);
      expect(parsed.balance).toBe(parsed.previousBalanceProof.balance - 5_000_000);
    }

    const ferdieChannelHoldRecord =
      await ferdiechain.openChannelHolds.importChannelHold(channelHoldJson);

    // get to 2500 in channelHold costs so that 20% is 500 (minimum vote)
    for (let i = 0n; i <= 10n; i++) {
      const next = await clientChannelHold.sign(5_000n + i * 200n);
      // now we would send to ferdie
      await expect(
        ferdieChannelHoldRecord.recordUpdatedSettlement(next.microgons, next.signature),
      ).resolves.toBeUndefined();
    }

    // now ferdie goes to claim it
    const result = await ferdiechain.balanceSync.sync({
      votesAddress: ferdieVotesAddress.address,
    });
    expect(result).toBeTruthy();
    expect(result.balanceChanges).toHaveLength(2);
    expect(result.channelHoldNotarizations).toHaveLength(0);

    const insideChannelHold = await ferdieChannelHoldRecord.channelHold;
    const currentTick = ferdiechain.currentTick;
    const ticker = ferdiechain.ticker;
    expect(insideChannelHold.expirationTick).toBe(
      holdTracker.tick + ticker.channelHoldExpirationTicks,
    );
    const timeForExpired = new Date(
      Number(ferdiechain.ticker.timeForTick(insideChannelHold.expirationTick)),
    );
    console.log(
      'ChannelHold expires in %s seconds. Current Tick=%s, expiration=%s',
      (timeForExpired.getTime() - Date.now()) / 1000,
      currentTick,
      insideChannelHold.expirationTick,
    );
    expect(timeForExpired.getTime() - Date.now()).toBeLessThan(30e3);
    await new Promise(resolve => setTimeout(resolve, timeForExpired.getTime() - Date.now() + 10));

    const ferdieMainchainClient = await ferdiechain.mainchainClient;
    // in the balance sync, we'd normally just keep trying to vote with the latest expiring channelHolds, but in this test, we only have 1, so we need to wait for a grandparent hash
    for (let i = 0; i < 10; i += 1) {
      try {
        const voteBlocks = await ferdieMainchainClient.getVoteBlockHash(ferdiechain.currentTick);
        if (voteBlocks.blockHash) {
          console.log('Vote block hash=%s', Buffer.from(voteBlocks.blockHash).toString('hex'));
          break;
        }
      } catch {}
      await new Promise(resolve =>
        setTimeout(resolve, Number(ferdiechain.ticker.millisToNextTick())),
      );
    }

    let syncResult = await ferdiechain.balanceSync.sync({
      votesAddress: ferdieVotesAddress.address,
    });
    console.log(
      'Result of balance sync notarization of channelHold. Balance Changes=%s, ChannelHolds=%s, Votes=%s',
      syncResult.balanceChanges.length,
      syncResult.channelHoldNotarizations.length,
      syncResult.blockVotes.length,
    );
    expect(syncResult.channelHoldNotarizations).toHaveLength(1);
    const notarization = syncResult.channelHoldNotarizations[0];
    const notarizationChannelHolds = await notarization.channelHolds;
    expect(notarizationChannelHolds).toHaveLength(1);
    expect(notarizationChannelHolds[0].id).toBe(insideChannelHold.id);

    if (syncResult.blockVotes.length === 0) {
      // try 10 more times to vote
      for (let i = 0; i < 10; i++) {
        await new Promise(resolve =>
          setTimeout(resolve, Number(ferdiechain.ticker.millisToNextTick())),
        );

        syncResult = await ferdiechain.balanceSync.sync({
          votesAddress: ferdieVotesAddress.address,
        });
        console.log(`Votes on try ${i + 2} -> ${syncResult.blockVotes.length}`);
        if (syncResult.blockVotes.length) break;
      }
    }
    expect(syncResult.blockVotes.length).toBe(1);
  }, 120e3);
});

async function transferMainchainToLocalchain(
  mainchainClient: ArgonClient,
  localchain: Localchain,
  account: KeyringPair,
  amount: number,
  notaryId: number,
): Promise<{
  notarization: NotarizationBuilder;
  balanceChange: BalanceChangeBuilder;
}> {
  const transferId = await transferToLocalchain(account, amount, notaryId, mainchainClient);
  const locMainchainClient = await localchain.mainchainClient;
  const transfer = await locMainchainClient.waitForLocalchainTransfer(transferId);
  await new Promise(resolve => setTimeout(resolve, 500));
  const notarization = localchain.beginChange();
  const balanceChange = await notarization.claimFromMainchain(transfer);
  return { notarization, balanceChange };
}

async function registerZoneRecord(
  client: ArgonClient,
  domainHash: Uint8Array,
  owner: KeyringPair,
  paymentAccount: Uint8Array,
  notaryId: number,
  versions: Record<string, ArgonPrimitivesDomainVersionHost>,
) {
  const codecVersions = new Map();
  for (const [version, host] of Object.entries(versions)) {
    const [major, minor, patch] = version.split('.');
    const versionCodec = client.createType('ArgonPrimitivesDomainSemver', {
      major,
      minor,
      patch,
    });
    codecVersions.set(versionCodec, client.createType('ArgonPrimitivesDomainVersionHost', host));
  }

  await new Promise((resolve, reject) =>
    client.tx.domains
      .setZoneRecord(domainHash, {
        paymentAccount,
        notaryId,
        versions: codecVersions,
      })
      .signAndSend(owner, ({ events, status }) => {
        if (status.isFinalized) {
          checkForExtrinsicSuccess(events, client).then(resolve).catch(reject);
        }
        if (status.isInBlock) {
          checkForExtrinsicSuccess(events, client).catch(reject);
        }
      }),
  );
}
