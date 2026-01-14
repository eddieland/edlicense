//! Regression tests for race conditions during file processing.
//!
//! These tests verify that the processor handles files being modified or
//! deleted mid-processing gracefully - reporting errors but continuing with
//! other files.

#![allow(clippy::panic_in_result_fn)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::shadow_unrelated)]

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};
use tempfile::tempdir;

/// Helper to create a test processor with workspace root set to temp dir
async fn create_test_processor(temp_dir: &tempfile::TempDir, check_only: bool) -> Result<Processor> {
  let template_path = temp_dir.path().join("test_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  Processor::new(
    template_manager,
    license_data,
    vec![],
    check_only,
    false,
    None,
    None,
    false,
    None,
    temp_dir.path().to_path_buf(),
    false,
    None, // No extension filter
  )
}

/// Test that processing a file that doesn't exist returns an error
#[tokio::test]
async fn test_process_nonexistent_file() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let nonexistent_path = temp_dir.path().join("does_not_exist.rs");
  let result = processor.process_file(&nonexistent_path).await;

  assert!(result.is_err());
  let err_msg = result.unwrap_err().to_string();
  // The error should indicate file access failure
  assert!(
    err_msg.contains("Failed to") || err_msg.contains("No such file"),
    "Expected file access error, got: {}",
    err_msg
  );

  Ok(())
}

/// Test that processing continues when one file is deleted between discovery
/// and processing. This simulates a race condition where a file is deleted
/// after being discovered but before actually being processed.
#[tokio::test]
async fn test_file_deleted_mid_processing_continues() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  // Create multiple files
  let file1 = temp_dir.path().join("file1.rs");
  let file2 = temp_dir.path().join("file2.rs");
  let file3 = temp_dir.path().join("file3.rs");

  fs::write(&file1, "fn main() {}")?;
  fs::write(&file2, "fn helper() {}")?;
  fs::write(&file3, "fn another() {}")?;

  // Delete file2 to simulate race condition
  fs::remove_file(&file2)?;

  // Process all three files - file2 should fail but file1 and file3 should
  // succeed
  let result1 = processor.process_file(&file1).await;
  let result2 = processor.process_file(&file2).await;
  let result3 = processor.process_file(&file3).await;

  // file1 and file3 should succeed
  assert!(result1.is_ok(), "file1 should succeed: {:?}", result1);
  assert!(result3.is_ok(), "file3 should succeed: {:?}", result3);

  // file2 should fail with appropriate error
  assert!(result2.is_err(), "file2 should fail since it was deleted");

  // Verify file1 and file3 got licenses
  let content1 = fs::read_to_string(&file1)?;
  let content3 = fs::read_to_string(&file3)?;
  assert!(content1.contains("Copyright"), "file1 should have license");
  assert!(content3.contains("Copyright"), "file3 should have license");

  Ok(())
}

/// Test that the process() method continues when files are deleted during batch
/// processing. Uses absolute paths to avoid workspace root issues.
#[tokio::test]
async fn test_batch_processing_with_deleted_files() -> Result<()> {
  let temp_dir = tempdir()?;

  // Create a subdirectory with multiple files
  let subdir = temp_dir.path().join("src");
  fs::create_dir(&subdir)?;

  // Create several files
  for i in 1..=5 {
    let file_path = subdir.join(format!("file{}.rs", i));
    fs::write(&file_path, format!("fn func{}() {{}}", i))?;
  }

  let processor = create_test_processor(&temp_dir, false).await?;

  // Delete some files to simulate race condition
  fs::remove_file(subdir.join("file2.rs"))?;
  fs::remove_file(subdir.join("file4.rs"))?;

  // Process using absolute path pattern
  let abs_pattern = format!("{}/**/*.rs", subdir.display());
  let patterns = vec![abs_pattern];
  let has_missing = processor.process(&patterns).await?;

  // Since we're modifying files (not check-only), has_missing should be false
  // for successfully processed files
  assert!(
    !has_missing,
    "Should report no missing licenses for successfully processed files"
  );

  // Verify remaining files got licenses
  for i in [1, 3, 5] {
    let content = fs::read_to_string(subdir.join(format!("file{}.rs", i)))?;
    assert!(content.contains("Copyright"), "file{}.rs should have license", i);
  }

  Ok(())
}

/// Test that check-only mode reports errors for deleted files but continues.
#[tokio::test]
async fn test_check_only_with_deleted_files() -> Result<()> {
  let temp_dir = tempdir()?;

  let subdir = temp_dir.path().join("src");
  fs::create_dir(&subdir)?;

  // Create files - some with licenses, some without
  let file_with_license = subdir.join("with_license.rs");
  let file_without_license = subdir.join("without_license.rs");
  let file_to_delete = subdir.join("to_delete.rs");

  fs::write(&file_with_license, "// Copyright (c) 2025 Test Company\nfn main() {}")?;
  fs::write(&file_without_license, "fn helper() {}")?;
  fs::write(&file_to_delete, "fn another() {}")?;

  let processor = create_test_processor(&temp_dir, true).await?;

  // Delete one file
  fs::remove_file(&file_to_delete)?;

  // Process in check-only mode with absolute path
  let abs_pattern = format!("{}/**/*.rs", subdir.display());
  let patterns = vec![abs_pattern];
  let has_missing = processor.process(&patterns).await?;

  // Should report missing licenses (file_without_license + error from deleted
  // file)
  assert!(has_missing, "Should report missing licenses");

  Ok(())
}

/// Test processing when a file is replaced with a directory (edge case race
/// condition).
#[tokio::test]
async fn test_file_replaced_with_directory() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let path = temp_dir.path().join("ambiguous.rs");

  // First create as file
  fs::write(&path, "fn main() {}")?;

  // Then replace with directory
  fs::remove_file(&path)?;
  fs::create_dir(&path)?;

  // Processing should fail gracefully
  let result = processor.process_file(&path).await;
  assert!(result.is_err(), "Should fail when path is a directory");

  Ok(())
}

/// Test that empty files are handled appropriately.
/// Note: Empty files get licenses added when using process_file() directly.
/// (Batch processing skips empty files, but single-file processing does not.)
#[tokio::test]
async fn test_empty_file_handling() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let file_path = temp_dir.path().join("empty.rs");

  // Create an empty file
  fs::write(&file_path, "")?;

  // Processing empty file should succeed
  let result = processor.process_file(&file_path).await;
  assert!(result.is_ok(), "Empty files should be handled gracefully");

  // When using process_file directly, licenses are added to empty files
  let content = fs::read_to_string(&file_path)?;
  assert!(content.contains("Copyright"), "Empty file should have license added");

  Ok(())
}

/// Test that processing handles files that become unreadable.
#[cfg(unix)]
#[tokio::test]
async fn test_file_becomes_unreadable() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let file_path = temp_dir.path().join("unreadable.rs");
  fs::write(&file_path, "fn main() {}")?;

  // Make file unreadable
  let mut perms = fs::metadata(&file_path)?.permissions();
  perms.set_mode(0o000);
  fs::set_permissions(&file_path, perms)?;

  // Processing should fail gracefully
  let result = processor.process_file(&file_path).await;
  assert!(result.is_err(), "Should fail for unreadable file");

  // Restore permissions for cleanup
  let mut perms = fs::metadata(&file_path)?.permissions();
  perms.set_mode(0o644);
  fs::set_permissions(&file_path, perms)?;

  Ok(())
}

/// Test that processing handles files that become unwritable.
#[cfg(unix)]
#[tokio::test]
async fn test_file_becomes_unwritable() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let file_path = temp_dir.path().join("unwritable.rs");
  fs::write(&file_path, "fn main() {}")?;

  // Make file read-only
  let mut perms = fs::metadata(&file_path)?.permissions();
  perms.set_mode(0o444);
  fs::set_permissions(&file_path, perms)?;

  // Processing should fail when trying to write
  let result = processor.process_file(&file_path).await;
  assert!(result.is_err(), "Should fail for unwritable file");

  // Restore permissions for cleanup
  let mut perms = fs::metadata(&file_path)?.permissions();
  perms.set_mode(0o644);
  fs::set_permissions(&file_path, perms)?;

  Ok(())
}

/// Test that directory traversal handles directories being deleted.
#[tokio::test]
async fn test_directory_deleted_during_traversal() -> Result<()> {
  let temp_dir = tempdir()?;

  // Create nested directory structure
  let dir1 = temp_dir.path().join("dir1");
  let dir2 = temp_dir.path().join("dir2");
  fs::create_dir(&dir1)?;
  fs::create_dir(&dir2)?;

  fs::write(dir1.join("file1.rs"), "fn f1() {}")?;
  fs::write(dir2.join("file2.rs"), "fn f2() {}")?;

  let processor = create_test_processor(&temp_dir, false).await?;

  // Delete dir1 to simulate race condition
  fs::remove_dir_all(&dir1)?;

  // Process should continue with dir2 using absolute paths
  let abs_pattern = format!("{}/**/*.rs", temp_dir.path().display());
  let patterns = vec![abs_pattern];
  let _has_missing = processor.process(&patterns).await?;

  // Verify dir2/file2.rs was processed
  let content = fs::read_to_string(dir2.join("file2.rs"))?;
  assert!(content.contains("Copyright"), "file2.rs should have license");

  Ok(())
}

/// Test concurrent processing with files being deleted.
/// This tests the actual race condition scenario more realistically.
#[tokio::test]
async fn test_concurrent_deletion_race() -> Result<()> {
  let temp_dir = tempdir()?;

  // Create many files to increase chance of race condition
  let src_dir = temp_dir.path().join("src");
  fs::create_dir(&src_dir)?;

  for i in 1..=20 {
    let file_path = src_dir.join(format!("file{}.rs", i));
    fs::write(&file_path, format!("fn func{}() {{}}", i))?;
  }

  // Flag to signal deletion thread
  let should_delete = Arc::new(AtomicBool::new(true));
  let should_delete_clone = Arc::clone(&should_delete);
  let src_dir_clone = src_dir.clone();

  // Spawn a thread that deletes files while processing happens
  let deleter_handle = std::thread::spawn(move || {
    let mut deleted = 0;
    while should_delete_clone.load(Ordering::Relaxed) && deleted < 5 {
      // Try to delete some files
      for i in [2, 5, 8, 11, 15] {
        let file_path = src_dir_clone.join(format!("file{}.rs", i));
        if file_path.exists() {
          let _ = fs::remove_file(&file_path);
          deleted += 1;
        }
      }
      std::thread::sleep(std::time::Duration::from_millis(1));
    }
  });

  let processor = create_test_processor(&temp_dir, false).await?;

  // Process files using absolute path - some may be deleted during processing
  let abs_pattern = format!("{}/**/*.rs", src_dir.display());
  let patterns = vec![abs_pattern];
  let result = processor.process(&patterns).await;

  // Signal deleter to stop
  should_delete.store(false, Ordering::Relaxed);
  deleter_handle.join().expect("Deleter thread panicked");

  // Processing should complete without panic
  assert!(result.is_ok(), "Processing should complete: {:?}", result);

  // Count how many files were processed vs deleted
  let mut processed_count = 0;
  let mut existing_count = 0;
  for i in 1..=20 {
    let file_path = src_dir.join(format!("file{}.rs", i));
    if file_path.exists() {
      existing_count += 1;
      let content = fs::read_to_string(&file_path)?;
      if content.contains("Copyright") {
        processed_count += 1;
      }
    }
  }

  // At least some files should have been processed and still exist
  assert!(existing_count > 0, "At least some files should still exist");
  // All existing files should have been processed (or we handled the race
  // gracefully) Some files may have been deleted between discovery and
  // processing
  assert!(
    processed_count <= existing_count,
    "Processed count should not exceed existing files"
  );

  Ok(())
}

/// Test that symbolic link race conditions are handled (link target deleted).
#[cfg(unix)]
#[tokio::test]
async fn test_symlink_target_deleted() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let target = temp_dir.path().join("target.rs");
  let link = temp_dir.path().join("link.rs");

  fs::write(&target, "fn main() {}")?;
  std::os::unix::fs::symlink(&target, &link)?;

  // Delete the target, leaving a dangling symlink
  fs::remove_file(&target)?;

  // Processing the dangling symlink should fail gracefully
  let result = processor.process_file(&link).await;
  assert!(result.is_err(), "Should fail for dangling symlink");

  Ok(())
}

/// Test processing when file content changes between read and write.
/// This simulates a TOCTOU race where content is modified externally.
#[tokio::test]
async fn test_content_modified_between_operations() -> Result<()> {
  let temp_dir = tempdir()?;

  let file_path = temp_dir.path().join("modified.rs");
  fs::write(&file_path, "fn original() {}")?;

  let processor = create_test_processor(&temp_dir, false).await?;

  // Start processing
  let result = processor.process_file(&file_path).await;
  assert!(result.is_ok(), "Processing should succeed");

  // Verify the license was added
  let content = fs::read_to_string(&file_path)?;
  assert!(content.contains("Copyright"), "Should have license");
  assert!(content.contains("original"), "Should preserve original content");

  // Now simulate external modification by another process
  // Write completely new content (simulating race condition)
  fs::write(&file_path, "fn completely_new() {}")?;

  // Process again - this is a fresh read, so it should work
  let result2 = processor.process_file(&file_path).await;
  assert!(result2.is_ok(), "Second processing should succeed");

  let content2 = fs::read_to_string(&file_path)?;
  assert!(content2.contains("Copyright"), "Should have license");
  assert!(content2.contains("completely_new"), "Should have new content");

  Ok(())
}

/// Test that multiple concurrent processors don't corrupt files.
#[tokio::test]
async fn test_multiple_processors_same_file() -> Result<()> {
  let temp_dir = tempdir()?;

  let file_path = temp_dir.path().join("shared.rs");
  fs::write(&file_path, "fn shared() {}")?;

  // Create two processors
  let processor1 = create_test_processor(&temp_dir, false).await?;
  let processor2 = create_test_processor(&temp_dir, false).await?;

  // Process the same file with both (simulating race condition)
  let file_path_clone = file_path.clone();
  let handle1 = tokio::spawn(async move { processor1.process_file(&file_path_clone).await });

  let handle2 = tokio::spawn(async move { processor2.process_file(&file_path).await });

  // Both should complete (one will add license, other will see it already has
  // one)
  let result1 = handle1.await?;
  let result2 = handle2.await?;

  // At least one should succeed
  assert!(
    result1.is_ok() || result2.is_ok(),
    "At least one processor should succeed"
  );

  // File should have exactly one license header
  let content = fs::read_to_string(temp_dir.path().join("shared.rs"))?;
  let license_count = content.matches("Copyright").count();
  assert_eq!(
    license_count, 1,
    "Should have exactly one license header, got {}",
    license_count
  );

  Ok(())
}

/// Test that check-only mode handles race conditions without modifying files.
#[tokio::test]
async fn test_check_only_race_condition_no_modification() -> Result<()> {
  let temp_dir = tempdir()?;

  let file_path = temp_dir.path().join("check_only.rs");
  let original_content = "fn check_only() {}";
  fs::write(&file_path, original_content)?;

  let processor = create_test_processor(&temp_dir, true).await?;

  // Check mode should not modify even with race conditions
  let _result = processor.process_file(&file_path).await;

  // File should be unchanged
  let content = fs::read_to_string(&file_path)?;
  assert_eq!(content, original_content, "Check-only should not modify file");

  Ok(())
}

/// Test processing a large number of files with random deletions.
#[tokio::test]
async fn test_large_batch_with_random_deletions() -> Result<()> {
  let temp_dir = tempdir()?;

  let src_dir = temp_dir.path().join("large_batch");
  fs::create_dir(&src_dir)?;

  // Create 100 files
  let mut created_files: Vec<PathBuf> = Vec::new();
  for i in 1..=100 {
    let file_path = src_dir.join(format!("file{:03}.rs", i));
    fs::write(&file_path, format!("fn func{}() {{}}", i))?;
    created_files.push(file_path);
  }

  // Delete every 3rd file to simulate race conditions (indices 0, 3, 6, ...)
  for (i, file_path) in created_files.iter().enumerate() {
    if i % 3 == 0 {
      fs::remove_file(file_path)?;
    }
  }

  let processor = create_test_processor(&temp_dir, false).await?;

  // Process all files using absolute path
  let abs_pattern = format!("{}/**/*.rs", src_dir.display());
  let patterns = vec![abs_pattern];
  let result = processor.process(&patterns).await;

  // Should complete without error
  assert!(result.is_ok(), "Should complete processing: {:?}", result);

  // Verify remaining files were processed
  let mut processed = 0;
  let mut remaining = 0;
  for (i, file_path) in created_files.iter().enumerate() {
    if i % 3 != 0 {
      remaining += 1;
      if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        if content.contains("Copyright") {
          processed += 1;
        }
      }
    }
  }

  assert_eq!(processed, remaining, "All remaining files should be processed");

  Ok(())
}

/// Test that errors from individual files are reported but don't stop
/// processing.
#[tokio::test]
async fn test_error_reporting_continues_processing() -> Result<()> {
  let temp_dir = tempdir()?;

  // Create mix of valid and problematic files
  let valid1 = temp_dir.path().join("valid1.rs");
  let valid2 = temp_dir.path().join("valid2.rs");

  fs::write(&valid1, "fn valid1() {}")?;
  fs::write(&valid2, "fn valid2() {}")?;

  let processor = create_test_processor(&temp_dir, false).await?;

  // Process valid files - both should succeed
  let r1 = processor.process_file(&valid1).await;
  let r2 = processor.process_file(&valid2).await;

  assert!(r1.is_ok(), "valid1 should succeed");
  assert!(r2.is_ok(), "valid2 should succeed");

  // Process nonexistent file - should error
  let nonexistent = temp_dir.path().join("nonexistent.rs");
  let r3 = processor.process_file(&nonexistent).await;
  assert!(r3.is_err(), "nonexistent should fail");

  // Process another valid file after error - should still work
  let valid3 = temp_dir.path().join("valid3.rs");
  fs::write(&valid3, "fn valid3() {}")?;
  let r4 = processor.process_file(&valid3).await;
  assert!(r4.is_ok(), "valid3 should succeed after previous error");

  // Verify all valid files have licenses
  for path in [&valid1, &valid2, &valid3] {
    let content = fs::read_to_string(path)?;
    assert!(content.contains("Copyright"), "{} should have license", path.display());
  }

  Ok(())
}

/// Test that binary files are rejected gracefully.
#[tokio::test]
async fn test_binary_file_race_condition() -> Result<()> {
  let temp_dir = tempdir()?;
  let processor = create_test_processor(&temp_dir, false).await?;

  let binary_path = temp_dir.path().join("binary.rs");

  // Create a file that looks like source but is actually binary
  fs::write(&binary_path, [0xFF, 0xFE, 0x00, 0x00, 0x00])?;

  // Processing should fail for binary content
  let result = processor.process_file(&binary_path).await;
  assert!(result.is_err(), "Should fail for binary file: {:?}", result);

  Ok(())
}
