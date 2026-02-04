#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// Checks if git is available on the system.
pub fn is_git_available() -> bool {
  Command::new("git").arg("--version").status().is_ok()
}

/// Runs a git command in the given directory, returning an error with stderr on
/// failure.
pub fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
  let output = Command::new("git")
    .args(args)
    .current_dir(dir)
    .output()
    .with_context(|| format!("Failed to execute git {:?}", args))?;

  if !output.status.success() {
    anyhow::bail!("git {:?} failed: {}", args, String::from_utf8_lossy(&output.stderr));
  }
  Ok(())
}

/// Initializes a git repository in the given directory with deterministic
/// settings.
///
/// Configures:
/// - Default branch name set to `main`
/// - User name and email for commits
/// - Disables commit signing for test isolation
pub fn init_git_repo(dir: &Path) -> Result<()> {
  run_git(dir, &["init"])?;
  run_git(dir, &["config", "init.defaultBranch", "main"])?;
  run_git(dir, &["branch", "-M", "main"])?;
  run_git(dir, &["config", "user.name", "Test User"])?;
  run_git(dir, &["config", "user.email", "test@example.com"])?;
  // Disable commit signing for test isolation
  run_git(dir, &["config", "commit.gpgsign", "false"])?;
  Ok(())
}

/// Creates a commit with all staged changes.
pub fn git_commit(dir: &Path, message: &str) -> Result<()> {
  run_git(dir, &["commit", "-m", message])
}

/// Stages a file and creates a commit.
pub fn git_add_and_commit(dir: &Path, file: &str, message: &str) -> Result<()> {
  run_git(dir, &["add", file])?;
  git_commit(dir, message)
}
