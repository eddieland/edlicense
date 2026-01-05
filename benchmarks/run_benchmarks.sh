#!/usr/bin/env bash
# Run edlicense benchmarks and generate visualizations.

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." &> /dev/null && pwd)"
cd "$ROOT_DIR"

for bin in cargo uv; do
  command -v "$bin" >/dev/null || { echo "Error: $bin is required." >&2; exit 1; }
done

mkdir -p dist/benchmark_visualizations

run_benchmark() {
  local test_name="$1"
  local display_name="$2"

  echo
  echo "=== $display_name ==="
  cargo nextest run "$test_name" --run-ignored=all --no-capture | tee "/tmp/${test_name}_output.txt"
  if [ "${PIPESTATUS[0]}" -ne 0 ]; then
    echo "Warning: $test_name did not complete successfully."
  fi
}

run_benchmark "benchmark_operations" "Operation Types (Add/Update/Check)"
run_benchmark "test_file_size_impact" "File Size Impact"
run_benchmark "test_thread_count_impact" "Thread Count Impact"

uv run benchmarks/generate_benchmarks.py
