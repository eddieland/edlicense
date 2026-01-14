use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::Result;
use edlicense::logging::set_quiet;
use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
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
    temp_dir.path().to_path_buf(),
    false,
    None, // No extension filter
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

/// Configuration for chaotic file structure generation
#[derive(Debug, Clone)]
struct ChaoticConfig {
  /// Target total file count (approximate)
  target_file_count: usize,
  /// Seed for reproducible randomness
  seed: u64,
  /// Whether to include license headers
  with_license: bool,
}

/// Statistics about generated chaotic structure
#[derive(Debug, Default)]
struct ChaoticStats {
  total_files: usize,
  total_dirs: usize,
  max_depth: usize,
  files_by_extension: std::collections::HashMap<String, usize>,
  empty_dirs: usize,
}

/// Generate a realistic chaotic directory structure mimicking real codebases
fn generate_chaotic_structure(dir: &Path, config: &ChaoticConfig) -> Result<ChaoticStats> {
  let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
  let mut stats = ChaoticStats::default();

  // Realistic directory name patterns
  let root_dirs = [
    "src",
    "lib",
    "pkg",
    "internal",
    "cmd",
    "tests",
    "examples",
    "vendor",
    "third_party",
  ];
  let sub_dirs = [
    "utils",
    "helpers",
    "common",
    "core",
    "api",
    "handlers",
    "models",
    "services",
    "controllers",
    "middleware",
    "config",
    "types",
    "interfaces",
    "impl",
    "proto",
    "generated",
    "mocks",
    "fixtures",
    "testdata",
    "bench",
    "docs",
    "scripts",
  ];
  let deep_dirs = [
    "v1",
    "v2",
    "internal",
    "detail",
    "private",
    "public",
    "experimental",
    "deprecated",
    "legacy",
    "compat",
    "platform",
    "arch",
    "os",
  ];

  // File extensions with realistic weights
  let extensions: Vec<(&str, u32)> = vec![
    ("rs", 30),   // Rust
    ("go", 25),   // Go
    ("js", 20),   // JavaScript
    ("ts", 15),   // TypeScript
    ("py", 15),   // Python
    ("java", 10), // Java
    ("c", 8),     // C
    ("cpp", 8),   // C++
    ("h", 5),     // C headers
    ("hpp", 3),   // C++ headers
    ("rb", 5),    // Ruby
    ("swift", 5), // Swift
    ("kt", 5),    // Kotlin
    ("scala", 3), // Scala
    ("sh", 3),    // Shell
  ];

  // File size distribution (bytes) - weighted towards smaller files
  let file_sizes: Vec<(usize, u32)> = vec![
    (200, 20),   // Tiny (interface files, constants)
    (500, 30),   // Small
    (1000, 25),  // Medium-small (1KB)
    (3000, 15),  // Medium
    (10000, 7),  // Large (10KB)
    (50000, 2),  // Very large
    (100000, 1), // Huge (100KB)
  ];

  // File naming patterns
  let prefixes = ["", "test_", "spec_", "bench_", "mock_", "fake_", "stub_"];
  let base_names = [
    "main",
    "lib",
    "mod",
    "index",
    "init",
    "utils",
    "helpers",
    "common",
    "client",
    "server",
    "handler",
    "service",
    "repository",
    "model",
    "controller",
    "middleware",
    "config",
    "types",
    "errors",
    "constants",
    "factory",
    "builder",
    "adapter",
    "proxy",
    "decorator",
    "observer",
    "strategy",
    "command",
    "state",
    "visitor",
    "iterator",
    "mediator",
  ];

  println!(
    "Generating chaotic file structure (target: {} files, seed: {})...",
    config.target_file_count, config.seed
  );

  let mut files_created = 0;

  // Keep generating until we hit the target - loop through root dirs multiple
  // times if needed
  let mut round = 0;
  while files_created < config.target_file_count {
    round += 1;

    // Create root-level directories with varying structures
    let num_root_dirs = rng.random_range(3..=root_dirs.len());
    let selected_roots: Vec<_> = root_dirs.choose_multiple(&mut rng, num_root_dirs).collect();

    for root_name in selected_roots {
      if files_created >= config.target_file_count {
        break;
      }

      // Add round suffix to avoid conflicts in subsequent rounds
      let root_path = if round == 1 {
        dir.join(*root_name)
      } else {
        dir.join(format!("{}_{}", root_name, round))
      };
      fs::create_dir_all(&root_path)?;
      stats.total_dirs += 1;

      // Each root gets a different "personality"
      let personality = rng.random_range(0..4);

      // Scale file counts based on how many we still need
      let remaining = config.target_file_count - files_created;
      let scale_factor = (remaining as f64 / 1000.0).max(1.0).min(10.0);

      match personality {
        0 => {
          // Flat structure with many files
          let file_count = (rng.random_range(50..=200) as f64 * scale_factor) as usize;
          let file_count = file_count.min(remaining);
          files_created += create_files_in_dir(
            &root_path,
            file_count,
            &extensions,
            &file_sizes,
            &prefixes,
            &base_names,
            config.with_license,
            &mut rng,
            &mut stats,
          )?;
        }
        1 => {
          // Deep nesting with files at each level
          let depth = rng.random_range(3..=8);
          let files_per_level = (rng.random_range(10..=50) as f64 * scale_factor) as usize;
          files_created += create_deep_structure(
            &root_path,
            depth,
            &sub_dirs,
            &deep_dirs,
            &extensions,
            &file_sizes,
            &prefixes,
            &base_names,
            files_per_level.min(remaining),
            config.with_license,
            &mut rng,
            &mut stats,
          )?;
        }
        2 => {
          // Moderate nesting with varying file counts - create more subdirs
          let num_subdirs = rng.random_range(10..=30);
          for subdir_idx in 0..num_subdirs {
            if files_created >= config.target_file_count {
              break;
            }
            let subdir_name = sub_dirs.choose(&mut rng).unwrap_or(&"misc");
            let subdir_path = root_path.join(format!("{}_{}", subdir_name, subdir_idx));
            fs::create_dir_all(&subdir_path)?;
            stats.total_dirs += 1;

            // Varying files per subdir (some have many, some have few)
            let files_here = if rng.random_bool(0.3) {
              rng.random_range(100..=300) // More "fat" directories
            } else {
              rng.random_range(10..=50)
            };
            let files_here = files_here.min(config.target_file_count - files_created);
            files_created += create_files_in_dir(
              &subdir_path,
              files_here,
              &extensions,
              &file_sizes,
              &prefixes,
              &base_names,
              config.with_license,
              &mut rng,
              &mut stats,
            )?;

            // Maybe add deeper nesting
            if rng.random_bool(0.3) {
              let extra_depth = rng.random_range(2..=5);
              files_created += create_deep_structure(
                &subdir_path,
                extra_depth,
                &sub_dirs,
                &deep_dirs,
                &extensions,
                &file_sizes,
                &prefixes,
                &base_names,
                (config.target_file_count - files_created).min(100),
                config.with_license,
                &mut rng,
                &mut stats,
              )?;
            }
          }
        }
        _ => {
          // Mixed: some files at root, many subdirs with varying structures
          let root_files = rng.random_range(20..=80).min(config.target_file_count - files_created);
          files_created += create_files_in_dir(
            &root_path,
            root_files,
            &extensions,
            &file_sizes,
            &prefixes,
            &base_names,
            config.with_license,
            &mut rng,
            &mut stats,
          )?;

          let num_subdirs = rng.random_range(5..=20);
          for i in 0..num_subdirs {
            if files_created >= config.target_file_count {
              break;
            }
            let subdir_name = if rng.random_bool(0.5) {
              sub_dirs.choose(&mut rng).unwrap_or(&"misc").to_string()
            } else {
              format!("module_{}", i)
            };
            let subdir_path = root_path.join(&subdir_name);
            fs::create_dir_all(&subdir_path)?;
            stats.total_dirs += 1;

            let files_here = rng.random_range(20..=100).min(config.target_file_count - files_created);
            files_created += create_files_in_dir(
              &subdir_path,
              files_here,
              &extensions,
              &file_sizes,
              &prefixes,
              &base_names,
              config.with_license,
              &mut rng,
              &mut stats,
            )?;
          }
        }
      }

      // Occasionally add empty directories
      if rng.random_bool(0.1) {
        let empty_dir = root_path.join(format!(
          ".{}",
          ["cache", "tmp", "build", "output"].choose(&mut rng).unwrap_or(&"empty")
        ));
        fs::create_dir_all(&empty_dir)?;
        stats.total_dirs += 1;
        stats.empty_dirs += 1;
      }
    }
  }

  // Add some files at the root level (like a real project)
  let root_files = rng.random_range(1..=10);
  files_created += create_files_in_dir(
    dir,
    root_files,
    &extensions,
    &file_sizes,
    &prefixes,
    &base_names,
    config.with_license,
    &mut rng,
    &mut stats,
  )?;

  stats.total_files = files_created;

  println!("Generated chaotic structure:");
  println!("  Total files: {}", stats.total_files);
  println!("  Total directories: {}", stats.total_dirs);
  println!("  Max depth: {}", stats.max_depth);
  println!("  Empty directories: {}", stats.empty_dirs);
  println!("  Files by extension (top 5): {:?}", {
    let mut ext_vec: Vec<_> = stats.files_by_extension.iter().collect();
    ext_vec.sort_by(|a, b| b.1.cmp(a.1));
    ext_vec.into_iter().take(5).collect::<Vec<_>>()
  });

  Ok(stats)
}

/// Create files in a directory with random properties
#[allow(clippy::too_many_arguments)]
fn create_files_in_dir(
  dir: &Path,
  count: usize,
  extensions: &[(&str, u32)],
  file_sizes: &[(usize, u32)],
  prefixes: &[&str],
  base_names: &[&str],
  with_license: bool,
  rng: &mut ChaCha8Rng,
  stats: &mut ChaoticStats,
) -> Result<usize> {
  let mut created = 0;

  // Track used names to avoid collisions
  let mut used_names = std::collections::HashSet::new();

  for _ in 0..count {
    // Pick random extension (weighted)
    let ext = weighted_choice(extensions, rng);

    // Pick random file size (weighted)
    let size = *weighted_choice(file_sizes, rng);

    // Generate filename
    let prefix = prefixes.choose(rng).unwrap_or(&"");
    let base = base_names.choose(rng).unwrap_or(&"file");
    let suffix = if rng.random_bool(0.3) {
      format!("_{}", rng.random_range(0..1000u32))
    } else {
      String::new()
    };

    let filename = format!("{}{}{}.{}", prefix, base, suffix, ext);

    // Avoid collisions
    if used_names.contains(&filename) {
      continue;
    }
    used_names.insert(filename.clone());

    let file_path = dir.join(&filename);
    let content = generate_file_content(*ext, size, with_license, rng);
    fs::write(&file_path, content)?;

    *stats.files_by_extension.entry(ext.to_string()).or_insert(0) += 1;
    created += 1;
  }

  Ok(created)
}

/// Create a deeply nested structure
#[allow(clippy::too_many_arguments)]
fn create_deep_structure(
  base: &Path,
  max_depth: usize,
  sub_dirs: &[&str],
  deep_dirs: &[&str],
  extensions: &[(&str, u32)],
  file_sizes: &[(usize, u32)],
  prefixes: &[&str],
  base_names: &[&str],
  max_files: usize,
  with_license: bool,
  rng: &mut ChaCha8Rng,
  stats: &mut ChaoticStats,
) -> Result<usize> {
  let mut files_created = 0;
  let mut current_path = base.to_path_buf();

  for depth in 0..max_depth {
    if files_created >= max_files {
      break;
    }

    // Pick directory name
    let dir_name = if depth < 2 {
      sub_dirs.choose(rng).unwrap_or(&"sub")
    } else {
      deep_dirs.choose(rng).unwrap_or(&"inner")
    };

    // Sometimes add numeric suffix
    let dir_name = if rng.random_bool(0.3) {
      format!("{}_{}", dir_name, rng.random_range(0..10u32))
    } else {
      dir_name.to_string()
    };

    current_path = current_path.join(&dir_name);
    fs::create_dir_all(&current_path)?;
    stats.total_dirs += 1;
    stats.max_depth = stats.max_depth.max(depth + 1);

    // Add files at this level (fewer files in deeper directories)
    let files_here = if depth < 3 {
      rng.random_range(2..=10)
    } else {
      rng.random_range(1..=3)
    };
    let files_here = files_here.min(max_files - files_created);

    files_created += create_files_in_dir(
      &current_path,
      files_here,
      extensions,
      file_sizes,
      prefixes,
      base_names,
      with_license,
      rng,
      stats,
    )?;

    // Sometimes branch out
    if rng.random_bool(0.2) && max_depth > 2 && depth < max_depth - 2 {
      let sibling_name = deep_dirs.choose(rng).unwrap_or(&"branch");
      let sibling_path = current_path.parent().map(|p| p.join(sibling_name));
      if let Some(sibling) = sibling_path {
        if !sibling.exists() {
          fs::create_dir_all(&sibling)?;
          stats.total_dirs += 1;
          let sibling_files = rng.random_range(1..=5).min(max_files - files_created);
          files_created += create_files_in_dir(
            &sibling,
            sibling_files,
            extensions,
            file_sizes,
            prefixes,
            base_names,
            with_license,
            rng,
            stats,
          )?;
        }
      }
    }
  }

  Ok(files_created)
}

/// Generate realistic file content for a given extension
fn generate_file_content(ext: &str, target_size: usize, with_license: bool, rng: &mut ChaCha8Rng) -> String {
  let license = if with_license {
    match ext {
      "rs" | "go" | "c" | "cpp" | "h" | "hpp" | "java" | "kt" | "scala" | "swift" | "js" | "ts" => {
        "// Copyright (c) 2024 Test Company\n// Licensed under MIT\n\n"
      }
      "py" | "rb" | "sh" => "# Copyright (c) 2024 Test Company\n# Licensed under MIT\n\n",
      _ => "// Copyright (c) 2024 Test Company\n\n",
    }
  } else {
    ""
  };

  let content_size = target_size.saturating_sub(license.len());
  let mut content = String::with_capacity(target_size);
  content.push_str(license);

  // Generate language-appropriate boilerplate
  match ext {
    "rs" => {
      content.push_str("use std::io::Result;\n\n");
      content.push_str("pub fn process() -> Result<()> {\n");
      fill_with_code(&mut content, content_size, "    // Processing logic\n", rng);
      content.push_str("    Ok(())\n}\n");
    }
    "go" => {
      content.push_str("package main\n\nimport \"fmt\"\n\n");
      content.push_str("func Process() error {\n");
      fill_with_code(&mut content, content_size, "\t// Processing logic\n", rng);
      content.push_str("\treturn nil\n}\n");
    }
    "py" => {
      content.push_str("\"\"\"Module docstring.\"\"\"\n\n");
      content.push_str("def process():\n");
      fill_with_code(&mut content, content_size, "    # Processing logic\n", rng);
      content.push_str("    pass\n");
    }
    "js" | "ts" => {
      content.push_str("'use strict';\n\n");
      content.push_str("function process() {\n");
      fill_with_code(&mut content, content_size, "  // Processing logic\n", rng);
      content.push_str("}\n\nmodule.exports = { process };\n");
    }
    "java" => {
      content.push_str("package com.example;\n\n");
      content.push_str("public class Processor {\n");
      content.push_str("    public void process() {\n");
      fill_with_code(&mut content, content_size, "        // Processing logic\n", rng);
      content.push_str("    }\n}\n");
    }
    "c" | "cpp" => {
      content.push_str("#include <stdio.h>\n\n");
      content.push_str("int process() {\n");
      fill_with_code(&mut content, content_size, "    /* Processing logic */\n", rng);
      content.push_str("    return 0;\n}\n");
    }
    _ => {
      fill_with_code(&mut content, content_size, "// Generic code line\n", rng);
    }
  }

  content
}

/// Fill content with code-like lines until target size
fn fill_with_code(content: &mut String, target_size: usize, line_template: &str, rng: &mut ChaCha8Rng) {
  let variations = [
    "    let x = 42;\n",
    "    let result = compute();\n",
    "    // TODO: implement\n",
    "    debug_assert!(true);\n",
    "    if condition { return; }\n",
    "    for i in 0..10 { process(i); }\n",
    "    match value { _ => {} }\n",
  ];

  while content.len() < target_size {
    if rng.random_bool(0.7) {
      content.push_str(line_template);
    } else {
      content.push_str(variations.choose(rng).unwrap_or(&line_template));
    }
  }
}

/// Weighted random choice helper
fn weighted_choice<'a, T>(choices: &'a [(T, u32)], rng: &mut ChaCha8Rng) -> &'a T {
  let total: u32 = choices.iter().map(|(_, w)| w).sum();
  let mut pick = rng.random_range(0..total);

  for (item, weight) in choices {
    if pick < *weight {
      return item;
    }
    pick -= weight;
  }

  &choices[0].0
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
    let mut cmd = Command::new("addlicense");

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

  set_quiet();

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

/// Extended benchmark result for chaotic scenarios
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChaoticBenchmarkResult {
  tool: String,
  operation: String,
  duration_ms: u128,
  file_count: usize,
  dir_count: usize,
  max_depth: usize,
  seed: u64,
  scenario: String,
}

/// Run chaotic benchmark for a specific operation
async fn run_chaotic_edlicense_benchmark(
  operation: &str,
  test_dir: &Path,
  check_only: bool,
  stats: &ChaoticStats,
  seed: u64,
  scenario: &str,
  iterations: usize,
) -> Result<Vec<ChaoticBenchmarkResult>> {
  println!(
    "\n=== Running edlicense chaotic benchmark: {} ({}) ===",
    operation, scenario
  );

  let mut results = Vec::with_capacity(iterations);

  for i in 1..=iterations {
    println!("Running iteration {}/{}...", i, iterations);

    let (processor, _template_dir) =
      create_test_processor("Copyright (c) {{year}} Test Company", vec![], check_only, false, None).await?;

    let start = Instant::now();
    processor.process_directory(test_dir).await?;
    let duration = start.elapsed();

    let result = ChaoticBenchmarkResult {
      tool: "edlicense".to_string(),
      operation: operation.to_string(),
      duration_ms: duration.as_millis(),
      file_count: stats.total_files,
      dir_count: stats.total_dirs,
      max_depth: stats.max_depth,
      seed,
      scenario: scenario.to_string(),
    };

    results.push(result);
    println!("Iteration {} completed in {:.2?}", i, duration);
  }

  Ok(results)
}

/// Run chaotic addlicense benchmark
fn run_chaotic_addlicense_benchmark(
  operation: &str,
  test_dir: &Path,
  check_only: bool,
  stats: &ChaoticStats,
  seed: u64,
  scenario: &str,
  iterations: usize,
) -> Result<Vec<ChaoticBenchmarkResult>> {
  if operation == "update" {
    println!("\n=== Skipping addlicense chaotic benchmark for update (unsupported) ===");
    return Ok(vec![]);
  }

  println!(
    "\n=== Running addlicense chaotic benchmark: {} ({}) ===",
    operation, scenario
  );

  let mut results = Vec::with_capacity(iterations);

  for i in 1..=iterations {
    println!("Running iteration {}/{}...", i, iterations);

    let mut cmd = Command::new("addlicense");

    if check_only {
      cmd.arg("-check");
    }

    cmd.arg("-c").arg("Test Company");
    cmd.arg("-y").arg("2025");
    cmd.arg("-l").arg("apache");
    cmd.arg(test_dir.to_str().unwrap());

    let start = Instant::now();
    let status = cmd.status()?;
    let duration = start.elapsed();

    if !status.success() && !check_only && operation != "check" {
      println!("Warning: addlicense command returned non-zero status: {}", status);
    }

    let result = ChaoticBenchmarkResult {
      tool: "addlicense".to_string(),
      operation: operation.to_string(),
      duration_ms: duration.as_millis(),
      file_count: stats.total_files,
      dir_count: stats.total_dirs,
      max_depth: stats.max_depth,
      seed,
      scenario: scenario.to_string(),
    };

    results.push(result);
    println!("Iteration {} completed in {:.2?}", i, duration);
  }

  Ok(results)
}

/// Chaotic benchmark test with realistic, irregular file structures
#[test]
#[ignore] // Ignored by default as it's a long-running test
fn chaotic_benchmark() -> Result<()> {
  set_quiet();

  let output_dir = PathBuf::from("target/benchmark_results");
  fs::create_dir_all(&output_dir)?;

  let iterations = 3;

  // Different chaotic scenarios with different seeds for variety
  let scenarios = [
    ("small_chaos", 5000, 42),     // ~5k files, seed 42
    ("medium_chaos", 10000, 123),  // ~10k files, seed 123
    ("large_chaos", 20000, 456),   // ~20k files, seed 456
    ("deep_nest", 3000, 789),      // Fewer files but likely deeper nesting
    ("varied_seed_a", 8000, 1001), // Same size, different seeds to test variance
    ("varied_seed_b", 8000, 2002),
  ];

  let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

  let mut all_results: Vec<ChaoticBenchmarkResult> = Vec::new();

  for (scenario_name, target_files, seed) in scenarios {
    println!("\n{}", "=".repeat(60));
    println!(
      "=== Chaotic Scenario: {} (target: {} files, seed: {}) ===",
      scenario_name, target_files, seed
    );
    println!("{}", "=".repeat(60));

    let operation_configs = [("add", false, false), ("update", true, false), ("check", true, true)];

    for (operation, with_license, check_only) in operation_configs {
      let temp_dir = tempdir()?;
      let base_dir = temp_dir.path().join(format!("chaotic_{}_{}", scenario_name, operation));
      let edlicense_dir = temp_dir
        .path()
        .join(format!("edlicense_{}_{}", scenario_name, operation));
      let addlicense_dir = temp_dir
        .path()
        .join(format!("addlicense_{}_{}", scenario_name, operation));

      fs::create_dir_all(&base_dir)?;

      // Generate chaotic structure
      let config = ChaoticConfig {
        target_file_count: target_files,
        seed,
        with_license,
      };

      let stats = generate_chaotic_structure(&base_dir, &config)?;

      // Copy for isolated runs
      copy_dir_recursive(&base_dir, &edlicense_dir)?;
      copy_dir_recursive(&base_dir, &addlicense_dir)?;

      // Run benchmarks
      let edlicense_results = runtime.block_on(async {
        run_chaotic_edlicense_benchmark(
          operation,
          &edlicense_dir,
          check_only,
          &stats,
          seed,
          scenario_name,
          iterations,
        )
        .await
      })?;

      let addlicense_results = run_chaotic_addlicense_benchmark(
        operation,
        &addlicense_dir,
        check_only,
        &stats,
        seed,
        scenario_name,
        iterations,
      )?;

      all_results.extend(edlicense_results);
      all_results.extend(addlicense_results);
    }
  }

  // Write all chaotic results
  let output_file = output_dir.join("benchmark_chaotic.json");
  let json = serde_json::to_string_pretty(&all_results)?;
  fs::write(&output_file, json)?;
  println!("\nChaotic benchmark results written to {}", output_file.display());

  // Print summary
  println!("\n=== Chaotic Benchmark Summary ===");
  println!(
    "{:<20} {:<10} {:<12} {:<12} {:<10}",
    "Scenario", "Operation", "edlicense", "addlicense", "Speedup"
  );
  println!("{}", "-".repeat(64));

  for scenario in [
    "small_chaos",
    "medium_chaos",
    "large_chaos",
    "deep_nest",
    "varied_seed_a",
    "varied_seed_b",
  ] {
    for operation in ["add", "check"] {
      let ed_times: Vec<u128> = all_results
        .iter()
        .filter(|r| r.scenario == scenario && r.operation == operation && r.tool == "edlicense")
        .map(|r| r.duration_ms)
        .collect();
      let add_times: Vec<u128> = all_results
        .iter()
        .filter(|r| r.scenario == scenario && r.operation == operation && r.tool == "addlicense")
        .map(|r| r.duration_ms)
        .collect();

      if !ed_times.is_empty() {
        let ed_avg = ed_times.iter().sum::<u128>() as f64 / ed_times.len() as f64;
        let add_avg = if !add_times.is_empty() {
          add_times.iter().sum::<u128>() as f64 / add_times.len() as f64
        } else {
          0.0
        };

        let speedup = if add_avg > 0.0 { add_avg / ed_avg } else { 0.0 };

        println!(
          "{:<20} {:<10} {:<12.1}ms {:<12}ms {:<10.2}x",
          scenario,
          operation,
          ed_avg,
          if add_avg > 0.0 {
            format!("{:.1}", add_avg)
          } else {
            "N/A".to_string()
          },
          if speedup > 0.0 { speedup } else { 0.0 }
        );
      }
    }
  }

  Ok(())
}
