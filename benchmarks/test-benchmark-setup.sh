#!/usr/bin/env bash
# Smoke test for the benchmark visualization setup.

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." &> /dev/null && pwd)"
cd "$ROOT_DIR"

command -v uv >/dev/null || { echo "Error: uv is required." >&2; exit 1; }

SAMPLE_DIR="dist/benchmark_visualizations"
mkdir -p "$SAMPLE_DIR"

TMP_SCRIPT="$(mktemp -t edlicense-benchmark-smoke.XXXXXX.py)"
trap 'rm -f "$TMP_SCRIPT"' EXIT

cat > "$TMP_SCRIPT" << 'PY'
# /// script
# requires-python = ">=3.9"
# dependencies = [
#   "matplotlib",
#   "numpy",
# ]
# ///
import matplotlib.pyplot as plt
import numpy as np

operations = ["Add License", "Update Year", "Check License"]
avg_times = [120.5, 85.3, 45.2]
min_times = [110.2, 75.8, 40.1]
max_times = [135.8, 98.6, 52.7]

plt.figure(figsize=(10, 6))
x = np.arange(len(operations))
width = 0.25

plt.bar(x - width, avg_times, width, label="Average")
plt.bar(x, min_times, width, label="Min")
plt.bar(x + width, max_times, width, label="Max")

plt.xlabel("Operation Type")
plt.ylabel("Time (milliseconds)")
plt.title("Edlicense Performance by Operation Type (Sample)")
plt.xticks(x, operations)
plt.legend()
plt.grid(axis="y", linestyle="--", alpha=0.7)

for i, v in enumerate(avg_times):
    plt.text(i - width, v + 5, f"{v:.1f}", ha="center")
for i, v in enumerate(min_times):
    plt.text(i, v + 5, f"{v:.1f}", ha="center")
for i, v in enumerate(max_times):
    plt.text(i + width, v + 5, f"{v:.1f}", ha="center")

plt.tight_layout()
plt.savefig("dist/benchmark_visualizations/sample_visualization_20250301_000000.png", dpi=150)
print("Sample visualization saved to dist/benchmark_visualizations/sample_visualization_20250301_000000.png")
PY

echo "Generating sample benchmark visualization..."
uv run "$TMP_SCRIPT"

echo "Testing export functionality..."
uv run benchmarks/export_benchmark_images.py

echo "Setup test complete."
