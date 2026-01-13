//! # Git Module
//!
//! This module contains functionality for interacting with git repositories,
//! such as identifying changed files relative to a reference and listing all
//! files tracked by git.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::Repository;

use crate::{info_log, verbose_log};

/// Checks if the current directory is inside a git repository.
///
/// This function uses the current working directory (`$CWD`) to determine if
/// we are inside a git repository. It's important that edlicense is run from
/// within the git repository when git detection mode is enabled.
///
/// # Returns
///
/// `true` if the current directory is inside a git repository, `false`
/// otherwise.
#[allow(dead_code)]
pub fn is_git_repository() -> bool {
  let current_dir = match std::env::current_dir() {
    Ok(dir) => dir,
    Err(_) => return false,
  };

  matches!(discover_repo_root(&current_dir), Ok(Some(_)))
}

/// Discover the root of a git repository starting from the given directory.
pub fn discover_repo_root(start_dir: &Path) -> Result<Option<PathBuf>> {
  match Repository::discover(start_dir) {
    Ok(repo) => Ok(repo.workdir().map(|root| root.to_path_buf())),
    Err(e) => {
      // Check if this is an ownership error (common in Docker/containers)
      if e.code() == git2::ErrorCode::Owner {
        info_log!(
          "Git repository found but ownership check failed: {}",
          e.message()
        );
        info_log!("Hint: Use --skip-git-owner-check to bypass this (common in Docker)");
      } else {
        verbose_log!(
          "Git repository discovery failed for {}: {} (code: {:?})",
          start_dir.display(),
          e.message(),
          e.code()
        );
      }
      Ok(None)
    }
  }
}

/// Gets all files tracked by git in the current repository.
///
/// This function is used to limit processing to only files that are tracked by
/// git. It discovers the git repository from the provided workspace root and
/// returns paths relative to that root.
///
/// # Returns
///
/// A `HashSet` of file paths that are tracked by git or an error if the git
/// operations fail. The paths are relative to the workspace root.
///
/// # Errors
///
/// Returns an error if:
/// - The git repository cannot be opened
/// - Git operations fail
pub fn get_git_tracked_files(workspace_root: &Path) -> Result<HashSet<PathBuf>> {
  verbose_log!("Getting all files tracked by git");

  let repo = Repository::discover(workspace_root).with_context(|| "Failed to discover git repository")?;
  let workdir = repo
    .workdir()
    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

  let mut tracked_files = HashSet::new();

  // Get HEAD commit
  let head = repo.head().with_context(|| "Failed to get HEAD reference")?;

  let tree = head.peel_to_tree().with_context(|| "Failed to get HEAD tree")?;

  // Fast path: if workspace_root equals workdir, we can skip path normalization
  // entirely This is the common case when running edlicense from the repository
  // root
  let use_fast_path = workspace_root == workdir;

  tree
    .walk(git2::TreeWalkMode::PreOrder, |root, entry| {
      if let Some(name) = entry.name() {
        // Skip . and .. entries
        if name != "." && name != ".." {
          let repo_relative_path = if root.is_empty() {
            PathBuf::from(name)
          } else {
            PathBuf::from(root).join(name)
          };

          if use_fast_path {
            // Fast path: repo-relative path is already workspace-relative
            tracked_files.insert(repo_relative_path);
          } else {
            // Slow path: need to convert through absolute path
            let abs_path = workdir.join(&repo_relative_path);
            let rel_path = abs_path
              .strip_prefix(workspace_root)
              .ok()
              .map(|path| path.to_path_buf())
              .or_else(|| pathdiff::diff_paths(&abs_path, workspace_root))
              .unwrap_or_else(|| repo_relative_path.clone());
            tracked_files.insert(rel_path);
          }
        }
      }
      0
    })
    .with_context(|| "Failed to walk tree")?;

  verbose_log!("Found {} tracked files", tracked_files.len());

  Ok(tracked_files)
}

/// Gets the list of files that have changed since a specific commit.
///
/// # Parameters
///
/// * `commit` - Git commit hash to compare against
///
/// # Returns
///
/// A `HashSet` of file paths that have changed since the commit or an error if
/// the git operations fail. The paths are relative to the current working
/// directory.
///
/// # Errors
///
/// Returns an error if:
/// - The git repository cannot be opened
/// - The specified commit cannot be found
/// - Git operations fail
#[allow(dead_code)]
pub fn get_changed_files(commit: &str) -> Result<HashSet<PathBuf>> {
  verbose_log!("Getting changed files since commit: {}", commit);

  let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
  verbose_log!("Current directory: {}", current_dir.display());

  get_changed_files_for_workspace(&current_dir, commit)
}

/// Gets the list of files that have changed since a specific commit.
///
/// The returned paths are relative to the provided workspace root.
pub fn get_changed_files_for_workspace(workspace_root: &Path, commit: &str) -> Result<HashSet<PathBuf>> {
  verbose_log!("Getting changed files since commit: {}", commit);

  let repo = Repository::discover(workspace_root).with_context(|| "Failed to discover git repository")?;
  let workdir = repo
    .workdir()
    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

  // Get the commit object for the reference commit
  let commit_obj = repo
    .revparse_single(commit)
    .with_context(|| format!("Failed to find commit: {}", commit))?;

  let ref_commit = commit_obj
    .as_commit()
    .ok_or_else(|| anyhow::anyhow!("Object is not a commit"))?;

  // Get the current HEAD commit
  let head = repo.head().with_context(|| "Failed to get HEAD reference")?;
  let head_commit = head.peel_to_commit().with_context(|| "Failed to get HEAD commit")?;

  verbose_log!(
    "Comparing {} with HEAD {}",
    ref_commit.id().to_string(),
    head_commit.id().to_string()
  );

  // Get trees for both commits
  let ref_tree = ref_commit
    .tree()
    .with_context(|| "Failed to get reference commit tree")?;
  let head_tree = head_commit.tree().with_context(|| "Failed to get HEAD tree")?;

  // Set up diff options
  let mut diff_options = git2::DiffOptions::new();
  diff_options.include_untracked(false);
  diff_options.recurse_untracked_dirs(false);

  // Diff between the reference commit and HEAD
  let diff = repo
    .diff_tree_to_tree(Some(&ref_tree), Some(&head_tree), Some(&mut diff_options))
    .with_context(|| "Failed to diff trees")?;

  let mut changed_files = HashSet::new();

  // Fast path: if workspace_root equals workdir, we can skip path normalization
  let use_fast_path = workspace_root == workdir;

  diff
    .foreach(
      &mut |_delta, _| true,
      None,
      None,
      Some(&mut |diff_delta, _progress, _path| {
        if let Some(new_file) = diff_delta.new_file().path() {
          verbose_log!("Found changed file in git: {:?}", new_file);

          if use_fast_path {
            // Fast path: git path is already workspace-relative
            changed_files.insert(new_file.to_path_buf());
          } else {
            // Slow path: need to convert through absolute path
            let abs_path = workdir.join(new_file);
            let rel_path = abs_path
              .strip_prefix(workspace_root)
              .ok()
              .map(|path| path.to_path_buf())
              .or_else(|| pathdiff::diff_paths(&abs_path, workspace_root))
              .unwrap_or_else(|| new_file.to_path_buf());
            verbose_log!("Added relative path: {}", rel_path.display());
            changed_files.insert(rel_path);
          }
        }
        true
      }),
    )
    .with_context(|| "Failed to process diff")?;

  verbose_log!("Found {} changed files", changed_files.len());
  for file in &changed_files {
    verbose_log!("  Changed file: {}", file.display());
  }

  Ok(changed_files)
}
