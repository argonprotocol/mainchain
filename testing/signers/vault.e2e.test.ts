import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { it } from 'vitest';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import { approvalFixture } from './fixtures';

// The first run also compiles Vault's pinned SDK. Rust owns the review-card assertions.
it(
  'passes current client payloads through the Polkadot Vault decoder tests',
  { timeout: 600_000 },
  async () => {
    await cryptoWaitReady();
    const transfer = approvalFixture('transfer');
    const dir = mkdtempSync(join(tmpdir(), 'argon-vault-review-'));
    const fixturesPath = join(dir, 'fixtures.json');
    writeFileSync(
      fixturesPath,
      JSON.stringify({
        metadata: Buffer.from(transfer.metadata.toU8a()).toString('hex'),
        payloads: {
          transfer: transfer.vault,
          batch: approvalFixture('batch').vault,
          register: approvalFixture('register').vault,
          updatePolicy: approvalFixture('updatePolicy').vault,
          wrongGenesis: { ...transfer.vault, genesis: '46'.repeat(32) },
        },
      }),
    );

    const cache = process.env.ARGON_SIGNER_CACHE ?? dir;
    const output = execFileSync(
      'cargo',
      [
        'test',
        '--locked',
        '--manifest-path',
        fileURLToPath(new URL('./vault/Cargo.toml', import.meta.url)),
        '--test',
        'tests',
      ],
      {
        encoding: 'utf8',
        env: {
          ...process.env,
          ARGON_VAULT_FIXTURES: fixturesPath,
          CARGO_BUILD_BUILD_DIR: join(cache, 'vault-build'),
          CARGO_TARGET_DIR: join(cache, 'vault-target'),
          CARGO_INCREMENTAL: '0',
        },
      },
    );
    process.stdout.write(output);
  },
);
