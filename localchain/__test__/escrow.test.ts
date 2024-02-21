import {
    AccountType,
    BalanceChangeBuilder, DataDomainStore,
    DataTLD, ESCROW_EXPIRATION_TICKS,
    Localchain,
    NotarizationBuilder,
    Signer,
} from "../index";
import {format} from 'node:util';
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
import * as Crypto from 'node:crypto';

if (process.env.RUST_LOG !== undefined) {
    // this stops jest from killing the logs
    global.console.log = (...args) => process.stdout.write(format(...args));
}
afterEach(teardown);
afterAll(teardown);

it('can create a zone record type', async () => {
    let mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const dataDomainHash = Crypto.createHash('sha256',).update('example.com').digest();
    const ferdieDomainAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//dataDomain//1');
    const ferdie = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie');

    await expect(registerZoneRecord(mainchainClient, dataDomainHash, ferdie, ferdieDomainAddress.publicKey, 1, {
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

it('can run a data domain escrow', async () => {
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
    const bobEscrow = new Keyring({type: 'sr25519'}).createFromUri('//Bob//escrows//1');
    const taxAddress = new Keyring({type: 'sr25519'}).createFromUri('//Bob//tax//1');

    const signer = new Signer(async (address, signatureMessage) => {
        for (const account of [bob, ferdie, bobEscrow, taxAddress, ferdieDomainAddress, ferdieVotesAddress]) {
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
        domainName: 'example',
        topLevelDomain: DataTLD.Analytics
    };
    const dataDomainHash = DataDomainStore.getHash("example.analytics");
    {
        const [bobChange, ferdieChange] = await Promise.all([
            transferMainchainToLocalchain(mainchainClient, bobchain, bob, 5000, 1),
            transferMainchainToLocalchain(mainchainClient, ferdiechain, ferdie, 5000, 1),
        ]);
        await ferdieChange.notarization.leaseDataDomain(ferdie.address, ferdie.address, "example.Analytics", ferdie.address);
        // need to send enough to create an escrow after tax
        await bobChange.notarization.moveToSubAddress(bob.address, bobEscrow.address, AccountType.Deposit, 4200n, taxAddress.address);
        let [ferdieTracker] = await Promise.all([
            bobChange.notarization.notarizeAndWaitForNotebook(signer),
            ferdieChange.notarization.notarizeAndWaitForNotebook(signer),
        ]);

        const domains = await ferdiechain.dataDomains.list;
        expect(domains[0].name).toBe(dataDomain.domainName);
        expect(domains[0].tld).toBe("analytics");

        const ferdieMainchainClient = await ferdiechain.mainchainClient;
        await ferdieTracker.waitForFinalized(ferdieMainchainClient);
        await expect(ferdieMainchainClient.getDataDomainRegistration(dataDomain.domainName, dataDomain.topLevelDomain)).resolves.toBeTruthy();
    }

    await registerZoneRecord(mainchainClient, dataDomainHash, ferdie, ferdieDomainAddress.publicKey, 1, {
        "1.0.0": mainchainClient.createType('UlxPrimitivesDataDomainVersionHost', {
            datastoreId: mainchainClient.createType('Bytes', 'default'),
            host: {
                ip: ipToInt32('192.168.1.1'),
                port: 80,
                isSecure: false
            }
        })
    });

    const mainchainClientBob = await bobchain.mainchainClient;
    const zoneRecord = await mainchainClientBob.getDataDomainZoneRecord(dataDomain.domainName, dataDomain.topLevelDomain);
    expect(zoneRecord).toBeTruthy();
    expect(zoneRecord.notaryId).toBe(1);
    expect(zoneRecord.paymentAddress).toBe(ferdieDomainAddress.address);
    const bobEscrowHold = bobchain.beginChange();
    const account = await bobEscrowHold.addAccount(bobEscrow.address, AccountType.Deposit, zoneRecord.notaryId);
    const change = await bobEscrowHold.getBalanceChange(account);
    await change.createEscrowHold(4000n, "example.Analytics", zoneRecord.paymentAddress);
    const holdTracker = await bobEscrowHold.notarizeAndWaitForNotebook(signer);

    const clientEscrow = await bobchain.openEscrows.openClientEscrow(account.id);
    await clientEscrow.sign(5n, signer);
    const escrowJson = await clientEscrow.exportForSend();
    {
        const parsed = JSON.parse(escrowJson.toString());
        console.log(parsed)
        expect(parsed).toBeTruthy();
        expect(parsed.escrowHoldNote).toBeTruthy();
        expect(parsed.escrowHoldNote.milligons).toBe(4000);
        expect(parsed.notes[0].milligons).toBe(5);
        expect(parsed.balance).toBe(parsed.previousBalanceProof.balance - 5);
    }

    const ferdieEscrowRecord = await ferdiechain.openEscrows.importEscrow(escrowJson);

    // get to 2500 in escrow costs so that 20% is 500 (minimum vote)
    for (let i = 0n; i <= 10n; i++) {
        const next = await clientEscrow.sign(500n + i * 200n, signer);
        // now we would send to ferdie
        await expect(ferdieEscrowRecord.recordUpdatedSettlement(next.milligons, next.signature)).resolves.toBeUndefined();
    }

    // now ferdie goes to claim it
    const result = await ferdiechain.balanceSync.sync({
        escrowTaxAddress: ferdieVotesAddress.address,
        escrowClaimsSendToAddress: ferdieDomainProfitsAddress.address,
        votesAddress: ferdieVotesAddress.address,
    }, signer);
    expect(result).toBeTruthy();
    expect(result.balanceChanges).toHaveLength(2);
    expect(result.escrowNotarizations).toHaveLength(0);

    const insideEscrow = await ferdieEscrowRecord.escrow;
    const currentTick = ferdiechain.currentTick;
    expect(insideEscrow.expirationTick).toBe(holdTracker.tick + ESCROW_EXPIRATION_TICKS);
    const timeForExpired = new Date(Number(ferdiechain.ticker.timeForTick(insideEscrow.expirationTick)));
    console.log('Escrow expires in %s seconds. Current Tick=%s, expiration=%s', (timeForExpired.getTime() - Date.now()) / 1000, currentTick, insideEscrow.expirationTick);
    expect(timeForExpired.getTime() - Date.now()).toBeLessThan(30e3);
    await new Promise(resolve => setTimeout(resolve, timeForExpired.getTime() - Date.now() + 10));
    const vote_result = await ferdiechain.balanceSync.sync({
        escrowTaxAddress: ferdieVotesAddress.address,
        escrowClaimsSendToAddress: ferdieDomainProfitsAddress.address,
        votesAddress: ferdieVotesAddress.address,
    }, signer);
    console.log("Result of balance sync notarization of escrow. Balance Changes=%s, Escrows=%s", vote_result.balanceChanges.length, vote_result.escrowNotarizations.length);
    expect(vote_result.escrowNotarizations).toHaveLength(1);
    const notarization = vote_result.escrowNotarizations[0];
    const notarizationEscrows = await notarization.escrows;
    expect(notarizationEscrows).toHaveLength(1);
    expect(notarizationEscrows[0].id).toBe(insideEscrow.id);
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
    const locMainchainClient = await localchain.mainchainClient;
    const transfer = await locMainchainClient.waitForLocalchainTransfer(account.address, nonce);
    const notarization = localchain.beginChange();
    const balanceChange = await notarization.claimFromMainchain(transfer);
    return {notarization, balanceChange};
}

async function registerZoneRecord(client: UlxClient, dataDomainHash: Uint8Array, owner: KeyringPair, paymentAccount: Uint8Array, notaryId: number, versions: Record<string, UlxPrimitivesDataDomainVersionHost>) {

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

    await new Promise((resolve, reject) => client.tx.dataDomain.setZoneRecord(dataDomainHash, {
        paymentAccount,
        notaryId,
        versions: codecVersions,
    }).signAndSend(owner, ({events, status}) => {
        if (status.isFinalized) {
            checkForExtrinsicSuccess(events, client).then(resolve).catch(reject);
        }
    }));
}