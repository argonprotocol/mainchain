import { readdir, readFile, stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

type EchidnaCall = {
  call?: {
    contents?: [string, Array<{ contents?: boolean }>];
  };
};

const securityDir = dirname(fileURLToPath(import.meta.url));
const corpusDir = join(securityDir, 'corpus', 'stabilization-pool-hook');
const coverageDir = join(corpusDir, 'coverage');
const calls = new Set<string>();
const swapModes = new Set<string>();

for (const filename of await readdir(coverageDir)) {
  if (!filename.endsWith('.txt')) continue;

  const sequence = JSON.parse(await readFile(join(coverageDir, filename), 'utf8')) as EchidnaCall[];
  for (const transaction of sequence) {
    const contents = transaction.call?.contents;
    if (!contents) continue;

    const [name, args] = contents;
    calls.add(name);
    if (name === 'swap') swapModes.add(`${args[0]?.contents}:${args[1]?.contents}`);
  }
}

for (const requiredCall of ['swap', 'changeLiquidity', 'advanceTime']) {
  if (!calls.has(requiredCall)) throw new Error(`Echidna corpus did not retain ${requiredCall}`);
}
for (const requiredMode of ['true:true', 'true:false', 'false:true', 'false:false']) {
  if (!swapModes.has(requiredMode))
    throw new Error(`Echidna corpus did not retain swap mode ${requiredMode}`);
}

const reports = (await readdir(corpusDir)).filter(
  filename => filename.startsWith('covered.') && filename.endsWith('.txt'),
);
if (reports.length === 0) throw new Error('Echidna did not write text coverage');

const reportTimes = await Promise.all(
  reports.map(async filename => ({
    filename,
    modified: (await stat(join(corpusDir, filename))).mtimeMs,
  })),
);
reportTimes.sort((left, right) => right.modified - left.modified);
const report = await readFile(join(corpusDir, reportTimes[0].filename), 'utf8');
const sectionStart = report.indexOf('/contracts/test/StabilizationPoolHookInvariant.sol\n');
if (sectionStart === -1) throw new Error('Coverage omitted StabilizationPoolHookInvariant');

const followingSection = report.indexOf('\n/', sectionStart + 1);
const harnessCoverage = report.slice(
  sectionStart,
  followingSection === -1 ? undefined : followingSection,
);
for (const successfulAction of [
  '++successfulLiquidityAdds;',
  '++successfulLiquidityRemovals;',
  '++successfulTimeAdvances;',
  '++generatedSellExactInputs;',
  '++generatedSellExactOutputs;',
  '++generatedBuyExactInputs;',
  '++generatedBuyExactOutputs;',
]) {
  const escapedAction = successfulAction.replace(/[+]/g, '\\+');
  const coveredAction = new RegExp(`\\| \\* +\\|.*${escapedAction}`);
  if (!coveredAction.test(harnessCoverage)) {
    throw new Error(`Echidna did not cover ${successfulAction}`);
  }
}

console.log(
  `Verified retained stabilization coverage: ${reports.length} reports, ${swapModes.size} swap modes, ` +
    'successful generated swaps, liquidity add/remove and time advance',
);
