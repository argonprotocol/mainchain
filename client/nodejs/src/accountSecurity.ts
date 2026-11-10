import type { Call } from '@polkadot/types/interfaces';
import type { HexString } from '@polkadot/util/types';
import type { ArgonClient } from './index';

/** Full native wallet payload for a detached approval, not the outer submission transaction. */
export function createAccountSecuritySigningPayload(
  api: Pick<ArgonClient, 'registry' | 'runtimeVersion' | 'genesisHash'>,
  signerAddress: string,
  call: Call,
  metadataHash?: HexString,
) {
  if (
    call.section !== 'accountSecurity' ||
    !['register', 'updatePolicy', 'execute', 'cancelPolicyUpdate'].includes(call.method)
  ) {
    throw new Error('Expected an AccountSecurity call that accepts detached approvals');
  }

  const method = api.registry.findMetaCall(call.callIndex);
  const canonical = method(...call.args.slice(0, -1), []);
  return api.registry
    .createType('SignerPayload', {
      address: signerAddress,
      method: canonical,
      runtimeVersion: api.runtimeVersion,
      genesisHash: api.genesisHash,
      blockHash: api.genesisHash,
      blockNumber: 0,
      era: '0x00',
      nonce: 0,
      tip: 0,
      mode: metadataHash ? 1 : 0,
      ...(metadataHash ? { metadataHash } : {}),
      signedExtensions: api.registry.signedExtensions,
      version: 4,
    })
    .toPayload();
}
