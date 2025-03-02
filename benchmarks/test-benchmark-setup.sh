#!/usr/bin/env bash
# Script to test the benchmark visualization setup with a quick example

set -euo pipefail

echo "============================================="
echo "  Testing benchmark visualization setup"
echo "============================================="
echo

# Make sure we have the Python environment
if [ ! -d "dist/env" ]; then
    echo "Setting up Python environment..."
    mkdir -p dist
    python -m venv dist/env
    source dist/env/bin/activate
    pip install matplotlib pandas numpy
else
    source dist/env/bin/activate
fi

# Create a directory for sample visualizations
SAMPLE_DIR="benchmark_visualizations"
mkdir -p "$SAMPLE_DIR"

# Create a simple test visualization to verify the setup
echo "Generating sample benchmark visualization..."
python - << 'EOF'
import matplotlib.pyplot as plt
import numpy as np
import os

# Sample benchmark data - these are just example numbers
operations = ["Add License", "Update Year", "Check License"]
avg_times = [120.5, 85.3, 45.2]  # milliseconds
min_times = [110.2, 75.8, 40.1]
max_times = [135.8, 98.6, 52.7]

# Create a visualization
plt.figure(figsize=(10, 6))
x = np.arange(len(operations))
width = 0.25

plt.bar(x - width, avg_times, width, label='Average')
plt.bar(x, min_times, width, label='Min')
plt.bar(x + width, max_times, width, label='Max')

plt.xlabel('Operation Type')
plt.ylabel('Time (milliseconds)')
plt.title('EdLicense Performance by Operation Type (SAMPLE)')
plt.xticks(x, operations)
plt.legend()
plt.grid(axis='y', linestyle='--', alpha=0.7)

# Add value labels
for i, v in enumerate(avg_times):
    plt.text(i - width, v + 5, f'{v:.1f}', ha='center')
for i, v in enumerate(min_times):
    plt.text(i, v + 5, f'{v:.1f}', ha='center')
for i, v in enumerate(max_times):
    plt.text(i + width, v + 5, f'{v:.1f}', ha='center')

# Save the figure
plt.tight_layout()
plt.savefig('benchmark_visualizations/sample_visualization_20250301_000000.png', dpi=150)
print("Sample visualization saved to benchmark_visualizations/sample_visualization_20250301_000000.png")
EOF

echo
echo "============================================="
echo "Testing export functionality..."
echo "============================================="
# Test the export script
python dist/export_benchmark_images.py

echo
echo "============================================="
echo "Setup Test Complete"
echo "============================================="
echo "The benchmark visualization setup is working correctly."
echo "You can now run the full benchmarks with: ./run_benchmarks.sh"
echo "This script will generate real benchmark data by running the performance tests."
echo