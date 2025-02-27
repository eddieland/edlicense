# edlicense

A tool that ensures source code files have copyright license headers by scanning directory patterns recursively.

`edlicense` modifies source files in place and avoids adding a license header to any file that already has one. It follows the Unix philosophy of tooling where possible and is designed with modern Rust best practices for high-performance CLI tools.

## Why edlicense over addlicense?

`edlicense` is inspired by Google's [addlicense](https://github.com/google/addlicense) tool but addresses several limitations and adds modern features:

| Feature | edlicense | addlicense |
|---------|-----------|------------|
| Implementation | Rust | Go |
| CLI interface | Modern long options (--option-name) | Short flags (-flag) |
| Default behavior | Dry run mode (non-destructive) | Modify mode |
| Automatic year updates | ✅ Updates copyright years automatically | ❌ No support for updating files w/ old years |
| Ratchet mode | ✅ Process only files changed since a git reference | ❌ Not available |

Key advantages of `edlicense`:

1. **Safety First**: Defaults to dry run mode, preventing accidental file modifications
2. **Git Integration**: Ratchet mode for CI/CD pipelines to process only changed files
3. **Automatic Updates**: Intelligently updates copyright years without manual intervention

## Features

- Recursively scan directories and add license headers to source files
- Automatic detection of file types and appropriate comment formatting
- Dry run mode to verify license headers without modifying files (default behavior)
- Ignore patterns to exclude specific files or directories
- **Automatic year reference updates** - automatically updates copyright year references when the year changes (e.g., `(c) 2024` → `(c) 2025`)
- **Ratchet mode** - only check and format files that have changed relative to a git reference (e.g., `origin/main`)

## Installation

### From crates.io

```bash
cargo install edlicense
```

### From source

```bash
git clone https://github.com/omenien/edlicense.git
cd edlicense
cargo install --path .
```

### Using Docker

edlicense is available as a Docker image, making it easy to run without installing Rust or any dependencies.

#### Building the Docker image

The project uses a single Dockerfile that can build both production and debug images:

```bash
# Build the lightweight production image
make docker-build

# Build the debug/development image
make docker-build-debug

# Build both images
make docker-build-all
```

#### Running with Docker

```bash
# Run edlicense on files in the current directory (dry run mode)
docker run --rm -v "$(pwd):/workspace" -w /workspace edlicense:latest src/

# Using the make target (equivalent to above)
make docker-run ARGS="src/"

# Run edlicense in modify mode
docker run --rm -v "$(pwd):/workspace" -w /workspace edlicense:latest --modify src/

# Run with the debug image for development purposes
make docker-run-debug ARGS="cargo test"
```

The Docker setup provides two image tags from the same Dockerfile:

1. **Lightweight image** (`edlicense:latest`): A minimal image containing only the compiled binary, optimized for CI/CD pipelines and production use.

2. **Debug image** (`edlicense:debug`): A development image containing the full Rust toolchain, source code, and development tools, useful for debugging and development.

For advanced Docker usage, including building downstream images and handling file permissions, see [Docker Usage Examples](examples/docker_usage.md).

## Usage

```
edlicense [OPTIONS] <PATTERNS>...
```

### Arguments

- `<PATTERNS>...` - File or directory patterns to process. Directories are processed recursively.

### Options

```
--dry-run                     Dry run mode: only check for license headers without modifying files (default)
--modify                      Modify mode: add or update license headers in files
--license-file <LICENSE_FILE> Custom license file to use
--ignore <IGNORE>...          File patterns to ignore (supports glob patterns)
--year <YEAR>                 Copyright year [default: current year]
--verbose                     Verbose logging
--ratchet <REFERENCE>         Ratchet mode: only check and format files that have changed relative to a git reference
--preserve-years              Preserve existing years in license headers
--help                        Print help
--version                     Print version
```

Note: `--dry-run` and `--modify` are mutually exclusive options. If neither is specified, dry run mode is used by default.

## Examples

Check if all files have license headers without modifying them (dry run mode):

```bash
edlicense --dry-run src/ tests/
```

Or simply (since dry run is the default):

```bash
edlicense src/ tests/
```

Ignore specific file patterns:

```bash
edlicense --ignore "**/*.json" --ignore "vendor/**" .
```

Use a specific year:

```bash
edlicense --year "2020" .
```

Only check files that have changed relative to origin/main (dry run mode):

```bash
edlicense --ratchet "origin/main" src/
```

Add or update license headers in files that have changed relative to origin/main:

```bash
edlicense --ratchet "origin/main" --modify src/
```

## Automatic Year Updates

Unlike the original `addlicense` tool, `edlicense` can automatically update copyright year references when the year changes. For example, if a file contains:

```
Copyright (c) 2024 Example Corp
```

And the current year is 2025, running `edlicense` will update it to:

```
Copyright (c) 2025 Example Corp
```

## Ratchet Mode

The ratchet mode allows you to only check and format files that have changed relative to a git reference (e.g., `origin/main`). This is particularly useful in CI/CD pipelines where you want to ensure that only new or modified files have proper license headers.

When using ratchet mode, `edlicense` will:

1. Identify files that have been added, modified, or renamed since the specified git reference
2. Only process those changed files, ignoring files that haven't changed
3. Apply the same license checking or formatting rules to the changed files

This can significantly speed up processing in large repositories where only a small subset of files have changed.

Example usage:

```bash
# Only check license headers in files changed since origin/main (dry run mode)
edlicense --ratchet "origin/main" src/

# Add license headers to files changed since a specific commit
edlicense --ratchet "abc123" --modify --license-file LICENSE.txt src/
```

## Supported File Types

`edlicense` supports a wide range of file types and automatically formats license headers with the appropriate comment style:

- C/C++/C#/Go/Rust/Swift/Dart: `// comment style`
- Java/Scala/Kotlin: `/* comment style */`
- JavaScript/TypeScript/CSS: `/** comment style */`
- Python/Shell/YAML/Ruby: `# comment style`
- HTML/XML/Vue: `<!-- comment style -->`
- And many more...

## Performance Testing

`edlicense` includes performance tests to measure how efficiently it processes large numbers of files. These tests are useful for benchmarking and optimizing the tool's performance, especially for large codebases.

### Running Performance Tests

Performance tests are disabled by default since they generate and process thousands of files. To run them, use the following Makefile targets:

```bash
# Run test for adding licenses to 10,000 files
make perf-test-add

# Run test for updating license years in 10,000 files
make perf-test-update

# Run test for checking license headers in 10,000 files
make perf-test-check

# Run test with different file sizes
make perf-test-file-size

# Run test with different thread counts
make perf-test-threads

# Run comprehensive benchmark tests
make perf-benchmark

# Run all performance tests (this may take a while)
make perf-test-all
```

Alternatively, you can run the tests directly with cargo:

```bash
# Run a specific performance test
cargo test --release test_add_license_performance -- --ignored --nocapture

# Run all performance tests
cargo test --release -- --ignored --nocapture
```

### Performance Test Results

The performance tests measure:

1. **Adding licenses** to large numbers of files (10,000+)
2. **Updating years** in existing license headers
3. **Checking for licenses** in check-only mode
4. Impact of **file size** on processing time
5. Impact of **thread count** on parallel processing performance

Results are displayed in the console with timing information, making it easy to identify performance bottlenecks or improvements.
