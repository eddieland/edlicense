# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build

# Test
cargo nextest run              # Run all tests (requires cargo-nextest)
cargo nextest run <test_name>  # Run a single test by name

# Lint and Format
cargo fmt --all                # Format code
cargo clippy --all-features -- -D warnings  # Run lints

# Install locally
cargo install --path .
```

## Before Committing

Always run formatting and linting before committing:

```bash
cargo fmt --all
cargo clippy --all-features -- -D warnings
```

## Architecture Overview

edlicense is a Rust CLI tool that ensures source code files have copyright license headers. Key design principles:

- **Safety-first defaults**: Dry-run mode by default (use `--modify` to actually change files)
- **Async file processing**: Uses tokio for async I/O with configurable concurrency
- **Git integration**: Can process only tracked files (`--git-only`) or changed files (`--ratchet`)

### Core Modules

- **`main.rs`**: CLI entry point using clap for argument parsing
- **`processor.rs`**: Central processing logic - handles file traversal, filtering, and license operations
- **`templates.rs`**: License template loading, rendering (supports `{{year}}`), and comment style formatting per file type
- **`license_detection.rs`**: Detects if files already have license headers
- **`file_filter.rs`**: Composite filter system for deciding which files to process
- **`ignore.rs`**: Handles `.licenseignore` files (gitignore-style pattern matching)
- **`git.rs`**: Git operations via git2 - tracked files list, changed files for ratchet mode
- **`workspace.rs`**: Resolves workspace root and determines if it's a git repository
- **`diff.rs`**: Generates unified diffs for `--show-diff` output
- **`report.rs`**: Generates HTML/JSON/CSV reports of license status

### Processing Flow

1. Patterns are resolved to files (respecting `.licenseignore`, `--git-only`, `--ratchet`)
2. Files are filtered by extension/type using the composite filter
3. Each file is checked for existing license headers
4. In modify mode: licenses are added or years are updated
5. In dry-run mode: missing licenses are reported, diffs shown if requested

### Output Conventions

- **User output**: All output intended for users goes to **stdout**. Keep stdout clean and predictable for scripting/piping.
- **Logging**: All internal/debug logging uses the **tracing** library. Logs go to stderr and are controlled by `RUST_LOG` or CLI verbosity flags. Never mix logging into stdout.

### Error Handling Philosophy

- **Fail fast**: For configuration errors (missing config file, invalid config, bad CLI args), fail immediately with a clear error message. Don't silently continue with defaults.
- **Explicit over implicit**: If the user specifies a config file or option, it must work or error—never silently ignore.

### Clippy Lints

The project enforces strict lints via `Cargo.toml` including: `unwrap_used`, `panic`, `todo`, `dbg_macro`. Use `expect()` with descriptive messages or proper error handling.

### Interactive Testing

To test edlicense interactively after building:

```bash
# Create a temp directory with test files
mkdir -p /tmp/edlicense-test && cd /tmp/edlicense-test

# Create a license template file
echo 'Copyright {{year}} Test' > license.txt

# Create test files with different copyright years
echo '// Copyright (c) 2020 Test' > outdated.rs    # Needs year update
echo '// Copyright (c) 2026 Test' > current.rs     # Year is current
echo '// No license' > missing.rs                   # Missing license

# Run check mode (dry-run, the default)
./target/release/edlicense -f license.txt /tmp/edlicense-test
```

**Important**: The year update detection regex requires specific copyright formats:

- `Copyright (c) YEAR` - matches
- `Copyright © YEAR` - matches
- `Copyright YEAR` - does NOT match for year updating (but is detected as having a license)

The summary output shows: `Summary: X OK, Y missing, Z outdated, W ignored`
