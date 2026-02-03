#!/bin/bash
set -e

# Script to generate weight files for all pallets in both Argon and Canary runtimes
#
# Features:
#   - Dynamically discovers pallets from runtime (no hardcoded list to maintain)
#   - Uses directory output for automatic instance-aware file generation
#   - Handles multiple pallet instances (e.g., pallet_balances → separate files)
#   - Consistent naming and file organization
#
# Usage: ./benchmark.sh [runtime] [repeat] [pallet]
# Arguments:
#   runtime: argon|canary|all (default: all)
#   repeat: number of benchmark repetitions (default: 20)
#   pallet: specific pallet name (e.g., pallet_block_rewards) or "all" (default: all)

RUNTIME_TYPE=${1:-all}
REPEAT_COUNT=${2:-20}
SPECIFIC_PALLET=${3:-all}

# Function to get dynamic list of pallets for a runtime
get_pallets_for_runtime() {
    local runtime_path=$1
    echo "Fetching dynamic pallet list for runtime: $runtime_path"

    # Get all pallets with benchmarks - include frame pallets as they may need custom weights
    RUST_LOG=warn frame-omni-bencher v1 benchmark pallet \
        --runtime="$runtime_path" \
        --genesis-builder-policy=none \
        --list=pallets
}

dedupe_component_ranges() {
    local file=$1
    if [ ! -f "$file" ]; then
        return
    fi

    # Drop consecutive duplicate component range docs emitted by some benchmarks.
    local tmp_file
    tmp_file=$(mktemp)
    awk '{
        if ($0 ~ /^\\s*\\/\\/\\/ The range of component/ && $0 == prev) { next }
        print
        prev = $0
    }' "$file" > "$tmp_file"
    mv "$tmp_file" "$file"
}

function benchmark_runtime() {
    local runtime_name=$1
    if [ "$runtime_name" = "canary" ]; then
        local runtime_path="./target/release/wbuild/argon-canary-runtime/argon_canary_runtime.compact.wasm"
    else
        local runtime_path="./target/release/wbuild/${runtime_name}-runtime/${runtime_name}_runtime.compact.wasm"
    fi
    local output_dir="runtime/${runtime_name}/src/weights"

    echo "=== Benchmarking ${runtime_name} runtime ==="
    echo "Runtime: $runtime_path"
    echo "Output: $output_dir"
    echo ""

    if [ ! -f "$runtime_path" ]; then
        echo "❌ Runtime WASM not found: $runtime_path"
        echo "Run: cargo build --release --features=runtime-benchmarks --bin argon-node"
        return 1
    fi

    # Get dynamic pallet list for this runtime or use specific pallet
    if [ "$SPECIFIC_PALLET" = "all" ]; then
        echo "Getting pallet list for ${runtime_name} runtime..."
        PALLETS=$(get_pallets_for_runtime "$runtime_path" | tail -n +2 | sort | grep -v '^pallet$' | grep -v '^frame_benchmarking$' | grep -v '^$')

        if [ -z "$PALLETS" ]; then
            echo "❌ Failed to get pallet list for $runtime_name runtime"
            return 1
        fi

        echo "Found pallets: $(echo "$PALLETS" | wc -l | tr -d ' ') pallets"
        echo "$PALLETS" | sed 's/^/  - /'
        echo ""
    else
        echo "Benchmarking specific pallet: $SPECIFIC_PALLET"
        PALLETS="$SPECIFIC_PALLET"
        echo ""
    fi

    # Benchmark each pallet using consistent directory output approach
    echo "$PALLETS" | while read -r pallet; do
        if [ -z "$pallet" ] || [ "$pallet" = "pallet" ] || [ "$pallet" = "frame_benchmarking" ]; then continue; fi

        echo "Benchmarking $pallet for ${runtime_name}..."

        # Run benchmark, capturing full output
        benchmark_log=$(mktemp)
        set +e
        frame-omni-bencher v1 benchmark pallet \
            --runtime="$runtime_path" \
            --pallet="$pallet" \
            --extrinsic="*" \
            --genesis-builder-policy=none \
            --template="scripts/weight_template.hbs" \
            --repeat="$REPEAT_COUNT" \
            --output="$output_dir/" 2>&1 | tee "$benchmark_log" \
            | grep -E "Starting benchmark:|Created file:|Completed|Error|error:"
        bench_ec=${PIPESTATUS[0]}
        set -e

        if [ "$bench_ec" -ne 0 ]; then
            echo "✗ Benchmark process failed for $pallet (exit $bench_ec)"
            cat "$benchmark_log"
            echo "---"
            rm -f "$benchmark_log"
            continue
        fi

        CREATED=$(grep 'Created file:' "$benchmark_log" || true)
        if [ -n "$CREATED" ]; then
            echo "✓ Generated weight file(s)"
            # if ismp_grandpa, we need to modify the pallet name in the generated file from pallet_ismp_grandpa::WeightInfo to ismp_grandpa::WeightInfo
            if [[ "$pallet" == "pallet_ismp_grandpa" ]]; then
                sed -i '' 's/pallet_ismp_grandpa::WeightInfo/ismp_grandpa::WeightInfo/g' "$output_dir/$pallet.rs"
            fi
            while IFS= read -r line; do
                file=${line#*Created file: }
                dedupe_component_ranges "$file"
            done <<< "$CREATED"
        else
            echo "✗ Errors encountered"
            cat "$benchmark_log"
        fi
        echo "---"
        rm -f "$benchmark_log"
    done
}

# Build runtime with benchmarking features
echo "Building runtime with benchmarking features..."
echo "Using repeat count: $REPEAT_COUNT"
if [ "$SPECIFIC_PALLET" != "all" ]; then
    echo "Targeting specific pallet: $SPECIFIC_PALLET"
fi
echo ""

RUNTIMES=""
if [ "$RUNTIME_TYPE" = "argon" ] || [ "$RUNTIME_TYPE" = "all" ]; then
    RUNTIMES="-p argon-runtime"
fi

if [ "$RUNTIME_TYPE" = "canary" ] || [ "$RUNTIME_TYPE" = "all" ]; then
    RUNTIMES="${RUNTIMES} -p argon-canary-runtime"
fi

echo "Building: ${RUNTIMES}"
cargo build --release --features=runtime-benchmarks ${RUNTIMES}

if [ "$RUNTIME_TYPE" = "argon" ] || [ "$RUNTIME_TYPE" = "all" ]; then
    benchmark_runtime "argon"
fi

if [ "$RUNTIME_TYPE" = "canary" ] || [ "$RUNTIME_TYPE" = "all" ]; then
    benchmark_runtime "canary"
fi


echo ""
if [ "$SPECIFIC_PALLET" = "all" ]; then
    echo "🎉 All benchmarks completed!"
else
    echo "🎉 Benchmark completed for $SPECIFIC_PALLET!"
fi
echo ""
echo "Next steps:"
echo "1. Update runtime configs to use generated weights"
echo "2. Add weight modules to runtime/*/src/weights/mod.rs"
echo "3. Test compilation: cargo check --features=runtime-benchmarks"
echo ""
echo "Examples:"
echo "  ./benchmark.sh argon 10 pallet_block_rewards    # Benchmark specific pallet"
echo "  ./benchmark.sh all 20                          # Benchmark all pallets"
