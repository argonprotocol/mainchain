import { createRequire } from 'node:module';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout } from 'node:timers/promises';
import { beforeAll, describe, expect, it } from 'vitest';
import { u8aToHex } from '@polkadot/util';
import { cryptoWaitReady, encodeAddress, signatureVerify } from '@polkadot/util-crypto';
import { merkleizeMetadata } from '@polkadot-api/merkleize-metadata';
import { PolkadotGenericApp } from '@zondax/ledger-substrate';
import { approvalFixture } from './fixtures';

// The upstream CommonJS transport avoids its package's unmarked ESM entry point.
const require = createRequire(import.meta.url);
const SpeculosTransport = require('@ledgerhq/hw-transport-node-speculos').default;
const apiUrl = process.env.ARGON_SPECULOS_URL;
if (!apiUrl) throw new Error('Set ARGON_SPECULOS_URL to the running Speculos REST endpoint');
const metadataPath = process.env.ARGON_SIGNER_METADATA_V15;
if (!metadataPath)
  throw new Error('Set ARGON_SIGNER_METADATA_V15 to runtime-exported V15 metadata');
const apduPort = Number(process.env.ARGON_SPECULOS_APDU_PORT ?? 9999);
const path = "m/44'/354'/0'/0'/0'";

describe('Ledger Generic Polkadot 100.0.30', () => {
  beforeAll(async () => {
    for (let attempt = 0; attempt < 100; attempt += 1) {
      try {
        const response = await fetch(`${apiUrl}/events?currentscreenonly=true`);
        if (response.ok) return;
      } catch {
        // The container can publish its ports before the REST server is listening.
      }
      await setTimeout(100);
    }
    throw new Error('Speculos did not become ready');
  });

  for (const kind of ['transfer', 'batch', 'register', 'updatePolicy'] as const) {
    it(`reviews and signs the exact ${kind} approval`, async () => {
      await cryptoWaitReady();
      // Ledger needs V15 proof metadata. Keep the client on V14 and export this separately
      // from the actual runtime; a synthesized metadata conversion is not authoritative.
      const merkle = merkleizeMetadata(readFileSync(metadataPath), {
        decimals: 6,
        tokenSymbol: 'ARGN',
      });
      const fixture = approvalFixture(kind, u8aToHex(merkle.digest()));
      const encoded = fixture.registry.createType('ExtrinsicPayloadV4', fixture.payload);
      const proof = merkle.getProofForExtrinsicPayload(fixture.bytes);
      const transport = await SpeculosTransport.open({ apduPort, host: '127.0.0.1' });
      try {
        const app = new PolkadotGenericApp(transport, 'dot');
        expect(await app.getVersion()).toMatchObject({ major: 100, minor: 0, patch: 30 });
        const { pubKey } = await app.getAddressEd25519(path, 42);
        let finished = false;
        const request = app.signWithMetadataEd25519(
          path,
          Buffer.from(fixture.bytes),
          Buffer.from(proof),
        );
        void request.then(
          () => {
            finished = true;
          },
          () => {
            finished = true;
          },
        );
        const screens: string[] = [];
        const dir = mkdtempSync(join(tmpdir(), 'argon-ledger-review-'));

        for (let page = 0; page < 240 && !finished; page += 1) {
          await setTimeout(80);
          const response = await fetch(`${apiUrl}/events?currentscreenonly=true`);
          const screen = await response.json();
          const text = screen.events.map((event: { text: string }) => event.text).join('\n');
          screens.push(text);
          const field = ['value', 'quorum', 'recovery_account'].find(field =>
            text.replace(/\s/g, '').startsWith(field),
          );
          if (field) {
            const screenshot = await fetch(`${apiUrl}/screenshot`);
            writeFileSync(join(dir, `${field}.png`), Buffer.from(await screenshot.arrayBuffer()));
          }
          const button = /Approve|Accept|Sign transaction/i.test(text) ? 'both' : 'right';
          await fetch(`${apiUrl}/button/${button}`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ action: 'press-and-release' }),
          });
        }

        const signature = (await request).signature;
        const review = screens.join('\n');
        writeFileSync(join(dir, 'screens.json'), JSON.stringify(screens, null, 2));
        writeFileSync(
          join(dir, 'proof.json'),
          JSON.stringify(
            {
              method: fixture.payload.method,
              signingBytes: u8aToHex(fixture.signingBytes),
              signature: u8aToHex(signature),
              publicKey: `0x${pubKey}`,
              metadataHash: u8aToHex(merkle.digest()),
            },
            null,
            2,
          ),
        );
        console.log(`Ledger ${kind} review: ${dir}`);
        expect(fixture.payload.nonce).toBe('0x00000000');
        expect(encoded.era.isImmortalEra).toBe(true);
        expect(encoded.tip.toBigInt()).toBe(0n);
        const publicKey = Buffer.from(pubKey, 'hex');
        expect(signatureVerify(fixture.signingBytes, signature, publicKey).isValid).toBe(true);
        const changed = fixture.signingBytes.slice();
        changed[0] ^= 1;
        expect(signatureVerify(changed, signature, publicKey).isValid).toBe(false);
        expect(review).toContain('AccountSecurity');
        const unwrapped = review.replace(/\s/g, '');
        if (kind === 'register' || kind === 'updatePolicy') {
          expect(unwrapped).toContain(kind === 'register' ? 'register' : 'update_policy');
          const compactScreens = screens.map(screen => screen.replace(/\s/g, ''));
          expect(compactScreens).toContain('quorum2');
          expect(compactScreens).toContain('recovery_delay1440');
          for (const [label, bytes] of [
            ['signers', [0x55, 0x66]],
            ['recovery_account', [0x88]],
          ] as const) {
            for (const byte of bytes) {
              expect(compactScreens).toContain(
                label + encodeAddress(new Uint8Array(32).fill(byte), 42),
              );
            }
          }
        } else {
          expect(review).toContain('execute');
          expect(unwrapped).toContain('transfer_keep_alive');
          expect(unwrapped).toContain(encodeAddress(new Uint8Array(32).fill(0x22), 42));
          expect(screens).toContain('value\nARGN 1.0');
          if (kind === 'batch') {
            expect(unwrapped).toContain('batch_all');
            expect(unwrapped).toContain(encodeAddress(new Uint8Array(32).fill(0x33), 42));
            expect(screens).toContain('value\nARGN 2.0');
          }
        }
      } finally {
        await transport.close();
      }
    });
  }
});
