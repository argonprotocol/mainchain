import * as fs from 'node:fs';


const dirname = import.meta.dirname;
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
let content = fs.readFileSync(path, 'utf8');

// Replace field names
content = content.replaceAll("readonly get:", 'readonly get_:').replaceAll("readonly values:", 'readonly values_:');

// Write the modified file back
fs.writeFileSync(path, content, 'utf8');
