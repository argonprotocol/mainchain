import { Metadata, TypeRegistry } from '@polkadot/types';
import type { Call } from '@polkadot/types/interfaces';
import { decorateExtrinsics } from '@polkadot/types/metadata/decorate';
import { compactAddLength, hexToU8a, u8aConcat, u8aToHex } from '@polkadot/util';
import { blake2AsU8a } from '@polkadot/util-crypto';
import { createAccountSecuritySigningPayload } from '../../client/nodejs/src/accountSecurity';
import runtimeMetadata from '../../client/nodejs/metadata.json';
import runtimeVersion from '../../client/nodejs/runtime_version.json';

export function approvalFixture(
  kind: 'transfer' | 'batch' | 'register' | 'updatePolicy',
  metadataHash?: `0x${string}`,
) {
  const registry = new TypeRegistry();
  const metadata = new Metadata(registry, runtimeMetadata.result as `0x${string}`);
  registry.setMetadata(metadata);
  registry.setChainProperties(
    registry.createType('ChainProperties', {
      ss58Format: 42,
      tokenDecimals: 6,
      tokenSymbol: 'ARGN',
    }),
  );

  const tx = decorateExtrinsics(registry, metadata.asLatest, metadata.version);
  const account = `0x${'11'.repeat(32)}`;
  const authorization = {
    group: account,
    nonce: 9,
    expiresAt: 100,
    policyHash: `0x${'44'.repeat(32)}`,
  };
  const policy = {
    signers: [`0x${'55'.repeat(32)}`, `0x${'66'.repeat(32)}`],
    quorum: 2,
    recoveryAccount: `0x${'88'.repeat(32)}`,
    recoveryDelay: 1_440,
  };

  let operation: Call;
  if (kind === 'register' || kind === 'updatePolicy') {
    operation = tx.accountSecurity[kind](
      account,
      policy.signers,
      policy.quorum,
      policy.recoveryAccount,
      policy.recoveryDelay,
      authorization,
      [],
    );
  } else {
    const transfer = tx.balances.transferKeepAlive(`0x${'22'.repeat(32)}`, 1_000_000);
    let call = transfer;
    if (kind === 'batch') {
      call = tx.utility.batchAll([
        transfer,
        tx.balances.transferKeepAlive(`0x${'33'.repeat(32)}`, 2_000_000),
      ]);
    }
    operation = tx.accountSecurity.execute(account, call, authorization, []);
  }
  const api = {
    registry,
    genesisHash: registry.createType('Hash', `0x${'45'.repeat(32)}`),
    runtimeVersion: registry.createType('RuntimeVersion', runtimeVersion),
  };
  const payload = createAccountSecuritySigningPayload(api, account, operation, metadataHash);
  const encoded = registry.createType('ExtrinsicPayload', payload, { version: 4 });
  const method = hexToU8a(payload.method);
  const bytes = encoded.toU8a({ method: true });

  return {
    registry,
    metadata,
    payload,
    bytes,
    signingBytes: bytes.length > 256 ? blake2AsU8a(bytes) : bytes,
    vault: {
      vault_payload: u8aToHex(
        u8aConcat(compactAddLength(method), bytes.subarray(method.length)),
      ).slice(2),
      genesis: api.genesisHash.toHex().slice(2),
      spec_version: api.runtimeVersion.specVersion.toNumber(),
    },
  };
}
