import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import packageJson from '../../package.json' with { type: 'json' };

const packageRoot = path.resolve(import.meta.dirname, '..', '..');
const packOutput = execSync('yarn pack --dry-run --json', {
  cwd: packageRoot,
  encoding: 'utf8',
});
const packedFiles = new Set(
  packOutput
    .trim()
    .split('\n')
    .map(line => (JSON.parse(line) as { location?: string }).location)
    .filter(location => location !== undefined)
    .map(location => location.replaceAll(path.win32.sep, path.posix.sep)),
);
const declarationFiles = [...packedFiles].filter(file => /\.d\.(?:c|m)?ts$/.test(file));
const workspaceDevDependencies = Object.entries(packageJson.devDependencies)
  .filter(([, version]) => version.startsWith('workspace:'))
  .map(([name]) => name);
const missingImports = new Set<string>();

for (const declarationFile of declarationFiles) {
  const declaration = readFileSync(path.join(packageRoot, declarationFile), 'utf8');
  const imports = ts.preProcessFile(declaration, true, true).importedFiles;

  for (const { fileName } of imports) {
    if (
      workspaceDevDependencies.some(name => fileName === name || fileName.startsWith(`${name}/`))
    ) {
      missingImports.add(`${declarationFile}: ${fileName}`);
      continue;
    }

    if (!fileName.startsWith('.')) continue;

    const resolvedImport = path.posix.normalize(
      path.posix.join(path.posix.dirname(declarationFile), fileName),
    );
    const candidates = [resolvedImport];

    if (resolvedImport.endsWith('.js')) {
      candidates.push(resolvedImport.replace(/\.js$/, '.d.ts'));
    } else if (resolvedImport.endsWith('.cjs')) {
      candidates.push(resolvedImport.replace(/\.cjs$/, '.d.cts'));
    } else if (resolvedImport.endsWith('.mjs')) {
      candidates.push(resolvedImport.replace(/\.mjs$/, '.d.mts'));
    } else if (!path.posix.extname(resolvedImport)) {
      candidates.push(`${resolvedImport}.d.ts`, `${resolvedImport}/index.d.ts`);
    }

    if (!candidates.some(candidate => packedFiles.has(candidate))) {
      missingImports.add(`${declarationFile}: ${fileName}`);
    }
  }
}

if (missingImports.size) {
  throw new Error(
    `Packed declarations contain unresolved imports:\n${[...missingImports].join('\n')}`,
  );
}
