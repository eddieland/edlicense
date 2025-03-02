#!/usr/bin/env python3
import os
import re
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from datetime import datetime

# Configure matplotlib for better output
plt.style.use("ggplot")
plt.rcParams["figure.figsize"] = (12, 8)
plt.rcParams["font.size"] = 12

# Create output directory for visualizations
output_dir = "dist/benchmark_visualizations"
os.makedirs(output_dir, exist_ok=True)


# Function to read benchmark output from saved files
def read_benchmark_output(test_name):
    print(f"Reading benchmark data for: {test_name}")
    output_file = f"/tmp/{test_name}_output.txt"

    try:
        with open(output_file, "r") as f:
            return f.read()
    except FileNotFoundError:
        print(f"Warning: Benchmark output file not found: {output_file}")
        return ""


# Function to extract durations from benchmark output
def extract_durations(output, pattern):
    matches = re.findall(pattern, output)
    durations = []

    for match in matches:
        # Convert to milliseconds for consistency
        if "ms" in match:
            durations.append(float(match.replace("ms", "").strip()))
        elif "µs" in match:
            durations.append(float(match.replace("µs", "").strip()) / 1000)
        elif "s" in match:
            durations.append(float(match.replace("s", "").strip()) * 1000)

    return durations


# Function to save the figure with timestamp
def save_figure(fig, filename):
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = os.path.join(output_dir, f"{filename}_{timestamp}.png")
    fig.savefig(path, dpi=150, bbox_inches="tight")
    print(f"Saved figure to {path}")
    return path


# Test 1: Operation Type Comparison
def run_operation_type_benchmark():
    print("\n=== Running Operation Type Benchmark ===")
    output = read_benchmark_output("benchmark_operations")

    # Extract statistics from the output
    avg_pattern = r"Average:\s+([0-9.]+[µms]+)"
    min_pattern = r"Min:\s+([0-9.]+[µms]+)"
    max_pattern = r"Max:\s+([0-9.]+[µms]+)"

    operations = ["Add License", "Update Year", "Check License"]
    metrics = {"Average": [], "Min": [], "Max": []}

    # Find the sections for each operation
    sections = re.split(r"=== Benchmark: ", output)[1:]  # Skip the first element which is before any benchmark

    for i, section in enumerate(sections):
        if i < len(operations):
            avg_matches = re.findall(avg_pattern, section)
            min_matches = re.findall(min_pattern, section)
            max_matches = re.findall(max_pattern, section)

            if avg_matches:
                avg_duration = extract_durations(avg_matches[0], r"([0-9.]+[µms]+)")[0]
                metrics["Average"].append(avg_duration)

            if min_matches:
                min_duration = extract_durations(min_matches[0], r"([0-9.]+[µms]+)")[0]
                metrics["Min"].append(min_duration)

            if max_matches:
                max_duration = extract_durations(max_matches[0], r"([0-9.]+[µms]+)")[0]
                metrics["Max"].append(max_duration)

    # Create a grouped bar chart
    df = pd.DataFrame(metrics, index=operations)
    ax = df.plot(kind="bar", figsize=(12, 8))
    ax.set_ylabel("Time (milliseconds)")
    ax.set_title("Performance by Operation Type")

    # Add value labels on top of bars
    for container in ax.containers:
        ax.bar_label(container, fmt="%.1f", padding=3)

    plt.legend(title="Metric")
    plt.grid(axis="y", linestyle="--", alpha=0.7)
    plt.tight_layout()

    return save_figure(plt.gcf(), "operation_type_comparison")


# Test 2: File Size Impact
def run_file_size_benchmark():
    print("\n=== Running File Size Impact Benchmark ===")
    output = read_benchmark_output("test_file_size_impact")

    # Extract duration information
    pattern = r"Test 'Process ([0-9]+)KB files \([0-9]+\)' completed in ([0-9.]+[µms]+)"
    matches = re.findall(pattern, output)

    file_sizes = []
    durations = []

    for match in matches:
        file_size = int(match[0])
        duration_str = match[1]
        duration = extract_durations(duration_str, r"([0-9.]+[µms]+)")[0]

        file_sizes.append(file_size)
        durations.append(duration)

    # Create a bar chart
    fig, ax = plt.subplots(figsize=(12, 8))
    bars = ax.bar(range(len(file_sizes)), durations, tick_label=[f"{size}KB" for size in file_sizes])

    # Add value labels on top of bars
    ax.bar_label(bars, fmt="%.1f", padding=3)

    ax.set_xlabel("File Size")
    ax.set_ylabel("Time (milliseconds)")
    ax.set_title("Impact of File Size on Processing Performance")
    plt.grid(axis="y", linestyle="--", alpha=0.7)
    plt.tight_layout()

    return save_figure(plt.gcf(), "file_size_impact")


# Test 3: Thread Count Impact
def run_thread_count_benchmark():
    print("\n=== Running Thread Count Impact Benchmark ===")
    output = read_benchmark_output("test_thread_count_impact")

    # Extract duration information
    pattern = r"Test 'Process with ([0-9]+) threads' completed in ([0-9.]+[µms]+)"
    matches = re.findall(pattern, output)

    thread_counts = []
    durations = []

    for match in matches:
        thread_count = int(match[0])
        duration_str = match[1]
        duration = extract_durations(duration_str, r"([0-9.]+[µms]+)")[0]

        thread_counts.append(thread_count)
        durations.append(duration)

    # Sort by thread count
    sorted_data = sorted(zip(thread_counts, durations))
    thread_counts, durations = zip(*sorted_data) if sorted_data else ([], [])

    # Create a line chart
    fig, ax = plt.subplots(figsize=(12, 8))
    ax.plot(thread_counts, durations, "o-", linewidth=2, markersize=10)

    # Add value labels
    for x, y in zip(thread_counts, durations):
        ax.annotate(f"{y:.1f}", (x, y), textcoords="offset points", xytext=(0, 10), ha="center")

    ax.set_xlabel("Number of Threads")
    ax.set_ylabel("Time (milliseconds)")
    ax.set_title("Impact of Thread Count on Processing Performance")
    ax.set_xticks(thread_counts)
    ax.set_xticklabels(thread_counts)

    # Add horizontal lines for better readability
    plt.grid(axis="y", linestyle="--", alpha=0.7)

    # Calculate and display optimal thread count based on performance
    if durations:
        min_duration_idx = durations.index(min(durations))
        optimal_threads = thread_counts[min_duration_idx]
        plt.axvline(x=optimal_threads, color="r", linestyle="--", alpha=0.5)
        plt.text(
            optimal_threads,
            max(durations) * 0.5,
            f"Optimal: {optimal_threads} threads",
            rotation=90,
            verticalalignment="center",
            color="r",
        )

    plt.tight_layout()

    return save_figure(plt.gcf(), "thread_count_impact")


# Test 4: Combined Visualization - Thread Count Efficiency
def generate_thread_efficiency_visualization(thread_data):
    if not thread_data or len(thread_data[0]) < 2:
        print("Not enough thread data for efficiency visualization")
        return None

    thread_counts, durations = thread_data

    if len(thread_counts) < 2:
        print("Need at least two thread count data points for efficiency visualization")
        return None

    # Calculate theoretical linear speedup based on single-thread performance
    single_thread_time = durations[0] if thread_counts[0] == 1 else None

    if single_thread_time is None:
        print("Missing single-thread benchmark data")
        return None

    ideal_times = [single_thread_time / t for t in thread_counts]
    efficiency = [100 * (single_thread_time / (t * duration)) for t, duration in zip(thread_counts, durations)]

    # Create a plot with two y-axes
    fig, ax1 = plt.subplots(figsize=(12, 8))

    color1 = "tab:blue"
    ax1.set_xlabel("Number of Threads")
    ax1.set_ylabel("Time (milliseconds)", color=color1)
    line1 = ax1.plot(thread_counts, durations, "o-", color=color1, label="Actual")
    line2 = ax1.plot(thread_counts, ideal_times, "x--", color="tab:green", label="Ideal Linear Speedup")
    ax1.tick_params(axis="y", labelcolor=color1)
    ax1.set_xticks(thread_counts)

    # Add a second y-axis for efficiency
    ax2 = ax1.twinx()
    color2 = "tab:red"
    ax2.set_ylabel("Parallel Efficiency (%)", color=color2)
    line3 = ax2.plot(thread_counts, efficiency, "s-", color=color2, label="Efficiency")
    ax2.tick_params(axis="y", labelcolor=color2)

    # Set y-range for efficiency to start from 0 to 100+
    max_efficiency = max(efficiency) if efficiency else 100
    ax2.set_ylim([0, max(105, max_efficiency * 1.1)])

    # Add a reference line at 100% efficiency
    ax2.axhline(y=100, color="tab:red", linestyle="--", alpha=0.5)

    # Combine legends from both axes
    lines = line1 + line2 + line3
    labels = [l.get_label() for l in lines]
    ax1.legend(lines, labels, loc="best")

    ax1.set_title("Thread Scaling Performance and Efficiency")
    ax1.grid(True, linestyle="--", alpha=0.7)

    plt.tight_layout()

    return save_figure(plt.gcf(), "thread_efficiency")


# Main function to run all benchmarks
def main():
    print("Generating benchmark visualizations for edlicense...")

    # For demonstration, create a summary chart with mock data
    # This will be replaced with actual data when the benchmarks are run
    operation_chart = run_operation_type_benchmark()
    file_size_chart = run_file_size_benchmark()
    thread_chart = run_thread_count_benchmark()

    # Extract thread count data for efficiency visualization
    thread_pattern = r"Test 'Process with ([0-9]+) threads' completed in ([0-9.]+[µms]+)"
    thread_output = read_benchmark_output("test_thread_count_impact")

    thread_matches = re.findall(thread_pattern, thread_output)
    thread_counts = []
    durations = []

    for match in thread_matches:
        thread_counts.append(int(match[0]))
        durations.append(extract_durations(match[1], r"([0-9.]+[µms]+)")[0])

    # Sort by thread count
    sorted_data = sorted(zip(thread_counts, durations))
    thread_counts, durations = zip(*sorted_data) if sorted_data else ([], [])

    # Generate the thread efficiency visualization
    if thread_counts and durations:
        efficiency_chart = generate_thread_efficiency_visualization((thread_counts, durations))

    # Create comparison of all operations
    print("\nAll benchmark visualizations have been generated in the 'benchmark_visualizations' directory.")


if __name__ == "__main__":
    main()
