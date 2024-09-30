import {getClient} from "../lib/types";
import {Keyring} from "@polkadot/api";

async function main() {
    let api = await getClient("ws://127.0.0.1:9944");
    console.log("got api");
    let domainTld = api.createType("ArgonPrimitivesDomainTopLevel", "Analytics");

    let result = await api.query.domain.registeredDomains({domainName: "test", topLevelDomain: domainTld});
    console.log("got a registered domain", result.isSome);

    console.log('current tick', (await api.query.ticks.currentTick()).toPrimitive());

    let account = api.createType("AccountId32", "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty");
    let aliceKeyring = new Keyring({type: 'sr25519'});
    let alice = aliceKeyring.addFromUri('//Alice');
    await new Promise<void>(resolve => api.tx.miningSlot.bid(null, {Account: account}).signAndSend(alice, ({
                                                                                                               events = [],
                                                                                                               status
                                                                                                           }) => {

        if (status.isInBlock) {
            console.log('Successful bid in block ' + status.asInBlock.toHex(), status.type);
            resolve();
        } else {
            console.log('Status of transfer: ' + status.type);
        }
    }));
    await api.disconnect();
}

main().catch(console.error).finally(() => process.exit());
