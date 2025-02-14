import { Keyring, waitForLoad } from '../index';
import { readFileSync } from 'fs';
import * as readline from 'readline';
import * as util from 'node:util';
import { decodePair } from '@polkadot/keyring/pair/decode';
import * as utilCrypto from '@polkadot/util-crypto';
import { base64Decode } from '@polkadot/util-crypto';

const jsonFile = process.argv[2];
if (!jsonFile) {
  console.error('Usage: node decodePkcs.js <jsonFile>');
  process.exit(1);
}
const json = JSON.parse(readFileSync(jsonFile, 'utf8'));

(async () => {
  await waitForLoad();
  const keyring = new Keyring();
  const account = keyring.addFromJson(json);
  console.log(json);

  // prompt for password
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  rl.question('Enter password: ', async (password: string) => {
    account.decodePkcs8(password);
    let secretKey = new Uint8Array();
    const decoded = decodePair(
      password,
      base64Decode(json.encoded),
      json.encoding.type,
    );
    if (decoded.secretKey.length === 64) {
      secretKey = decoded.secretKey;
    } else {
      const type = json.encoding.content[1] as 'ecdsa' | 'ed25519' | 'sr25519';
      const pair = {
        ecdsa: utilCrypto.secp256k1PairFromSeed,
        ed25519: utilCrypto.ed25519PairFromSeed,
        sr25519: utilCrypto.sr25519PairFromSeed,
      }[type](decoded.secretKey);
      secretKey = pair.secretKey;
    }
    console.log('SecretKey: 0x%s', Buffer.from(secretKey).toString('hex'));

    console.log(util.inspect(account, true, null, true));
    rl.close();
  });
})();
