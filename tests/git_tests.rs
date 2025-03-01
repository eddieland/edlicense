use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

use edlicense::git;

// Helper function to check if git is available
fn is_git_available() -> bool {
  Command::new("git").arg("--version").status().is_ok()
}

// Helper function to initialize a git repository in a temporary directory
fn init_temp_git_repo() -> Result<tempfile::TempDir> {
  let temp_dir = tempdir()?;

  // Initialize git repository
  Command::new("git")
    .args(["init"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to initialize git repository")?;

  // Configure git user for commits
  Command::new("git")
    .args(["config", "user.name", "Test User"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to configure git user name")?;

  Command::new("git")
    .args(["config", "user.email", "test@example.com"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to configure git user email")?;

  // Create and commit a file to establish HEAD
  fs::write(temp_dir.path().join("initial.txt"), "Initial content")?;

  Command::new("git")
    .args(["add", "initial.txt"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to add initial file")?;

  Command::new("git")
    .args(["commit", "-m", "Initial commit"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to create initial commit")?;

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
  Command::new("git")
    .args(["add", "tracked.rs"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to add tracked file")?;

  Command::new("git")
    .args(["commit", "-m", "Add tracked file"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to commit tracked file")?;

  // Change to the git repository
  std::env::set_current_dir(&temp_dir)?;

  // Get tracked files
  let tracked_files = git::get_git_tracked_files()?;

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

  Command::new("git")
    .args(["add", "test.txt"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to add test file")?;

  Command::new("git")
    .args(["commit", "-m", "Add test file"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to commit test file")?;

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
  Command::new("git")
    .args(["add", "."])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to add files to git")?;

  Command::new("git")
    .args(["commit", "-m", "Add test files"])
    .current_dir(&temp_dir)
    .status()
    .with_context(|| "Failed to commit files")?;

  // Change to the subsubdir
  std::env::set_current_dir(temp_dir.path().join("subdir/subsubdir"))?;

  // Get tracked files from subsubdir
  let tracked_files = git::get_git_tracked_files()?;

  // Print the tracked files for debugging
  println!("Tracked files from subsubdir:");
  for file in &tracked_files {
    println!("  {}", file.display());
  }

  // Verify expected files are tracked
  assert!(
    tracked_files.contains(&PathBuf::from("../../initial.txt")),
    "Should contain initial.txt"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("../../root.rs")),
    "Should contain root.rs"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("../subdir.rs")),
    "Should contain subdir.rs"
  );
  assert!(
    tracked_files.contains(&PathBuf::from("subsubdir.rs")),
    "Should contain subsubdir.rs"
  );

  // Keep temp directory in scope until the end of the test
  drop(temp_dir);

  Ok(())
}
