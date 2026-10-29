import * as fs from 'node:fs';


const dirname = import.meta.dirname;

function replaceInFile(path, replacements) {
    if (!fs.existsSync(path)) return;

    let content = fs.readFileSync(path, 'utf8');

    for (const [from, to] of replacements) {
        content = content.replaceAll(from, to);
    }

    fs.writeFileSync(path, content, 'utf8');
}

// Read the generated file
try {
    fs.rmSync(`${dirname}/src/interfaces/index.ts`);
} catch {
}
try {
    fs.rmSync(`${dirname}/src/interfaces/types.ts`);
} catch {
}

let path = `${dirname}/src/interfaces/types-lookup.ts`;
replaceInFile(path, [
    ["readonly get:", 'readonly get_:'],
    ["readonly values:", 'readonly values_:'],
]);

const interfacesDir = `${dirname}/src/interfaces`;
for (const file of fs.readdirSync(interfacesDir)) {
    if (!file.endsWith('.ts')) continue;

    replaceInFile(`${interfacesDir}/${file}`, [
        ['/* eslint-disable */\n', ''],
        ['/* eslint-disable sort-keys */\n', ''],
    ]);
}

const typeOnlyAugmentImports = [
    ['src/interfaces/augment-api-consts.ts', '@polkadot/api-base/types/consts'],
    ['src/interfaces/augment-api-errors.ts', '@polkadot/api-base/types/errors'],
    ['src/interfaces/augment-api-events.ts', '@polkadot/api-base/types/events'],
    ['src/interfaces/augment-api-query.ts', '@polkadot/api-base/types/storage'],
    ['src/interfaces/augment-api-tx.ts', '@polkadot/api-base/types/submittable'],
    ['src/interfaces/augment-api-rpc.ts', '@polkadot/rpc-core/types/jsonrpc'],
    ['src/interfaces/augment-api-runtime.ts', '@polkadot/api-base/types/calls'],
    ['src/interfaces/augment-types.ts', '@polkadot/types/types/registry'],
];

for (const [file, specifier] of typeOnlyAugmentImports) {
    replaceInFile(`${dirname}/${file}`, [
        [`import '${specifier}';`, `import type {} from '${specifier}';`],
    ]);
}
