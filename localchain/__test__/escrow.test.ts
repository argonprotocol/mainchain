import {
    BalanceChangeBuilder,
    DataDomainStore,
    DataTLD,
    Localchain,
    NotarizationBuilder,
} from "../index";
import {format} from 'node:util';
import TestMainchain from "./TestMainchain";
import TestNotary from "./TestNotary";
import {
    checkForExtrinsicSuccess,
    getClient,
    Keyring,
    KeyringPair,
    ArgonClient,
    ArgonPrimitivesDataDomainVersionHost
} from "@argonprotocol/mainchain";
import {
    activateNotary,
    createLocalchain, describeIntegration,
    disconnectOnTeardown,
    KeyringSigner,
    teardown,
    transferToLocalchain
} from "./testHelpers";
import * as Crypto from 'node:crypto';
import {beforeAll} from "jest-circus";
import retryTimes = jest.retryTimes;

if (process.env.RUST_LOG !== undefined) {
    // this stops jest from killing the logs
    global.console.log = (...args) => process.stdout.write(format(...args));
}
afterEach(teardown);
afterAll(teardown);
beforeAll(() => {
    retryTimes(3, {
        logErrorsBeforeRetry: true,
    });
})

describeIntegration("Escrow integration", () => {
    it('can create a zone record type', async () => {
        let mainchain = new TestMainchain();
        const mainchainUrl = await mainchain.launch();
        const mainchainClient = await getClient(mainchainUrl);
        disconnectOnTeardown(mainchainClient);
        const dataDomainHash = Crypto.createHash('sha256',).update('example.com').digest();
        const ferdieDomainAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//dataDomain//1');
        const ferdie = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie');

        await expect(registerZoneRecord(mainchainClient, dataDomainHash, ferdie, ferdieDomainAddress.publicKey, 1, {
            "1.0.0": mainchainClient.createType('ArgonPrimitivesDataDomainVersionHost', {
                datastoreId: mainchainClient.createType('Bytes', 'default'),
                host: 'ws://192.168.1.1:80'
            })
        })).rejects.toThrow("ExtrinsicFailed:: dataDomain.DomainNotRegistered");
    }, 120e3);


    it('can run a data domain escrow', async () => {
        let mainchain = new TestMainchain();
        const mainchainUrl = await mainchain.launch();
        const notary = new TestNotary();
        await notary.start(mainchainUrl);

        const ferdiekeys = await KeyringSigner.load("//Ferdie");

        const sudo = new Keyring({type: 'sr25519'}).createFromUri('//Alice');
        const bobkeys = new Keyring({type: 'sr25519'});

        const bob = bobkeys.addFromUri('//Bob');
        const ferdieVotesAddress = new Keyring({type: 'sr25519'}).createFromUri('//Ferdie//voter//1');

        const mainchainClient = await getClient(mainchainUrl);
        disconnectOnTeardown(mainchainClient);
        await activateNotary(sudo, mainchainClient, notary);

        const bobchain = await createLocalchain(mainchainUrl);

        await bobchain.keystore.useExternal(bob.address, async (address, signatureMessage) => {
            return bobkeys.getPair(address)?.sign(signatureMessage, {withType: true});
        }, async hd_path => {
            return bobkeys.addPair(bob.derive(hd_path)).address;
        });

        const ferdiechain = await createLocalchain(mainchainUrl);
        await ferdiechain.keystore.useExternal(ferdiekeys.address, ferdiekeys.sign, ferdiekeys.derive);

        const dataDomain = {
            domainName: 'example',
            topLevelDomain: DataTLD.Analytics
        };
        const dataDomainHash = DataDomainStore.getHash("example.analytics");
        {
            const [bobChange, ferdieChange] = await Promise.all([
                transferMainchainToLocalchain(mainchainClient, bobchain, bob, 5200, 1),
                transferMainchainToLocalchain(mainchainClient, ferdiechain, ferdiekeys.defaultPair, 5000, 1),
            ]);
            await ferdieChange.notarization.leaseDataDomain("example.Analytics", ferdiekeys.address);
            let [ferdieTracker] = await Promise.all([
                bobChange.notarization.notarizeAndWaitForNotebook(),
                ferdieChange.notarization.notarizeAndWaitForNotebook(),
            ]);

            const domains = await ferdiechain.dataDomains.list;
            expect(domains[0].name).toBe(dataDomain.domainName);
            expect(domains[0].tld).toBe("analytics");

            const ferdieMainchainClient = await ferdiechain.mainchainClient;
            await ferdieTracker.waitForImmortalized(ferdieMainchainClient);
            await expect(ferdieMainchainClient.getDataDomainRegistration(dataDomain.domainName, dataDomain.topLevelDomain)).resolves.toBeTruthy();
        }

        await registerZoneRecord(mainchainClient, dataDomainHash, ferdiekeys.defaultPair, ferdiekeys.defaultPair.publicKey, 1, {
            "1.0.0": mainchainClient.createType('ArgonPrimitivesDataDomainVersionHost', {
                datastoreId: mainchainClient.createType('Bytes', 'default'),
                host: 'ws://192.168.1.1:80'
            })
        });

        const mainchainClientBob = await bobchain.mainchainClient;
        const zoneRecord = await mainchainClientBob.getDataDomainZoneRecord(dataDomain.domainName, dataDomain.topLevelDomain);
        expect(zoneRecord).toBeTruthy();
        expect(zoneRecord.notaryId).toBe(1);
        expect(zoneRecord.paymentAddress).toBe(ferdiekeys.address);
        const escrowFunding = bobchain.beginChange();
        const jumpAccount = await escrowFunding.fundJumpAccount(5200n);
        await escrowFunding.notarize();

        const bobEscrowHold = bobchain.beginChange();
        const change = await bobEscrowHold.addAccountById(jumpAccount.localAccountId);
        await change.createEscrowHold(5000n, "example.Analytics", zoneRecord.paymentAddress);
        const holdTracker = await bobEscrowHold.notarizeAndWaitForNotebook();

        const clientEscrow = await bobchain.openEscrows.openClientEscrow(jumpAccount.localAccountId);
        await clientEscrow.sign(5n);
        const escrowJson = await clientEscrow.exportForSend();
        {
            const parsed = JSON.parse(escrowJson.toString());
            console.log(parsed)
            expect(parsed).toBeTruthy();
            expect(parsed.escrowHoldNote).toBeTruthy();
            expect(parsed.escrowHoldNote.milligons).toBe(5000);
            expect(parsed.notes[0].milligons).toBe(5);
            expect(parsed.balance).toBe(parsed.previousBalanceProof.balance - 5);
        }

        const ferdieEscrowRecord = await ferdiechain.openEscrows.importEscrow(escrowJson);

        // get to 2500 in escrow costs so that 20% is 500 (minimum vote)
        for (let i = 0n; i <= 10n; i++) {
            const next = await clientEscrow.sign(500n + i * 200n,);
            // now we would send to ferdie
            await expect(ferdieEscrowRecord.recordUpdatedSettlement(next.milligons, next.signature)).resolves.toBeUndefined();
        }

        // now ferdie goes to claim it
        const result = await ferdiechain.balanceSync.sync({
            votesAddress: ferdieVotesAddress.address,
        },);
        expect(result).toBeTruthy();
        expect(result.balanceChanges).toHaveLength(2);
        expect(result.escrowNotarizations).toHaveLength(0);

        const insideEscrow = await ferdieEscrowRecord.escrow;
        const currentTick = ferdiechain.currentTick;
        const ticker = ferdiechain.ticker;
        expect(insideEscrow.expirationTick).toBe(holdTracker.tick + ticker.escrowExpirationTicks);
        const timeForExpired = new Date(Number(ferdiechain.ticker.timeForTick(insideEscrow.expirationTick)));
        console.log('Escrow expires in %s seconds. Current Tick=%s, expiration=%s', (timeForExpired.getTime() - Date.now()) / 1000, currentTick, insideEscrow.expirationTick);
        expect(timeForExpired.getTime() - Date.now()).toBeLessThan(30e3);
        await new Promise(resolve => setTimeout(resolve, timeForExpired.getTime() - Date.now() + 10));

        const ferdieMainchainClient = await ferdiechain.mainchainClient;
        // in the balance sync, we'd normally just keep trying to vote with the latest expiring escrows, but in this test, we only have 1, so we need to wait for a grandparent hash
        for (let i = 0; i < 10; i += 1) {
            try {
                const voteBlocks = await ferdieMainchainClient.getVoteBlockHash(ferdiechain.currentTick);
                if (voteBlocks.blockHash) {
                    break;
                }
            } catch {
            }
            await new Promise(resolve => setTimeout(resolve, Number(ferdiechain.ticker.millisToNextTick())));
        }

        const voteResult = await ferdiechain.balanceSync.sync({
            votesAddress: ferdieVotesAddress.address,
        },);
        console.log("Result of balance sync notarization of escrow. Balance Changes=%s, Escrows=%s", voteResult.balanceChanges.length, voteResult.escrowNotarizations.length);
        expect(voteResult.escrowNotarizations).toHaveLength(1);
        const notarization = voteResult.escrowNotarizations[0];
        const notarizationEscrows = await notarization.escrows;
        expect(notarizationEscrows).toHaveLength(1);
        expect(notarizationEscrows[0].id).toBe(insideEscrow.id);
        const json = JSON.parse(await notarization.toJSON());
        expect(json).toBeTruthy();

        expect(json.blockVotes).toHaveLength(1);

    }, 120e3);
});

async function transferMainchainToLocalchain(mainchainClient: ArgonClient, localchain: Localchain, account: KeyringPair, amount: number, notaryId: number): Promise<{
    notarization: NotarizationBuilder,
    balanceChange: BalanceChangeBuilder
}> {
    const transferId = await transferToLocalchain(account, amount, notaryId, mainchainClient);
    const locMainchainClient = await localchain.mainchainClient;
    const transfer = await locMainchainClient.waitForLocalchainTransfer(transferId);
    const notarization = localchain.beginChange();
    const balanceChange = await notarization.claimFromMainchain(transfer);
    return {notarization, balanceChange};
}

async function registerZoneRecord(client: ArgonClient, dataDomainHash: Uint8Array, owner: KeyringPair, paymentAccount: Uint8Array, notaryId: number, versions: Record<string, ArgonPrimitivesDataDomainVersionHost>) {

    const codecVersions = new Map();
    for (const [version, host] of Object.entries(versions)) {
        const [major, minor, patch] = version.split('.');
        const versionCodec = client.createType('ArgonPrimitivesDataDomainSemver', {
            major,
            minor,
            patch,
        });
        codecVersions.set(versionCodec, client.createType('ArgonPrimitivesDataDomainVersionHost', host));
    }

    await new Promise((resolve, reject) => client.tx.dataDomain.setZoneRecord(dataDomainHash, {
        paymentAccount,
        notaryId,
        versions: codecVersions,
    }).signAndSend(owner, ({events, status}) => {
        if (status.isFinalized) {
            checkForExtrinsicSuccess(events, client).then(resolve).catch(reject);
        }
        if (status.isInBlock) {
            checkForExtrinsicSuccess(events, client).catch(reject);
        }
    }));
}
