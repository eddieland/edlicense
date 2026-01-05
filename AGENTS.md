# AGENTS.md

This file provides guidance to agents working with code in this repository.

## Build and Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo nextest run        # Run tests (uses nextest)
cargo nextest run -E 'test(test_name)'  # Run single test

# Lint and Format
cargo fmt --all          # Format code
cargo clippy -- -D warnings  # Lint

# Install
cargo install --path .   # Install locally
```

### Performance Testing

Performance tests are ignored by default. Run with:

```bash
make perf-test-add       # Test adding licenses to 10k files
make perf-test-update    # Test updating license years
make perf-test-check     # Test checking license headers
make perf-benchmark      # Comprehensive benchmarks
```

### Docker

```bash
make docker-build             # Build production image
make docker-build-distroless  # Build minimal distroless image
make docker-run ARGS="src/"   # Run in container
```

## Architecture Overview

This is a high-performance Rust CLI tool that adds/checks license headers in source files.

### Core Flow

1. **CLI (`main.rs`)**: Parses args with clap, sets up `Processor` with `TemplateManager` and `DiffManager`
2. **Processor (`processor.rs`)**: Central orchestrator - processes patterns/directories, delegates to filters, manages concurrent file processing with tokio
3. **File Filtering (`file_filter.rs`)**: `CompositeFilter` chains multiple `FileFilter` trait implementations:
   - `IgnoreFilter`: Glob pattern matching via `IgnoreManager`
   - `GitFilter`: Only git-tracked files
   - `RatchetFilter`: Only files changed since a git ref
4. **Template Rendering (`templates.rs`)**: `TemplateManager` loads license templates, renders `{{year}}` variable, formats with `CommentStyle` per file extension
5. **License Detection (`license_detection.rs`)**: `LicenseDetector` trait with `SimpleLicenseDetector` implementation checks for existing headers

### Key Design Patterns

- **Async processing**: Uses tokio for async file I/O, `buffer_unordered` for concurrent file processing
- **Filter composition**: `CompositeFilter` allows chaining arbitrary `FileFilter` implementations
- **Trait-based detection**: `LicenseDetector` trait allows custom detection strategies via dependency injection
- **Channel-based reporting**: `mpsc` channels collect `FileReport` data to avoid mutex contention during parallel processing

### File Type Support

Comment styles are determined by file extension in `templates.rs:get_comment_style_for_file()`. Supports:

- Block comments (`/* */`): C, Java, Scala, Kotlin
- JSDoc comments (`/** */`): JS, TS, CSS
- Line comments (`//`): Rust, Go, C++, Swift
- Hash comments (`#`): Python, Shell, YAML, Ruby, TOML
- HTML comments (`<!-- -->`): HTML, XML, Vue

### Git Integration

- `git.rs`: Interfaces with git via `git2` crate
- `get_git_tracked_files()`: Returns set of tracked files for `--git-only` mode
- `get_changed_files(ref)`: Returns files changed since a ref for `--ratchet` mode
