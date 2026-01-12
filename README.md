# edlicense

A tool that ensures source code files have copyright license headers by scanning directory patterns recursively.

`edlicense` modifies source files in place and avoids adding a license header to any file that already has one. It follows the Unix philosophy of tooling where possible and is designed with modern Rust best practices for high-performance CLI tools.

## Philosophy

`edlicense` is built for **large polyglot monorepos** (100k+ files) that lack centralized build systems like Bazel. Our guiding principles:

- **Best-effort, not perfect**: We use fast heuristics adopted from [addlicense](https://github.com/google/addlicense), like checking for the word "copyright" in the first N characters rather than matching exact license text. This trades precision for speed.
- **Human review is the safety net**: We assume every important file goes through code review where a human can catch formatting oddities. The goal is to get licenses on files reliably, not to handle every edge case perfectly.
- **Heavy use of `.licenseignore`**: We expect you to liberally ignore files you don't care about or shouldn't add your license to (vendored code, generated files, assets, etc.).
- **Git-first**: While `edlicense` works without a git repository, we're fundamentally a git-first tool. Features like `--git-only` (default in git repos) and `--ratchet` reflect this.

If you need byte-perfect license validation or work in a tightly controlled build environment, a build-system-integrated solution may serve you better. If you need something fast and pragmatic that scales to massive repos, `edlicense` is for you.

## Why edlicense over addlicense?

`edlicense` is inspired by Google's [addlicense](https://github.com/google/addlicense) tool but addresses several limitations:

| Feature | edlicense | addlicense |
|---------|-----------|------------|
| Implementation | Rust | Go |
| CLI interface | Modern long options (--option-name) | Short flags (-flag) |
| Default behavior | Dry run mode (non-destructive) | Modify mode |
| Automatic year updates | ✅ Updates copyright years automatically | ❌ No support for updating files w/ old years |
| Ratchet mode | ✅ Process only files changed since a git reference | ❌ Not available |
| Git integration | ✅ Option to only process git-tracked files | ❌ Not available |

Key advantages of `edlicense`:

1. **Safety First**: Defaults to dry run mode, preventing accidental file modifications
2. **Git Integration**:
   - Ratchet mode for CI/CD pipelines to process only changed files
   - Option to only process git-tracked files (default when in a git repository)
3. **Automatic Updates**: Intelligently updates copyright years without manual intervention

## Features

- Recursively scan directories and add license headers to source files
- Automatic detection of file types and appropriate comment formatting
- Dry run mode to verify license headers without modifying files (default behavior)
- Ignore patterns to exclude specific files or directories (via CLI or `.licenseignore` files)
- Support for `.licenseignore` files with gitignore-style pattern matching
- Global ignore file support via `GLOBAL_LICENSE_IGNORE` environment variable
- **Automatic year reference updates** - automatically updates copyright year references when the year changes (e.g., `(c) 2024` → `(c) 2025`)
- **Ratchet mode** - only check and format files that have changed relative to a git reference (e.g., `origin/main`)
- **Git repository integration** - option to only process files tracked by git (default when in a git repository)

## Installation

### From crates.io

```bash
cargo install edlicense
```

### From source

```bash
git clone https://github.com/eddieland/edlicense.git
cd edlicense
cargo install --path .
```

### Using Docker

edlicense is available as a Docker image, making it easy to run without installing Rust or any dependencies.

#### Using Pre-built Images from GitHub Container Registry

Pre-built Docker images are automatically published to GitHub Container Registry when code is pushed to the main branch.

```bash
# Pull the latest production image
docker pull ghcr.io/eddieland/edlicense:latest

# Pull the latest distroless image
docker pull ghcr.io/eddieland/edlicense:distroless-latest

# Run using the production image
docker run --rm -v "$(pwd):/workspace" -w /workspace ghcr.io/eddieland/edlicense:latest src/

# Run using the distroless image
docker run --rm -v "$(pwd):/workspace" -w /workspace ghcr.io/eddieland/edlicense:distroless-latest src/

# Run in modify mode
docker run --rm -v "$(pwd):/workspace" -w /workspace ghcr.io/eddieland/edlicense:latest --modify src/
```

Available image tags:
- `ghcr.io/eddieland/edlicense:latest` - Standard production image (latest version)
- `ghcr.io/eddieland/edlicense:distroless-latest` - Distroless image (latest version)
- `ghcr.io/eddieland/edlicense:<commit-hash>` - Production image at a specific commit
- `ghcr.io/eddieland/edlicense:distroless-<commit-hash>` - Distroless image at a specific commit

#### Building the Docker image locally

The project uses a single Dockerfile that can build production, distroless, and debug images:

```bash
# Build the lightweight production image (Debian-based)
make docker-build

# Build the minimal distroless image
make docker-build-distroless

# Build the debug/development image
make docker-build-debug

# Build all images
make docker-build-all
```

#### Running with locally built Docker images

```bash
# Run edlicense on files in the current directory (dry run mode)
docker run --rm -v "$(pwd):/workspace" -w /workspace edlicense:latest src/

# Using the make target (equivalent to above)
make docker-run ARGS="src/"

# Run with the distroless image (smaller and more secure)
make docker-run-distroless ARGS="src/"

# Run edlicense in modify mode
docker run --rm -v "$(pwd):/workspace" -w /workspace edlicense:latest --modify src/

# Run with the debug image for development purposes
make docker-run-debug ARGS="cargo nextest run"
```

The Docker setup provides three image tags from the same Dockerfile:

1. **Lightweight image** (`edlicense:latest`): A Debian-based image containing only the compiled binary and minimal dependencies, optimized for CI/CD pipelines and general production use.

2. **Distroless image** (`edlicense:distroless`): An ultra-minimal image based on Google's distroless container, containing only the compiled binary and essential libraries with no shell or package manager. This provides the smallest possible attack surface and image size, ideal for security-sensitive deployments.

3. **Debug image** (`edlicense:debug`): A development image containing the full Rust toolchain, source code, and development tools, useful for debugging and development.

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
--show-diff                   Show diff of changes in dry run mode
--save-diff <FILE>            Save diff of changes to a file in dry run mode
--license-file <LICENSE_FILE> Custom license file to use
--ignore <IGNORE>...          File patterns to ignore (supports glob patterns)
--year <YEAR>                 Copyright year [default: current year]
--verbose                     Verbose logging
--ratchet <REFERENCE>         Ratchet mode: only check and format files that have changed relative to a git reference
--preserve-years              Preserve existing years in license headers
--global-ignore-file <FILE>   Path to a global license ignore file (overrides GLOBAL_LICENSE_IGNORE environment variable)
--git-only                    Only consider files in the current git repository (default when in a git repository)
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

Show diff of changes in dry run mode:

```bash
edlicense --show-diff src/ tests/
```

Save diff of changes to a file in dry run mode:

```bash
edlicense --save-diff=changes.diff src/ tests/
```

Show and save diff of changes:

```bash
edlicense --show-diff --save-diff=changes.diff src/ tests/
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

Only process files tracked by git (this is the default when in a git repository):

```bash
edlicense --git-only src/
```

Process all files, including those not tracked by git:

```bash
edlicense --git-only=false src/
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

## Using .licenseignore Files

You can use `.licenseignore` files to specify patterns for files that should be ignored during license checking and updates, similar to how `.gitignore` files work:

```bash
# Create a .licenseignore file in your project
echo "*.json" > .licenseignore
echo "vendor/" >> .licenseignore
echo "**/node_modules/" >> .licenseignore

# Run edlicense (it will automatically use the .licenseignore file)
edlicense src/
```

You can also set a global ignore file using the `GLOBAL_LICENSE_IGNORE` environment variable:

```bash
export GLOBAL_LICENSE_IGNORE=/path/to/global/licenseignore
edlicense src/
```

For more details and examples, see [.licenseignore Files](examples/licenseignore.md).

## Git Repository Integration

By default, when running in a git repository, `edlicense` will only process files that are tracked by git. This helps ensure that only files that are part of your project get license headers, while ignoring build artifacts, temporary files, and other untracked files.

> **Important**: When git detection mode is enabled, `edlicense` uses your current working directory (`$CWD`) to determine whether it should only look at tracked files. You should always run edlicense from inside the git repository for correct operation.

You can control this behavior with the `--git-only` option:

```bash
# Only process files tracked by git (default when in a git repository)
edlicense --git-only src/

# Process all files, including those not tracked by git
edlicense --git-only=false src/
```

This feature works well with the ratchet mode, allowing you to focus only on files that are both tracked by git and have changed since a specific reference:

```bash
# Only process files that are tracked by git and have changed since origin/main
edlicense --git-only --ratchet "origin/main" src/
```

If you run `edlicense` from outside your git repository while using git detection mode, it will not be able to properly identify git-tracked files, which may result in no files being processed or incorrect files being processed.

For more details and examples, see [Git Integration](examples/git_integration.md).

For information on using edlicense in pre-commit hooks, see [Pre-commit Hooks](examples/pre_commit_hook.md).

## Supported File Types

`edlicense` supports a wide range of file types and automatically formats license headers with the appropriate comment style:

### File Extensions by Comment Type

| Comment Style | File Extensions and Types |
|---------------|---------------------------|
| Block comments<br>`/* ... */` | `.c`, `.h`, `.gv`, `.java`, `.scala`, `.kt`, `.kts` |
| JSDoc comments<br>`/** ... */` | `.js`, `.mjs`, `.cjs`, `.jsx`, `.tsx`, `.css`, `.scss`, `.sass`, `.ts` |
| Line comments<br>`// ...` | `.cc`, `.cpp`, `.cs`, `.go`, `.hcl`, `.hh`, `.hpp`, `.m`, `.mm`, `.proto`, `.rs`, `.swift`, `.dart`, `.groovy`, `.v`, `.sv` |
| Hash comments<br>`# ...` | `.py`, `.sh`, `.yaml`, `.yml`, `.rb`, `.tcl`, `.tf`, `.bzl`, `.pl`, `.pp`, `.toml` |
| Lisp comments<br>`;; ...` | `.el`, `.lisp` |
| Erlang comments<br>`% ...` | `.erl` |
| SQL/Haskell comments<br>`-- ...` | `.hs`, `.sql`, `.sdl` |
| HTML comments<br>`<!-- ... -->` | `.html`, `.xml`, `.vue`, `.wxi`, `.wxl`, `.wxs` |
| Jinja2 comments<br>`{# ... #}` | `.j2` |
| OCaml comments<br>`(** ... *)` | `.ml`, `.mli`, `.mll`, `.mly` |

### Special File Handling

In addition to extensions, `edlicense` also handles special file types by name:

- `cmakelists.txt`, `*.cmake.in`, `*.cmake`: Hash comments (`# ...`)
- `dockerfile`, `*.dockerfile`: Hash comments (`# ...`)

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
cargo nextest run --release -E 'test(test_add_license_performance)' --run-ignored=all --no-capture

# Run all performance tests
cargo nextest run --release --run-ignored=all --no-capture
```

### Performance Test Results

The performance tests measure:

1. **Adding licenses** to large numbers of files (10,000+)
2. **Updating years** in existing license headers
3. **Checking for licenses** in check-only mode
4. Impact of **file size** on processing time
5. Impact of **thread count** on parallel processing performance

Results are displayed in the console with timing information, making it easy to identify performance bottlenecks or improvements.
