import { readFileSync, writeFileSync } from 'node:fs';

const metadata = readFileSync(new URL('../../../metadata.scale', import.meta.url));
writeFileSync(
  new URL('../../metadata.json', import.meta.url),
  JSON.stringify({ result: `0x${metadata.toString('hex')}` }),
);
