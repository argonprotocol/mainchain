import {
    AccountType,
    BalanceChangeBuilder,
    DataDomain,
    DataTLD,
    Localchain,
    NotarizationBuilder,
    Signer,
} from "../index";
import { format } from 'node:util';
import TestMainchain from "./TestMainchain";
import TestNotary from "./TestNotary";
import {
    checkForExtrinsicSuccess,
    getClient,
    Keyring,
    KeyringPair,
    UlxClient,
    UlxPrimitivesDataDomainVersionHost
} from "@ulixee/mainchain";
import {
    activateNotary,
    createLocalchain,
    disconnectOnTeardown,
    ipToInt32,
    teardown,
    transferToLocalchain
} from "./testHelpers";

if (process.env.RUST_LOG !== undefined) {
    // this stops jest from killing the logs
    global.console.log = (...args) => process.stdout.write(format(...args));
}
afterAll(teardown);

it('can create a zone record type', async () => {
    let mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const dataDomain = {
        domainName: 'example.com',
        topLevelDomain: DataTLD.Analytics
    };
    const ferdieDomainAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//dataDomain//1');
    const ferdie = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie');

    await expect(registerZoneRecord(mainchainClient, dataDomain, ferdie, ferdieDomainAddress.publicKey, {
        "1.0.0": mainchainClient.createType('UlxPrimitivesDataDomainVersionHost', {
            datastoreId: mainchainClient.createType('Bytes', 'default'),
            host: {
                ip: ipToInt32('192.168.1.1'),
                port: 80,
                isSecure: false
            }
        })
    })).rejects.toThrow("ExtrinsicFailed:: dataDomain.DomainNotRegistered");
}, 30e3);
it('can run a data domain channel', async () => {
    let mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    await notary.start(mainchainUrl);

    const sudo = new Keyring({type: 'sr25519'}).createFromUri('//Alice');
    const bob = new Keyring({type: 'sr25519'}).createFromUri('//Bob');
    const ferdie = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie');
    const ferdieDomainAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//dataDomain//1');
    const ferdieDomainProfitsAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//dataDomainProfits//1');
    const ferdieVotesAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//voter//1');
    const bobChannel = new Keyring({type: 'sr25519'}).createFromUri('//Bob//channels//1');
    const taxAddress = new Keyring({type: 'sr25519'}).createFromUri('//Bob//tax//1');

    const signer = new Signer(async (address, signatureMessage) => {
        for (const account of [bob, ferdie, bobChannel, taxAddress, ferdieDomainAddress, ferdieVotesAddress]) {
            if (account.address === address) {
                return account.sign(signatureMessage, {withType: true});
            }
        }
        throw new Error("Unsupported address requested");
    });

    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    await activateNotary(sudo, mainchainClient, notary);

    const bobchain = await createLocalchain(mainchainUrl);
    const ferdiechain = await createLocalchain(mainchainUrl);

    const dataDomain = {
        domainName: 'example.com',
        topLevelDomain: DataTLD.Analytics
    };
    {
        const [bobChange, ferdieChange] = await Promise.all([
            transferMainchainToLocalchain(mainchainClient, bobchain, bob, 5000, 1),
            transferMainchainToLocalchain(mainchainClient, ferdiechain, ferdie, 5000, 1),
        ]);
        await ferdieChange.notarization.leaseDataDomain(ferdie.address, ferdie.address, dataDomain, ferdie.address);
        // need to send enough to create a channel after tax
        await bobChange.notarization.moveToSubAddress(bob.address, bobChannel.address, AccountType.Deposit, 4200n, taxAddress.address);
        let [ferdieTracker] = await Promise.all([
            bobChange.notarization.notarizeAndWaitForNotebook(signer),
            ferdieChange.notarization.notarizeAndWaitForNotebook(signer),
        ]);

        const domains = await ferdiechain.dataDomains.list;
        expect(domains[0].name).toBe(dataDomain.domainName);
        expect(domains[0].tld).toBe(dataDomain.topLevelDomain);

        await ferdieTracker.waitForFinalized(ferdiechain.mainchainClient);
        await expect(ferdiechain.mainchainClient.getDataDomainRegistration(dataDomain.domainName, dataDomain.topLevelDomain)).resolves.toBeTruthy();
    }

    await registerZoneRecord(mainchainClient, dataDomain, ferdie, ferdieDomainAddress.publicKey, {
        "1.0.0": mainchainClient.createType('UlxPrimitivesDataDomainVersionHost', {
            datastoreId: mainchainClient.createType('Bytes', 'default'),
            host: {
                ip: ipToInt32('192.168.1.1'),
                port: 80,
                isSecure: false
            }
        })
    });

    const zoneRecord = await bobchain.mainchainClient.getDataDomainZoneRecord(dataDomain.domainName, dataDomain.topLevelDomain);
    expect(zoneRecord).toBeTruthy();
    expect(zoneRecord.paymentAddress).toBe(ferdieDomainAddress.address);
    const bobChannelHold = bobchain.beginChange();
    const account = await bobChannelHold.addAccount(bobChannel.address, AccountType.Deposit, 1);
    const change = await bobChannelHold.getBalanceChange(account);
    await change.createChannelHold(4000n, dataDomain, zoneRecord.paymentAddress);
    const holdTracker = await bobChannelHold.notarizeAndWaitForNotebook(signer);

    const clientChannel = await bobchain.openChannels.openClientChannel(account.id);
    await clientChannel.sign(0n, signer);
    const channelJson = await clientChannel.exportForSend();
    {
        const parsed = JSON.parse(channelJson.toString());
        console.log(parsed)
        expect(parsed).toBeTruthy();
        expect(parsed.channelHoldNote).toBeTruthy();
        expect(parsed.channelHoldNote.milligons).toBe(4000);
        expect(parsed.notes[0].milligons).toBe(0);
        expect(parsed.balance).toBe(parsed.previousBalanceProof.balance);
    }

    const ferdieChannelRecord= await ferdiechain.openChannels.importChannel(channelJson);

    // get to 2500 in channel costs so that 20% is 500 (minimum vote)
    for (let i= 0n; i <= 10n; i++) {
        const next = await clientChannel.sign(500n + i*200n, signer);
        // now we would send to ferdie
        await expect(ferdieChannelRecord.recordUpdatedSettlement(next.milligons, next.signature)).resolves.toBeUndefined();
    }

    // now ferdie goes to claim it
    const result = await ferdiechain.balanceSync.sync({
        channelTaxAddress: ferdieVotesAddress.address,
        channelClaimsSendToAddress: ferdieDomainProfitsAddress.address,
        votesAddress: ferdieVotesAddress.address,
    }, signer);
    expect(result).toBeTruthy();
    expect(result.balanceChanges).toHaveLength(2);
    expect(result.channelNotarizations).toHaveLength(0);

    const insideChannel = await ferdieChannelRecord.channel;
    const currentTick = ferdiechain.currentTick;
    expect(insideChannel.expirationTick).toBe(holdTracker.tick + ferdiechain.constants.channelConstants.expirationTicks);
    const timeForExpired = new Date(Number(ferdiechain.ticker.timeForTick(insideChannel.expirationTick)));
    console.log('Channel expires in %s seconds. Current Tick=%s, expiration=%s', (timeForExpired.getTime() - Date.now())/1000, currentTick, insideChannel.expirationTick);
    expect(timeForExpired.getTime() - Date.now()).toBeLessThan(30e3);
    await new Promise(resolve => setTimeout(resolve, timeForExpired.getTime() - Date.now() + 10));
    const vote_result = await ferdiechain.balanceSync.sync({
        channelTaxAddress: ferdieVotesAddress.address,
        channelClaimsSendToAddress: ferdieDomainProfitsAddress.address,
        votesAddress: ferdieVotesAddress.address,
    }, signer);
    console.log("Result of balance sync notarization of channel. Balance Changes=%s, Channels=%s", vote_result.balanceChanges.length, vote_result.channelNotarizations.length);
    expect(vote_result.channelNotarizations).toHaveLength(1);
    const notarization = vote_result.channelNotarizations[0];
    const notarizationChannels = await notarization.channels;
    expect(notarizationChannels).toHaveLength(1);
    expect(notarizationChannels[0].id).toBe(insideChannel.id);
    const json = JSON.parse(await notarization.toJson());
    expect(json).toBeTruthy();
    expect(json.blockVotes).toBeTruthy();
    expect(json.blockVotes).toHaveLength(1);

}, 60e3);

async function transferMainchainToLocalchain(mainchainClient: UlxClient, localchain: Localchain, account: KeyringPair, amount: number, notaryId: number): Promise<{
    notarization: NotarizationBuilder,
    balanceChange: BalanceChangeBuilder
}> {
    const nonce = await transferToLocalchain(account, amount, notaryId, mainchainClient);
    const transfer = await localchain.mainchainClient.waitForLocalchainTransfer(account.address, nonce);
    const notarization = localchain.beginChange();
    const balanceChange = await notarization.claimFromMainchain(transfer);
    return {notarization, balanceChange};
}

async function registerZoneRecord(client: UlxClient, dataDomain: DataDomain, owner: KeyringPair, paymentAccount:Uint8Array, versions: Record<string, UlxPrimitivesDataDomainVersionHost>) {

    const codecVersions = new Map();
    for (const [version, host] of Object.entries(versions)) {
        const [major, minor, patch] = version.split('.');
        const versionCodec = client.createType('UlxPrimitivesDataDomainSemver', {
            major,
            minor,
            patch,
        });
        codecVersions.set(versionCodec, client.createType('UlxPrimitivesDataDomainVersionHost', host));
    }
    await new Promise((resolve, reject) => client.tx.dataDomain.setZoneRecord({
        domainName: client.createType('Bytes', dataDomain.domainName),
        topLevelDomain: client.createType('UlxPrimitivesDataTLD', dataDomain.topLevelDomain),
    }, {
        paymentAccount,
        versions: codecVersions,
    }).signAndSend(owner, ({events, status}) => {
        if (status.isFinalized) {
            checkForExtrinsicSuccess(events, client).then(resolve).catch(reject);
        }
    }));
}