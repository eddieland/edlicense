#!/usr/bin/env bash
# Script to export edlicense Docker images for offline use

# Exit on error, undefined variables, and pipefail
set -euo pipefail

# Define error handling function
die() {
  echo "ERROR: $*" >&2
  exit 1
}

# Define the images we want to export
images=(
  "ghcr.io/eddieland/edlicense:latest"
  "ghcr.io/eddieland/edlicense:distroless-latest"
)

# Create a directory for the exports
export_dir="dist/edlicense-exports"
rm -rf "${export_dir}"
mkdir -p "${export_dir}"
echo "Created export directory: ${export_dir}"
echo

# Pull and export each image
for image in "${images[@]}"; do  
  # Extract image name for the output filename
  image_filename="${export_dir}/$(echo "${image}" | tr '/:' '_').tar"
  
  docker pull "${image}" || die "Error pulling ${image}. Ensure you have internet access and Docker is running."
  docker save -o "${image_filename}" "${image}" || die "Error saving ${image}"
  
  echo
done

echo "=========================================================="
echo "  Export Complete!"
echo "=========================================================="
echo "The following Docker images have been exported:"
for image in "${images[@]}"; do
  image_filename="$(echo "${image}" | tr '/:' '_').tar"
  echo "  - ${image} → ${export_dir}/${image_filename}"
done

# Create import instructions file
instructions_file="${export_dir}/import_instructions.md"

cat > "${instructions_file}" << 'EOF'
# EdLicense Docker Images Import Instructions

These instructions will guide you through the process of importing the EdLicense Docker images on a machine without internet access.

## Prerequisites

- Docker must be installed on the target machine
- The tar archive files must be transferred to the target machine

## Import Procedure

1. Copy all the `.tar` files from this directory to the target machine using a USB drive, network transfer, or any other method available.

2. On the target machine, open a terminal and navigate to the directory containing the `.tar` files.

3. Import each image using the `docker load` command:

   ```bash
   # Import the standard image
   docker load -i ghcr_io_eddieland_edlicense_latest.tar
   
   # Import the distroless image
   docker load -i ghcr_io_eddieland_edlicense_distroless-latest.tar
   ```

4. Verify that the images were imported correctly:

   ```bash
   docker images | grep edlicense
   ```

   You should see both images listed in the output.

## Using the Imported Images

Once imported, you can use these images the same way you would if you had pulled them from the registry:

```bash
# Run using the standard image (process files in the current directory)
docker run --rm -v "$(pwd):/workspace" -w /workspace ghcr.io/eddieland/edlicense:latest .

# Run using the distroless image
docker run --rm -v "$(pwd):/workspace" -w /workspace ghcr.io/eddieland/edlicense:distroless-latest .

# Run in modify mode
docker run --rm -v "$(pwd):/workspace" -w /workspace ghcr.io/eddieland/edlicense:latest --modify .
```

## Troubleshooting

If you encounter any issues when importing or using the images:

1. Ensure the tar files were transferred completely and are not corrupted
2. Check that Docker is running on the target machine
3. Try running Docker with elevated privileges if needed (using sudo)
4. Verify that you have sufficient disk space for the imported images
EOF

echo "Import instructions created at: ${instructions_file}"
echo

# Check if du exists and use it safely
if command -v du >/dev/null 2>&1; then
  echo "Note: The total size of the exported archives is: $(du -sh "${export_dir}" | cut -f1)"
else
  echo "Note: Export complete (total size calculation not available)"
fi
echo "=========================================================="