import { ArgonClient, Keyring, KeyringPair, TxSubmitter } from '@argonprotocol/mainchain';
import TestNotary from './TestNotary';
import TestMainchain from './TestMainchain';
import TestOracle from './TestOracle';
import TestEthereum from './TestEthereum';
import { startNetwork } from './TestNetwork';
export {
  addTeardown,
  cleanHostForDocker,
  closeOnTeardown,
  disconnectOnTeardown,
  getDockerPortMapping,
  getProxy,
  type ITeardownable,
  projectRoot,
  runOnTeardown,
  runTestScript,
  SKIP_E2E,
  teardown,
} from './support';

export {
  mineLaterExecutionAnchorReceipt,
  signGatewayPermit,
  syncEthereumVerifierUntilAnchorCovers,
  toArgonKeccakSignature,
  toEvmRecoverableSignature,
  waitForExecutionReceipt,
  waitForFinalizedBeaconExecutionAtOrAbove,
} from './EthereumE2eUtils';
export {
  getReadyEthereumGatewayUpdates,
  type EthereumGatewayUpdateBatch,
} from './EthereumGatewayQueue';
export {
  EthereumProofE2eHarness,
  TestMintingAuthorityActor,
  TestMintingGateway,
} from './TestEthereumProofActors';
export { TestNotary, TestMainchain, TestOracle, TestEthereum, startNetwork };

export function stringifyExt(obj: any): any {
  return JSON.stringify(
    obj,
    (_key, value) => {
      if (typeof value === 'bigint') {
        return value.toString() + 'n'; // Append 'n' to indicate bigint
      }
      if (Buffer.isBuffer(value) || value instanceof Uint8Array) {
        return `0x${Buffer.from(value).toString('hex')}`; // Convert Buffer to hex string
      }

      return value;
    },
    2,
  );
}

export function sudo(): KeyringPair {
  return new Keyring({ type: 'sr25519' }).createFromUri('//Alice');
}

export async function activateNotary(sudo: KeyringPair, client: ArgonClient, notary: TestNotary) {
  await notary.register(client);
  const txResult = await new TxSubmitter(
    client,
    client.tx.sudo.sudo(client.tx.notaries.activate(notary.operator!.publicKey)),
    sudo,
  ).submit();
  await txResult.waitForInFirstBlock;
}
