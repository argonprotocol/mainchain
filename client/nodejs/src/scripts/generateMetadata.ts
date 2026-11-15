import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// The CommonJS exports resolve templates with native filesystem paths on Windows.
import * as generators from '@polkadot/typegen/cjs/generate/index';
import { generateTsDef } from '@polkadot/typegen/cjs/generate/tsDef';
import { generateInterfaceTypes } from '@polkadot/typegen/cjs/generate/interfaceRegistry';
import { HEADER, writeFile } from '@polkadot/typegen/cjs/util/index';
import substrateDefinitions from '@polkadot/types/cjs/interfaces/definitions';
import type { HexString } from '@polkadot/util/types';

const metadata = readFileSync(new URL('../../../metadata.scale', import.meta.url));
const metadataHex: HexString = `0x${metadata.toString('hex')}`;
writeFileSync(
  new URL('../../metadata.json', import.meta.url),
  JSON.stringify({ result: metadataHex }),
);

const outputDirectory = fileURLToPath(new URL('../interfaces/', import.meta.url));
const packageName = '@argonprotocol/mainchain/interfaces';
const definitions = await import(new URL('../interfaces/definitions.ts', import.meta.url).href);
const extraTypes = { [packageName]: definitions };
const allDefinitions = {
  '@polkadot/types/interfaces': Object.fromEntries(
    Object.entries(substrateDefinitions).filter(([name]) => !(name in definitions)),
  ),
  ...extraTypes,
};

generateTsDef(allDefinitions, outputDirectory, packageName);
generateInterfaceTypes(allDefinitions, join(outputDirectory, 'augment-types.ts'));
generators.generateDefaultLookup(outputDirectory, metadataHex);

const { default: lookupTypes } = await import(
  new URL('../interfaces/lookup.ts', import.meta.url).href
);
const lookupDefinitions = { rpc: {}, types: lookupTypes };

generators.generateDefaultConsts(
  join(outputDirectory, 'augment-api-consts.ts'),
  metadataHex,
  extraTypes,
  true,
  lookupDefinitions,
);
generators.generateDefaultErrors(
  join(outputDirectory, 'augment-api-errors.ts'),
  metadataHex,
  extraTypes,
  true,
);
generators.generateDefaultEvents(
  join(outputDirectory, 'augment-api-events.ts'),
  metadataHex,
  extraTypes,
  true,
  lookupDefinitions,
);
generators.generateDefaultQuery(
  join(outputDirectory, 'augment-api-query.ts'),
  metadataHex,
  extraTypes,
  true,
  lookupDefinitions,
);
generators.generateDefaultRpc(join(outputDirectory, 'augment-api-rpc.ts'), extraTypes);
generators.generateDefaultRuntime(
  join(outputDirectory, 'augment-api-runtime.ts'),
  metadataHex,
  extraTypes,
  true,
  lookupDefinitions,
);
generators.generateDefaultTx(
  join(outputDirectory, 'augment-api-tx.ts'),
  metadataHex,
  extraTypes,
  true,
  lookupDefinitions,
);

writeFile(join(outputDirectory, 'augment-api.ts'), () =>
  [
    HEADER('chain'),
    ...['consts', 'errors', 'events', 'query', 'tx', 'rpc', 'runtime'].map(
      name => `import './augment-api-${name}.js';\n`,
    ),
  ].join(''),
);
