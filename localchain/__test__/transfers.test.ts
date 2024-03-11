import {AccountType, CryptoScheme} from "../index";
import TestMainchain from "./TestMainchain";
import TestNotary from "./TestNotary";
import {getClient, Keyring} from "@ulixee/mainchain";
import {
    activateNotary,
    createLocalchain,
    disconnectOnTeardown,
    getMainchainBalance,
    KeyringSigner,
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
    const bob = new KeyringSigner("//Bob");
    await activateNotary(alice, mainchainClient, notary);

    const nonce = await transferToLocalchain(bob.defaultPair, 5000, 1, mainchainClient);
    const bobchain = await createLocalchain(mainchainUrl);
    await bobchain.signer.importSuriToEmbedded("//Bob", CryptoScheme.Sr25519);
    const bobMainchainClient = await bobchain.mainchainClient;
    const transfer = await bobMainchainClient.waitForLocalchainTransfer(bob.address, nonce);
    expect(transfer).toBeTruthy();
    expect(transfer?.amount).toBe(5000n);
    const notarization = bobchain.beginChange();
    const balanceChange = await notarization.addAccount(bob.address, AccountType.Deposit, 1);
    await balanceChange.claimFromMainchain(transfer);

    const tracker = await notarization.notarizeAndWaitForNotebook();
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
    const bob = new Keyring({type: 'sr25519'}).createFromUri('//Bob');

    await activateNotary(alice, mainchainClient, notary);

    const nonce = await transferToLocalchain(bob, 5000, 1, mainchainClient);
    const bobchain = await createLocalchain(mainchainUrl);

    const ferdie = new Keyring().createFromUri("//Ferdie//1", {}, 'sr25519');

    // TODO: convert this to export pkcs8 and then import it
    await bobchain.signer.importSuriToEmbedded("//Bob", CryptoScheme.Sr25519);
    {
        const bobMainchainClient = await bobchain.mainchainClient;
        const transfer = await bobMainchainClient.waitForLocalchainTransfer(bob.address, nonce);
        const notarization = bobchain.beginChange();
        await notarization.claimFromMainchain(transfer);

        const tracker = await notarization.notarizeAndWaitForNotebook();
        await tracker.getNotebookProof();
    }

    const bobSendArgonFile = await bobchain.transactions.send(5000n, [ferdie.address]);

    let expectedAliceBalance = 0n;
    {
        const ferdiechain = await createLocalchain(mainchainUrl);
        await ferdiechain.signer.importSuriToEmbedded("//Ferdie//1", CryptoScheme.Sr25519);
        const notarization = ferdiechain.beginChange();
        await notarization.importArgonFile(bobSendArgonFile);
        const balanceChange = (await notarization.balanceChangeBuilders).find(x => x.address == ferdie.address && x.accountType == AccountType.Deposit);
        const balance = await balanceChange?.balance;
        expectedAliceBalance = balance;
        await balanceChange.sendToMainchain(balance);
        const tracker = await notarization.notarizeAndWaitForNotebook();
        console.log('Notarized. Waiting for notebook %s at %s', tracker.notebookNumber, tracker.notaryId);
        const mainchainClient = await ferdiechain.mainchainClient;
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

    const ferdieMainchainBalance = await getMainchainBalance(mainchainClient, ferdie.address);
    expect(ferdieMainchainBalance).toBeGreaterThanOrEqual(expectedAliceBalance);


}, 60e3);

