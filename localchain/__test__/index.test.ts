import {CryptoScheme, Localchain} from "../index";
import {closeOnTeardown, teardown} from "./testHelpers";


afterAll(teardown);

it('can create and reload a localchain', async () => {
    const ticker = {
        genesisUtcTime: Date.now(),
        tickDurationMillis: 1000,
        escrowExpirationTicks: 2,
    };
    const localchain = await Localchain.loadWithoutMainchain(':memory:', ticker);
    closeOnTeardown(localchain);
    const address1 = await localchain.keystore.importSuri('//Alice', CryptoScheme.Sr25519, {password: Buffer.from('1234')});

    const localchain2 = await Localchain.loadWithoutMainchain(':memory:', ticker);
    const address2 = await localchain2.keystore.importSuri('//Alice', CryptoScheme.Sr25519, {password: Buffer.from('1234')});
    expect(address1).toBe(address2);
});
