use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::Result;
use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

/// Comparative Benchmarking test suite
/// Compares edlicense performance against Google's addlicense

/// Test configuration struct
#[derive(Debug)]
struct BenchmarkConfig {
  file_count: usize,
  file_size_bytes: usize,
  iterations: usize,
  include_addlicense: bool,
}

/// Result of a single benchmark run
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BenchmarkResult {
  tool: String,
  operation: String,
  duration_ms: u128,
  file_count: usize,
  file_size_kb: usize,
  thread_count: Option<usize>,
}

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

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
  fs::create_dir_all(dest)?;

  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let path = entry.path();
    let dest_path = dest.join(entry.file_name());

    if path.is_dir() {
      copy_dir_recursive(&path, &dest_path)?;
    } else {
      fs::copy(&path, &dest_path)?;
    }
  }

  Ok(())
}

/// Run an edlicense benchmark
async fn run_edlicense_benchmark(
  operation: &str,
  test_dir: &Path,
  check_only: bool,
  config: &BenchmarkConfig,
) -> Result<Vec<BenchmarkResult>> {
  println!("\n=== Running edlicense benchmark: {} ===", operation);

  let mut results = Vec::with_capacity(config.iterations);

  for i in 1..=config.iterations {
    println!("Running iteration {}/{}...", i, config.iterations);

    // Create processor for benchmark
    let (processor, _template_dir) =
      create_test_processor("Copyright (c) {{year}} Test Company", vec![], check_only, false, None).await?;

    // Run benchmark and measure time
    let start = Instant::now();
    processor.process_directory(test_dir).await?;
    let duration = start.elapsed();

    let result = BenchmarkResult {
      tool: "edlicense".to_string(),
      operation: operation.to_string(),
      duration_ms: duration.as_millis(),
      file_count: config.file_count,
      file_size_kb: config.file_size_bytes / 1000,
      thread_count: None,
    };

    results.push(result.clone());
    println!("Iteration {} completed in {:.2?}", i, duration);
  }

  Ok(results)
}

/// Run an addlicense benchmark
fn run_addlicense_benchmark(
  operation: &str,
  test_dir: &Path,
  check_only: bool,
  config: &BenchmarkConfig,
) -> Result<Vec<BenchmarkResult>> {
  if operation == "update" {
    println!("\n=== Skipping addlicense benchmark for update (unsupported) ===");
    return Ok(vec![]);
  }

  if !config.include_addlicense {
    println!("\n=== Skipping addlicense benchmark (disabled) ===");
    return Ok(vec![]);
  }

  println!("\n=== Running addlicense benchmark: {} ===", operation);

  let mut results = Vec::with_capacity(config.iterations);

  for i in 1..=config.iterations {
    println!("Running iteration {}/{}...", i, config.iterations);

    // Build command based on operation
    let mut cmd = Command::new("/home/ejones/go/bin/addlicense");

    if check_only {
      cmd.arg("-check");
    }

    cmd.arg("-c").arg("Test Company");
    cmd.arg("-y").arg("2025");
    cmd.arg("-l").arg("apache");
    cmd.arg(test_dir.to_str().unwrap());

    // Run benchmark and measure time
    let start = Instant::now();
    let status = cmd.status()?;
    let duration = start.elapsed();

    if !status.success() && !check_only {
      // It's OK for check to fail (expected if files don't have licenses)
      if operation != "check" {
        println!("Warning: addlicense command returned non-zero status: {}", status);
      }
    }

    let result = BenchmarkResult {
      tool: "addlicense".to_string(),
      operation: operation.to_string(),
      duration_ms: duration.as_millis(),
      file_count: config.file_count,
      file_size_kb: config.file_size_bytes / 1000,
      thread_count: None,
    };

    results.push(result.clone());
    println!("Iteration {} completed in {:.2?}", i, duration);
  }

  Ok(results)
}

/// Write benchmark results to a JSON file
fn write_benchmark_results(results: &[BenchmarkResult], output_file: &Path) -> Result<()> {
  let json = serde_json::to_string_pretty(&results)?;
  fs::write(output_file, json)?;
  println!("Results written to {}", output_file.display());
  Ok(())
}

/// Run benchmarks for a specific file size
async fn run_file_size_benchmarks(file_size: usize, config: &BenchmarkConfig, output_dir: &Path) -> Result<()> {
  let operation_configs = [
    ("add", false, false),   // (operation name, with_license, check_only)
    ("update", true, false), // Update existing licenses
    ("check", true, true),   // Check mode with existing licenses
  ];

  for (operation, with_license, check_only) in operation_configs {
    // Create a dedicated temp directory for this test
    let temp_dir = tempdir()?;
    let base_dir = temp_dir.path().join(format!("bench_src_{}_{}", operation, file_size));
    let edlicense_dir = temp_dir
      .path()
      .join(format!("bench_edlicense_{}_{}", operation, file_size));
    let addlicense_dir = temp_dir
      .path()
      .join(format!("bench_addlicense_{}_{}", operation, file_size));

    fs::create_dir_all(&base_dir)?;

    // Generate test files
    generate_test_files(&base_dir, config.file_count, with_license, file_size)?;

    // Keep tool runs isolated to avoid one run mutating the other's inputs.
    copy_dir_recursive(&base_dir, &edlicense_dir)?;
    copy_dir_recursive(&base_dir, &addlicense_dir)?;

    // Run edlicense benchmark
    let edlicense_results = run_edlicense_benchmark(operation, &edlicense_dir, check_only, config).await?;

    // Run addlicense benchmark
    let addlicense_results = run_addlicense_benchmark(operation, &addlicense_dir, check_only, config)?;

    // Combine results and write to file
    let mut all_results = Vec::new();
    all_results.extend(edlicense_results);
    all_results.extend(addlicense_results);

    let output_file = output_dir.join(format!("benchmark_{}_{}kb.json", operation, file_size / 1000));
    write_benchmark_results(&all_results, &output_file)?;
  }

  Ok(())
}

/// Run thread count impact benchmarks
fn run_thread_count_benchmarks(config: &BenchmarkConfig, output_dir: &Path) -> Result<()> {
  // Thread counts to test
  let thread_counts = [1, 2, 4, 8, 16];
  let file_size = 1000; // 1KB
  let mut all_results = Vec::new();

  println!("\n=== Testing impact of thread count on performance ===");

  for &threads in &thread_counts {
    let runtime = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(threads)
      .enable_all()
      .build()?;

    let mut results = runtime.block_on(async {
      // Create a dedicated temp directory for this test
      let temp_dir = tempdir()?;
      let test_dir = temp_dir.path().join(format!("thread_test_{}", threads));
      fs::create_dir_all(&test_dir)?;

      // Generate test files
      generate_test_files(&test_dir, config.file_count, false, file_size)?;

      // Run edlicense benchmark
      println!("\n=== Running edlicense benchmark with {} threads ===", threads);

      let mut results = Vec::with_capacity(config.iterations);

      for i in 1..=config.iterations {
        println!("Running iteration {}/{}...", i, config.iterations);

        // Create processor for benchmark
        let (processor, _template_dir) =
          create_test_processor("Copyright (c) {{year}} Test Company", vec![], false, false, None).await?;

        // Run benchmark and measure time
        let start = Instant::now();
        processor.process_directory_with_concurrency(&test_dir, threads).await?;
        let duration = start.elapsed();

        let result = BenchmarkResult {
          tool: "edlicense".to_string(),
          operation: "add".to_string(),
          duration_ms: duration.as_millis(),
          file_count: config.file_count,
          file_size_kb: file_size / 1000,
          thread_count: Some(threads),
        };

        results.push(result);
        println!("Iteration {} completed in {:.2?}", i, duration);
      }

      Ok::<Vec<BenchmarkResult>, anyhow::Error>(results)
    })?;

    all_results.append(&mut results);
  }

  // Write results
  let output_file = output_dir.join("benchmark_thread_impact.json");
  write_benchmark_results(&all_results, &output_file)?;

  Ok(())
}

/// Main benchmark test function to compare edlicense vs addlicense
#[test]
#[ignore] // Ignored by default as it's a long-running test
fn comparative_benchmark() -> Result<()> {
  use std::fs;

  // Create output directory for benchmark results
  let output_dir = PathBuf::from("target/benchmark_results");
  fs::create_dir_all(&output_dir)?;

  // Benchmark configuration
  let small_config = BenchmarkConfig {
    file_count: 10000,
    file_size_bytes: 1000, // 1KB
    iterations: 3,
    include_addlicense: true,
  };

  let medium_config = BenchmarkConfig {
    file_count: 5000,
    file_size_bytes: 10_000, // 10KB
    iterations: 3,
    include_addlicense: true,
  };

  let large_config = BenchmarkConfig {
    file_count: 1000,
    file_size_bytes: 100_000, // 100KB
    iterations: 3,
    include_addlicense: true,
  };

  let thread_config = BenchmarkConfig {
    file_count: 10000,
    file_size_bytes: 1000, // 1KB
    iterations: 3,
    include_addlicense: false, // addlicense doesn't have configurable thread count
  };

  println!("=== Running File Size Impact Benchmarks ===");

  let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

  // Run benchmarks for different file sizes
  runtime.block_on(async {
    println!("Running small file benchmarks (1KB)");
    run_file_size_benchmarks(1000, &small_config, &output_dir).await?;

    println!("Running medium file benchmarks (10KB)");
    run_file_size_benchmarks(10_000, &medium_config, &output_dir).await?;

    println!("Running large file benchmarks (100KB)");
    run_file_size_benchmarks(100_000, &large_config, &output_dir).await?;

    Ok::<(), anyhow::Error>(())
  })?;

  // Run thread count benchmarks (edlicense only)
  println!("Running thread count impact benchmarks");
  run_thread_count_benchmarks(&thread_config, &output_dir)?;

  println!("All benchmarks completed. Results saved to: {}", output_dir.display());
  Ok(())
}
