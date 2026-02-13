//! # Git Module
//!
//! This module contains functionality for interacting with git repositories,
//! such as identifying changed files relative to a reference and listing all
//! files tracked by git.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::{Delta, Repository, RepositoryState};
use tracing::{debug, trace, warn};

use crate::info_log;

/// Options controlling which changes are included in ratchet mode.
///
/// By default, ratchet mode includes staged and unstaged changes in addition
/// to committed changes. This is useful for local development workflows.
/// Use [`RatchetOptions::committed_only`] in CI environments to only check
/// committed changes.
#[derive(Debug, Clone)]
pub struct RatchetOptions {
  /// Include staged (index) changes in the ratchet diff.
  pub include_staged: bool,
  /// Include unstaged (working directory) changes in the ratchet diff.
  pub include_unstaged: bool,
}

impl Default for RatchetOptions {
  /// Returns options for local development that include staged and unstaged changes.
  ///
  /// This is the default behavior for `--ratchet` without `--ratchet-committed-only`.
  fn default() -> Self {
    Self {
      include_staged: true,
      include_unstaged: true,
    }
  }
}

impl RatchetOptions {
  /// Returns options for CI that only include committed changes.
  ///
  /// This is the behavior when `--ratchet-committed-only` is specified.
  pub const fn committed_only() -> Self {
    Self {
      include_staged: false,
      include_unstaged: false,
    }
  }
}

/// Returns a human-readable description of a non-clean repository state.
///
/// During operations like rebase, merge, cherry-pick, or bisect, the repository's
/// HEAD, index, and working directory may be in an intermediate state that makes
/// diff computations unreliable. This function translates the git2 state enum into
/// a user-facing message.
const fn describe_repository_state(state: RepositoryState) -> Option<&'static str> {
  match state {
    RepositoryState::Clean => None,
    RepositoryState::Merge => Some("merge"),
    RepositoryState::Revert | RepositoryState::RevertSequence => Some("revert"),
    RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Some("cherry-pick"),
    RepositoryState::Bisect => Some("bisect"),
    RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge => Some("rebase"),
    RepositoryState::ApplyMailbox | RepositoryState::ApplyMailboxOrRebase => Some("apply-mailbox"),
  }
}

/// Checks that the repository is in a clean state suitable for ratchet mode.
///
/// Ratchet mode computes diffs against a reference commit to find changed files.
/// During in-progress operations like rebase, merge, cherry-pick, or bisect, HEAD
/// and the index are in an intermediate state that can produce incorrect diffs —
/// leading to files being missed or spuriously included. In modify mode this could
/// write license headers into files that are about to be rewritten by the ongoing
/// operation.
///
/// Returns `Ok(())` if the state is clean, or an error with a descriptive message
/// explaining what operation is in progress and how to resolve it.
fn check_repo_state_for_ratchet(repo: &Repository) -> Result<()> {
  let state = repo.state();
  if let Some(operation) = describe_repository_state(state) {
    let hint = match operation {
      "bisect" => String::from(
        "Please finish the bisect before running with --ratchet:\n\
         - End bisect: git bisect reset",
      ),
      _ => format!(
        "Please complete or abort the {operation} before running with --ratchet:\n\
         - Complete: git {operation} --continue\n\
         - Abort:   git {operation} --abort",
      ),
    };
    return Err(anyhow::anyhow!(
      "Repository has a {operation} in progress. \
       Running ratchet mode during an in-progress git operation can produce \
       incorrect results because HEAD and the index are in an intermediate state.\n\n\
       {hint}"
    ));
  }
  Ok(())
}

/// Normalize a path by resolving `.` and `..` components without following symlinks.
///
/// This is needed to produce clean paths in diagnostic messages when a relative
/// gitdir reference (e.g. `../../.git/worktrees/foo`) is joined with a base
/// directory.
fn normalize_path(path: &Path) -> PathBuf {
  use std::path::Component;

  let mut components = Vec::new();
  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        if let Some(Component::Normal(_)) = components.last() {
          components.pop();
        } else {
          components.push(component);
        }
      }
      _ => components.push(component),
    }
  }

  components.iter().collect()
}

/// Diagnoses a broken git worktree reference.
///
/// In a git worktree, the `.git` entry is a **file** (not a directory) containing
/// a `gitdir:` reference to the main repository's `.git/worktrees/<name>` directory.
/// When a worktree is volume-mounted into a Docker container without also mounting
/// the main repository's `.git` directory, this reference points to a path that
/// doesn't exist inside the container, causing `Repository::discover()` to fail.
///
/// This function walks up from `start_dir` looking for a `.git` file with a
/// `gitdir:` reference. If the referenced path doesn't exist, it returns a
/// diagnostic message explaining the problem.
fn diagnose_broken_worktree_gitdir(start_dir: &Path) -> Option<String> {
  let mut dir = start_dir;
  loop {
    let git_entry = dir.join(".git");
    if git_entry.is_file() {
      // Read the .git file — it should contain "gitdir: <path>"
      let contents = std::fs::read_to_string(&git_entry).ok()?;
      let gitdir_ref = contents.trim().strip_prefix("gitdir: ")?;

      // Resolve the gitdir path relative to the .git file's parent directory.
      // Worktree .git files may use relative paths (e.g. "../../.git/worktrees/foo")
      // which must be resolved against the directory containing the .git file,
      // not the process CWD.
      let base = git_entry.parent().unwrap_or(dir);
      let gitdir_path = normalize_path(&base.join(gitdir_ref));

      if !gitdir_path.exists() {
        // Resolve the parent .git directory from the gitdir path.
        // The gitdir typically looks like:
        //   /path/to/repo/.git/worktrees/<name>
        // We want to show /path/to/repo/.git
        let git_dir = find_parent_git_dir(&gitdir_path);

        let msg = format!(
          "This appears to be a git worktree, but the worktree's gitdir \
           reference points to a path that doesn't exist:\n\
           \n\
           \x20 gitdir: {gitdir_ref}\n\
           \x20 resolved to: {}\n\
           \n\
           The main repository's .git directory is expected at:\n\
           \x20 {}\n\
           \n\
           Ensure the main repository's .git directory is accessible at that path. \
           This commonly happens when running inside a container where only the \
           worktree directory is mounted, but the main .git directory is not.",
          gitdir_path.display(),
          git_dir.display(),
        );

        return Some(msg);
      }

      // gitdir exists, no diagnostic needed
      return None;
    }

    // If there's a .git directory, this is a regular repo, not a broken worktree
    if git_entry.is_dir() {
      return None;
    }

    // Walk up to the parent directory
    dir = dir.parent()?;
  }
}

/// Given a gitdir path like `/repo/.git/worktrees/name`, extracts the
/// parent `.git` directory (`/repo/.git`). Falls back to the grandparent
/// of the input path if the expected `worktrees` structure isn't found.
fn find_parent_git_dir(gitdir_path: &Path) -> PathBuf {
  // Walk up looking for a component named ".git" — e.g.
  //   /repo/.git/worktrees/name  →  /repo/.git
  for ancestor in gitdir_path.ancestors() {
    if ancestor.file_name().is_some_and(|n| n == ".git") {
      return ancestor.to_path_buf();
    }
  }

  // Fallback: strip the last two components (worktrees/<name>)
  gitdir_path
    .parent()
    .and_then(|p| p.parent())
    .unwrap_or(gitdir_path)
    .to_path_buf()
}

/// Checks `.git/objects/info/alternates` for paths that don't exist on the filesystem.
///
/// When a git repository is volume-mounted into a Docker container, the alternates file
/// may contain host-absolute paths that are unreachable inside the container. This causes
/// libgit2 to fail with opaque "object not found" errors when resolving commits.
///
/// Returns a diagnostic message if broken alternates are detected, or `None` if the file
/// doesn't exist or all paths are valid.
fn diagnose_broken_alternates(repo: &Repository) -> Option<String> {
  let alternates_path = repo.path().join("objects").join("info").join("alternates");
  let contents = std::fs::read_to_string(&alternates_path).ok()?;

  let broken_paths: Vec<&str> = contents
    .lines()
    .filter(|line| !line.is_empty() && !line.starts_with('#'))
    .filter(|line| !Path::new(line).exists())
    .collect();

  if broken_paths.is_empty() {
    return None;
  }

  let mut msg = String::from(
    "Broken git alternates detected — .git/objects/info/alternates references \
     paths that don't exist on this filesystem:\n",
  );
  for path in &broken_paths {
    msg.push_str(&format!("  - {path}\n"));
  }
  msg.push_str(
    "\nThis commonly happens when a git workspace is volume-mounted into a Docker container \
     and the alternates file contains host-absolute paths.\n\n\
     To fix this, either:\n\
     1. Run 'git repack -ad' in the host workspace before mounting into Docker \
        (repacks objects locally so alternates are not needed)\n\
     2. Mount the alternates path into the container at the same absolute path",
  );

  Some(msg)
}

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
#[allow(dead_code)] // Used by integration tests
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
        info_log!("Git repository found but ownership check failed: {}", e.message());
        info_log!("Hint: Use --skip-git-owner-check to bypass this (common in Docker)");
      } else {
        debug!(
          "Git repository discovery failed for {}: {} (code: {:?})",
          start_dir.display(),
          e.message(),
          e.code()
        );

        // Check if this is a worktree with a broken gitdir reference
        // (common when volume-mounting worktrees into Docker without the
        // main .git directory).
        if let Some(diagnostic) = diagnose_broken_worktree_gitdir(start_dir) {
          return Err(anyhow::anyhow!(diagnostic));
        }
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
  debug!("Getting all files tracked by git");

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
      // Only include blobs (files), skip trees (directories) and commits (submodules)
      if entry.kind() != Some(git2::ObjectType::Blob) {
        return 0;
      }

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

  debug!("Found {} tracked files", tracked_files.len());

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
/// A `HashSet` of file paths that have changed since the merge base of the
/// commit and `HEAD` or an error if the git operations fail. The paths are
/// relative to the current working directory.
///
/// # Errors
///
/// Returns an error if:
/// - The git repository cannot be opened
/// - The specified commit cannot be found
/// - Git operations fail
#[allow(dead_code)] // Used by integration tests
pub fn get_changed_files(commit: &str) -> Result<HashSet<PathBuf>> {
  debug!("Getting changed files since commit: {}", commit);

  let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
  debug!("Current directory: {}", current_dir.display());

  get_changed_files_for_workspace(&current_dir, commit, &RatchetOptions::default())
}

/// Gets the list of files that have changed since a specific commit.
///
/// The returned paths are relative to the provided workspace root and are
/// computed from the merge base of the reference and `HEAD`.
///
/// When `options.include_staged` is true, staged (index) changes are also included.
/// When `options.include_unstaged` is true, unstaged (working directory) changes are also included.
///
/// In shallow or partial clones (common in CI), merge base computation may
/// fail because the full history is not available. In that case, this function
/// falls back to a direct diff between the reference commit and HEAD, which
/// may include extra files but will not miss any changed files.
pub fn get_changed_files_for_workspace(
  workspace_root: &Path,
  commit: &str,
  options: &RatchetOptions,
) -> Result<HashSet<PathBuf>> {
  debug!(
    "Getting changed files since commit: {} (include_staged: {}, include_unstaged: {})",
    commit, options.include_staged, options.include_unstaged
  );

  let repo = Repository::discover(workspace_root).with_context(|| "Failed to discover git repository")?;
  let workdir = repo
    .workdir()
    .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

  // Bail out early if the repository is in the middle of a rebase, merge,
  // cherry-pick, bisect, or similar operation.  The index and HEAD are in an
  // intermediate state that makes diff computation unreliable.
  check_repo_state_for_ratchet(&repo)?;

  // Get the commit object for the reference commit.
  // In shallow or partial clones the ref may not be reachable at all.
  let is_shallow = repo.is_shallow();
  let commit_obj = match repo.revparse_single(commit) {
    Ok(obj) => obj,
    Err(e) => {
      let hint = if is_shallow {
        format!(
          "Cannot resolve '{}' in this shallow clone: {}. \
           Try 'git fetch --unshallow' or use a full clone.",
          commit, e
        )
      } else if e.code() == git2::ErrorCode::NotFound {
        match diagnose_broken_alternates(&repo) {
          Some(diagnostic) => format!("Failed to find commit '{commit}': {e}\n\n{diagnostic}"),
          None => format!("Failed to find commit '{commit}': {e}"),
        }
      } else {
        format!("Failed to find commit '{commit}': {e}")
      };
      return Err(anyhow::anyhow!(hint));
    }
  };

  let ref_commit = commit_obj
    .as_commit()
    .ok_or_else(|| anyhow::anyhow!("Object is not a commit"))?;

  // Get the current HEAD commit
  let head = repo.head().with_context(|| "Failed to get HEAD reference")?;
  let head_commit = head.peel_to_commit().with_context(|| "Failed to get HEAD commit")?;

  // Try to find the merge base. This may fail in shallow/partial clones
  // because the common ancestor is outside the fetched history.  When it
  // fails, fall back to a direct diff between the reference and HEAD —
  // this may include extra files but won't miss any changed files.
  let base_tree = match repo.merge_base(ref_commit.id(), head_commit.id()) {
    Ok(merge_base) => {
      let base_commit = repo
        .find_commit(merge_base)
        .with_context(|| "Failed to load merge base commit for ratchet mode")?;
      debug!(
        "Comparing merge base {} with HEAD {} (reference: {})",
        base_commit.id(),
        head_commit.id(),
        ref_commit.id()
      );
      base_commit.tree().with_context(|| "Failed to get merge base tree")?
    }
    Err(e) => {
      warn!(
        "Merge base resolution failed ({}). \
         Falling back to direct diff between '{}' and HEAD. \
         This may report more changed files than expected.{}",
        e,
        commit,
        if is_shallow {
          " Try 'git fetch --unshallow' for accurate results."
        } else {
          ""
        }
      );
      debug!(
        "Falling back to direct diff: {} -> {}",
        ref_commit.id(),
        head_commit.id()
      );
      ref_commit
        .tree()
        .with_context(|| "Failed to get reference commit tree")?
    }
  };

  let head_tree = head_commit.tree().with_context(|| "Failed to get HEAD tree")?;

  // Start with committed changes
  let mut changed_files = collect_diff_files(&repo, workdir, workspace_root, &base_tree, &head_tree)?;

  // Add staged files if requested
  if options.include_staged {
    let staged = get_staged_files(&repo, workdir, workspace_root, &head_tree)?;
    debug!("Adding {} staged files to ratchet set", staged.len());
    changed_files.extend(staged);
  }

  // Add unstaged files if requested
  if options.include_unstaged {
    let unstaged = get_unstaged_files(&repo, workdir, workspace_root)?;
    debug!("Adding {} unstaged files to ratchet set", unstaged.len());
    changed_files.extend(unstaged);
  }

  Ok(changed_files)
}

/// Collects changed file paths from a tree-to-tree diff.
///
/// Returns paths relative to the provided workspace root.
/// Deleted files are excluded since they no longer exist on disk.
fn collect_diff_files(
  repo: &Repository,
  workdir: &Path,
  workspace_root: &Path,
  old_tree: &git2::Tree<'_>,
  new_tree: &git2::Tree<'_>,
) -> Result<HashSet<PathBuf>> {
  let mut diff_options = git2::DiffOptions::new();
  diff_options.include_untracked(false);
  diff_options.recurse_untracked_dirs(false);

  let diff = repo
    .diff_tree_to_tree(Some(old_tree), Some(new_tree), Some(&mut diff_options))
    .with_context(|| "Failed to diff trees")?;

  collect_diff_paths(&diff, workdir, workspace_root)
}

/// Collects file paths from a diff object, filtering out deleted files.
///
/// This is a shared helper used by tree-to-tree, tree-to-index, and index-to-workdir diffs.
/// Returns paths relative to the provided workspace root.
fn collect_diff_paths(diff: &git2::Diff<'_>, workdir: &Path, workspace_root: &Path) -> Result<HashSet<PathBuf>> {
  let mut changed_files = HashSet::new();

  // Fast path: if workspace_root equals workdir, we can skip path normalization
  let use_fast_path = workspace_root == workdir;

  diff
    .foreach(
      &mut |delta, _progress| {
        // Skip deleted files - they no longer exist on disk and would cause file access errors
        if delta.status() == Delta::Deleted {
          trace!("Skipping deleted file in git diff: {:?}", delta.old_file().path());
          return true;
        }

        if let Some(new_file) = delta.new_file().path() {
          trace!("Found changed file in git: {:?}", new_file);

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
            trace!("Added relative path: {}", rel_path.display());
            changed_files.insert(rel_path);
          }
        }
        true
      },
      None,
      None,
      None,
    )
    .with_context(|| "Failed to process diff")?;

  debug!("Found {} changed files", changed_files.len());
  for file in &changed_files {
    trace!("  Changed file: {}", file.display());
  }

  Ok(changed_files)
}

/// Gets files that are staged (in the index) but not yet committed.
///
/// Returns paths relative to the provided workspace root.
/// Deleted files are excluded since they no longer exist on disk.
fn get_staged_files(
  repo: &Repository,
  workdir: &Path,
  workspace_root: &Path,
  head_tree: &git2::Tree<'_>,
) -> Result<HashSet<PathBuf>> {
  debug!("Getting staged files");

  let mut diff_options = git2::DiffOptions::new();
  diff_options.include_untracked(false);

  let diff = repo
    .diff_tree_to_index(Some(head_tree), None, Some(&mut diff_options))
    .with_context(|| "Failed to diff tree to index")?;

  collect_diff_paths(&diff, workdir, workspace_root)
}

/// Gets files that have unstaged changes in the working directory.
///
/// Returns paths relative to the provided workspace root.
/// Deleted files are excluded since they no longer exist on disk.
fn get_unstaged_files(repo: &Repository, workdir: &Path, workspace_root: &Path) -> Result<HashSet<PathBuf>> {
  debug!("Getting unstaged files");

  let mut diff_options = git2::DiffOptions::new();
  diff_options.include_untracked(false);

  let diff = repo
    .diff_index_to_workdir(None, Some(&mut diff_options))
    .with_context(|| "Failed to diff index to workdir")?;

  collect_diff_paths(&diff, workdir, workspace_root)
}
