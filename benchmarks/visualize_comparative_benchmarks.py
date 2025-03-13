#!/usr/bin/env python3
"""
Visualize comparative benchmark results between edlicense and addlicense.
This script reads the JSON result files from the benchmark tests and
generates visualizations comparing the performance of both tools.
"""

import argparse
import json
import os
import glob
from pathlib import Path
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import re


def load_benchmark_results(results_dir):
    """Load all benchmark JSON files from the specified directory."""
    all_results = []

    # Find all JSON files in the results directory
    json_files = glob.glob(os.path.join(results_dir, "benchmark_*.json"))

    for file_path in json_files:
        try:
            with open(file_path, "r") as f:
                results = json.load(f)
                # Add the filename as a reference
                for result in results:
                    result["source_file"] = os.path.basename(file_path)
                all_results.extend(results)
        except Exception as e:
            print(f"Error loading {file_path}: {e}")

    return pd.DataFrame(all_results)


def plot_operation_comparison(df, output_dir):
    """Create a plot comparing operations (add/update/check) between tools."""
    # Filter out thread-specific benchmarks
    df_ops = df[df["source_file"].str.contains("add|update|check")]

    # Group by relevant factors and calculate mean duration
    grouped = df_ops.groupby(["tool", "operation", "file_size_kb"]).agg({"duration_ms": ["mean", "std"]}).reset_index()

    # Flatten the MultiIndex
    grouped.columns = ["_".join(col).strip("_") for col in grouped.columns.values]

    # Create plots for each file size
    for file_size in grouped["file_size_kb"].unique():
        plt.figure(figsize=(10, 6))

        # Filter data for this file size
        df_size = grouped[grouped["file_size_kb"] == file_size]

        # Get file count (should be the same for all operations at this file size)
        file_count = df_ops[df_ops["file_size_kb"] == file_size]["file_count"].iloc[0]

        # Set up bar positions
        operations = ["add", "update", "check"]
        x_pos = np.arange(len(operations))
        width = 0.35

        # Get data for each tool
        edlicense_data = df_size[df_size["tool"] == "edlicense"]
        addlicense_data = df_size[df_size["tool"] == "addlicense"]

        # Prepare bar heights and error data
        edlicense_heights = []
        edlicense_errors = []
        addlicense_heights = []
        addlicense_errors = []

        for op in operations:
            # edlicense data
            ed_op_data = edlicense_data[edlicense_data["operation"] == op]
            if not ed_op_data.empty:
                edlicense_heights.append(ed_op_data["duration_ms_mean"].values[0])
                edlicense_errors.append(ed_op_data["duration_ms_std"].values[0])
            else:
                edlicense_heights.append(0)
                edlicense_errors.append(0)

            # addlicense data
            add_op_data = addlicense_data[addlicense_data["operation"] == op]
            if not add_op_data.empty:
                addlicense_heights.append(add_op_data["duration_ms_mean"].values[0])
                addlicense_errors.append(add_op_data["duration_ms_std"].values[0])
            else:
                addlicense_heights.append(0)
                addlicense_errors.append(0)

        # Create the grouped bar chart
        plt.bar(
            x_pos - width / 2,
            edlicense_heights,
            width,
            yerr=edlicense_errors,
            label="edlicense",
            color="#5d9cf5",
            capsize=5,
        )
        plt.bar(
            x_pos + width / 2,
            addlicense_heights,
            width,
            yerr=addlicense_errors,
            label="addlicense",
            color="#f55d5d",
            capsize=5,
        )

        # Add labels, title and legend
        plt.xlabel("Operation")
        plt.ylabel("Duration (ms)")
        plt.title(f"Performance Comparison - {file_size}KB Files ({file_count} files)")
        plt.xticks(x_pos, operations)
        plt.legend()
        plt.grid(axis="y", linestyle="--", alpha=0.7)

        # Add value labels on top of bars
        for i, v in enumerate(edlicense_heights):
            plt.text(i - width / 2, v + 5, f"{v:.0f}", ha="center", va="bottom", fontsize=9)

        for i, v in enumerate(addlicense_heights):
            plt.text(i + width / 2, v + 5, f"{v:.0f}", ha="center", va="bottom", fontsize=9)

        # Save the figure
        output_path = os.path.join(output_dir, f"comparison_{file_size}KB.png")
        plt.tight_layout()
        plt.savefig(output_path, dpi=300)
        print(f"Saved {output_path}")
        plt.close()


def plot_thread_impact(df, output_dir):
    """Create a plot showing the impact of thread count on edlicense performance."""
    # Filter for thread count benchmarks
    df_threads = df[df["source_file"].str.contains("thread_impact")]

    if df_threads.empty:
        print("No thread impact data found.")
        return

    # Get file count (should be the same for all thread counts)
    file_count = df_threads["file_count"].iloc[0]

    # Group by thread count and calculate statistics
    grouped = df_threads.groupby("thread_count").agg({"duration_ms": ["mean", "std"]}).reset_index()

    # Flatten the MultiIndex
    grouped.columns = ["_".join(col).strip("_") for col in grouped.columns.values]

    plt.figure(figsize=(10, 6))

    # Sort by thread count
    grouped = grouped.sort_values("thread_count")

    # Extract data
    thread_counts = grouped["thread_count"].tolist()
    durations = grouped["duration_ms_mean"].tolist()
    errors = grouped["duration_ms_std"].tolist()

    # Create the bar chart
    bars = plt.bar(thread_counts, durations, yerr=errors, capsize=5, color="#5d9cf5")

    # Add labels and title
    plt.xlabel("Thread Count")
    plt.ylabel("Duration (ms)")
    plt.title(f"Impact of Thread Count on edlicense Performance ({file_count} files)")
    plt.grid(axis="y", linestyle="--", alpha=0.7)

    # Add value labels on top of bars
    for bar in bars:
        height = bar.get_height()
        plt.text(bar.get_x() + bar.get_width() / 2.0, height + 5, f"{height:.0f}", ha="center", va="bottom", fontsize=9)

    # Save the figure
    output_path = os.path.join(output_dir, "thread_impact.png")
    plt.tight_layout()
    plt.savefig(output_path, dpi=300)
    print(f"Saved {output_path}")
    plt.close()


def plot_file_size_impact(df, output_dir):
    """Create a plot showing the impact of file size on performance."""
    # Filter out thread-specific benchmarks
    df_sizes = df[~df["source_file"].str.contains("thread_impact")]

    # Group by relevant factors and calculate mean duration
    grouped = df_sizes.groupby(["tool", "operation", "file_size_kb"]).agg({"duration_ms": ["mean"]}).reset_index()

    # Flatten the MultiIndex
    grouped.columns = ["_".join(col).strip("_") for col in grouped.columns.values]

    # Create plots for each operation
    for operation in grouped["operation"].unique():
        plt.figure(figsize=(10, 6))

        # Filter data for this operation
        df_op = grouped[grouped["operation"] == operation]

        # Get file counts for each file size
        file_counts = {}
        for size in sorted(df_op["file_size_kb"].unique()):
            # Get file count for this size (should be the same for all tools)
            file_count = df_sizes[df_sizes["file_size_kb"] == size]["file_count"].iloc[0]
            file_counts[size] = file_count

        # Prepare data for each tool
        tools = df_op["tool"].unique()
        file_sizes = sorted(df_op["file_size_kb"].unique())

        for tool in tools:
            tool_data = df_op[df_op["tool"] == tool]
            tool_data = tool_data.sort_values("file_size_kb")

            # Plot line for this tool
            plt.plot(tool_data["file_size_kb"], tool_data["duration_ms_mean"], marker="o", linewidth=2, label=tool)

        # Add labels and title
        plt.xlabel("File Size (KB)")
        plt.ylabel("Duration (ms)")
        plt.title(f"Impact of File Size on {operation.capitalize()} Operation")
        plt.legend()
        plt.grid(linestyle="--", alpha=0.7)

        # Add file count annotations
        for i, size in enumerate(file_sizes):
            plt.annotate(
                f"{file_counts[size]} files",
                xy=(size, 0),
                xytext=(0, 10),
                textcoords="offset points",
                ha="center",
                va="bottom",
                fontsize=8,
            )

        # Use log scale for x-axis to better show the range
        plt.xscale("log")
        plt.xticks(file_sizes, [str(size) for size in file_sizes])

        # Save the figure
        output_path = os.path.join(output_dir, f"filesize_impact_{operation}.png")
        plt.tight_layout()
        plt.savefig(output_path, dpi=300)
        print(f"Saved {output_path}")
        plt.close()


def generate_summary_table(df, output_dir):
    """Generate a summary table of benchmark results."""
    # Filter out thread-specific benchmarks
    df_summary = df[~df["source_file"].str.contains("thread_impact")]

    # Group by relevant factors and calculate statistics
    grouped = (
        df_summary.groupby(["tool", "operation", "file_size_kb"])
        .agg({"duration_ms": ["mean", "std", "min", "max"]})
        .reset_index()
    )

    # Flatten the MultiIndex
    grouped.columns = ["_".join(col).strip("_") for col in grouped.columns.values]

    # Calculate performance ratio (addlicense / edlicense)
    summary_data = []

    for op in grouped["operation"].unique():
        for size in sorted(grouped["file_size_kb"].unique()):
            ed_data = grouped[
                (grouped["tool"] == "edlicense") & (grouped["operation"] == op) & (grouped["file_size_kb"] == size)
            ]

            add_data = grouped[
                (grouped["tool"] == "addlicense") & (grouped["operation"] == op) & (grouped["file_size_kb"] == size)
            ]

            if not ed_data.empty and not add_data.empty:
                ed_mean = ed_data["duration_ms_mean"].values[0]
                add_mean = add_data["duration_ms_mean"].values[0]

                ratio = add_mean / ed_mean if ed_mean > 0 else 0
                percent_diff = ((add_mean - ed_mean) / ed_mean) * 100 if ed_mean > 0 else 0

                summary_data.append(
                    {
                        "Operation": op,
                        "File Size (KB)": size,
                        "File Count": int(ed_data["file_count"].values[0]),
                        "edlicense (ms)": round(ed_mean, 2),
                        "addlicense (ms)": round(add_mean, 2),
                        "Ratio (addlicense/edlicense)": round(ratio, 2),
                        "Percent Difference": f"{round(percent_diff, 1)}%",
                    }
                )

    # Create DataFrame from summary data
    summary_df = pd.DataFrame(summary_data)

    # Write to CSV
    csv_path = os.path.join(output_dir, "benchmark_summary.csv")
    summary_df.to_csv(csv_path, index=False)
    print(f"Saved summary to {csv_path}")

    return summary_df


def plot_speedup_comparison(summary_df, output_dir):
    """Create a plot showing the speedup ratio between tools."""
    if summary_df.empty:
        print("No summary data available for speedup comparison.")
        return

    plt.figure(figsize=(10, 6))

    # Get unique operations and file sizes
    operations = summary_df["Operation"].unique()
    file_sizes = sorted(summary_df["File Size (KB)"].unique())

    # Get file counts for each file size
    file_counts = {}
    for size in file_sizes:
        file_count = summary_df[summary_df["File Size (KB)"] == size]["File Count"].iloc[0]
        file_counts[size] = file_count

    # Set up bar positions
    x_pos = np.arange(len(file_sizes))
    width = 0.25

    # Plot bars for each operation
    for i, operation in enumerate(operations):
        op_data = summary_df[summary_df["Operation"] == operation]
        op_data = op_data.sort_values("File Size (KB)")

        ratios = op_data["Ratio (addlicense/edlicense)"].tolist()

        plt.bar(x_pos + (i - 1) * width, ratios, width, label=operation, alpha=0.8)

    # Add reference line for ratio=1 (equal performance)
    plt.axhline(y=1, color="r", linestyle="--", alpha=0.5, label="Equal Performance")

    # Add labels and title
    plt.xlabel("File Size (KB)")
    plt.ylabel("Speedup Ratio (addlicense/edlicense)")
    plt.title("Performance Comparison: Speedup Ratio")
    plt.xticks(x_pos, [f"{size} KB\n({file_counts[size]} files)" for size in file_sizes])
    plt.legend()
    plt.grid(axis="y", linestyle="--", alpha=0.7)

    # Save the figure
    output_path = os.path.join(output_dir, "speedup_ratio.png")
    plt.tight_layout()
    plt.savefig(output_path, dpi=300)
    print(f"Saved {output_path}")
    plt.close()


def generate_report(df, summary_df, output_dir):
    """Generate an HTML report with all benchmark results."""
    # Create a simple HTML report
    html_content = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <title>edlicense vs addlicense Benchmark Results</title>
        <style>
            body {{ font-family: Arial, sans-serif; margin: 20px; }}
            h1 {{ color: #333; }}
            h2 {{ color: #555; margin-top: 30px; }}
            table {{ border-collapse: collapse; width: 100%; margin-top: 20px; }}
            th, td {{ text-align: left; padding: 12px; }}
            th {{ background-color: #f2f2f2; }}
            tr:nth-child(even) {{ background-color: #f9f9f9; }}
            .highlight {{ font-weight: bold; color: #2c7fb8; }}
            img {{ max-width: 100%; margin-top: 20px; }}
            .section {{ margin-top: 40px; }}
            .summary {{ background-color: #f5f5f5; padding: 15px; border-radius: 5px; }}
        </style>
    </head>
    <body>
        <h1>edlicense vs addlicense Benchmark Results</h1>
        
        <div class="summary">
            <h2>Performance Summary</h2>
            <p>
                These benchmarks compare performance between edlicense and Google's addlicense tool
                across different operations, file sizes, and file counts. The tables and charts include
                information about the number of files processed in each benchmark to provide a more
                comprehensive view of performance characteristics.
            </p>
        </div>
        
        <div class="section">
            <h2>Summary Table</h2>
            <table border="1">
                <tr>
                    <th>Operation</th>
                    <th>File Size (KB)</th>
                    <th>File Count</th>
                    <th>edlicense (ms)</th>
                    <th>addlicense (ms)</th>
                    <th>Ratio (addlicense/edlicense)</th>
                    <th>Percent Difference</th>
                </tr>
    """

    # Add summary table rows
    for _, row in summary_df.iterrows():
        ratio = row["Ratio (addlicense/edlicense)"]
        highlight = "highlight" if ratio > 1 else ""

        html_content += f"""
                <tr>
                    <td>{row["Operation"]}</td>
                    <td>{row["File Size (KB)"]}</td>
                    <td>{row["File Count"]}</td>
                    <td>{row["edlicense (ms)"]}</td>
                    <td>{row["addlicense (ms)"]}</td>
                    <td class="{highlight}">{ratio}</td>
                    <td class="{highlight}">{row["Percent Difference"]}</td>
                </tr>
        """

    html_content += """
            </table>
        </div>
        
        <div class="section">
            <h2>Performance Comparisons</h2>
    """

    # Add all generated charts
    chart_files = sorted(glob.glob(os.path.join(output_dir, "*.png")))
    for chart_file in chart_files:
        file_name = os.path.basename(chart_file)
        chart_title = file_name.replace(".png", "").replace("_", " ").title()

        html_content += f"""
            <div>
                <h3>{chart_title}</h3>
                <img src="{file_name}" alt="{chart_title}">
            </div>
        """

    html_content += """
        </div>
    </body>
    </html>
    """

    # Write HTML report
    report_path = os.path.join(output_dir, "benchmark_report.html")
    with open(report_path, "w") as f:
        f.write(html_content)

    print(f"Generated HTML report at {report_path}")


def main():
    parser = argparse.ArgumentParser(description="Visualize benchmark results")
    parser.add_argument(
        "--results-dir", default="target/benchmark_results", help="Directory containing benchmark JSON results"
    )
    parser.add_argument("--output-dir", default="benchmark_visualizations", help="Directory to save visualizations")

    args = parser.parse_args()

    # Ensure output directory exists
    os.makedirs(args.output_dir, exist_ok=True)

    # Load benchmark results
    df = load_benchmark_results(args.results_dir)

    if df.empty:
        print("No benchmark results found.")
        return

    # Generate visualizations
    plot_operation_comparison(df, args.output_dir)
    plot_thread_impact(df, args.output_dir)
    plot_file_size_impact(df, args.output_dir)

    # Generate summary table
    summary_df = generate_summary_table(df, args.output_dir)

    # Generate speedup comparison
    plot_speedup_comparison(summary_df, args.output_dir)

    # Generate HTML report
    generate_report(df, summary_df, args.output_dir)

    print("Visualization complete!")


if __name__ == "__main__":
    main()
