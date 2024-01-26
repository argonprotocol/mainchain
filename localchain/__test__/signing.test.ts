import {Keyring} from "@ulixee/mainchain";
import {AccountType, Localchain, Signer} from "../index";
import TestMainchain from "./TestMainchain";
import { teardown, closeOnTeardown} from "./testHelpers";


afterAll(teardown);

it('can sign a message from javscript', async () => {
    let mainchain = new TestMainchain();
    const mainchainUrl = await mainchain.launch();
    const bobchain = await Localchain.load({
        mainchainUrl: mainchainUrl,
        path: ':memory:',
    });
    closeOnTeardown(bobchain);
    const addressKeyring = new Keyring();
    const bob = addressKeyring.createFromUri('//Bob', {type: 'ed25519'});
    const notarization = bobchain.beginChange();
    const bobLocalAccount = await notarization.addAccount(bob.address, AccountType.Deposit, 1);
    const balanceChange = await notarization.getBalanceChange(bobLocalAccount);
    await balanceChange.claimFromMainchain({
        address: bob.address,
        amount: 5000n,
        expirationBlock: 1,
        notaryId: 1,
        accountNonce: 1
    });
    const signer = new Signer(async (address, signatureMessage) => {
        expect(address).toBe(bob.address);
        return bob.sign(signatureMessage, {withType: true});
    });
    await expect(notarization.sign(signer)).resolves.toBeUndefined();
    await expect(notarization.verify()).resolves.toBeUndefined();
});