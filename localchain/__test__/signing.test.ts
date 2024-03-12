import {CryptoScheme, Localchain} from "../index";
import TestMainchain from "./TestMainchain";
import {closeOnTeardown, KeyringSigner, teardown} from "./testHelpers";
import {Keyring} from "@ulixee/mainchain";


afterAll(teardown);

it('can sign a message from javscript', async () => {
    let mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const bobchain = await Localchain.load({
        mainchainUrl: mainchainUrl,
        path: ':memory:',
    });
    closeOnTeardown(bobchain);
    const bob = new KeyringSigner("//Bob");
    await bobchain.keystore.useExternal(bob.address, bob.sign, bob.derive);
    const notarization = bobchain.beginChange();
    const balanceChange = await notarization.defaultDepositAccount();
    await balanceChange.claimFromMainchain({
        address: bob.address,
        amount: 5000n,
        expirationBlock: 1,
        notaryId: 1,
        accountNonce: 1
    });
    await expect(notarization.sign()).resolves.toBeUndefined();
    await expect(notarization.verify()).resolves.toBeUndefined();
});
it('can sign using built-in', async () => {
    let mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const bobchain = await Localchain.load({
        mainchainUrl: mainchainUrl,
        path: ':memory:',
    });
    closeOnTeardown(bobchain);

    await bobchain.keystore.importSuri("//Bob", CryptoScheme.Ed25519, {});
    await expect(bobchain.address).resolves.toBe(new Keyring().createFromUri("//Bob", {}, 'ed25519').address);

    const notarization = bobchain.beginChange();
    const balanceChange = await notarization.defaultDepositAccount();
    await balanceChange.claimFromMainchain({
        address: await bobchain.address,
        amount: 5000n,
        expirationBlock: 1,
        notaryId: 1,
        accountNonce: 1
    });
    await expect(notarization.sign()).resolves.toBeUndefined();
    await expect(notarization.verify()).resolves.toBeUndefined();
});