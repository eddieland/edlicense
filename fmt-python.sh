#!/usr/bin/env bash
# fmt-python.sh - Format Python files using ruff via uv

set -eo pipefail

# Print usage information
usage() {
  echo "Usage: $0 [OPTIONS] [PATHS...]"
  echo ""
  echo "Format Python files using ruff formatter via uv"
  echo ""
  echo "Options:"
  echo "  --check          Check if files are formatted correctly without modifying them"
  echo "  --verbose, -v    Enable verbose output"
  echo "  --help, -h       Show this help message and exit"
  echo ""
  echo "If no paths are specified, formats all Python files in the benchmarks directory"
}

# Parse command-line options
CHECK=false
VERBOSE=false
PATHS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      CHECK=true
      shift
      ;;
    --verbose|-v)
      VERBOSE=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    -*)
      echo "Error: Unknown option: $1"
      usage
      exit 1
      ;;
    *)
      PATHS+=("$1")
      shift
      ;;
  esac
done

# If no paths specified, default to benchmarks directory
if [ ${#PATHS[@]} -eq 0 ]; then
  PATHS=("benchmarks")
fi

# Set default options
RUFF_OPTIONS=()

# Add options based on command line arguments
if [ "$CHECK" = true ]; then
  RUFF_OPTIONS+=(--check)
fi

if [ "$VERBOSE" = true ]; then
  RUFF_OPTIONS+=(--verbose)
fi

# Print command if verbose
if [ "$VERBOSE" = true ]; then
  echo "Running: uvx ruff format ${RUFF_OPTIONS[*]} ${PATHS[*]}"
fi

# Run ruff via uv
if ! command -v uvx &> /dev/null; then
  echo "Error: uv is not installed or not in PATH"
  echo "Install uv with: curl -sSf https://install.python-uv.org | python3"
  exit 1
fi

# Execute the command and capture exit status
if [ "$CHECK" = true ]; then
  echo "Checking Python formatting..."
else
  echo "Formatting Python files..."
fi

# Run formatter through uv and capture output for better error reporting
OUTPUT=$(uvx ruff format "${RUFF_OPTIONS[@]}" "${PATHS[@]}" 2>&1) || {
  EXIT_CODE=$?
  echo -e "Formatting failed with exit code $EXIT_CODE:\n$OUTPUT" >&2
  exit $EXIT_CODE
}

# Print success message with details
if [ -n "$OUTPUT" ]; then
  echo "$OUTPUT"
fi

if [ "$CHECK" = true ]; then
  echo "Format check completed successfully"
else
  echo "Formatting completed successfully"
fi