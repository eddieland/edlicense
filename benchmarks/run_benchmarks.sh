#!/bin/bash

# Script to run edlicense benchmarks and generate visualizations
# This script automatically runs the performance tests and generates 
# visualizations using matplotlib

set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
pushd "${SCRIPT_DIR}/.."

# Check for required tools
echo "Checking dependencies..."
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed or not in PATH"
    exit 1
fi

# Create benchmark output directory
echo "Setting up benchmark environment..."
BENCHMARK_DIR="dist/benchmark_visualizations"
mkdir -p "$BENCHMARK_DIR"

# Activate Python virtual environment
echo "Activating Python virtual environment..."
if [ ! -d "benchmarks/.venv" ]; then
    echo "Error: Python virtual environment not found in dist/env"
    echo "Please run: mkdir -p dist && cd dist && python -m venv env && source env/bin/activate && pip install matplotlib pandas numpy"
    exit 1
fi

# Function to run a benchmark test
run_benchmark() {
    local test_name="$1"
    local display_name="$2"
    
    echo
    echo "============================================="
    echo "Running benchmark: $display_name"
    echo "============================================="
    
    # Run the test with nextest, including ignored tests, and tee output to a file
    cargo nextest run "$test_name" --run-ignored=all --no-capture | tee "/tmp/${test_name}_output.txt"
    
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
        echo "Warning: Test '$test_name' did not complete successfully."
        echo "This may be expected for tests that check for missing licenses."
    fi
}

echo "Starting benchmarks..."

run_benchmark "benchmark_operations" "Operation Types (Add/Update/Check)"
run_benchmark "test_file_size_impact" "File Size Impact"
run_benchmark "test_thread_count_impact" "Thread Count Impact"

benchmarks/.venv/bin/python benchmarks/generate_benchmarks.py
