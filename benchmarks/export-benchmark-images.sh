#!/usr/bin/env bash
# Export edlicense benchmark visualization images for sharing.

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." &> /dev/null && pwd)"
cd "$ROOT_DIR"

command -v uv >/dev/null || { echo "Error: uv is required." >&2; exit 1; }

VISUALIZATIONS_DIR="dist/benchmark_visualizations"
if [ ! -d "$VISUALIZATIONS_DIR" ] || [ -z "$(ls -A "$VISUALIZATIONS_DIR" 2>/dev/null)" ]; then
  echo "Error: no benchmark visualizations found in $VISUALIZATIONS_DIR." >&2
  exit 1
fi

echo "Exporting benchmark visualization images..."
uv run benchmarks/export_benchmark_images.py
