use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use tempfile::tempdir;

// Helper function to create a test environment
fn setup_test_environment() -> Result<tempfile::TempDir> {
  let temp_dir = tempdir()?;

  // Create a license template
  let template_content = "Copyright (c) {{year}} Test Company\nAll rights reserved.";
  fs::write(temp_dir.path().join("license_template.txt"), template_content)?;

  // Create a test directory structure
  let src_dir = temp_dir.path().join("src");
  fs::create_dir_all(&src_dir)?;

  // Create some test files
  fs::write(
    src_dir.join("main.rs"),
    "fn main() {\n    println!(\"Hello, world!\");\n}",
  )?;
  fs::write(
    src_dir.join("lib.rs"),
    "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
  )?;

  // Create a file with an existing license
  let licensed_content = "// Copyright (c) 2024 Test Company\n// All rights reserved.\n\npub fn subtract(a: i32, b: i32) -> i32 {\n    a - b\n}";
  fs::write(src_dir.join("licensed.rs"), licensed_content)?;

  // Create a file with a shebang
  let shebang_content = "#!/usr/bin/env python3\n\ndef main():\n    print('Hello, world!')";
  let script_path = temp_dir.path().join("script.py");
  fs::write(&script_path, shebang_content)?;
  println!("Created Python script at: {:?}", script_path);

  // Create a directory to be ignored
  let vendor_dir = temp_dir.path().join("vendor");
  fs::create_dir_all(&vendor_dir)?;
  fs::write(vendor_dir.join("external.rs"), "fn external() {}")?;

  Ok(temp_dir)
}

// Helper function to run edlicense with the given arguments
fn run_edlicense(args: &[&str], current_dir: &Path) -> Result<(i32, String, String)> {
  // Get the path to the target directory
  let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
  let target_dir = Path::new(&manifest_dir).join("target").join("debug");
  let binary_path = target_dir.join("edlicense");

  // Print the command we're about to run for debugging
  println!(
    "Running: {:?} with args: {:?} in dir: {:?}",
    binary_path, args, current_dir
  );

  // Run the binary with the provided arguments
  let output = Command::new(binary_path).args(args).current_dir(current_dir).output()?;

  let status = output.status.code().unwrap_or(-1);
  let stdout = String::from_utf8_lossy(&output.stdout).to_string();
  let stderr = String::from_utf8_lossy(&output.stderr).to_string();

  // Print the output for debugging
  println!("Status: {}", status);
  println!("Stdout: {}", stdout);
  println!("Stderr: {}", stderr);

  Ok((status, stdout, stderr))
}

#[test]
fn test_add_license() -> Result<()> {
  let temp_dir = setup_test_environment()?;

  // Run edlicense to add licenses
  // Use -vvv for trace-level output which includes per-file processing messages
  let args = &[
    "--modify",
    "--year=2025",
    "--license-file",
    "license_template.txt",
    "-vvv",
    "src",
    "script.py",
  ];

  let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Check that the files were processed (trace-level output)
  assert!(stderr.contains("Processing file:"));

  // Check that the licenses were added
  let main_content = fs::read_to_string(temp_dir.path().join("src/main.rs"))?;
  assert!(main_content.contains("// Copyright (c)"));
  assert!(main_content.contains("Test Company"));

  let lib_content = fs::read_to_string(temp_dir.path().join("src/lib.rs"))?;
  assert!(lib_content.contains("// Copyright (c)"));

  // Check that the year was updated in the licensed file
  let licensed_content = fs::read_to_string(temp_dir.path().join("src/licensed.rs"))?;
  assert!(!licensed_content.contains("2024"));
  assert!(licensed_content.contains("2025"));

  // Check that the shebang was preserved
  let script_content = fs::read_to_string(temp_dir.path().join("script.py"))?;
  println!("Script content after processing:\n{}", script_content);
  assert!(script_content.starts_with("#!/usr/bin/env python3"));
  assert!(script_content.contains("# Copyright (c)"));

  Ok(())
}

#[test]
fn test_dry_run_mode() -> Result<()> {
  let temp_dir = setup_test_environment()?;

  // Run edlicense in dry run mode
  let args = &["--dry-run", "--license-file", "license_template.txt", "src"];

  let (status, stdout, _stderr) = run_edlicense(args, temp_dir.path())?;

  // Check that the command failed (because some files don't have licenses)
  assert_ne!(status, 0, "Command should have failed but succeeded");
  // Check that the output shows files missing license headers
  assert!(
    stdout.contains("missing license headers"),
    "Expected missing license message in stdout: {}",
    stdout
  );

  // Check that the files were not modified
  let main_content = fs::read_to_string(temp_dir.path().join("src/main.rs"))?;
  assert!(!main_content.contains("Copyright"));

  // Now add licenses to all files
  let add_args = &["--modify", "--license-file", "license_template.txt", "src"];

  let (add_status, _, _) = run_edlicense(add_args, temp_dir.path())?;
  assert_eq!(add_status, 0);

  // Run dry run mode again
  let (check_status, _, check_stderr) = run_edlicense(args, temp_dir.path())?;

  // This time it should succeed
  assert_eq!(check_status, 0, "Dry run command failed with stderr: {}", check_stderr);

  Ok(())
}

#[test]
fn test_ignore_patterns() -> Result<()> {
  let temp_dir = setup_test_environment()?;

  // Run edlicense with ignore patterns
  let args = &[
    "--modify",
    "--license-file",
    "license_template.txt",
    "--ignore",
    "vendor/**",
    "--verbose",
    ".",
  ];

  println!("Vendor directory path: {:?}", temp_dir.path().join("vendor"));

  let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Print the ignore patterns and check if the vendor directory was ignored
  println!("Ignore patterns: {:?}", args);
  println!("Stderr content: {}", stderr);

  // Check if the vendor directory was ignored
  let vendor_content = fs::read_to_string(temp_dir.path().join("vendor/external.rs"))?;
  println!("Vendor file content: {}", vendor_content);

  // For now, let's just check that the vendor file was not modified
  assert!(!vendor_content.contains("Copyright"));

  // Check that the vendor file was not modified
  let vendor_content = fs::read_to_string(temp_dir.path().join("vendor/external.rs"))?;
  assert!(!vendor_content.contains("Copyright"));

  Ok(())
}

#[test]
fn test_preserve_years() -> Result<()> {
  let temp_dir = setup_test_environment()?;

  // Run edlicense with preserve-years flag
  let args = &[
    "--modify",
    "--license-file",
    "license_template.txt",
    "--preserve-years",
    "src",
  ];

  let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Check that the year was preserved in the licensed file
  let licensed_content = fs::read_to_string(temp_dir.path().join("src/licensed.rs"))?;
  assert!(licensed_content.contains("2024"));
  assert!(!licensed_content.contains("2025"));

  Ok(())
}

#[test]
fn test_custom_year() -> Result<()> {
  let temp_dir = setup_test_environment()?;

  // Run edlicense with a custom year
  let args = &[
    "--modify",
    "--license-file",
    "license_template.txt",
    "--year",
    "2030",
    "src",
  ];

  let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Check that the custom year was used
  let main_content = fs::read_to_string(temp_dir.path().join("src/main.rs"))?;
  assert!(main_content.contains("2030"));

  // Check that the year was updated in the licensed file
  let licensed_content = fs::read_to_string(temp_dir.path().join("src/licensed.rs"))?;
  assert!(!licensed_content.contains("2024"));
  assert!(licensed_content.contains("2030"));

  Ok(())
}

// Helper function to check if git is available
fn is_git_available() -> bool {
  Command::new("git").arg("--version").status().is_ok()
}

#[test]
fn test_git_only_from_subdirectory() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = tempdir()?;

  // Initialize git repository
  Command::new("git").args(["init"]).current_dir(&temp_dir).status()?;

  // Configure git user for commits
  Command::new("git")
    .args(["config", "user.name", "Test User"])
    .current_dir(&temp_dir)
    .status()?;

  Command::new("git")
    .args(["config", "user.email", "test@example.com"])
    .current_dir(&temp_dir)
    .status()?;

  // Create a license template at the repo root
  let template_content = "Copyright (c) {{year}} Test Company\nAll rights reserved.";
  fs::write(temp_dir.path().join("license_template.txt"), template_content)?;

  // Create a subdirectory structure
  fs::create_dir_all(temp_dir.path().join("src/nested"))?;

  // Create test files at different levels
  fs::write(temp_dir.path().join("root.rs"), "fn root() {}")?;
  fs::write(temp_dir.path().join("src/lib.rs"), "fn lib() {}")?;
  fs::write(temp_dir.path().join("src/nested/module.rs"), "fn module() {}")?;

  // Add and commit all files
  Command::new("git").args(["add", "."]).current_dir(&temp_dir).status()?;

  Command::new("git")
    .args(["commit", "-m", "Initial commit"])
    .current_dir(&temp_dir)
    .status()?;

  // Run edlicense from the src/nested subdirectory with --git-only
  // Use **/*.rs pattern to match all Rust files across the repo
  let subdir = temp_dir.path().join("src/nested");
  let repo_root = temp_dir.path().to_string_lossy().to_string();
  let args = &[
    "--modify",
    "--git-only=true",
    "--year=2025",
    "--license-file",
    "../../license_template.txt",
    "--verbose",
    &format!("{}/**/*.rs", repo_root),
  ];

  let (status, _stdout, stderr) = run_edlicense(args, &subdir)?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Verify all .rs files at different levels got license headers
  // (This proves paths resolve correctly even when CWD is a subdirectory)
  let root_content = fs::read_to_string(temp_dir.path().join("root.rs"))?;
  assert!(
    root_content.contains("Copyright (c)"),
    "root.rs should have license header but has:\n{}",
    root_content
  );

  let lib_content = fs::read_to_string(temp_dir.path().join("src/lib.rs"))?;
  assert!(
    lib_content.contains("Copyright (c)"),
    "src/lib.rs should have license header but has:\n{}",
    lib_content
  );

  let module_content = fs::read_to_string(temp_dir.path().join("src/nested/module.rs"))?;
  assert!(
    module_content.contains("Copyright (c)"),
    "src/nested/module.rs should have license header but has:\n{}",
    module_content
  );

  Ok(())
}

/// Test that glob patterns with `..` segments work correctly when running
/// from a subdirectory in git-only mode.
///
/// This verifies that patterns like `../other/**/*.rs` are normalized correctly
/// when prefixed with the workspace-relative CWD (e.g., `src/nested/../other`
/// becomes `src/other`).
#[test]
fn test_git_only_glob_with_parent_dir_segments() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = tempdir()?;

  // Initialize git repository
  Command::new("git").args(["init"]).current_dir(&temp_dir).status()?;

  // Configure git user for commits
  Command::new("git")
    .args(["config", "user.name", "Test User"])
    .current_dir(&temp_dir)
    .status()?;

  Command::new("git")
    .args(["config", "user.email", "test@example.com"])
    .current_dir(&temp_dir)
    .status()?;

  // Create a license template at the repo root
  let template_content = "Copyright (c) {{year}} Test Company\nAll rights reserved.";
  fs::write(temp_dir.path().join("license_template.txt"), template_content)?;

  // Create sibling directories under src/
  fs::create_dir_all(temp_dir.path().join("src/nested"))?;
  fs::create_dir_all(temp_dir.path().join("src/other/deep"))?;

  // Create test files in both directories
  fs::write(temp_dir.path().join("src/nested/module.rs"), "fn module() {}")?;
  fs::write(temp_dir.path().join("src/other/sibling.rs"), "fn sibling() {}")?;
  fs::write(temp_dir.path().join("src/other/deep/nested.rs"), "fn nested() {}")?;

  // Add and commit all files
  Command::new("git").args(["add", "."]).current_dir(&temp_dir).status()?;

  Command::new("git")
    .args(["commit", "-m", "Initial commit"])
    .current_dir(&temp_dir)
    .status()?;

  // Run edlicense from the src/nested subdirectory with --git-only
  // Use a relative pattern with `..` to match files in the sibling directory
  let subdir = temp_dir.path().join("src/nested");
  let args = &[
    "--modify",
    "--git-only=true",
    "--year=2025",
    "--license-file",
    "../../license_template.txt",
    "--verbose",
    "../other/**/*.rs", // This pattern uses .. to reach the sibling directory
  ];

  let (status, _stdout, stderr) = run_edlicense(args, &subdir)?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Verify the files in src/other/ got license headers
  // (This proves that ../other/**/*.rs was normalized correctly)
  let sibling_content = fs::read_to_string(temp_dir.path().join("src/other/sibling.rs"))?;
  assert!(
    sibling_content.contains("Copyright (c)"),
    "src/other/sibling.rs should have license header but has:\n{}",
    sibling_content
  );

  let nested_content = fs::read_to_string(temp_dir.path().join("src/other/deep/nested.rs"))?;
  assert!(
    nested_content.contains("Copyright (c)"),
    "src/other/deep/nested.rs should have license header but has:\n{}",
    nested_content
  );

  // Verify the file in src/nested/ was NOT modified (not matched by pattern)
  let module_content = fs::read_to_string(temp_dir.path().join("src/nested/module.rs"))?;
  assert!(
    !module_content.contains("Copyright (c)"),
    "src/nested/module.rs should NOT have license header but has:\n{}",
    module_content
  );

  Ok(())
}

/// Test that directory paths with `..` segments work correctly when running
/// from a subdirectory in git-only mode.
///
/// This verifies that patterns like `../other` (an existing directory) are
/// normalized correctly so that `src/nested/../other` becomes `src/other`
/// and matches git-tracked files in that directory.
#[test]
fn test_git_only_directory_with_parent_dir_segments() -> Result<()> {
  // Skip test if git is not available
  if !is_git_available() {
    println!("Skipping git test because git command is not available");
    return Ok(());
  }

  let temp_dir = tempdir()?;

  // Initialize git repository
  Command::new("git").args(["init"]).current_dir(&temp_dir).status()?;

  // Configure git user for commits
  Command::new("git")
    .args(["config", "user.name", "Test User"])
    .current_dir(&temp_dir)
    .status()?;

  Command::new("git")
    .args(["config", "user.email", "test@example.com"])
    .current_dir(&temp_dir)
    .status()?;

  // Create a license template at the repo root
  let template_content = "Copyright (c) {{year}} Test Company\nAll rights reserved.";
  fs::write(temp_dir.path().join("license_template.txt"), template_content)?;

  // Create sibling directories under src/
  fs::create_dir_all(temp_dir.path().join("src/nested"))?;
  fs::create_dir_all(temp_dir.path().join("src/other/deep"))?;

  // Create test files in both directories
  fs::write(temp_dir.path().join("src/nested/module.rs"), "fn module() {}")?;
  fs::write(temp_dir.path().join("src/other/sibling.rs"), "fn sibling() {}")?;
  fs::write(temp_dir.path().join("src/other/deep/nested.rs"), "fn nested() {}")?;

  // Add and commit all files
  Command::new("git").args(["add", "."]).current_dir(&temp_dir).status()?;

  Command::new("git")
    .args(["commit", "-m", "Initial commit"])
    .current_dir(&temp_dir)
    .status()?;

  // Run edlicense from the src/nested subdirectory with --git-only
  // Use a relative directory path with `..` to match files in the sibling
  // directory
  let subdir = temp_dir.path().join("src/nested");
  let args = &[
    "--modify",
    "--git-only=true",
    "--year=2025",
    "--license-file",
    "../../license_template.txt",
    "--verbose",
    "../other", // This is a directory path (not a glob) that uses .. to reach the sibling
  ];

  let (status, _stdout, stderr) = run_edlicense(args, &subdir)?;

  // Check that the command succeeded
  assert_eq!(status, 0, "Command failed with stderr: {}", stderr);

  // Verify the files in src/other/ got license headers
  // (This proves that ../other was normalized correctly to src/other)
  let sibling_content = fs::read_to_string(temp_dir.path().join("src/other/sibling.rs"))?;
  assert!(
    sibling_content.contains("Copyright (c)"),
    "src/other/sibling.rs should have license header but has:\n{}",
    sibling_content
  );

  let nested_content = fs::read_to_string(temp_dir.path().join("src/other/deep/nested.rs"))?;
  assert!(
    nested_content.contains("Copyright (c)"),
    "src/other/deep/nested.rs should have license header but has:\n{}",
    nested_content
  );

  // Verify the file in src/nested/ was NOT modified (not matched by pattern)
  let module_content = fs::read_to_string(temp_dir.path().join("src/nested/module.rs"))?;
  assert!(
    !module_content.contains("Copyright (c)"),
    "src/nested/module.rs should NOT have license header but has:\n{}",
    module_content
  );

  Ok(())
}
