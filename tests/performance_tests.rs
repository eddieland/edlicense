use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::tempdir;

use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};

/// Helper function to create a test processor
fn create_test_processor(
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
fn run_performance_test<F>(name: &str, test_fn: F) -> Result<Duration>
where
    F: FnOnce() -> Result<()>,
{
    println!("\n=== Running performance test: {} ===", name);

    let start = Instant::now();
    test_fn()?;
    let duration = start.elapsed();

    println!("Test '{}' completed in {:.2?}", name, duration);
    Ok(duration)
}

/// Performance test for adding licenses to a large number of files
#[test]
#[ignore] // Ignore by default as it's a long-running test
fn test_add_license_performance() -> Result<()> {
    // Configuration
    let file_count = 10_000;
    let file_size_bytes = 1_000; // 1KB per file

    // Create processor and test directory
    let (processor, temp_dir) =
        create_test_processor("Copyright (c) {{Year}} Test Company", vec![], false, false, None)?;

    // Generate test files without licenses
    let test_dir = temp_dir.path().join("perf_test_add");
    fs::create_dir_all(&test_dir)?;

    println!("Setting up test environment...");
    generate_test_files(&test_dir, file_count, false, file_size_bytes)?;

    // Run the performance test
    run_performance_test("Add License to 10K Files", || {
        let _ = processor.process_directory(&test_dir)?;
        Ok(())
    })?;

    // Verify a sample of files to ensure licenses were added
    let sample_file = test_dir.join("subdir_0").join("test_file_0.rs");
    let content = fs::read_to_string(sample_file)?;
    assert!(content.contains("Copyright (c) 2025 Test Company"));

    Ok(())
}

/// Performance test for updating license years in a large number of files
#[test]
#[ignore] // Ignore by default as it's a long-running test
fn test_update_year_performance() -> Result<()> {
    // Configuration
    let file_count = 10_000;
    let file_size_bytes = 1_000; // 1KB per file

    // Create processor and test directory
    let (processor, temp_dir) = create_test_processor(
        "Copyright (c) {{Year}} Test Company",
        vec![],
        false,
        false, // preserve_years = false to ensure years are updated
        None,
    )?;

    // Generate test files with outdated licenses
    let test_dir = temp_dir.path().join("perf_test_update");
    fs::create_dir_all(&test_dir)?;

    println!("Setting up test environment...");
    generate_test_files(&test_dir, file_count, true, file_size_bytes)?;

    // Run the performance test
    run_performance_test("Update Year in 10K Files", || {
        let _ = processor.process_directory(&test_dir)?;
        Ok(())
    })?;

    // Verify a sample of files to ensure years were updated
    let sample_file = test_dir.join("subdir_0").join("test_file_0.rs");
    let content = fs::read_to_string(sample_file)?;
    assert!(content.contains("Copyright (c) 2025 Test Company"));
    assert!(!content.contains("Copyright (c) 2024 Test Company"));

    Ok(())
}

/// Performance test for checking license headers in a large number of files
#[test]
#[ignore] // Ignore by default as it's a long-running test
fn test_check_license_performance() -> Result<()> {
    // Configuration
    let file_count = 10_000;
    let file_size_bytes = 1_000; // 1KB per file

    // Create processor in check-only mode
    let (processor, temp_dir) = create_test_processor(
        "Copyright (c) {{Year}} Test Company",
        vec![],
        true, // check_only = true
        false,
        None,
    )?;

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
    let result = run_performance_test("Check License in 10K Files", || {
        // We expect this to return an error since some files don't have licenses
        let _ = processor.process_directory(&test_dir);
        Ok(())
    });

    // We expect the test to complete, even if the processor returns an error
    assert!(result.is_ok());

    Ok(())
}

/// Performance test with different file sizes
#[test]
#[ignore] // Ignore by default as it's a long-running test
fn test_file_size_impact() -> Result<()> {
    // Test with different file sizes
    let file_sizes = [1_000, 10_000, 100_000]; // 1KB, 10KB, 100KB
    let file_count = 1_000; // Use fewer files for larger sizes

    println!("\n=== Testing impact of file size on performance ===");

    for &size in &file_sizes {
        // Create processor and test directory
        let (processor, temp_dir) =
            create_test_processor("Copyright (c) {{Year}} Test Company", vec![], false, false, None)?;

        let test_dir = temp_dir.path().join(format!("size_test_{}", size));
        fs::create_dir_all(&test_dir)?;

        println!("Setting up test for {}KB files...", size / 1_000);
        generate_test_files(&test_dir, file_count, false, size)?;

        let test_name = format!("Process {}KB files ({})", size / 1_000, file_count);
        run_performance_test(&test_name, || {
            let _ = processor.process_directory(&test_dir)?;
            Ok(())
        })?;
    }

    Ok(())
}

/// Performance test with varying thread counts
#[test]
#[ignore] // Ignore by default as it's a long-running test
fn test_thread_count_impact() -> Result<()> {
    use std::env;

    // Test with different thread counts
    let thread_counts = [1, 2, 4, 8, 16];
    let file_count = 5_000;
    let file_size = 1_000; // 1KB

    println!("\n=== Testing impact of thread count on performance ===");

    for &threads in &thread_counts {
        // Set the number of threads for rayon
        // Use unsafe block for environment variable modification
        unsafe {
            env::set_var("RAYON_NUM_THREADS", threads.to_string());
        }

        // Create processor and test directory
        let (processor, temp_dir) =
            create_test_processor("Copyright (c) {{Year}} Test Company", vec![], false, false, None)?;

        let test_dir = temp_dir.path().join(format!("thread_test_{}", threads));
        fs::create_dir_all(&test_dir)?;

        println!("Setting up test for {} threads...", threads);
        generate_test_files(&test_dir, file_count, false, file_size)?;

        let test_name = format!("Process with {} threads", threads);
        run_performance_test(&test_name, || {
            let _ = processor.process_directory(&test_dir)?;
            Ok(())
        })?;
    }

    // Reset the thread count
    // Use unsafe block for environment variable modification
    unsafe {
        env::remove_var("RAYON_NUM_THREADS");
    }

    Ok(())
}

/// Benchmark helper that runs multiple iterations and reports statistics
fn run_benchmark<F>(
    name: &str,
    iterations: usize,
    setup_fn: impl Fn() -> Result<F>,
    test_fn: impl Fn(&F) -> Result<()>,
) -> Result<()> {
    println!("\n=== Benchmark: {} ({} iterations) ===", name, iterations);

    let mut durations = Vec::with_capacity(iterations);

    for i in 1..=iterations {
        println!("Running iteration {}/{}...", i, iterations);

        // Setup
        let test_env = setup_fn()?;

        // Run test and measure time
        let start = Instant::now();
        test_fn(&test_env)?;
        let duration = start.elapsed();

        durations.push(duration);
        println!("Iteration {} completed in {:.2?}", i, duration);
    }

    // Calculate statistics
    if durations.is_empty() {
        return Ok(());
    }

    durations.sort();

    // Calculate total duration manually since sum() is unsafe
    let total = durations.iter().fold(Duration::new(0, 0), |acc, &x| acc + x);
    let avg = total / durations.len() as u32;
    let min = durations.first().unwrap();
    let max = durations.last().unwrap();
    let median = durations[durations.len() / 2];

    // Print results
    println!("\nResults for {}:", name);
    println!("  Iterations: {}", iterations);
    println!("  Average:    {:.2?}", avg);
    println!("  Median:     {:.2?}", median);
    println!("  Min:        {:.2?}", min);
    println!("  Max:        {:.2?}", max);

    Ok(())
}

/// Comprehensive benchmark test
#[test]
#[ignore] // Ignore by default as it's a long-running test
fn benchmark_operations() -> Result<()> {
    // Configuration
    let iterations = 3;
    let file_count = 5_000;
    let file_size = 1_000; // 1KB

    // Benchmark adding licenses
    run_benchmark(
        "Add License",
        iterations,
        || {
            let (processor, temp_dir) =
                create_test_processor("Copyright (c) {{Year}} Test Company", vec![], false, false, None)?;

            let test_dir = temp_dir.path().join("bench_add");
            fs::create_dir_all(&test_dir)?;
            generate_test_files(&test_dir, file_count, false, file_size)?;

            Ok((processor, test_dir))
        },
        |(processor, test_dir)| {
            processor.process_directory(test_dir)?;
            Ok(())
        },
    )?;

    // Benchmark updating years
    run_benchmark(
        "Update Year",
        iterations,
        || {
            let (processor, temp_dir) =
                create_test_processor("Copyright (c) {{Year}} Test Company", vec![], false, false, None)?;

            let test_dir = temp_dir.path().join("bench_update");
            fs::create_dir_all(&test_dir)?;
            generate_test_files(&test_dir, file_count, true, file_size)?;

            Ok((processor, test_dir))
        },
        |(processor, test_dir)| {
            processor.process_directory(test_dir)?;
            Ok(())
        },
    )?;

    // Benchmark check-only mode
    run_benchmark(
        "Check License",
        iterations,
        || {
            let (processor, temp_dir) =
                create_test_processor("Copyright (c) {{Year}} Test Company", vec![], true, false, None)?;

            let test_dir = temp_dir.path().join("bench_check");
            fs::create_dir_all(&test_dir)?;

            // Create half with licenses, half without
            let with_license_dir = test_dir.join("with_license");
            let without_license_dir = test_dir.join("without_license");

            fs::create_dir_all(&with_license_dir)?;
            fs::create_dir_all(&without_license_dir)?;

            generate_test_files(&with_license_dir, file_count / 2, true, file_size)?;
            generate_test_files(&without_license_dir, file_count / 2, false, file_size)?;

            Ok((processor, test_dir))
        },
        |(processor, test_dir)| {
            // We expect this to return an error since some files don't have licenses
            let _ = processor.process_directory(test_dir);
            Ok(())
        },
    )?;

    Ok(())
}
