use anyhow::{Result, anyhow};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
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

/// Helper function to create a test processor with git-only mode
async fn create_test_processor_with_git(
  template_content: &str,
  ignore_patterns: Vec<String>,
  check_only: bool,
  preserve_years: bool,
  ratchet_reference: Option<String>,
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

  let processor = Processor::new(
    template_manager,
    license_data,
    ignore_patterns,
    check_only,
    preserve_years,
    ratchet_reference,
    None,
    git_only,
    None, // Use default LicenseDetector
  )?;

  Ok((processor, temp_dir))
}

fn env_usize(name: &str, default_value: usize) -> usize {
  env::var(name)
    .ok()
    .and_then(|value| value.parse::<usize>().ok())
    .unwrap_or(default_value)
}

fn env_bool(name: &str, default_value: bool) -> bool {
  env::var(name)
    .ok()
    .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
    .unwrap_or(default_value)
}

fn ensure_git_available() -> bool {
  std::process::Command::new("git")
    .args(["--version"])
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

fn run_git(repo_dir: &Path, args: &[&str]) -> Result<()> {
  let output = std::process::Command::new("git")
    .args(args)
    .current_dir(repo_dir)
    .output()?;

  if !output.status.success() {
    return Err(anyhow!(
      "git {:?} failed: {}",
      args,
      String::from_utf8_lossy(&output.stderr)
    ));
  }

  Ok(())
}

fn git_head_sha(repo_dir: &Path) -> Result<String> {
  let output = std::process::Command::new("git")
    .args(["rev-parse", "HEAD"])
    .current_dir(repo_dir)
    .output()?;

  if !output.status.success() {
    return Err(anyhow!(
      "git rev-parse failed: {}",
      String::from_utf8_lossy(&output.stderr)
    ));
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

fn generate_monorepo_files(
  dir: &Path,
  module_count: usize,
  files_per_module: usize,
  file_size_bytes: usize,
  with_license: bool,
) -> Result<Vec<PathBuf>> {
  let license_header = if with_license {
    "// Copyright (c) 2025 Test Company\n\n"
  } else {
    ""
  };

  let file_content = create_file_content(license_header, file_size_bytes);
  let mut files = Vec::with_capacity(module_count * files_per_module);

  for module_index in 0..module_count {
    let module_dir = dir.join(format!("module_{module_index}")).join("src");
    fs::create_dir_all(&module_dir)?;

    for file_index in 0..files_per_module {
      let file_path = module_dir.join(format!("lib_{file_index}.rs"));
      fs::write(&file_path, &file_content)?;
      files.push(file_path);
    }
  }

  Ok(files)
}

fn apply_commit_changes(files: &[PathBuf], commit_index: usize, change_count: usize) -> Result<()> {
  if files.is_empty() || change_count == 0 {
    return Ok(());
  }

  for change_index in 0..change_count {
    let file_index = (commit_index * change_count + change_index) % files.len();
    let mut file = fs::OpenOptions::new().append(true).open(&files[file_index])?;
    writeln!(file, "// commit {commit_index} change {change_index}")?;
  }

  Ok(())
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

/// Performance benchmark for a synthetic large-scale monorepo with git history.
/// Tune with env vars:
/// - MONOREPO_MODULES (default 50)
/// - MONOREPO_FILES_PER_MODULE (default 200)
/// - MONOREPO_FILE_SIZE_BYTES (default 1000)
/// - MONOREPO_HISTORY_COMMITS (default 200)
/// - MONOREPO_CHANGE_PER_COMMIT (default 50)
/// - MONOREPO_RATCHET_COMMITS_BACK (default 20)
/// - MONOREPO_CHECK_ONLY (default true)
#[tokio::test]
#[ignore] // Ignore by default as it's a long-running test
async fn test_monorepo_git_history_benchmark() -> Result<()> {
  if !ensure_git_available() {
    println!("Skipping test_monorepo_git_history_benchmark: git not available");
    return Ok(());
  }

  let module_count = env_usize("MONOREPO_MODULES", 50);
  let files_per_module = env_usize("MONOREPO_FILES_PER_MODULE", 200);
  let file_size_bytes = env_usize("MONOREPO_FILE_SIZE_BYTES", 1_000);
  let history_commits = env_usize("MONOREPO_HISTORY_COMMITS", 200);
  let change_per_commit = env_usize("MONOREPO_CHANGE_PER_COMMIT", 50);
  let ratchet_commits_back = env_usize("MONOREPO_RATCHET_COMMITS_BACK", 20);
  let check_only = env_bool("MONOREPO_CHECK_ONLY", true);

  let temp_dir = tempdir()?;
  let repo_dir = temp_dir.path().join("monorepo");
  fs::create_dir_all(&repo_dir)?;

  run_git(&repo_dir, &["init"])?;
  run_git(&repo_dir, &["config", "user.name", "Perf Test User"])?;
  run_git(&repo_dir, &["config", "user.email", "perf@example.com"])?;
  run_git(&repo_dir, &["config", "init.defaultBranch", "main"])?;

  println!(
    "Generating monorepo: {} modules x {} files ({} bytes each)",
    module_count, files_per_module, file_size_bytes
  );
  let files = generate_monorepo_files(&repo_dir, module_count, files_per_module, file_size_bytes, true)?;

  run_git(&repo_dir, &["add", "."])?;
  run_git(&repo_dir, &["commit", "-m", "Initial import"])?;

  let mut commits = Vec::with_capacity(history_commits + 1);
  commits.push(git_head_sha(&repo_dir)?);

  for commit_index in 0..history_commits {
    apply_commit_changes(&files, commit_index, change_per_commit)?;
    run_git(&repo_dir, &["add", "."])?;
    if change_per_commit == 0 {
      run_git(&repo_dir, &["commit", "--allow-empty", "-m", "Synthetic change"])?;
    } else {
      run_git(&repo_dir, &["commit", "-m", "Synthetic change"])?;
    }
    commits.push(git_head_sha(&repo_dir)?);
  }

  let ratchet_back = ratchet_commits_back.min(commits.len().saturating_sub(1));
  let ratchet_ref = commits
    .get(commits.len().saturating_sub(1 + ratchet_back))
    .cloned()
    .unwrap_or_else(|| commits[0].clone());

  let test_name = format!(
    "Monorepo benchmark ({} modules, {} files/module, {} commits back)",
    module_count, files_per_module, ratchet_back
  );

  let original_dir = std::env::current_dir()?;
  std::env::set_current_dir(&repo_dir)?;

  let (processor, _) = create_test_processor_with_git(
    "Copyright (c) {{year}} Test Company",
    vec![],
    check_only,
    false,
    Some(ratchet_ref),
    true,
  )
  .await?;

  let result = run_performance_test(&test_name, || async {
    let _ = processor.process_directory(&repo_dir).await?;
    Ok(())
  })
  .await;

  std::env::set_current_dir(original_dir)?;

  result?;
  Ok(())
}
