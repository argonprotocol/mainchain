import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// Use Slither's own interpreter so pipx and CI resolve the same package as the scanner.
const executable = execFileSync('which', ['slither'], { encoding: 'utf8' }).trim();
const launcher = readFileSync(realpathSync(executable), 'utf8');
const interpreterCommand = launcher
  .match(/^#!([^\r\n]+)/)?.[1]
  .trim()
  .split(/\s+/);
if (!interpreterCommand?.length) throw new Error('Could not identify the Slither interpreter');

const [interpreter, ...interpreterArgs] = interpreterCommand;
const packageInfo = execFileSync(
  interpreter,
  [...interpreterArgs, '-m', 'pip', 'show', 'slither-analyzer'],
  { encoding: 'utf8' },
);
const analyzerVersion = packageInfo.match(/^Version: (.+)$/m)?.[1].trim();
if (analyzerVersion !== '0.11.6') {
  throw new Error('Recheck the Slither declaration-ID fix before changing analyzer versions');
}

const packageLocation = packageInfo.match(/^Location: (.+)$/m)?.[1].trim();
if (!packageLocation) throw new Error('Could not locate the installed Slither package');

const sourcePath = join(packageLocation, 'slither/solc_parsing/solidity_types/type_parsing.py');
const source = readFileSync(sourcePath);
const checksum = createHash('sha256').update(source).digest('hex');
const originalChecksum = '4fa51359aa2066a8e4328b2f58489cc6f6efb2f5d15f4fd6c93838d34e0f5047';
const patchedChecksum = 'bd7a258b2db96b5c17752bff51d8941c09eeb97ebb4d3124d1481ee1ef05bd57';

if (checksum === patchedChecksum) {
  console.log('Slither declaration-ID fix is already applied');
} else if (checksum === originalChecksum) {
  // Solidity identifies declarations even when imported libraries share a name.
  // Keep Slither's fallback for older ASTs and non-contract declarations.
  const anchor = '    if t[key] == "ElementaryTypeName":\n';
  const correction = `    if is_compact_ast and t[key] in ("UserDefinedTypeName", "IdentifierPath"):
        referenced_contract = caller_context.slither_parser.contracts_by_id.get(
            t.get("referencedDeclaration")
        )
        if referenced_contract is not None:
            type_found = UserDefinedType(referenced_contract)
            _add_type_references(type_found, t["src"], sl)
            return type_found

`;
  const patched = source.toString('utf8').replace(anchor, correction + anchor);
  if (createHash('sha256').update(patched).digest('hex') !== patchedChecksum) {
    throw new Error('Slither declaration-ID fix did not produce the expected source');
  }
  writeFileSync(sourcePath, patched);
  console.log('Applied Slither declaration-ID fix');
} else {
  throw new Error('Slither source differs from the pinned release; refusing to modify it');
}
