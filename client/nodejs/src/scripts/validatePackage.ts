import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import ts from 'typescript';
import type { Abi } from 'viem';
import type { EvmContractArtifact } from '../EvmContracts.js';
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

const stabilizationContracts = [
  ['StabilizationPoolHook', 'stabilizationPoolHook'],
  ['StabilizationPoolSubscriber', 'stabilizationPoolSubscriber'],
  ['UniswapV4PriceQuoter', 'uniswapV4PriceQuoter'],
] as const;

for (const entry of ['lib/index.js', 'lib/index.cjs', 'browser/index.js']) {
  const { EvmContracts } = await import(pathToFileURL(path.join(packageRoot, entry)).href);
  const outputDirectory = path.posix.dirname(entry);

  for (const [contractName, exportName] of stabilizationContracts) {
    const abi = EvmContracts[`${exportName}Abi`] as Abi | undefined;
    const artifact = EvmContracts[`${exportName}Artifact`] as EvmContractArtifact | undefined;
    if (!Array.isArray(abi) || !abi.length) {
      throw new Error(`${entry} does not export EvmContracts.${exportName}Abi`);
    }
    if (!artifact?.bytecode || artifact.bytecode === '0x') {
      throw new Error(`${entry} does not export deployable EvmContracts.${exportName}Artifact`);
    }

    const artifactPath = `${outputDirectory}/artifacts/contracts/${contractName}.sol/${contractName}.json`;
    if (!packedFiles.has(artifactPath)) {
      throw new Error(`Package does not include ${artifactPath}`);
    }

    const packedArtifact = JSON.parse(
      readFileSync(path.join(packageRoot, artifactPath), 'utf8'),
    ) as EvmContractArtifact;
    if (artifact.bytecode !== packedArtifact.bytecode) {
      throw new Error(`${entry} exports different bytecode from ${artifactPath}`);
    }

    const exportedFunctions = abi.filter(item => item.type === 'function');
    const artifactFunctions = packedArtifact.abi.filter(item => item.type === 'function');
    if (!isDeepStrictEqual(exportedFunctions, artifactFunctions)) {
      throw new Error(`${entry} exports a different function ABI from ${artifactPath}`);
    }
  }
}
