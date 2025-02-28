//! # Git Module
//!
//! This module contains functionality for interacting with git repositories,
//! such as identifying changed files relative to a reference and listing all files
//! tracked by git.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::Repository;

use crate::verbose_log;

/// Checks if the current directory is inside a git repository.
///
/// This function uses the current working directory (`$CWD`) to determine if
/// we are inside a git repository. It's important that edlicense is run from
/// within the git repository when git detection mode is enabled.
///
/// # Returns
///
/// `true` if the current directory is inside a git repository, `false` otherwise.
pub fn is_git_repository() -> bool {
    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(_) => return false,
    };

    match Repository::discover(&current_dir) {
        Ok(repo) => repo.workdir().is_some(),
        Err(_) => false,
    }
}

/// Gets all files tracked by git in the current repository.
///
/// This function is used to limit processing to only files that are tracked by git.
/// It works correctly even when called from a subdirectory of the git repository.
/// The function uses your current working directory (`$CWD`) to determine the git repository
/// and which files are tracked. You should always run edlicense from inside the git repository
/// when git detection mode is enabled.
///
/// # Returns
///
/// A `HashSet` of file paths that are tracked by git or an error if the git operations fail.
/// The paths are relative to the current working directory.
///
/// # Errors
///
/// Returns an error if:
/// - The git repository cannot be opened
/// - Git operations fail
pub fn get_git_tracked_files() -> Result<HashSet<PathBuf>> {
    verbose_log!("Getting all files tracked by git");

    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;

    let repo = Repository::discover(&current_dir).with_context(|| "Failed to discover git repository")?;

    let mut tracked_files = HashSet::new();

    // Get HEAD commit
    let head = repo.head().with_context(|| "Failed to get HEAD reference")?;

    let tree = head.peel_to_tree().with_context(|| "Failed to get HEAD tree")?;

    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if let Some(name) = entry.name() {
            // Skip . and .. entries
            if name != "." && name != ".." {
                let repo_relative_path = if root.is_empty() {
                    PathBuf::from(name)
                } else {
                    PathBuf::from(root).join(name)
                };

                // Convert the repository-relative path to an absolute path
                if let Some(workdir) = repo.workdir() {
                    let abs_path = workdir.join(&repo_relative_path);
                    // Get path relative to current directory
                    if let Ok(rel_path) = abs_path.strip_prefix(&current_dir) {
                        tracked_files.insert(rel_path.to_path_buf());
                    } else if let Some(rel_path) = pathdiff::diff_paths(&abs_path, &current_dir) {
                        tracked_files.insert(rel_path);
                    }
                }
            }
        }
        0
    })
    .with_context(|| "Failed to walk tree")?;

    verbose_log!("Found {} tracked files", tracked_files.len());

    Ok(tracked_files)
}

/// Gets the list of files that have changed in a specific commit.
///
/// # Parameters
///
/// * `commit` - Git commit hash
///
/// # Returns
///
/// A `HashSet` of file paths that were changed in the commit or an error if the git operations fail.
/// The paths are relative to the current working directory.
///
/// # Errors
///
/// Returns an error if:
/// - The git repository cannot be opened
/// - The specified commit cannot be found
/// - Git operations fail
pub fn get_changed_files(commit: &str) -> Result<HashSet<PathBuf>> {
    verbose_log!("Getting changed files for commit: {}", commit);

    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;

    let repo = Repository::discover(&current_dir).with_context(|| "Failed to discover git repository")?;

    let commit_obj = repo
        .revparse_single(commit)
        .with_context(|| format!("Failed to find commit: {}", commit))?;

    let commit = commit_obj
        .as_commit()
        .ok_or_else(|| anyhow::anyhow!("Object is not a commit"))?;

    let tree = commit.tree().with_context(|| "Failed to get commit tree")?;

    let mut diff_options = git2::DiffOptions::new();

    // Get parent commit if it exists
    let parent_tree = if let Ok(parent) = commit.parent(0) {
        Some(parent.tree()?)
    } else {
        None
    };

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_options))
        .with_context(|| "Failed to diff trees")?;

    let mut changed_files = HashSet::new();

    diff.foreach(
        &mut |_delta, _| true,
        None,
        None,
        Some(&mut |diff_delta, _progress, _path| {
            if let Some(new_file) = diff_delta.new_file().path() {
                let abs_path = repo
                    .workdir()
                    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))
                    .ok()
                    .map(|workdir| workdir.join(new_file));

                if let Some(abs_path) = abs_path {
                    if let Some(rel_path) = pathdiff::diff_paths(&abs_path, &current_dir) {
                        changed_files.insert(rel_path);
                    }
                }
            }
            true
        }),
    )
    .with_context(|| "Failed to process diff")?;

    verbose_log!("Found {} changed files", changed_files.len());

    Ok(changed_files)
}
