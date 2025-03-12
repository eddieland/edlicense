#!/bin/bash

# Script to run comparative benchmarks between edlicense and addlicense
# and generate visualizations of the results

set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
pushd "${SCRIPT_DIR}/.."

# Check for required tools
echo "Checking dependencies..."
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed or not in PATH"
    exit 1
fi

# Create output directories
echo "Setting up benchmark environment..."
RESULTS_DIR="target/benchmark_results"
VISUALIZATIONS_DIR="benchmark_visualizations"
mkdir -p "$RESULTS_DIR"
mkdir -p "$VISUALIZATIONS_DIR"

# Check for Python virtual environment
echo "Checking Python environment..."
if [ ! -d "benchmarks/.venv" ]; then
    echo "Python virtual environment not found in benchmarks/.venv"
    echo "Creating Python virtual environment..."
    
    # Check if python3 is installed
    if ! command -v python3 &> /dev/null; then
        echo "Error: python3 is not installed or not in PATH"
        exit 1
    fi
    
    # Create and set up virtual environment
    python3 -m venv benchmarks/.venv
    
    # Activate virtual environment and install dependencies
    source benchmarks/.venv/bin/activate
    pip install pandas matplotlib numpy
    
    echo "Python environment setup complete"
else
    echo "Python virtual environment found"
fi

# Run the comparative benchmark tests
echo "Running comparative benchmarks..."
echo "This may take a while..."

# Run the benchmark tests with nextest and tee output to a file
OUTPUT_FILE="/tmp/comparative_benchmark_output.txt"
cargo nextest run comparative_benchmark --run-ignored=all --no-capture | tee "$OUTPUT_FILE"

# Check the exit code from nextest
if [ "${PIPESTATUS[0]}" -ne 0 ]; then
    echo "Warning: Benchmark tests did not complete successfully."
    echo "Please check $OUTPUT_FILE for details."
    echo "This might be expected for some check operations that intentionally fail."
fi

# Visualize the results
echo "Generating visualizations..."
benchmarks/.venv/bin/python benchmarks/visualize_comparative_benchmarks.py \
    --results-dir "$RESULTS_DIR" \
    --output-dir "$VISUALIZATIONS_DIR"

echo "Benchmark complete!"
echo "Results saved in $RESULTS_DIR"
echo "Visualizations saved in $VISUALIZATIONS_DIR"
echo "HTML report: $VISUALIZATIONS_DIR/benchmark_report.html"

# Make the benchmark report path absolute for easier opening
REPORT_PATH="$(pwd)/$VISUALIZATIONS_DIR/benchmark_report.html"
echo "You can view the report by opening: $REPORT_PATH"

popd