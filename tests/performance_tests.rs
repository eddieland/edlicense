use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::tempdir;

use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};

/// Helper function to create a test processor
async fn create_test_processor(
  template_content: &str,
  ignore_patterns: Vec<String>,
  check_only: bool,
  preserve_years: bool,
  ratchet_reference: Option<String>,
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

  let processor = Processor::new(
    template_manager,
    license_data,
    ignore_patterns,
    check_only,
    preserve_years,
    ratchet_reference,
    None,
    false,
    None, // Use default LicenseDetector
  )?;

  Ok((processor, temp_dir))
}

/// Helper function to generate a large number of test files
fn generate_test_files(dir: &Path, count: usize, with_license: bool, file_size_bytes: usize) -> Result<()> {
  // Create subdirectories to avoid too many files in one directory
  let subdirs_count = (count as f64).sqrt().ceil() as usize;
  let files_per_subdir = count / subdirs_count + 1;

  println!(
    "Generating {} test files across {} subdirectories...",
    count, subdirs_count
  );

  // Generate content for files
  let license_header = if with_license {
    "// Copyright (c) 2024 Test Company\n\n"
  } else {
    ""
  };

  // Generate some dummy content to reach the desired file size
  let content_size = file_size_bytes.saturating_sub(license_header.len());
  let mut content = String::with_capacity(content_size);
  content.push_str("fn main() {\n");

  // Add enough lines to reach the desired size
  let line = "    println!(\"This is a test line for performance testing.\");\n";
  let lines_needed = (content_size as f64 / line.len() as f64).ceil() as usize;

  for _ in 0..lines_needed {
    content.push_str(line);
  }
  content.push_str("}\n");

  // Ensure we're close to the target size
  let file_content = format!("{}{}", license_header, content);

  // Create files in subdirectories
  let mut file_count = 0;
  for i in 0..subdirs_count {
    let subdir = dir.join(format!("subdir_{}", i));
    fs::create_dir_all(&subdir)?;

    for j in 0..files_per_subdir {
      if file_count >= count {
        break;
      }

      let file_path = subdir.join(format!("test_file_{}.rs", j));
      fs::write(&file_path, &file_content)?;
      file_count += 1;

      // Print progress every 1000 files
      if file_count % 1000 == 0 {
        println!("Generated {} files...", file_count);
      }
    }
  }

  println!("Generated {} test files", file_count);
  Ok(())
}

/// Helper function to run a performance test and print results
async fn run_performance_test<F, Fut>(name: &str, test_fn: F) -> Result<Duration>
where
  F: FnOnce() -> Fut,
  Fut: std::future::Future<Output = Result<()>>,
{
  println!("\n=== Running performance test: {} ===", name);

  let start = Instant::now();
  test_fn().await?;
  let duration = start.elapsed();

  println!("Test '{}' completed in {:.2?}", name, duration);
  Ok(duration)
}

/// Performance test for adding licenses to a large number of files
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_add_license_performance() -> Result<()> {
  // Configuration
  let file_count = 10_000;
  let file_size_bytes = 1_000; // 1KB per file

  // Create processor and test directory
  let (processor, temp_dir) =
    create_test_processor("Copyright (c) {{year}} Test Company", vec![], false, false, None).await?;

  // Generate test files without licenses
  let test_dir = temp_dir.path().join("perf_test_add");
  fs::create_dir_all(&test_dir)?;

  println!("Setting up test environment...");
  generate_test_files(&test_dir, file_count, false, file_size_bytes)?;

  // Run the performance test
  run_performance_test("Add License to 10K Files", || async {
    let _ = processor.process_directory(&test_dir).await?;
    Ok(())
  })
  .await?;

  // Verify a sample of files to ensure licenses were added
  let sample_file = test_dir.join("subdir_0").join("test_file_0.rs");
  let content = fs::read_to_string(sample_file)?;
  assert!(content.contains("Copyright (c) 2025 Test Company"));

  Ok(())
}

/// Performance test for updating license years in a large number of files
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_update_year_performance() -> Result<()> {
  // Configuration
  let file_count = 10_000;
  let file_size_bytes = 1_000; // 1KB per file

  // Create processor and test directory
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    false,
    false, // preserve_years = false to ensure years are updated
    None,
  )
  .await?;

  // Generate test files with outdated licenses
  let test_dir = temp_dir.path().join("perf_test_update");
  fs::create_dir_all(&test_dir)?;

  println!("Setting up test environment...");
  generate_test_files(&test_dir, file_count, true, file_size_bytes)?;

  // Run the performance test
  run_performance_test("Update year in 10K Files", || async {
    let _ = processor.process_directory(&test_dir).await?;
    Ok(())
  })
  .await?;

  // Verify a sample of files to ensure years were updated
  let sample_file = test_dir.join("subdir_0").join("test_file_0.rs");
  let content = fs::read_to_string(sample_file)?;
  assert!(content.contains("Copyright (c) 2025 Test Company"));
  assert!(!content.contains("Copyright (c) 2024 Test Company"));

  Ok(())
}

/// Performance test for checking license headers in a large number of files
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_check_license_performance() -> Result<()> {
  // Configuration
  let file_count = 10_000;
  let file_size_bytes = 1_000; // 1KB per file

  // Create processor in check-only mode
  let (processor, temp_dir) = create_test_processor(
    "Copyright (c) {{year}} Test Company",
    vec![],
    true, // check_only = true
    false,
    None,
  )
  .await?;

  // Generate test files with licenses (half with, half without)
  let test_dir = temp_dir.path().join("perf_test_check");
  let with_license_dir = test_dir.join("with_license");
  let without_license_dir = test_dir.join("without_license");

  fs::create_dir_all(&with_license_dir)?;
  fs::create_dir_all(&without_license_dir)?;

  println!("Setting up test environment...");
  generate_test_files(&with_license_dir, file_count / 2, true, file_size_bytes)?;
  generate_test_files(&without_license_dir, file_count / 2, false, file_size_bytes)?;

  // Run the performance test
  let result = run_performance_test("Check License in 10K Files", || async {
    // We expect this to return an error since some files don't have licenses
    let _ = processor.process_directory(&test_dir).await;
    Ok(())
  })
  .await;

  // We expect the test to complete, even if the processor returns an error
  assert!(result.is_ok());

  Ok(())
}

/// Performance test with different file sizes
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_file_size_impact() -> Result<()> {
  // Test with different file sizes
  let file_sizes = [1_000, 10_000, 100_000]; // 1KB, 10KB, 100KB
  let file_count = 1_000; // Use fewer files for larger sizes

  println!("\n=== Testing impact of file size on performance ===");

  for &size in &file_sizes {
    // Create processor and test directory
    let (processor, temp_dir) =
      create_test_processor("Copyright (c) {{year}} Test Company", vec![], false, false, None).await?;

    let test_dir = temp_dir.path().join(format!("size_test_{}", size));
    fs::create_dir_all(&test_dir)?;

    println!("Setting up test for {}KB files...", size / 1_000);
    generate_test_files(&test_dir, file_count, false, size)?;

    let test_name = format!("Process {}KB files ({})", size / 1_000, file_count);
    run_performance_test(&test_name, || async {
      let _ = processor.process_directory(&test_dir).await?;
      Ok(())
    })
    .await?;
  }

  Ok(())
}

/// Performance test with tokio runtime
/// Note: To test different concurrency levels, set the TOKIO_WORKER_THREADS
/// environment variable before running the test
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_tokio_runtime_performance() -> Result<()> {
  // Get the current tokio worker threads setting if available
  let worker_threads = match env::var("TOKIO_WORKER_THREADS") {
    Ok(val) => match val.parse::<usize>() {
      Ok(n) => n,
      Err(_) => 4, // Default to 4 if parsing fails
    },
    Err(_) => 4, // Default to 4 if env var not set
  };

  let file_count = 5_000;
  let file_size = 1_000; // 1KB

  println!("\n=== Testing with tokio worker threads: {} ===", worker_threads);

  // Create processor and test directory
  let (processor, temp_dir) =
    create_test_processor("Copyright (c) {{year}} Test Company", vec![], false, false, None).await?;

  let test_dir = temp_dir.path().join(format!("tokio_test_{}", worker_threads));
  fs::create_dir_all(&test_dir)?;

  println!("Setting up test environment...");
  generate_test_files(&test_dir, file_count, false, file_size)?;

  let test_name = format!("Process with {} worker threads", worker_threads);
  run_performance_test(&test_name, || async {
    let _ = processor.process_directory(&test_dir).await?;
    Ok(())
  })
  .await?;

  Ok(())
}

/// Helper function to generate test files with specific distribution of licenses
fn generate_mixed_test_files(
  dir: &Path,
  total_files: usize,
  percent_missing_license: f64,
  percent_outdated_year: f64,
  file_size_bytes: usize,
) -> Result<()> {
  let missing_license_count = ((total_files as f64) * percent_missing_license / 100.0).round() as usize;
  let outdated_year_count = ((total_files as f64) * percent_outdated_year / 100.0).round() as usize;
  let current_license_count = total_files - missing_license_count - outdated_year_count;

  println!(
    "Generating {} total files: {} missing licenses, {} outdated years, {} current",
    total_files, missing_license_count, outdated_year_count, current_license_count
  );

  // Create subdirectories to avoid too many files in one directory
  let subdirs_count = (total_files as f64).sqrt().ceil() as usize;
  let files_per_subdir = total_files / subdirs_count + 1;

  println!("Distributing files across {} subdirectories...", subdirs_count);

  // Generate content for the different types of files
  let no_license_content = create_file_content("", file_size_bytes);
  let outdated_license_content = create_file_content("// Copyright (c) 2024 Test Company\n\n", file_size_bytes);
  let current_license_content = create_file_content("// Copyright (c) 2025 Test Company\n\n", file_size_bytes);

  // Create files in subdirectories
  let mut file_count = 0;
  let mut missing_license_created = 0;
  let mut outdated_year_created = 0;
  let mut current_license_created = 0;

  for i in 0..subdirs_count {
    let subdir = dir.join(format!("subdir_{}", i));
    fs::create_dir_all(&subdir)?;

    for j in 0..files_per_subdir {
      if file_count >= total_files {
        break;
      }

      let file_path = subdir.join(format!("test_file_{}.rs", j));

      // Determine which type of file to create next
      let content = if missing_license_created < missing_license_count {
        missing_license_created += 1;
        &no_license_content
      } else if outdated_year_created < outdated_year_count {
        outdated_year_created += 1;
        &outdated_license_content
      } else if current_license_created < current_license_count {
        current_license_created += 1;
        &current_license_content
      } else {
        // Fallback - shouldn't happen but just in case
        &no_license_content
      };

      fs::write(&file_path, content)?;
      file_count += 1;

      // Print progress every 1000 files
      if file_count % 1000 == 0 {
        println!("Generated {} files...", file_count);
      }
    }
  }

  println!(
    "Generated {} total files: {} missing licenses, {} outdated years, {} current",
    file_count, missing_license_created, outdated_year_created, current_license_created
  );

  Ok(())
}

/// Helper function to create file content of a specific size with a given header
fn create_file_content(header: &str, file_size_bytes: usize) -> String {
  // Generate some dummy content to reach the desired file size
  let content_size = file_size_bytes.saturating_sub(header.len());
  let mut content = String::with_capacity(file_size_bytes);
  content.push_str(header);
  content.push_str("fn main() {\n");

  // Add enough lines to reach the desired size
  let line = "    println!(\"This is a test line for performance testing.\");\n";
  let lines_needed = (content_size as f64 / line.len() as f64).ceil() as usize;

  for _ in 0..lines_needed {
    content.push_str(line);
  }
  content.push_str("}\n");

  content
}

/// Performance test with realistic repository conditions
/// This test simulates a repository where most files already have correct licenses,
/// and only a small percentage need to be fixed (more typical of real-world usage).
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_realistic_repository_performance() -> Result<()> {
  // Configuration
  let file_count = 10_000;
  let file_size = 1_000; // 1KB

  // We'll simulate repository where:
  // - 1% of files are missing licenses
  // - 1% of files have outdated years
  // - 98% of files have current licenses (no changes needed)
  let percent_missing_license = 1.0;
  let percent_outdated_year = 1.0;

  println!("\n=== Testing performance with realistic repository conditions ===");

  // Create processor and test directory
  let (processor, temp_dir) =
    create_test_processor("Copyright (c) {{year}} Test Company", vec![], false, false, None).await?;

  let test_dir = temp_dir.path().join("realistic_test");
  fs::create_dir_all(&test_dir)?;

  println!("Setting up test environment...");
  generate_mixed_test_files(
    &test_dir,
    file_count,
    percent_missing_license,
    percent_outdated_year,
    file_size,
  )?;

  let test_name = "Realistic repository processing";
  run_performance_test(test_name, || async {
    let _ = processor.process_directory(&test_dir).await?;
    Ok(())
  })
  .await?;

  Ok(())
}
