# Git Repository Integration

`edlicense` provides powerful integration with git repositories, making it easy to manage license headers in your project.

## Only Processing Git-Tracked Files

By default, when running in a git repository, `edlicense` will only process files that are tracked by git. This helps ensure that only files that are part of your project get license headers, while ignoring build artifacts, temporary files, and other untracked files.

The git integration works correctly regardless of which directory you run `edlicense` from within your repository. Whether you're at the root of the repository or in a deeply nested subdirectory, `edlicense` will correctly identify the git repository and process only the tracked files.

### Default Behavior

When you run `edlicense` in a git repository, it automatically detects this and only processes files that are tracked by git:

```bash
# Only process files tracked by git (default when in a git repository)
edlicense src/
```

### Explicit Control

You can explicitly control this behavior with the `--git-only` option:

```bash
# Only process files tracked by git
edlicense --git-only src/

# Process all files, including those not tracked by git
edlicense --git-only=false src/
```

## Combining with Ratchet Mode

The git-only feature works well with ratchet mode, allowing you to focus only on files that are both tracked by git and have changed since a specific reference:

```bash
# Only process files that are tracked by git and have changed since origin/main
edlicense --git-only --ratchet "origin/main" src/
```

This combination is particularly useful in CI/CD pipelines, where you want to ensure that:

1. Only files that are part of your project (tracked by git) are processed
2. Only files that have changed in the current PR or branch are checked

## Benefits in CI/CD Pipelines

Using the git integration features in CI/CD pipelines offers several advantages:

1. **Efficiency**: Only process relevant files, reducing processing time
2. **Accuracy**: Avoid adding license headers to temporary or generated files
3. **Focus**: Only check files that have actually changed, reducing noise in PR reviews

## Example CI/CD Configuration

Here's an example GitHub Actions workflow that uses `edlicense` with git integration:

```yaml
name: License Check

on:
  pull_request:
    branches: [ main ]

jobs:
  check-licenses:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # Needed for git history

      - name: Install edlicense
        run: cargo install edlicense

      - name: Check license headers
        run: |
          edlicense --ratchet "origin/main" --show-diff src/
        # Note: git-only is enabled by default since we're in a git repository
```

## Handling Non-Git Repositories

When running `edlicense` outside of a git repository:

1. If `--git-only` is explicitly set to `true`, no files will be processed (since there are no git-tracked files)
2. If `--git-only` is not specified or set to `false`, all files will be processed normally

This ensures that `edlicense` behaves predictably in all environments, whether you're working in a git repository or not.