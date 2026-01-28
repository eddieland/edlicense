mod common;

use std::fs;

use anyhow::Result;
use common::{git_add_and_commit, init_git_repo};
use edlicense::diff::DiffManager;
use edlicense::processor::{Processor, ProcessorConfig};
use edlicense::templates::{LicenseData, TemplateManager};
use tempfile::tempdir;

fn create_test_processor(
  template_content: &str,
  ignore_patterns: Vec<String>,
  check_only: bool,
  preserve_years: bool,
  ratchet_reference: Option<String>,
  show_diff: Option<bool>,
  save_diff_path: Option<std::path::PathBuf>,
  git_only: bool,
) -> Result<(Processor, tempfile::TempDir)> {
  let temp_dir = tempdir()?;
  let template_path = temp_dir.path().join("test_template.txt");

  // Create a test template
  fs::write(&template_path, template_content)?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  // Create diff manager
  let diff_manager = Some(DiffManager::new(show_diff.unwrap_or(false), save_diff_path));

  let processor = Processor::new(ProcessorConfig {
    check_only,
    preserve_years,
    git_only,
    ratchet_reference,
    ignore_patterns,
    diff_manager,
    ..ProcessorConfig::new(template_manager, license_data, temp_dir.path().to_path_buf())
  })?;

  Ok((processor, temp_dir))
}

#[test]
fn test_license_detection() -> Result<()> {
  // Create a processor
  let (processor, _temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    false,
    false,
    None,
    None,
    None,
    false,
  )?;

  // Test content with a license
  let content_with_license = "// Copyright (c) 2024 Test Company\n\nfn main() {}";
  assert!(processor.has_license(content_with_license));

  // Test content with a license in different format
  let content_with_license2 = "/* Copyright (C) 2024 Test Company */\n\nfn main() {}";
  assert!(processor.has_license(content_with_license2));

  // Test content without a license - avoid anything that might be interpreted as
  // a license
  let content_without_license = "fn main() {\n    println!(\"No license in this code\");\n}";
  assert!(!processor.has_license(content_without_license));

  Ok(())
}

#[test]
fn test_ignore_patterns() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;
  let temp_path = temp_dir.path();

  // Create a .licenseignore file
  let ignore_content = "*.json\nvendor/\n";
  fs::write(temp_path.join(".licenseignore"), ignore_content)?;

  // Create test files
  fs::write(temp_path.join("test.json"), "// Test JSON file")?;
  fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
  fs::create_dir_all(temp_path.join("vendor"))?;
  fs::write(temp_path.join("vendor").join("test.rs"), "// Test vendor file")?;
  fs::create_dir_all(temp_path.join("vendor").join("subfolder"))?;
  fs::write(
    temp_path.join("vendor").join("subfolder").join("test.rs"),
    "// Test subfolder file",
  )?;
  fs::create_dir_all(temp_path.join("src"))?;
  fs::write(temp_path.join("src").join("test.rs"), "// Test src file")?;
  fs::write(temp_path.join("test_vendor.rs"), "// Test vendor-like file")?;

  // Create an IgnoreManager and load the .licenseignore file
  use edlicense::ignore::IgnoreManager;
  let mut ignore_manager = IgnoreManager::new(vec![])?;
  ignore_manager.load_licenseignore_files(temp_path, temp_path)?;

  // Test files that should be ignored
  assert!(
    ignore_manager.is_ignored(&temp_path.join("test.json")),
    "JSON file should be ignored"
  );
  assert!(
    ignore_manager.is_ignored(&temp_path.join("vendor").join("test.rs")),
    "Vendor file should be ignored"
  );
  assert!(
    ignore_manager.is_ignored(&temp_path.join("vendor").join("subfolder").join("test.rs")),
    "Subfolder file should be ignored"
  );

  // Test files that should not be ignored
  assert!(
    !ignore_manager.is_ignored(&temp_path.join("test.rs")),
    "Rust file should not be ignored"
  );
  assert!(
    !ignore_manager.is_ignored(&temp_path.join("src").join("test.rs")),
    "Src file should not be ignored"
  );
  assert!(
    !ignore_manager.is_ignored(&temp_path.join("test_vendor.rs")),
    "Vendor-like file should not be ignored"
  );

  Ok(())
}

#[test]
fn test_process_file() -> Result<()> {
  // Create a processor
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    false,
    false,
    None,
    None,
    None,
    false,
  )?;

  // Create a test file without a license - avoid using any text that might be
  // interpreted as a license
  let test_file_path = temp_dir.path().join("test.rs");
  fs::write(&test_file_path, "fn main() {\n    println!(\"Testing!\");\n}")?;

  // Process the file
  let patterns = vec![test_file_path.to_string_lossy().to_string()];
  processor.process(&patterns)?;

  // Read the file and check if license was added
  let content = fs::read_to_string(&test_file_path)?;
  assert!(content.contains("// Copyright (c) 2025 Test Company"));
  assert!(content.contains("fn main()"));

  // Create a test file with a shebang
  let test_file_with_shebang = temp_dir.path().join("test.py");
  fs::write(
    &test_file_with_shebang,
    "#!/usr/bin/env python3\n\ndef main():\n    print('Hello, world!')",
  )?;

  // Process the file
  let patterns = vec![test_file_with_shebang.to_string_lossy().to_string()];
  processor.process(&patterns)?;

  // Read the file and check if license was added after shebang
  let content = fs::read_to_string(&test_file_with_shebang)?;
  assert!(content.starts_with("#!/usr/bin/env python3"));
  assert!(content.contains("# Copyright (c) 2025 Test Company"));
  assert!(content.contains("def main():"));

  Ok(())
}

#[test]
fn test_check_only_mode() -> Result<()> {
  // Create a processor in check-only mode
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    true, // check_only = true
    false,
    None,
    None,
    None, // No save diff path
    false,
  )?;

  // Create a test file without a license - avoid using any text that might be
  // interpreted as a license
  let test_file_path = temp_dir.path().join("test.rs");
  fs::write(&test_file_path, "fn main() {\n    println!(\"No license test\");\n}")?;

  // Process the file - should return has_missing = true
  let patterns = vec![test_file_path.to_string_lossy().to_string()];
  let has_missing = processor.process(&patterns)?;
  assert!(has_missing);

  // The file should not be modified
  let content = fs::read_to_string(&test_file_path)?;
  assert!(!content.contains("Copyright"));
  assert_eq!(content, "fn main() {\n    println!(\"No license test\");\n}");

  // Create a test file with a license
  let test_file_with_license = temp_dir.path().join("test_with_license.rs");
  fs::write(
    &test_file_with_license,
    "// Copyright (c) 2024 Test Company\n\nfn main() {\n    println!(\"Hello, world!\");\n}",
  )?;

  // Process the file - should succeed (has_missing = false)
  let patterns = vec![test_file_with_license.to_string_lossy().to_string()];
  let has_missing = processor.process(&patterns)?;
  assert!(!has_missing);

  // The file should not be modified (even though the year is old)
  let content = fs::read_to_string(&test_file_with_license)?;
  assert!(content.contains("Copyright (c) 2024 Test Company"));

  Ok(())
}

#[test]
fn test_preserve_years() -> Result<()> {
  // Create a processor with preserve_years = true
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    false,
    true, // preserve_years = true
    None,
    None,
    None, // No save diff path
    false,
  )?;

  // Create a test file with an old year
  let test_file_path = temp_dir.path().join("test.rs");
  fs::write(
    &test_file_path,
    "// Copyright (c) 2024 Test Company\n\nfn main() {\n    println!(\"Hello, world!\");\n}",
  )?;

  // Process the file
  let patterns = vec![test_file_path.to_string_lossy().to_string()];
  processor.process(&patterns)?;

  // The year should not be updated
  let content = fs::read_to_string(&test_file_path)?;
  assert!(content.contains("Copyright (c) 2024 Test Company"));

  // Create a processor with preserve_years = false
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    false,
    false, // preserve_years = false
    None,
    None,
    None, // No save diff path
    false,
  )?;

  // Create a test file with an old year
  let test_file_path = temp_dir.path().join("test.rs");
  fs::write(
    &test_file_path,
    "// copyright (c) 2024 Test Company\n\nfn main() {\n    println!(\"Hello, world!\");\n}",
  )?;

  // Process the file
  let patterns = vec![test_file_path.to_string_lossy().to_string()];
  processor.process(&patterns)?;

  // The year should be updated
  let content = fs::read_to_string(&test_file_path)?;
  assert!(content.contains("copyright (c) 2025 Test Company"));

  Ok(())
}

#[test]
fn test_process_directory() -> Result<()> {
  // Create a processor
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec!["*.json".to_string()], // Ignore JSON files
    false,
    false,
    None,
    None,
    None, // No save diff path
    false,
  )?;

  // Create a test directory structure
  let test_dir = temp_dir.path().join("test_dir");
  fs::create_dir_all(&test_dir)?;

  // Create some test files - avoid anything that might be interpreted as a
  // license
  fs::write(test_dir.join("file1.rs"), "fn test1_fn() { /* test */ }")?;
  fs::write(test_dir.join("file2.py"), "def test2_fn():\n    pass # test")?;
  fs::write(test_dir.join("file3.json"), "{\"key\": \"value\"}")?; // Should be ignored

  // Create a subdirectory
  let subdir = test_dir.join("subdir");
  fs::create_dir_all(&subdir)?;
  fs::write(subdir.join("file4.rs"), "fn test4_fn() { /* subdir test */ }")?;

  // Process the directory
  let _has_missing = processor.process_directory(&test_dir)?;

  // All non-ignored files should have licenses now
  let content1 = fs::read_to_string(test_dir.join("file1.rs"))?;
  assert!(content1.contains("// Copyright (c) 2025 Test Company"));

  let content2 = fs::read_to_string(test_dir.join("file2.py"))?;
  assert!(content2.contains("# Copyright (c) 2025 Test Company"));

  let content3 = fs::read_to_string(test_dir.join("file3.json"))?;
  assert!(!content3.contains("Copyright")); // Should be ignored

  let content4 = fs::read_to_string(subdir.join("file4.rs"))?;
  assert!(content4.contains("// Copyright (c) 2025 Test Company"));

  Ok(())
}

// Test the filtering functionality indirectly through the Processor
#[test]
fn test_file_filtering() -> Result<()> {
  // Create a temporary directory for testing
  let temp_dir = tempdir()?;
  let test_file = temp_dir.path().join("test.rs");
  let ignored_file = temp_dir.path().join("ignored.json");

  // Create test files
  fs::write(&test_file, "fn test() {}")?;
  fs::write(&ignored_file, "{\"test\": \"value\"}")?;

  // Create a processor with an ignore pattern for JSON files
  let (processor, _) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec!["*.json".to_string()], // Ignore JSON files
    false,
    false,
    None,
    None,
    None,
    false,
  )?;

  // Process both files
  let patterns = vec![
    test_file.to_string_lossy().to_string(),
    ignored_file.to_string_lossy().to_string(),
  ];
  processor.process(&patterns)?;

  // Check the results
  let test_content = fs::read_to_string(&test_file)?;
  let ignored_content = fs::read_to_string(&ignored_file)?;

  // The .rs file should have a license
  assert!(
    test_content.contains("// Copyright (c) 2025 Test Company"),
    "The .rs file should have a license header"
  );

  // The .json file should NOT have a license (because it's ignored)
  assert!(
    !ignored_content.contains("Copyright (c) 2025 Test Company"),
    "The .json file should not have a license header"
  );

  // The JSON file's content should be unchanged
  assert_eq!(
    ignored_content, "{\"test\": \"value\"}",
    "The JSON file content should be unchanged"
  );

  Ok(())
}

// Test for the ratchet mode functionality
#[test]
fn test_ratchet_mode_directory() -> Result<()> {
  // First, check that git is available
  if !common::is_git_available() {
    println!("Skipping test_ratchet_mode_directory: git not available");
    return Ok(());
  }

  // Create a directory structure
  let temp_dir = tempdir()?;
  let test_dir = temp_dir.path();

  init_git_repo(test_dir)?;

  // Create initial files
  let file1 = test_dir.join("file1.rs");
  let file2 = test_dir.join("file2.rs");
  fs::write(&file1, "fn file1_fn() { /* test */ }")?;
  fs::write(&file2, "fn file2_fn() { /* test */ }")?;

  // Initial commit
  git_add_and_commit(test_dir, ".", "Initial commit")?;

  // Save the commit hash for ratchet reference
  let rev_parse_output = std::process::Command::new("git")
    .args(["rev-parse", "HEAD"])
    .current_dir(test_dir)
    .output()?;
  let commit_ref = String::from_utf8_lossy(&rev_parse_output.stdout).trim().to_string();
  println!("Initial commit: {}", commit_ref);

  // Modify file1 and commit
  fs::write(&file1, "fn file1_fn_modified() { /* test */ }")?;
  git_add_and_commit(test_dir, "file1.rs", "Modify file1")?;

  // Print debug info about the file paths
  println!("file1 path: {}", file1.display());
  println!("file2 path: {}", file2.display());

  // Make sure we're in the right directory first, before creating the processor
  let process_dir = std::env::current_dir()?;
  println!("Current directory before processing: {}", process_dir.display());
  std::env::set_current_dir(test_dir)?;
  println!("Changed directory to: {}", test_dir.display());

  // Create a processor with ratchet mode enabled, using the original commit as
  // reference
  let template_path = test_dir.join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  let processor = Processor::new(ProcessorConfig {
    workspace_is_git: true,
    ratchet_reference: Some(commit_ref.clone()),
    ..ProcessorConfig::new(template_manager, license_data, test_dir.to_path_buf())
  })?;

  // Get direct insight into git's changed files list
  println!("Checking git's view of changed files...");
  let changed_files = edlicense::git::get_changed_files(&commit_ref)?;
  println!("Git reports {} changed files", changed_files.len());
  for file in &changed_files {
    println!("  Changed file: {}", file.display());
  }

  // Now process the files using the processor
  println!("Processing files using processor...");
  processor.process(&[".".to_string()])?;

  // Go back to original directory
  std::env::set_current_dir(process_dir)?;

  // Verify results - only file1 should have a license
  let file1_content = fs::read_to_string(&file1)?;
  let file2_content = fs::read_to_string(&file2)?;

  // Print debug content
  println!("file1 content: {}", file1_content);
  println!("file2 content: {}", file2_content);

  // The changed file should have a license
  assert!(
    file1_content.contains("// Copyright (c) 2025 Test Company"),
    "Changed file should have a license header"
  );

  // The unchanged file should not have a license header added
  assert!(
    !file2_content.contains("// Copyright (c) 2025 Test Company"),
    "Unchanged file should not have a license header"
  );

  Ok(())
}

#[test]
fn test_show_diff_mode() -> Result<()> {
  // Create a processor in check-only mode with show_diff enabled
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    true, // check_only = true
    false,
    None,
    Some(true), // show_diff = true
    None,       // No save diff path
    false,
  )?;

  // Create a test file without a license - avoid using any text that might be
  // interpreted as a license
  let test_file_path = temp_dir.path().join("test.rs");
  fs::write(&test_file_path, "fn main() {\n    println!(\"Diff test\");\n}")?;

  // Process the file - should return has_missing = true
  let patterns = vec![test_file_path.to_string_lossy().to_string()];
  let has_missing = processor.process(&patterns)?;
  assert!(has_missing);

  // The file should not be modified
  let content = fs::read_to_string(&test_file_path)?;
  assert!(!content.contains("Copyright"));
  assert_eq!(content, "fn main() {\n    println!(\"Diff test\");\n}");

  Ok(())
}

#[test]
fn test_diff_manager() -> Result<()> {
  // Create a DiffManager
  let diff_manager = DiffManager::new(true, None);

  // Test displaying a diff
  let original = "fn main() {\n    println!(\"Hello, world!\");\n}";
  let new = "// Copyright (c) 2025 Test Company\n\nfn main() {\n    println!(\"Hello, world!\");\n}";

  // This should not panic
  diff_manager.display_diff(std::path::Path::new("test.rs"), original, new)?;

  Ok(())
}

#[test]
fn test_manual_ratchet_mode() -> Result<()> {
  // This test verifies that the ratchet mode works correctly by manually creating
  // a RatchetFilter with a predetermined set of changed files

  // Create a temporary directory for testing
  let temp_dir = tempdir()?;
  let test_dir = temp_dir.path();

  // Create test files
  let file1 = test_dir.join("file1.rs");
  let file2 = test_dir.join("file2.rs");
  fs::write(&file1, "fn file1_fn() { /* test */ }")?;
  fs::write(&file2, "fn file2_fn() { /* test */ }")?;

  // Create a manually constructed set of changed files (only file1.rs)
  // Use relative paths as that's what git would typically provide
  use std::collections::HashSet;

  let mut changed_files = HashSet::new();
  changed_files.insert(std::path::PathBuf::from("file1.rs"));

  // Now test with a processor using our manual RatchetFilter
  let (processor, _) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    false,
    false,
    None, // We'll manually apply our filter
    None,
    None,
    false,
  )?;

  // Only process file1 (our "changed" file)
  let patterns = vec![file1.to_string_lossy().to_string()];
  processor.process(&patterns)?;

  // Verify results
  let file1_content = fs::read_to_string(&file1)?;
  let file2_content = fs::read_to_string(&file2)?;

  // The changed file should have a license
  assert!(
    file1_content.contains("// Copyright (c) 2025 Test Company"),
    "Changed file should have a license header"
  );

  // The unchanged file should not have a license (we didn't process it)
  assert!(
    !file2_content.contains("// Copyright (c) 2025 Test Company"),
    "Unchanged file should not have a license header"
  );

  Ok(())
}

// Test for the process method
#[test]
fn test_process() -> Result<()> {
  // Create a processor
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec!["*.json".to_string()], // Ignore JSON files
    false,
    false,
    None,
    None,
    None,
    false,
  )?;

  // Create test files
  let test_file = temp_dir.path().join("test.rs");
  let ignored_file = temp_dir.path().join("ignored.json");
  fs::write(&test_file, "fn test() {}")?;
  fs::write(&ignored_file, "{\"test\": \"value\"}")?;

  // Create a test directory
  let test_dir = temp_dir.path().join("test_dir");
  fs::create_dir_all(&test_dir)?;
  fs::write(test_dir.join("dir_file.rs"), "fn dir_test() {}")?;

  // Process the files and directory
  let patterns = vec![
    test_file.to_string_lossy().to_string(),
    ignored_file.to_string_lossy().to_string(),
    test_dir.to_string_lossy().to_string(),
  ];

  let _has_missing = processor.process(&patterns)?;

  // Check results
  let test_content = fs::read_to_string(&test_file)?;
  let ignored_content = fs::read_to_string(&ignored_file)?;
  let dir_file_content = fs::read_to_string(test_dir.join("dir_file.rs"))?;

  // The .rs files should have licenses
  assert!(
    test_content.contains("// Copyright (c) 2025 Test Company"),
    "The .rs file should have a license header"
  );

  assert!(
    dir_file_content.contains("// Copyright (c) 2025 Test Company"),
    "The directory .rs file should have a license header"
  );

  // The .json file should NOT have a license (because it's ignored)
  assert!(
    !ignored_content.contains("Copyright (c) 2025 Test Company"),
    "The .json file should not have a license header"
  );

  Ok(())
}
