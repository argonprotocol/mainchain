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
  // Concentrated wallets maximize calculation work; independent wallets maximize
  // storage fan-out. Reserve their measured components together for both clearing entrypoints.
  const clearingDocs = [...source.matchAll(pattern)]
    .filter(
      match =>
        match.groups.name.startsWith('live_auction') || match.groups.name.startsWith('settle_bids'),
    )
    .map(match => match.groups.docs)
    .join('');
  const clearingBatches = measurements.filter(
    batch =>
      batch.benchmark.startsWith('live_auction') || batch.benchmark.startsWith('settle_bids'),
  );
  const admissionNames = new Set([
    'bid',
    'bid_renewal',
    'bid_with_max',
    'bid_with_max_renewal',
    'bid_with_max_adjustment',
  ]);
  const admissionDocs = [...source.matchAll(pattern)]
    .filter(match => admissionNames.has(match.groups.name))
    .map(match => match.groups.docs)
    .join('');
  const admissionBatches = measurements.filter(batch => admissionNames.has(batch.benchmark));
  let count = 0;
  const result = source.replace(pattern, (...args) => {
    const { docs, head, name, end } = args.at(-1);
    const jointClearingBound = name === 'live_auction' || name === 'settle_bids';
    const jointAdmissionBound = name === 'bid' || name === 'bid_with_max';
    let proofDocs = docs;
    if (jointClearingBound) proofDocs = clearingDocs;
    else if (jointAdmissionBound) proofDocs = admissionDocs;
    const accesses = Array.from(
      proofDocs.matchAll(
        /Storage: ([^\n]*?) \(r:(\d+) w:(\d+)\)\n\t\/\/\/ Proof: [^\n]*added: (\d+), mode: `MaxEncodedLen`/g,
      ),
    );
    if (accesses.length !== (proofDocs.match(/\/\/\/ Storage:/g) ?? []).length) {
      throw new Error('Every mining storage access needs a bounded proof estimate');
    }
    if (accesses.length === 0) throw new Error('Missing bounded storage metadata');
    const prefixes = new Map();
    for (const [, prefix, r, w, size] of accesses) {
      const previous = prefixes.get(prefix) ?? [0n, 0n, 0n];
      prefixes.set(
        prefix,
        [r, w, size].map((value, index) =>
          BigInt(value) > previous[index] ? BigInt(value) : previous[index],
        ),
      );
    }
    const entries = [...prefixes.values()];
    const batch = batches.get(name);
    if (!batch) throw new Error(`Missing raw measurements for ${name}`);
    batches.delete(name);
    let selected = [batch];
    if (jointClearingBound) selected = clearingBatches;
    else if (jointAdmissionBound) selected = admissionBatches;
    const dbResults = selected.flatMap(result => result.db_results);
    const timeResults = selected.flatMap(result => result.time_results);
    const reads = maximum(dbResults, 'reads');
    const writes = maximum(dbResults, 'writes');
    const execution = maximum(timeResults, 'extrinsic_time') * 1_000n;
    let proof = entries.reduce((sum, [r, w, size]) => sum + (r + w) * size, 0n);
    proof += entries.reduce((sum, [r, w]) => sum + (r + w > 0n ? TRIE_PREFIX_OVERHEAD : 0n), 0n);
    // Prefix comments can describe a smaller path than another sampled point.
    // Bound remaining accesses at the largest known storage-key proof size.
    const documented = entries.reduce((sum, [r, w]) => sum + r + w, 0n);
    const largest = entries.reduce((max, [, , size]) => (size > max ? size : max), 0n);
    const remaining = reads + writes > documented ? reads + writes - documented : 0n;
    proof += remaining * (largest + TRIE_PREFIX_OVERHEAD);
    if (proof < maximum([...dbResults, ...timeResults], 'proof_size')) {
      throw new Error('Bounded proof estimate is below an observed proof');
    }
    count += 1;
    return (
      docs +
      head.replace(/(?<!\w)([a-z]\w*): u32/g, '_$1: u32') +
      (jointClearingBound || jointAdmissionBound
        ? '\n\t\t// Independent maximum execution, storage, and proof bounds across the applicable auction variants.'
        : '\n\t\t// Maximum observed costs across the supported benchmark range.') +
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
