import {AccountType, Signer} from "../index";
import TestMainchain from "./TestMainchain";
import TestNotary from "./TestNotary";
import {getClient, Keyring} from "@ulixee/mainchain";
import {
    activateNotary,
    createLocalchain,
    disconnectOnTeardown,
    getMainchainBalance,
    teardown,
    transferToLocalchain
} from "./testHelpers";


afterAll(teardown);

it('can transfer from mainchain to local', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    await notary.start(mainchainUrl);

    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const alice = new Keyring({type: 'sr25519'}).createFromUri('//Alice');
    const bob = new Keyring({type: 'sr25519'}).createFromUri('//Bob');
    await activateNotary(alice, mainchainClient, notary);

    const nonce = await transferToLocalchain(bob, 5000, 1, mainchainClient);
    const bobchain = await createLocalchain(mainchainUrl);
    const bobMainchainClient = await bobchain.mainchainClient;
    const transfer = await bobMainchainClient.waitForLocalchainTransfer(bob.address, nonce);
    expect(transfer).toBeTruthy();
    expect(transfer?.amount).toBe(5000n);
    const notarization = bobchain.beginChange();
    const bobLocalAccount = await notarization.addAccount(bob.address, AccountType.Deposit, 1);
    const balanceChange = await notarization.getBalanceChange(bobLocalAccount);
    await balanceChange.claimFromMainchain(transfer);
    const signer = new Signer(async (address, signatureMessage) => {
        if (address == bob.address) {
            return bob.sign(signatureMessage, {withType: true});
        }
        throw new Error("Unsupported address requested");
    });
    const tracker = await notarization.notarizeAndWaitForNotebook(signer);
    const proof = await tracker.getNotebookProof();
    console.log("Got proof", proof);
    expect(proof).toHaveLength(1);
    expect(proof[0].proof).toHaveLength(0);
    expect(proof[0].address).toBe(bob.address);
    expect(proof[0].balance).toBe(5000n);
    expect(proof[0].changeNumber).toBe(1);
    expect(proof[0].numberOfLeaves).toBe(1);

}, 60e3);

it('can transfer from mainchain to local to mainchain', async () => {
    const mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const notary = new TestNotary();
    await notary.start(mainchainUrl);

    const mainchainClient = await getClient(mainchainUrl);
    disconnectOnTeardown(mainchainClient);
    const alice = new Keyring({type: 'sr25519'}).createFromUri('//Alice');
    const alice1 = new Keyring({type: 'sr25519'}).createFromUri('//Alice//1');
    const bob = new Keyring({type: 'sr25519'}).createFromUri('//Bob');
    const signer = new Signer(async (address, signatureMessage) => {
        if (address == bob.address) {
            return bob.sign(signatureMessage, {withType: true});
        }
        if (address == alice.address) {
            return alice.sign(signatureMessage, {withType: true});
        }
        if (address == alice1.address) {
            return alice1.sign(signatureMessage, {withType: true});
        }
        throw new Error("Unsupported address requested");
    });

    await activateNotary(alice, mainchainClient, notary);

    const nonce = await transferToLocalchain(bob, 5000, 1, mainchainClient);
    const bobchain = await createLocalchain(mainchainUrl);
    {
        const bobMainchainClient = await bobchain.mainchainClient;
        const transfer = await bobMainchainClient.waitForLocalchainTransfer(bob.address, nonce);
        const notarization = bobchain.beginChange();
        const bobLocalAccount = await notarization.addAccount(bob.address, AccountType.Deposit, 1);
        const balanceChange = await notarization.getBalanceChange(bobLocalAccount);
        await balanceChange.claimFromMainchain(transfer);

        const tracker = await notarization.notarizeAndWaitForNotebook(signer);
        await tracker.getNotebookProof();
    }

    let jsonClaims: string;
    {
        const notarization = bobchain.beginChange();
        const bobLocalAccount = await notarization.addAccount(bob.address, AccountType.Deposit, 1);
        const balanceChange = await notarization.getBalanceChange(bobLocalAccount);
        await balanceChange.send(5000n, [alice1.address]);
        await notarization.sign(signer);
        jsonClaims = await notarization.exportForSend();
    }

    let expectedAliceBalance = 0n;
    {
        const alicechain = await createLocalchain(mainchainUrl);
        const notarization = alicechain.beginChange();
        await notarization.claimReceivedBalance(jsonClaims, alice1.address, alice.address);
        const balanceChange = (await notarization.balanceChangeBuilders).find(x=>x.address == alice1.address && x.accountType == AccountType.Deposit);
        const balance = await balanceChange?.balance;
        expectedAliceBalance = balance;
        await balanceChange.sendToMainchain(balance);
        const tracker = await notarization.notarizeAndWaitForNotebook(signer);
        console.log('Notarized. Waiting for notebook %s at %s', tracker.notebookNumber, tracker.notaryId);
        const mainchainClient = await alicechain.mainchainClient;
        const finalized = await mainchainClient.waitForNotebookFinalized(tracker.notaryId, tracker.notebookNumber);
        console.log('Finalized notebook %s at %s', tracker.notebookNumber, finalized);
        const changesRoot = await mainchainClient.getAccountChangesRoot(tracker.notaryId, tracker.notebookNumber);
        console.log('Changes root', changesRoot);
        expect(changesRoot).toBeTruthy();
        try {
            const finalizedBlock = await tracker.waitForFinalized(mainchainClient);
            console.log('Finalized block %s', finalizedBlock);
        } catch (err) {
            console.error(err);
        }
        const proof = await tracker.getNotebookProof();
        console.log("Got proof", proof);
    }

    const aliceEndBalance = await getMainchainBalance(mainchainClient, alice1.address);
    expect(aliceEndBalance).toBeGreaterThanOrEqual(expectedAliceBalance);


}, 60e3);

