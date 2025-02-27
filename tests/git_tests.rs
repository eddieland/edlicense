use anyhow::{Context, Result};
use std::collections::HashSet;
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
        .current_dir(temp_dir.path())
        .status()
        .with_context(|| "Failed to initialize git repository")?;

    // Configure git user for commits
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp_dir.path())
        .status()
        .with_context(|| "Failed to configure git user name")?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp_dir.path())
        .status()
        .with_context(|| "Failed to configure git user email")?;

    // Create and commit a file to establish HEAD
    let initial_file = temp_dir.path().join("initial.txt");
    fs::write(&initial_file, "Initial content")?;

    Command::new("git")
        .args(["add", "initial.txt"])
        .current_dir(temp_dir.path())
        .status()
        .with_context(|| "Failed to add initial file")?;

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_dir.path())
        .status()
        .with_context(|| "Failed to create initial commit")?;

    Ok(temp_dir)
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

    // Create a new file that will be detected as changed
    let new_file = temp_dir.path().join("new_file.txt");
    fs::write(&new_file, "New content")?;

    // Modify the initial file
    let initial_file = temp_dir.path().join("initial.txt");
    fs::write(&initial_file, "Modified content")?;

    // Change to the temporary directory for the git operations
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(temp_dir.path())?;

    // Get changed files relative to HEAD
    let changed_files = git::get_changed_files("HEAD")?;

    // Change back to the original directory
    std::env::set_current_dir(original_dir)?;

    // Verify that both files are detected as changed
    assert!(changed_files.contains(&PathBuf::from("new_file.txt")));
    assert!(changed_files.contains(&PathBuf::from("initial.txt")));
    assert_eq!(changed_files.len(), 2);

    Ok(())
}

// This is a mock implementation for testing purposes
// It allows tests to use a mock version of get_changed_files
#[cfg(test)]
pub fn mock_get_changed_files(changed_paths: Vec<&str>) -> HashSet<PathBuf> {
    let mut changed_files = HashSet::new();
    for path in changed_paths {
        changed_files.insert(PathBuf::from(path));
    }
    changed_files
}
