import {CryptoScheme, Localchain} from "../index";
import {closeOnTeardown, KeyringSigner, teardown} from "./testHelpers";
import {Keyring} from "@argonprotocol/mainchain";


afterAll(teardown);

it('can sign a message from javscript', async () => {
    const bobchain = await Localchain.loadWithoutMainchain(':memory:', {
        genesisUtcTime: Date.now(),
        tickDurationMillis: 1000,
        escrowExpirationTicks: 2,
    });
    closeOnTeardown(bobchain);
    const bob = await KeyringSigner.load("//Bob");
    await bobchain.keystore.useExternal(bob.address, bob.sign, bob.derive);
    const notarization = bobchain.beginChange();
    const balanceChange = await notarization.defaultDepositAccount();
    await balanceChange.claimFromMainchain({
        address: bob.address,
        amount: 5000n,
        expirationTick: 1,
        notaryId: 1,
        transferId: 1
    });
    await expect(notarization.sign()).resolves.toBeUndefined();
    await expect(notarization.verify()).resolves.toBeUndefined();
});
it('can sign using built-in', async () => {
    const bobchain = await Localchain.loadWithoutMainchain(':memory:', {
        genesisUtcTime: Date.now(),
        tickDurationMillis: 1000,
        escrowExpirationTicks: 2,
    });
    closeOnTeardown(bobchain);

    await bobchain.keystore.importSuri("//Bob", CryptoScheme.Ed25519, {});
    await expect(bobchain.address).resolves.toBe(new Keyring().createFromUri("//Bob", {}, 'ed25519').address);

    const notarization = bobchain.beginChange();
    const balanceChange = await notarization.defaultDepositAccount();
    await balanceChange.claimFromMainchain({
        address: await bobchain.address,
        amount: 5000n,
        expirationTick: 1,
        notaryId: 1,
        transferId: 1
    });
    await expect(notarization.sign()).resolves.toBeUndefined();
    await expect(notarization.verify()).resolves.toBeUndefined();
});
