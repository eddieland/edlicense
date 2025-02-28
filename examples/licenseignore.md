# Using .licenseignore Files

The `.licenseignore` file feature allows you to specify patterns for files that should be ignored during license checking and updates, similar to how `.gitignore` files work.

## Basic Usage

Create a `.licenseignore` file in your project directory:

```
# This is a comment
*.json
*.md
vendor/
**/node_modules/
```

This will ignore:

- All JSON files
- All Markdown files
- The vendor directory
- Any node_modules directory at any level

## Pattern Format

`.licenseignore` files use the same pattern format as `.gitignore` files:

- Blank lines or lines starting with `#` are ignored (comments)
- Standard glob patterns work:
  - `*` matches any number of characters except `/`
  - `?` matches a single character except `/`
  - `**` matches any number of directories
  - `[abc]` matches any character inside the brackets
  - `[a-z]` matches any character in the range
- Patterns ending with `/` match directories only
- Patterns starting with `/` match from the directory containing the `.licenseignore` file
- Patterns starting with `!` negate a previous pattern (include a previously excluded file)

## Hierarchical Matching

`.licenseignore` files are hierarchical, similar to `.gitignore` files:

- A `.licenseignore` file in a subdirectory overrides patterns from parent directories for files in that subdirectory
- The tool will look for `.licenseignore` files in each directory from the current directory up to the root

## Global Ignore File

You can specify a global ignore file that applies to all projects by setting the `GLOBAL_LICENSE_IGNORE` environment variable:

```bash
# Linux/macOS
export GLOBAL_LICENSE_IGNORE=/path/to/global/licenseignore

# Windows
set GLOBAL_LICENSE_IGNORE=C:\path\to\global\licenseignore
```

Alternatively, you can use the `--global-ignore-file` command-line option:

```bash
edlicense --global-ignore-file /path/to/global/licenseignore src/
```

## Example Configurations

### For a Typical Web Project

```
# Build artifacts
dist/
build/
*.min.js

# Dependencies
node_modules/
vendor/

# Configuration files
*.config.js
*.json
.env*

# Documentation
*.md
docs/
```

### For a Rust Project

```
# Generated files
target/
Cargo.lock

# Documentation
*.md
docs/

# Configuration
*.toml
.cargo/
```

### For a Docker Project

```
# Docker files (often have their own licenses)
Dockerfile*
*.dockerfile
.dockerignore

# Configuration
*.yaml
*.yml
*.json
```

## Combining with CLI Ignore Patterns

`.licenseignore` files work in addition to the `--ignore` command-line option:

```bash
edlicense --ignore "**/*.go" --ignore "tests/**" src/
```

This will ignore all Go files and all files in the tests directory, in addition to any patterns specified in `.licenseignore` files.
