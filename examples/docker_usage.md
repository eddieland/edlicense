# Docker Usage Examples

This document provides examples for using the edlicense Docker image, including building downstream images for specific use cases.

## Building a Downstream Docker Image

You can create your own Docker image based on the edlicense image to customize it for your specific needs. This is particularly useful for CI/CD pipelines or when you want to distribute a pre-configured version of edlicense.

### Example Dockerfile for a Downstream Image

```dockerfile
# Use the edlicense production image as the base
FROM edlicense:latest

# Create a directory for license templates
WORKDIR /licenses

# Copy your custom license template
COPY my-license-template.txt .

# Set proper permissions for the license file
# This ensures the file is readable regardless of the user running the container
RUN chmod 644 my-license-template.txt

# Set the working directory where files will be processed
WORKDIR /workspace

# Set default command to use your custom license template
ENTRYPOINT ["edlicense", "--license-file", "/licenses/my-license-template.txt"]

# Default arguments (can be overridden)
CMD ["."]  # Dry run mode is the default
```

### Building the Downstream Image

```bash
# Build your custom image
docker build -t my-edlicense:latest -f Dockerfile.custom .
```

### Running the Downstream Image

```bash
# Run your custom image on the current directory
docker run --rm -v "$(pwd):/workspace" my-edlicense:latest

# Override the default arguments
docker run --rm -v "$(pwd):/workspace" my-edlicense:latest --verbose src/
```

## File Permissions in Docker

When working with Docker containers, file permissions can be tricky. Here are some best practices:

1. **Template Files**: Make sure license template files have read permissions for all users (`chmod 644`). This ensures the files can be read regardless of the user running the container.

2. **Mounted Volumes**: When mounting volumes, be aware that the files will be accessed with the user ID of the process inside the container. By default, this is root, which can lead to permission issues when files are created or modified.

3. **User Permissions**: For better security, you can specify a non-root user in your Dockerfile:

```dockerfile
# Create a non-root user
RUN adduser --disabled-password --gecos "" eduser

# Switch to the non-root user
USER eduser

# Set the entrypoint
ENTRYPOINT ["edlicense", "--license-file", "/licenses/my-license-template.txt"]
```

However, when using a non-root user, you may encounter permission issues when modifying files in mounted volumes. To address this, you can:

- Match the user ID inside the container with your host user ID
- Use appropriate volume mount options
- Ensure the mounted directories have appropriate permissions

## Example: CI/CD Pipeline Image

Here's an example of a downstream image specifically designed for CI/CD pipelines:

```dockerfile
FROM edlicense:latest

# Copy license template and configuration
COPY ci/license-template.txt /licenses/
COPY ci/edlicense-config.sh /usr/local/bin/

# Make the configuration script executable
RUN chmod +x /usr/local/bin/edlicense-config.sh

# Set the entrypoint to the configuration script
ENTRYPOINT ["/usr/local/bin/edlicense-config.sh"]
```

The `edlicense-config.sh` script could look like:

```bash
#!/bin/bash
set -e

# Default to dry run mode (no modification)
MODIFY_MODE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --modify)
      MODIFY_MODE="--modify"
      shift
      ;;
    *)
      ARGS="$ARGS $1"
      shift
      ;;
  esac
done

# Run edlicense with the appropriate options
exec edlicense $MODIFY_MODE --license-file /licenses/license-template.txt $ARGS
```

This allows you to run the container with a simpler interface:

```bash
# Dry run mode (default)
docker run --rm -v "$(pwd):/workspace" -w /workspace my-ci-edlicense src/

# Modify mode
docker run --rm -v "$(pwd):/workspace" -w /workspace my-ci-edlicense --modify src/