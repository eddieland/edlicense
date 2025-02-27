//! # Git Module
//!
//! This module contains functionality for interacting with git repositories,
//! such as identifying changed files relative to a reference.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::{Repository, StatusOptions};

use crate::verbose_log;

/// Gets the list of files that have changed relative to a git reference.
///
/// This function is used in ratchet mode to identify files that have been added,
/// modified, or renamed since the specified git reference.
///
/// # Parameters
///
/// * `reference` - Git reference (branch, tag, or commit hash)
///
/// # Returns
///
/// A `HashSet` of file paths that have changed or an error if the git operations fail.
///
/// # Errors
///
/// Returns an error if:
/// - The git repository cannot be opened
/// - The specified reference cannot be found
/// - Git operations fail
pub fn get_changed_files(reference: &str) -> Result<HashSet<PathBuf>> {
    verbose_log!("Getting changed files relative to: {}", reference);

    // Open the git repository
    let repo = Repository::open(".").with_context(|| "Failed to open git repository")?;

    // Get the reference commit
    let reference_obj = repo
        .revparse_single(reference)
        .with_context(|| format!("Failed to find git reference: {}", reference))?;

    let reference_commit = reference_obj
        .peel_to_commit()
        .with_context(|| format!("Failed to get commit for reference: {}", reference))?;

    // Create a diff between the reference commit and the working directory
    let reference_tree = reference_commit
        .tree()
        .with_context(|| "Failed to get tree for reference commit")?;

    let mut changed_files = HashSet::new();

    // Get the status of files in the working directory
    let mut status_opts = StatusOptions::new();
    status_opts.include_untracked(true);

    let statuses = repo
        .statuses(Some(&mut status_opts))
        .with_context(|| "Failed to get git status")?;

    // Add all changed files to the set
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            let status = entry.status();

            // Check if the file is modified, added, or untracked
            if status.is_wt_modified()
                || status.is_wt_new()
                || status.is_wt_renamed()
                || status.is_index_modified()
                || status.is_index_new()
                || status.is_index_renamed()
            {
                verbose_log!("Changed file: {}", path);
                changed_files.insert(PathBuf::from(path));
            }
        }
    }

    // Also check for files that have been modified between the reference and HEAD
    let head_obj = repo.head().with_context(|| "Failed to get HEAD reference")?;

    let head_commit = head_obj.peel_to_commit().with_context(|| "Failed to get HEAD commit")?;

    let head_tree = head_commit
        .tree()
        .with_context(|| "Failed to get tree for HEAD commit")?;

    let diff = repo
        .diff_tree_to_tree(Some(&reference_tree), Some(&head_tree), None)
        .with_context(|| "Failed to create diff between reference and HEAD")?;

    diff.foreach(
        &mut |delta, _| {
            if let Some(new_file) = delta.new_file().path() {
                verbose_log!("Changed file (in diff): {:?}", new_file);
                changed_files.insert(PathBuf::from(new_file));
            }
            true
        },
        None,
        None,
        None,
    )
    .with_context(|| "Failed to process diff")?;

    verbose_log!("Found {} changed files", changed_files.len());

    Ok(changed_files)
}
