#!/usr/bin/env bash
# Script to export edlicense benchmark visualization images for sharing

# Exit on error, undefined variables, and pipefail
set -euo pipefail

# Define error handling function
die() {
  echo "ERROR: $*" >&2
  exit 1
}

# Check if Python environment exists
PYTHON_ENV="benchmarks/.venv"
if [ ! -d "$PYTHON_ENV" ]; then
  die "Python virtual environment not found in $PYTHON_ENV. Run ./run_benchmarks.sh first."
fi

# Check if benchmark visualizations exist
VISUALIZATIONS_DIR="dist/benchmark_visualizations"
if [ ! -d "$VISUALIZATIONS_DIR" ] || [ -z "$(ls -A "$VISUALIZATIONS_DIR" 2>/dev/null)" ]; then
  die "No benchmark visualizations found in $VISUALIZATIONS_DIR. Run ./run_benchmarks.sh first."
fi

# Run the Python export script
echo "Exporting benchmark visualization images..."
"${PYTHON_ENV}/.bin/python" benchmarks/export_benchmark_images.py || die "Failed to export benchmark images"
