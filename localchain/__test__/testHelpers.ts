import {
  ArgonClient,
  checkForExtrinsicSuccess,
  KeypairType,
  Keyring,
  KeyringPair,
  waitForLoad,
} from '@argonprotocol/mainchain';
import { Localchain } from '../index';
import { closeOnTeardown } from '@argonprotocol/testing';

export {
  activateNotary,
  describeIntegration,
  disconnectOnTeardown,
  TestNotary,
  closeOnTeardown,
  TestMainchain,
  teardown,
} from '@argonprotocol/testing';

export class KeyringSigner {
  readonly keyring: Keyring;
  readonly defaultPair: KeyringPair;

  get address(): string {
    return this.defaultPair.address;
  }

  private constructor(mainSuri: string, type: KeypairType = 'sr25519') {
    this.keyring = new Keyring();
    this.defaultPair = this.keyring.addFromUri(mainSuri, {}, type);
    this.sign = this.sign.bind(this);
    this.derive = this.derive.bind(this);
  }

  async sign(address: string, message: Uint8Array): Promise<Uint8Array> {
    return this.keyring.getPair(address)?.sign(message, { withType: true });
  }

  async derive(hdPath: string): Promise<string> {
    const pair = this.defaultPair.derive(hdPath);
    return this.keyring.addPair(pair).address;
  }

  static async load(mainSuri: string, type: KeypairType = 'sr25519'): Promise<KeyringSigner> {
    await waitForLoad();
    return new KeyringSigner(mainSuri, type);
  }
}

export async function createLocalchain(mainchainUrl: string): Promise<Localchain> {
  const localchain = await Localchain.load({
    mainchainUrl,
    path: ':memory:',
  });
  closeOnTeardown(localchain);
  return localchain;
}

export async function getMainchainBalance(client: ArgonClient, address: string): Promise<bigint> {
  const { data } = await client.query.system.account(address);
  return data.free.toBigInt();
}

export async function transferToLocalchain(
  account: KeyringPair,
  amount: number,
  viaNotaryId: number,
  client: ArgonClient,
): Promise<number> {
  return new Promise<number>((resolve, reject) => {
    client.tx.chainTransfer
      .sendToLocalchain(amount, viaNotaryId)
      .signAndSend(account, ({ events, status }) => {
        if (status.isFinalized) {
          checkForExtrinsicSuccess(events, client)
            .then(() => {
              for (const { event } of events) {
                if (client.events.chainTransfer.TransferToLocalchain.is(event)) {
                  const transferId = event.data.transferId.toPrimitive() as number;
                  resolve(transferId);
                }
              }
            })
            .catch(reject);
        }
        if (status.isInBlock) {
          checkForExtrinsicSuccess(events, client).catch(reject);
        }
      });
  });
}
