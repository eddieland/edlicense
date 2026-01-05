#!/usr/bin/env bash
# Run comparative benchmarks between edlicense and addlicense and render plots.

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." &> /dev/null && pwd)"
cd "$ROOT_DIR"

for bin in cargo uv; do
  command -v "$bin" >/dev/null || { echo "Error: $bin is required." >&2; exit 1; }
done

RESULTS_DIR="target/benchmark_results"
VISUALIZATIONS_DIR="benchmark_visualizations"
mkdir -p "$RESULTS_DIR" "$VISUALIZATIONS_DIR"

OUTPUT_FILE="/tmp/comparative_benchmark_output.txt"
cargo nextest run comparative_benchmark --run-ignored=all --no-capture | tee "$OUTPUT_FILE"
if [ "${PIPESTATUS[0]}" -ne 0 ]; then
  echo "Warning: comparative_benchmark did not complete successfully."
  echo "Check $OUTPUT_FILE for details."
fi

uv run benchmarks/visualize_comparative_benchmarks.py \
  --results-dir "$RESULTS_DIR" \
  --output-dir "$VISUALIZATIONS_DIR"

REPORT_PATH="$(pwd)/$VISUALIZATIONS_DIR/benchmark_report.html"
echo "Results: $RESULTS_DIR"
echo "Visualizations: $VISUALIZATIONS_DIR"
echo "HTML report: $REPORT_PATH"
