#!/usr/bin/env node
/**
 * Finalize conservative mining weights from raw benchmark measurements.
 *
 * The benchmark writer takes the maximum bound across storage prefixes. Mining
 * can read two full cohorts simultaneously, so also charge the sum of the logged
 * maximum key counts times their MaxEncodedLen proof bounds. This deliberately
 * does not assume trie-node sharing or overlap between read and write keys.
 * Execution and database weights reserve the maximum observed over the supported
 * range to avoid underfitting allocation and replacement path transitions.
 * Run through benchmark.sh using the repository's supported Node.js version.
 */
import { readFileSync, writeFileSync } from 'node:fs';

// Match the explicitly configured CLI trie depth in benchmark.sh.
const TRIE_PREFIX_OVERHEAD = 15n * 33n * 2n;

function maximum(samples, field) {
  if (!Array.isArray(samples) || samples.length === 0) {
    throw new Error(`Missing benchmark samples for ${field}`);
  }
  return samples.reduce((max, sample) => {
    const value = sample[field];
    // JSON numbers must be exact before conversion; all subsequent arithmetic uses BigInt.
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new Error(`Invalid or unsafe benchmark integer for ${field}`);
    }
    return BigInt(value) > max ? BigInt(value) : max;
  }, 0n);
}

function rustInteger(value) {
  return value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, '_');
}

function finalize(source, measurements) {
  const batches = new Map(measurements.map(batch => [batch.benchmark, batch]));
  if (batches.size !== measurements.length) throw new Error('Duplicate benchmark measurements');
  const pattern =
    /(?<docs>(?:\t\/\/\/[^\n]*\n)+)(?<head>\tfn (?<name>\w+)\([^\n]*\) -> Weight \{).*?(?<end>\n\t\t})/gs;
  let count = 0;
  const result = source.replace(pattern, (...args) => {
    const { docs, head, name, end } = args.at(-1);
    const entries = Array.from(
      docs.matchAll(
        /Storage: [^\n]*\(r:(\d+) w:(\d+)\)\n\t\/\/\/ Proof: [^\n]*added: (\d+), mode: `MaxEncodedLen`/g,
      ),
      match => match.slice(1).map(BigInt),
    );
    if (entries.length !== (docs.match(/\/\/\/ Storage:/g) ?? []).length) {
      throw new Error('Every mining storage access needs a bounded proof estimate');
    }
    if (entries.length === 0) throw new Error('Missing bounded storage metadata');
    const batch = batches.get(name);
    if (!batch) throw new Error(`Missing raw measurements for ${name}`);
    batches.delete(name);
    const reads = maximum(batch.db_results, 'reads');
    const writes = maximum(batch.db_results, 'writes');
    const execution = maximum(batch.time_results, 'extrinsic_time') * 1_000n;
    let proof = entries.reduce((sum, [r, w, size]) => sum + (r + w) * size, 0n);
    proof += entries.reduce((sum, [r, w]) => sum + (r + w > 0n ? TRIE_PREFIX_OVERHEAD : 0n), 0n);
    // Prefix comments can describe a smaller path than another sampled point.
    // Bound remaining accesses at the largest known storage-key proof size.
    const documented = entries.reduce((sum, [r, w]) => sum + r + w, 0n);
    const largest = entries.reduce((max, [, , size]) => (size > max ? size : max), 0n);
    const remaining = reads + writes > documented ? reads + writes - documented : 0n;
    proof += remaining * (largest + TRIE_PREFIX_OVERHEAD);
    if (proof < maximum([...batch.db_results, ...batch.time_results], 'proof_size')) {
      throw new Error('Bounded proof estimate is below an observed proof');
    }
    count += 1;
    return (
      docs +
      head.replace(/(?<!\w)([a-z]\w*): u32/g, '_$1: u32') +
      '\n\t\t// Maximum observed costs across the supported benchmark range.' +
      '\n\t\t// Proof bounds include simultaneous prefixes and distinct read/write keys.' +
      `\n\t\tWeight::from_parts(${rustInteger(execution)}, ${rustInteger(proof)})` +
      `\n\t\t\t.saturating_add(T::DbWeight::get().reads(${reads}))` +
      `\n\t\t\t.saturating_add(T::DbWeight::get().writes(${writes}))` +
      end
    );
  });
  const methods = (source.match(/^\tfn \w+\(/gm) ?? []).length;
  if (count === 0 || count !== methods || batches.size !== 0) {
    throw new Error('Every mining weight function must match its raw measurements');
  }
  return result;
}

if (process.argv.length !== 4) {
  throw new Error(
    'Usage: node scripts/finalize_mining_weights.mjs <weights.rs> <measurements.json>',
  );
}
const [, , path, measurementsPath] = process.argv;
writeFileSync(
  path,
  finalize(readFileSync(path, 'utf8'), JSON.parse(readFileSync(measurementsPath, 'utf8'))),
);
