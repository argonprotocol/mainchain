import { describe, expect, it } from 'vitest';
import { Metadata, TypeRegistry } from '@polkadot/types';
import { decorateExtrinsics } from '@polkadot/types/metadata/decorate';
import { blake2AsHex, blake2AsU8a } from '@polkadot/util-crypto';
import { createAccountSecuritySigningPayload } from '../accountSecurity';
import runtimeMetadata from '../../metadata.json';
import runtimeVersion from '../../runtime_version.json';

describe('native account-security signatures', () => {
  it('uses V14 metadata to encode P256 and preserves existing signature tags', () => {
    const registry = new TypeRegistry();
    const metadata = new Metadata(registry, runtimeMetadata.result as `0x${string}`);
    expect(metadata.version).toBe(14);
    registry.setMetadata(metadata);

    const deviceSignature = new Uint8Array(65).fill(7);
    deviceSignature[64] = 1;
    const signature = registry.createType('ExtrinsicSignature', { P256: deviceSignature });
    expect(signature.toU8a()).toEqual(new Uint8Array([4, ...deviceSignature]));
    expect(registry.createType('ExtrinsicSignature', signature.toU8a()).toHex()).toBe(
      signature.toHex(),
    );

    for (const [tag, variant, length] of [
      [0, 'Ed25519', 64],
      [1, 'Sr25519', 64],
      [2, 'Ecdsa', 65],
      [3, 'Eth', 65],
    ] as const) {
      const bytes = new Uint8Array(length).fill(5);
      expect(registry.createType('ExtrinsicSignature', { [variant]: bytes }).toU8a()).toEqual(
        new Uint8Array([tag, ...bytes]),
      );
    }
  });

  it('keeps the complete nested call in a device approval for decoding and display', () => {
    const registry = new TypeRegistry();
    const metadata = new Metadata(registry, runtimeMetadata.result as `0x${string}`);
    registry.setMetadata(metadata);
    const tx = decorateExtrinsics(registry, metadata.asLatest, metadata.version);
    const account = `0x${'11'.repeat(32)}`;
    const transfer = tx.balances.transferKeepAlive(`0x${'22'.repeat(32)}`, 123);
    const authorization = {
      group: account,
      nonce: 9,
      expiresAt: 100,
      policyHash: `0x${'33'.repeat(32)}`,
    };
    const operation = tx.accountSecurity.execute(account, transfer, authorization, [
      {
        signer: `0x${'44'.repeat(32)}`,
        signature: { Sr25519: `0x${'55'.repeat(64)}` },
      },
    ]);
    const api = {
      registry,
      // FRAME's GenesisConfig test hash is 69 (0x45) in each byte.
      genesisHash: registry.createType('Hash', `0x${'45'.repeat(32)}`),
      runtimeVersion: registry.createType('RuntimeVersion', runtimeVersion),
    };
    const payload = createAccountSecuritySigningPayload(api, `0x${'44'.repeat(32)}`, operation);
    const decoded = registry.createType('Call', payload.method);

    expect(decoded.section).toBe('accountSecurity');
    expect(decoded.method).toBe('execute');
    expect(decoded.args[1].toU8a()).toEqual(transfer.toU8a());
    expect(decoded.args[3].toJSON()).toEqual([]);
    expect(operation.args[3].toJSON()).toHaveLength(1);

    const encoded = registry
      .createType('ExtrinsicPayload', payload, { version: 4 })
      .toU8a({ method: true });
    const signingBytes = encoded.length > 256 ? blake2AsU8a(encoded) : encoded;
    // Same vector as both real-runtime native signing tests, not a separately invented format.
    expect(blake2AsHex(signingBytes)).toBe(
      '0xbf10963a6b5779a7ab2c969651abe1bd65a7122d11243fd9a47c9bad149bc934',
    );
  });

  it('encodes registration and policy updates with one signer set and quorum', () => {
    const registry = new TypeRegistry();
    const metadata = new Metadata(registry, runtimeMetadata.result as `0x${string}`);
    registry.setMetadata(metadata);
    const tx = decorateExtrinsics(registry, metadata.asLatest, metadata.version);
    const account = `0x${'11'.repeat(32)}`;

    // SCALE vectors for the single-quorum policy, including its replay authorization.
    for (const [kind, expected] of [
      ['register', '0x40924fe06d3f5167e12fdef8f3d8b949351d17f83925a5c0a5772665b4cde01b'],
      ['updatePolicy', '0x6fb3afb4d498fc9e9fc19153c7c646306e2269572558ab5a3f2a854bc3674ad1'],
    ] as const) {
      const operation = tx.accountSecurity[kind](
        account,
        [`0x${'55'.repeat(32)}`, `0x${'66'.repeat(32)}`],
        2,
        `0x${'88'.repeat(32)}`,
        1_440,
        { group: account, nonce: 9, expiresAt: 100, policyHash: `0x${'44'.repeat(32)}` },
        [],
      );
      expect(blake2AsHex(operation.toU8a())).toBe(expected);
    }
  });

  it('binds cancellation to one pending update and clears proofs only from the signed copy', () => {
    const registry = new TypeRegistry();
    const metadata = new Metadata(registry, runtimeMetadata.result as `0x${string}`);
    registry.setMetadata(metadata);
    const tx = decorateExtrinsics(registry, metadata.asLatest, metadata.version);
    const account = `0x${'11'.repeat(32)}`;
    const operation = tx.accountSecurity.cancelPolicyUpdate(
      account,
      7,
      { group: account, nonce: 9, expiresAt: 100, policyHash: `0x${'44'.repeat(32)}` },
      [
        {
          signer: `0x${'55'.repeat(32)}`,
          signature: { Sr25519: `0x${'66'.repeat(64)}` },
          withMetadataHash: false,
        },
      ],
    );
    const api = {
      registry,
      genesisHash: registry.createType('Hash', `0x${'45'.repeat(32)}`),
      runtimeVersion: registry.createType('RuntimeVersion', runtimeVersion),
    };
    const payload = createAccountSecuritySigningPayload(api, account, operation);
    const decoded = registry.createType('Call', payload.method);
    expect(decoded.method).toBe('cancelPolicyUpdate');
    expect(decoded.args[1].toString()).toBe('7');
    expect(decoded.args[2].toU8a()).toEqual(operation.args[2].toU8a());
    expect(decoded.args[3].toJSON()).toEqual([]);
    expect(operation.args[3].toJSON()).toHaveLength(1);
    expect(() =>
      createAccountSecuritySigningPayload(
        api,
        account,
        tx.accountSecurity.completePolicyUpdate(account, 7),
      ),
    ).toThrow('accepts detached approvals');
  });
});
