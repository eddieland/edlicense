#!/usr/bin/env python3
# /// script
# requires-python = ">=3.9"
# dependencies = []
# ///
import os
import shutil
import glob
import datetime


def export_benchmark_images():
    """
    Export the most recent benchmark visualizations to a timestamped directory
    that can be easily shared.
    """
    # Source directory containing the benchmark visualizations
    source_dir = "dist/benchmark_visualizations"

    # Create a timestamp for the export directory name
    timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    export_dir = f"benchmark_exports_{timestamp}"

    # Ensure source directory exists
    if not os.path.exists(source_dir):
        print(f"Error: Source directory '{source_dir}' not found.")
        print("Run ./run_benchmarks.sh first to generate visualizations.")
        return False

    # Create export directory
    os.makedirs(export_dir, exist_ok=True)
    print(f"Created export directory: {export_dir}")

    # Find the latest image of each type
    image_types = {}

    for image_path in glob.glob(f"{source_dir}/*.png"):
        filename = os.path.basename(image_path)
        # Extract the base name (removing timestamp)
        base_name = "_".join(filename.split("_")[:-1])

        if base_name not in image_types or os.path.getmtime(image_path) > os.path.getmtime(image_types[base_name]):
            image_types[base_name] = image_path

    # Copy the latest images to the export directory
    for base_name, image_path in image_types.items():
        new_filename = f"{base_name}.png"
        destination = os.path.join(export_dir, new_filename)
        shutil.copy2(image_path, destination)
        print(f"Exported: {new_filename}")

    print(f"\nAll benchmark images have been exported to: {export_dir}")
    return True


if __name__ == "__main__":
    export_benchmark_images()
