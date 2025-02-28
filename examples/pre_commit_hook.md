# Using edlicense in Git Pre-commit Hooks

Pre-commit hooks are a powerful way to automatically check or enforce license headers before changes are committed to your repository. This guide demonstrates how to set up edlicense as part of your Git pre-commit workflow.

## Basic Pre-commit Hook Setup

Create a `.git/hooks/pre-commit` file in your repository and make it executable:

```bash
touch .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### Example: Pre-commit Hook in Dry Run Mode (Check Only)

This example creates a pre-commit hook that will check for license headers but not modify files. The commit will be blocked if any files are missing license headers:

```bash
#!/bin/bash
set -e

# Dry run mode - check for missing license headers without modifying files
echo "Checking for missing license headers..."

# Run edlicense on staged files only (using git diff to get the file list)
# This uses xargs to handle filenames with spaces correctly
git diff --cached --name-only --diff-filter=ACM | xargs -I{} \
  edlicense --dry-run --verbose --git-only "{}"

# If edlicense reports any missing headers, the script will exit with a non-zero
# status due to set -e, which will abort the commit
echo "All files have proper license headers."
```

### Example: Pre-commit Hook in Modify Mode (Auto-fix)

This example creates a pre-commit hook that will automatically add or update license headers in files before committing:

```bash
#!/bin/bash
set -e

# Modified files will be automatically re-staged
echo "Checking and adding license headers if needed..."

# Get list of staged files
FILES=$(git diff --cached --name-only --diff-filter=ACM)

if [ -n "$FILES" ]; then
  # Run edlicense in modify mode on staged files
  echo "$FILES" | xargs -I{} edlicense --modify --verbose --git-only "{}"
  
  # Re-stage the files that edlicense may have modified
  echo "$FILES" | xargs git add
  
  echo "License headers have been checked and fixed if needed."
fi
```

## Using with Pre-commit Framework

If you're using the [pre-commit framework](https://pre-commit.com/), you can add edlicense to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: edlicense
        name: Check license headers
        entry: edlicense
        language: system
        args: [--dry-run, --verbose]
        types: [file]  # You may want to limit this to specific file types
```

For auto-fixing with the pre-commit framework:

```yaml
repos:
  - repo: local
    hooks:
      - id: edlicense-fix
        name: Fix license headers
        entry: edlicense
        language: system
        args: [--modify, --verbose]
        types: [file]  # You may want to limit this to specific file types
```

## Docker Integration

If you prefer to use edlicense via Docker in your pre-commit hook, you can modify the scripts as follows:

### Dry Run Mode with Docker

```bash
#!/bin/bash
set -e

echo "Checking for missing license headers using Docker..."

# Get repository root directory for correct mounting
REPO_ROOT=$(git rev-parse --show-toplevel)

# Run edlicense in Docker on staged files
FILES=$(git diff --cached --name-only --diff-filter=ACM)

if [ -n "$FILES" ]; then
  for FILE in $FILES; do
    # Skip files that don't exist (might have been deleted)
    [ -f "$FILE" ] || continue
    
    # Use relative path from repo root for Docker volume mapping
    REL_PATH=$(realpath --relative-to="$REPO_ROOT" "$FILE")
    DIR_PATH=$(dirname "$REL_PATH")
    
    docker run --rm -v "$REPO_ROOT:/workspace" -w "/workspace" \
      edlicense:latest --dry-run --verbose "$REL_PATH"
  done
fi

echo "All files have proper license headers."
```

### Modify Mode with Docker

```bash
#!/bin/bash
set -e

echo "Checking and adding license headers if needed using Docker..."

# Get repository root directory for correct mounting
REPO_ROOT=$(git rev-parse --show-toplevel)

# Get list of staged files
FILES=$(git diff --cached --name-only --diff-filter=ACM)

if [ -n "$FILES" ]; then
  for FILE in $FILES; do
    # Skip files that don't exist (might have been deleted)
    [ -f "$FILE" ] || continue
    
    # Use relative path from repo root for Docker volume mapping
    REL_PATH=$(realpath --relative-to="$REPO_ROOT" "$FILE")
    
    # Run edlicense in Docker with modify mode
    docker run --rm -v "$REPO_ROOT:/workspace" -w "/workspace" \
      edlicense:latest --modify --verbose "$REL_PATH"
    
    # Re-stage the file that may have been modified
    git add "$FILE"
  done
  
  echo "License headers have been checked and fixed if needed."
fi
```

## Advanced Features in Pre-commit Hooks

### Using Ratchet Mode

You can enhance your pre-commit hook with ratchet mode to only check files that have changed since a specific reference:

```bash
#!/bin/bash
set -e

echo "Checking license headers in changed files only..."

# Use ratchet mode to check only files that have changed
edlicense --ratchet "origin/main" --git-only --dry-run --verbose

echo "All changed files have proper license headers."
```

### Custom License Template

If you use a custom license template, include it in your pre-commit hook:

```bash
#!/bin/bash
set -e

# Path to custom license template (relative to repository root)
LICENSE_TEMPLATE="path/to/license_template.txt"

echo "Checking license headers with custom template..."

# Run edlicense with custom license template
git diff --cached --name-only --diff-filter=ACM | xargs -I{} \
  edlicense --dry-run --verbose --license-file "$LICENSE_TEMPLATE" "{}"

echo "All files have proper license headers."
```

## Bypassing the Pre-commit Hook

Sometimes you may need to commit without checking license headers. You can bypass pre-commit hooks with:

```bash
git commit --no-verify -m "Commit message"
```

## Tips for Pre-commit Hooks

1. **Performance**: For large repositories, consider using the `--ratchet` option to only check files that have changed.

2. **Specific File Types**: Use the `--ignore` option to exclude file types that don't need license headers.

3. **Team Adoption**: Document the pre-commit hook in your project's README so that all team members know how to set it up.

4. **CI/CD Integration**: While pre-commit hooks are great for individual developers, also consider running license checks in your CI/CD pipeline to catch cases where developers may have bypassed the hook.

5. **Exit Codes**: Pre-commit hooks should exit with a non-zero status code if they detect an issue that should block the commit.