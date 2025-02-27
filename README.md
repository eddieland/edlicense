# edlicense

A tool that ensures source code files have copyright license headers by scanning directory patterns recursively.

`edlicense` modifies source files in place and avoids adding a license header to any file that already has one. It follows the Unix philosophy of tooling where possible and is designed with modern Rust best practices for high-performance CLI tools.

## Features

- Recursively scan directories and add license headers to source files
- Support for multiple license types (Apache-2.0, MIT, BSD, MPL-2.0)
- Automatic detection of file types and appropriate comment formatting
- Check-only mode to verify license headers without modifying files
- Ignore patterns to exclude specific files or directories
- SPDX identifier support
- **Automatic year reference updates** - automatically updates copyright year references when the year changes (e.g., `(c) 2024` → `(c) 2025`)

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

## Usage

```
edlicense [OPTIONS] <PATTERNS>...
```

### Arguments

- `<PATTERNS>...` - File or directory patterns to process. Directories are processed recursively.

### Options

```
--check                       Check only mode: verify presence of license headers and exit with non-zero code if missing
--license-file <LICENSE_FILE> Custom license file to use
--ignore <IGNORE>...          File patterns to ignore (supports glob patterns)
--year <YEAR>                 Copyright year [default: current year]
--verbose                     Verbose logging
--help                        Print help
--version                     Print version
```

## Examples

Check if all files have license headers without modifying them:

```bash
edlicense --check src/ tests/
```

Ignore specific file patterns:

```bash
edlicense --ignore "**/*.json" --ignore "vendor/**" .
```

Use a specific year:

```bash
edlicense --year "2020" .
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

## Supported File Types

`edlicense` supports a wide range of file types and automatically formats license headers with the appropriate comment style:

- C/C++/C#/Go/Rust/Swift/Dart: `// comment style`
- Java/Scala/Kotlin: `/* comment style */`
- JavaScript/TypeScript/CSS: `/** comment style */`
- Python/Shell/YAML/Ruby: `# comment style`
- HTML/XML/Vue: `<!-- comment style -->`
- And many more...
