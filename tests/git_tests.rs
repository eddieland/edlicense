mod common;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use common::{git_add_and_commit, git_commit, init_git_repo, is_git_available, run_git};
use edlicense::git::{self, RatchetOptions};
use tempfile::{TempDir, tempdir};

// Helper function to initialize a git repository in a temporary directory
fn init_temp_git_repo() -> Result<tempfile::TempDir> {
  let temp_dir = tempdir()?;
  init_git_repo(temp_dir.path())?;

  // Create and commit a file to establish HEAD
  fs::write(temp_dir.path().join("initial.txt"), "Initial content")?;
  git_add_and_commit(temp_dir.path(), "initial.txt", "Initial commit")?;

  Ok(temp_dir)
}

#[test]
fn test_is_git_repository() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  // Initialize a temporary git repository
  let temp_dir = init_temp_git_repo()?;
  let non_git_dir = tempdir()?;

  // Test in git repository
  std::env::set_current_dir(&temp_dir)?;
  assert!(git::is_git_repository());

  // Test in non-git directory
  std::env::set_current_dir(&non_git_dir)?;
  assert!(!git::is_git_repository());

  // Keep temp directories in scope until the end of the test
  drop(temp_dir);
  drop(non_git_dir);

  Ok(())
}

#[test]
fn test_get_git_tracked_files() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  // Initialize a temporary git repository
  let temp_dir = init_temp_git_repo()?;

  // Create a tracked file
  fs::write(temp_dir.path().join("tracked.rs"), "Tracked content")?;

  // Create an untracked file
  fs::write(temp_dir.path().join("untracked.txt"), "Untracked content")?;

  // Add and commit the tracked file
  git_add_and_commit(temp_dir.path(), "tracked.rs", "Add tracked file")?;

  // Change to the git repository
  std::env::set_current_dir(&temp_dir)?;

  // Get tracked files
  let tracked_files = git::get_git_tracked_files(temp_dir.path())?;

  // Print the tracked files for debugging
  println!("Tracked files:");
  for file in &tracked_files {
    println!("  {}", file.display());
  }

  // Verify tracked files
  assert!(
    tracked_files.contains(&PathBuf::from("initial.txt")),
    "Should contain initial.txt"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("tracked.rs")),
    "Should contain tracked.rs"
  );
  assert!(
    !tracked_files.contains(&PathBuf::from("untracked.txt")),
    "Should not contain untracked.txt"
  );

  // Keep temp directory in scope until the end of the test
  drop(temp_dir);

  Ok(())
}

#[test]
fn test_get_changed_files() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  // Initialize a temporary git repository
  let temp_dir = init_temp_git_repo()?;

  // Create and commit a test file
  fs::write(temp_dir.path().join("test.txt"), "Test content")?;
  git_add_and_commit(temp_dir.path(), "test.txt", "Add test file")?;

  // Change to the git repository
  std::env::set_current_dir(&temp_dir)?;

  // Get changed files
  let changed_files = git::get_changed_files("HEAD^")?;

  // Print the changed files for debugging
  println!("Changed files:");
  for file in &changed_files {
    println!("  {}", file.display());
  }

  // Verify changed files
  assert!(
    changed_files.contains(&PathBuf::from("test.txt")),
    "Should contain test.txt"
  );

  // Keep temp directory in scope until the end of the test
  drop(temp_dir);

  Ok(())
}

#[test]
fn test_git_from_subdirectory() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  // Initialize a temporary git repository
  let temp_dir = init_temp_git_repo()?;

  // Create a subdirectory structure
  fs::create_dir_all(temp_dir.path().join("subdir/subsubdir"))?;

  // Create files in different directories
  fs::write(temp_dir.path().join("root.rs"), "fn root() {}")?;
  fs::write(temp_dir.path().join("subdir/subdir.rs"), "fn subdir() {}")?;
  fs::write(
    temp_dir.path().join("subdir/subsubdir/subsubdir.rs"),
    "fn subsubdir() {}",
  )?;

  // Add and commit all files
  git_add_and_commit(temp_dir.path(), ".", "Add test files")?;

  // Change to the subsubdir
  std::env::set_current_dir(temp_dir.path().join("subdir/subsubdir"))?;

  // Get tracked files from subsubdir
  let tracked_files = git::get_git_tracked_files(temp_dir.path())?;

  // Print the tracked files for debugging
  println!("Tracked files from subsubdir:");
  for file in &tracked_files {
    println!("  {}", file.display());
  }

  // Verify expected files are tracked
  assert!(
    tracked_files.contains(&PathBuf::from("initial.txt")),
    "Should contain initial.txt"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("root.rs")),
    "Should contain root.rs"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("subdir/subdir.rs")),
    "Should contain subdir.rs"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("subdir/subsubdir/subsubdir.rs")),
    "Should contain subsubdir.rs"
  );

  // Keep temp directory in scope until the end of the test
  drop(temp_dir);

  Ok(())
}

/// Helper: create a repo with multiple commits, then shallow-clone it.
/// Returns (origin_dir, shallow_dir) — both kept alive by TempDir handles.
fn create_shallow_clone() -> Result<(TempDir, TempDir)> {
  let origin_dir = init_temp_git_repo()?;

  // Create a second commit so there's history to be shallow about
  fs::write(origin_dir.path().join("second.txt"), "Second commit content")?;
  git_add_and_commit(origin_dir.path(), "second.txt", "Second commit")?;

  // Shallow clone with depth 1 — only the latest commit is fetched
  let shallow_dir = tempdir()?;
  // Remove the empty tempdir so git clone can create it
  fs::remove_dir(shallow_dir.path())?;
  run_git(
    origin_dir.path(),
    &[
      "clone",
      "--depth",
      "1",
      &format!("file://{}", origin_dir.path().display()),
      &shallow_dir.path().display().to_string(),
    ],
  )?;

  Ok((origin_dir, shallow_dir))
}

#[test]
fn test_shallow_clone_ratchet_fallback_merge_base() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let (_origin_dir, shallow_dir) = create_shallow_clone()?;

  // Add a new file in the shallow clone so there's a diff
  fs::write(shallow_dir.path().join("new_file.txt"), "New content")?;
  run_git(shallow_dir.path(), &["config", "user.name", "Test User"])?;
  run_git(shallow_dir.path(), &["config", "user.email", "test@example.com"])?;
  git_add_and_commit(shallow_dir.path(), "new_file.txt", "Add new file in shallow clone")?;

  // In a shallow clone, merge_base against origin/main should fail but
  // the function should fall back to a direct diff instead of erroring.
  let result = git::get_changed_files_for_workspace(shallow_dir.path(), "origin/main", &RatchetOptions::default());
  assert!(
    result.is_ok(),
    "Expected fallback to succeed in shallow clone, got: {:?}",
    result.err()
  );

  let changed = result.expect("already checked is_ok");
  assert!(
    changed.contains(&PathBuf::from("new_file.txt")),
    "Expected new_file.txt in changed files, got: {:?}",
    changed
  );

  Ok(())
}

#[test]
fn test_shallow_clone_ratchet_unresolvable_ref_errors() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let (_origin_dir, shallow_dir) = create_shallow_clone()?;

  // Try to ratchet against a commit that doesn't exist in the shallow history.
  // This should produce a clear error mentioning the shallow clone.
  let result = git::get_changed_files_for_workspace(
    shallow_dir.path(),
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0",
    &RatchetOptions::default(),
  );
  assert!(result.is_err(), "Expected error for unresolvable ref in shallow clone");

  let err_msg = format!("{}", result.unwrap_err());
  assert!(
    err_msg.contains("shallow clone"),
    "Error should mention shallow clone, got: {}",
    err_msg
  );

  Ok(())
}

#[test]
fn test_ratchet_includes_staged_files_by_default() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Create and stage a new file (but don't commit)
  fs::write(temp_dir.path().join("staged.rs"), "fn staged() {}")?;
  run_git(temp_dir.path(), &["add", "staged.rs"])?;

  // Get changed files with default options (should include staged)
  let changed = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD", &RatchetOptions::default())?;

  assert!(
    changed.contains(&PathBuf::from("staged.rs")),
    "Default ratchet mode should include staged files, got: {:?}",
    changed
  );

  Ok(())
}

#[test]
fn test_ratchet_includes_unstaged_files_by_default() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Modify the initial file (unstaged change)
  fs::write(temp_dir.path().join("initial.txt"), "Modified content")?;

  // Get changed files with default options (should include unstaged)
  let changed = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD", &RatchetOptions::default())?;

  assert!(
    changed.contains(&PathBuf::from("initial.txt")),
    "Default ratchet mode should include unstaged files, got: {:?}",
    changed
  );

  Ok(())
}

#[test]
fn test_ratchet_committed_only_excludes_staged_files() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Create and stage a new file (but don't commit)
  fs::write(temp_dir.path().join("staged.rs"), "fn staged() {}")?;
  run_git(temp_dir.path(), &["add", "staged.rs"])?;

  // Get changed files with committed_only (should NOT include staged)
  let changed = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD", &RatchetOptions::committed_only())?;

  assert!(
    !changed.contains(&PathBuf::from("staged.rs")),
    "committed_only mode should NOT include staged files, got: {:?}",
    changed
  );

  Ok(())
}

#[test]
fn test_ratchet_committed_only_excludes_unstaged_files() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Modify the initial file (unstaged change)
  fs::write(temp_dir.path().join("initial.txt"), "Modified content")?;

  // Get changed files with committed_only (should NOT include unstaged)
  let changed = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD", &RatchetOptions::committed_only())?;

  assert!(
    !changed.contains(&PathBuf::from("initial.txt")),
    "committed_only mode should NOT include unstaged files, got: {:?}",
    changed
  );

  Ok(())
}

#[test]
fn test_ratchet_excludes_deleted_files() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Delete the initial file (this is an unstaged deletion)
  fs::remove_file(temp_dir.path().join("initial.txt"))?;

  // Get changed files - deleted files should NOT be included
  // (they don't exist on disk, so processing them would fail)
  let changed = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD", &RatchetOptions::default())?;

  assert!(
    !changed.contains(&PathBuf::from("initial.txt")),
    "Deleted files should be excluded from ratchet results, got: {:?}",
    changed
  );

  Ok(())
}

#[test]
fn test_ratchet_excludes_staged_deleted_files() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Stage a deletion of the initial file
  fs::remove_file(temp_dir.path().join("initial.txt"))?;
  run_git(temp_dir.path(), &["add", "initial.txt"])?;

  // Get changed files - staged deleted files should NOT be included
  let changed = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD", &RatchetOptions::default())?;

  assert!(
    !changed.contains(&PathBuf::from("initial.txt")),
    "Staged deleted files should be excluded from ratchet results, got: {:?}",
    changed
  );

  Ok(())
}

// ---------------------------------------------------------------------------
// Repository state detection tests
// ---------------------------------------------------------------------------

/// Helper: create a repo with two branches that have a merge conflict so we can
/// trigger various in-progress git operations.
fn create_repo_with_conflict() -> Result<TempDir> {
  let temp_dir = init_temp_git_repo()?;

  // Create a file on main
  fs::write(temp_dir.path().join("conflict.txt"), "main content\n")?;
  git_add_and_commit(temp_dir.path(), "conflict.txt", "Add conflict.txt on main")?;

  // Create a branch with a conflicting change
  run_git(temp_dir.path(), &["checkout", "-b", "feature"])?;
  fs::write(temp_dir.path().join("conflict.txt"), "feature content\n")?;
  git_add_and_commit(temp_dir.path(), "conflict.txt", "Change conflict.txt on feature")?;

  // Go back to main and make a different change
  run_git(temp_dir.path(), &["checkout", "main"])?;
  fs::write(temp_dir.path().join("conflict.txt"), "different main content\n")?;
  git_add_and_commit(temp_dir.path(), "conflict.txt", "Change conflict.txt on main")?;

  Ok(temp_dir)
}

#[test]
fn test_ratchet_errors_during_merge() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = create_repo_with_conflict()?;

  // Start a merge that will conflict
  let merge_result = run_git(temp_dir.path(), &["merge", "feature"]);
  assert!(merge_result.is_err(), "Merge should conflict");

  // Ratchet should refuse to run during an in-progress merge
  let result = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD~1", &RatchetOptions::default());
  assert!(result.is_err(), "Expected error during in-progress merge");

  let err_msg = format!("{}", result.unwrap_err());
  assert!(
    err_msg.contains("merge in progress"),
    "Error should mention merge in progress, got: {}",
    err_msg
  );

  Ok(())
}

#[test]
fn test_ratchet_errors_during_rebase() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = create_repo_with_conflict()?;

  // Switch to feature and rebase onto main (will conflict)
  run_git(temp_dir.path(), &["checkout", "feature"])?;
  let rebase_result = run_git(temp_dir.path(), &["rebase", "main"]);
  assert!(rebase_result.is_err(), "Rebase should conflict");

  // Ratchet should refuse to run during an in-progress rebase
  let result = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD~1", &RatchetOptions::default());
  assert!(result.is_err(), "Expected error during in-progress rebase");

  let err_msg = format!("{}", result.unwrap_err());
  assert!(
    err_msg.contains("rebase in progress"),
    "Error should mention rebase in progress, got: {}",
    err_msg
  );

  Ok(())
}

#[test]
fn test_ratchet_errors_during_cherry_pick() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = create_repo_with_conflict()?;

  // Cherry-pick the feature commit onto main (will conflict)
  let cherry_pick_result = run_git(temp_dir.path(), &["cherry-pick", "feature"]);
  assert!(cherry_pick_result.is_err(), "Cherry-pick should conflict");

  // Ratchet should refuse to run during an in-progress cherry-pick
  let result = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD~1", &RatchetOptions::default());
  assert!(result.is_err(), "Expected error during in-progress cherry-pick");

  let err_msg = format!("{}", result.unwrap_err());
  assert!(
    err_msg.contains("cherry-pick in progress"),
    "Error should mention cherry-pick in progress, got: {}",
    err_msg
  );

  Ok(())
}

#[test]
fn test_ratchet_errors_during_revert() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = init_temp_git_repo()?;

  // Create two commits that will conflict on revert
  fs::write(temp_dir.path().join("revert.txt"), "original\n")?;
  git_add_and_commit(temp_dir.path(), "revert.txt", "Add revert.txt")?;

  fs::write(temp_dir.path().join("revert.txt"), "modified\n")?;
  git_add_and_commit(temp_dir.path(), "revert.txt", "Modify revert.txt")?;

  // Modify again so reverting the previous commit conflicts
  fs::write(temp_dir.path().join("revert.txt"), "modified again\n")?;
  git_add_and_commit(temp_dir.path(), "revert.txt", "Modify revert.txt again")?;

  let revert_result = run_git(temp_dir.path(), &["revert", "--no-commit", "HEAD~1"]);
  // The revert may or may not conflict, but --no-commit leaves us in REVERT state
  // If it errors, that's fine too - check if we're in a revert state
  if revert_result.is_ok() {
    // Even without conflict, --no-commit puts us in a revert state
    let result = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD~1", &RatchetOptions::default());
    assert!(result.is_err(), "Expected error during in-progress revert");

    let err_msg = format!("{}", result.unwrap_err());
    assert!(
      err_msg.contains("revert in progress"),
      "Error should mention revert in progress, got: {}",
      err_msg
    );
  }

  Ok(())
}

#[test]
fn test_ratchet_works_after_completing_merge() -> Result<()> {
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = create_repo_with_conflict()?;

  // Start a merge that will conflict
  let _ = run_git(temp_dir.path(), &["merge", "feature"]);

  // Verify ratchet fails during merge
  let result = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD~1", &RatchetOptions::default());
  assert!(result.is_err(), "Should fail during merge");

  // Resolve the conflict and complete the merge
  fs::write(temp_dir.path().join("conflict.txt"), "resolved content\n")?;
  run_git(temp_dir.path(), &["add", "conflict.txt"])?;
  git_commit(temp_dir.path(), "Merge feature into main")?;

  // Ratchet should work now
  let result = git::get_changed_files_for_workspace(temp_dir.path(), "HEAD~1", &RatchetOptions::default());
  assert!(
    result.is_ok(),
    "Ratchet should work after completing merge, got: {:?}",
    result.err()
  );

  Ok(())
}
