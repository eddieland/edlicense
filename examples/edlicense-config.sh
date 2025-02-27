#!/bin/bash
# Example configuration script for CI/CD usage
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