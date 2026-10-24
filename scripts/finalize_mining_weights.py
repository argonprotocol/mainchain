#!/usr/bin/env python3
"""Finalize conservative mining weights from raw benchmark measurements.

The benchmark writer takes the maximum bound across storage prefixes. Mining
can read two full cohorts simultaneously, so also charge the sum of the logged
maximum key counts times their MaxEncodedLen proof bounds. This deliberately
does not assume trie-node sharing or overlap between read and write keys.
Execution and database weights reserve the maximum observed over the supported
range. This avoids underfitting the allocation and replacement path transitions.
"""

import json
import pathlib
import re
import sys

# Match the explicitly configured CLI trie depth in benchmark.sh.
TRIE_PREFIX_OVERHEAD = 15 * 33 * 2


def finalize(source: str, measurements: list) -> str:
    batches = {batch["benchmark"]: batch for batch in measurements}
    if len(batches) != len(measurements):
        raise ValueError("Duplicate benchmark measurements")
    pattern = re.compile(
        r"(?P<docs>(?:\t///[^\n]*\n)+)"
        r"(?P<head>\tfn (?P<name>\w+)\([^\n]*\) -> Weight \{)"
        r"(?P<body>.*?)(?P<end>\n\t\t})",
        re.S,
    )

    def bound(match: re.Match) -> str:
        docs = match["docs"]
        entries = re.findall(
            r"Storage: [^\n]*\(r:(\d+) w:(\d+)\)\n"
            r"\t/// Proof: [^\n]*added: (\d+), mode: `MaxEncodedLen`",
            docs,
        )
        if len(entries) != docs.count("/// Storage:"):
            raise ValueError("Every mining storage access needs a bounded proof estimate")
        if not entries:
            raise ValueError("Missing bounded storage metadata")
        batch = batches.pop(match["name"])
        reads = max(item["reads"] for item in batch["db_results"])
        writes = max(item["writes"] for item in batch["db_results"])
        execution = max(item["extrinsic_time"] for item in batch["time_results"]) * 1_000
        proof = sum((int(r) + int(w)) * int(size) for r, w, size in entries)
        proof += sum(TRIE_PREFIX_OVERHEAD for r, w, _ in entries if int(r) + int(w) > 0)
        # Prefix comments can describe a smaller path than another sampled point.
        # Bound any remaining accesses at the largest known storage-key proof size.
        documented = sum(int(r) + int(w) for r, w, _ in entries)
        proof += max(0, reads + writes - documented) * (
            max(int(size) for _, _, size in entries) + TRIE_PREFIX_OVERHEAD
        )
        if proof < max(item["proof_size"] for item in batch["db_results"] + batch["time_results"]):
            raise ValueError("Bounded proof estimate is below an observed proof")
        head = re.sub(r"(?<!\w)([a-z]\w*): u32", r"_\1: u32", match["head"])
        return (
            docs + head
            + "\n\t\t// Maximum observed costs across the supported benchmark range."
            + "\n\t\t// Proof bounds include simultaneous prefixes and distinct read/write keys."
            + f"\n\t\tWeight::from_parts({execution:_}, {proof:_})"
            + f"\n\t\t\t.saturating_add(T::DbWeight::get().reads({reads}))"
            + f"\n\t\t\t.saturating_add(T::DbWeight::get().writes({writes}))"
            + match["end"]
        )

    result, count = pattern.subn(bound, source)
    methods = len(re.findall(r"^\tfn \w+\(", source, re.M))
    if count == 0 or count != methods or batches:
        raise ValueError("Every mining weight function must match its raw measurements")
    return result


if __name__ == "__main__":
    path = pathlib.Path(sys.argv[1])
    path.write_text(finalize(path.read_text(), json.loads(pathlib.Path(sys.argv[2]).read_text())))
