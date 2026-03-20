#!/usr/bin/env bash

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

declare -a staged_files=()
while IFS= read -r -d '' file; do
    staged_files+=("$file")
done < <(git diff --cached --name-only -z --diff-filter=ACMR)

declare -a rust_files=()
if (($# > 0)); then
    for file in "$@"; do
        if [[ "$file" == *.rs ]]; then
            rust_files+=("$file")
        fi
    done
else
    while IFS= read -r -d '' file; do
        rust_files+=("$file")
    done < <(git diff --cached --name-only -z --diff-filter=ACMR -- '*.rs')
fi

if ((${#rust_files[@]} == 0)); then
    exit 0
fi

echo "Running cargo make format"
cargo make format

declare -a restaged_files=()

for file in "${staged_files[@]}"; do
    if [[ ! -e "$file" ]]; then
        continue
    fi

    if git diff --quiet -- "$file"; then
        continue
    fi

    git add -- "$file"
    restaged_files+=("$file")
done

if ((${#restaged_files[@]} > 0)); then
    printf 'Restaged hook updates for:\n'
    printf '  %s\n' "${restaged_files[@]}"
fi
