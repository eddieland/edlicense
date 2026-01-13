//! # Tree Command
//!
//! This module implements the tree command that lists files that would be
//! checked for license headers, based on filtering rules.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Args;

use crate::file_filter::{FileFilter, IgnoreFilter, create_default_filter};
use crate::git;
use crate::ignore::IgnoreManager;
use crate::logging::{ColorMode, set_quiet, set_verbose};
use crate::workspace::resolve_workspace;
use crate::{info_log, verbose_log};

/// Arguments for the tree command
#[derive(Args, Debug, Default)]
pub struct TreeArgs {
  /// File or directory patterns to process. Directories are processed
  /// recursively.
  #[arg(required = false)]
  pub patterns: Vec<String>,

  /// File patterns to ignore (supports glob patterns)
  #[arg(long, short = 'i')]
  pub ignore: Vec<String>,

  /// Verbose mode: show why files are skipped
  #[arg(long, short = 'v')]
  pub verbose: bool,

  /// Quiet mode: suppress all output except file paths
  #[arg(long, short = 'q')]
  pub quiet: bool,

  /// Ratchet mode: only list files that have changed relative to a git
  /// reference
  #[arg(long, value_name = "REF")]
  pub ratchet: Option<String>,

  /// Path to a global license ignore file (overrides GLOBAL_LICENSE_IGNORE
  /// environment variable)
  #[arg(long, value_name = "FILE")]
  pub global_ignore_file: Option<PathBuf>,

  /// Only consider files in the current git repository
  #[arg(long, default_value = "false", default_missing_value = "true", num_args = 0..=1)]
  pub git_only: Option<bool>,

  /// Control when to use colored output (auto, never, always)
  #[arg(
    long,
    value_name = "WHEN",
    num_args = 0..=1,
    default_value_t = ColorMode::Auto,
    default_missing_value = "always",
    value_enum
  )]
  pub colors: ColorMode,

  /// Skip git repository ownership check. Useful when running in Docker or
  /// other containerized environments where the repository may be owned by a
  /// different user.
  #[arg(long)]
  pub skip_git_owner_check: bool,
}

impl TreeArgs {
  /// Validate the arguments and return an error if invalid
  fn validate(&self) -> Result<(), String> {
    if self.patterns.is_empty() {
      return Err("Missing required argument: <PATTERNS>...".to_string());
    }
    Ok(())
  }
}

/// Run the tree command with the given arguments
pub async fn run_tree(args: TreeArgs) -> Result<()> {
  // Validate arguments
  if let Err(e) = args.validate() {
    eprintln!("ERROR: {e}");
    process::exit(1);
  }

  if args.quiet && args.verbose {
    eprintln!("ERROR: Cannot use --quiet and --verbose together");
    process::exit(1);
  } else if args.verbose {
    set_verbose();
  } else if args.quiet {
    set_quiet();
  }
  args.colors.apply();

  // Disable git ownership check if requested (useful in Docker)
  if args.skip_git_owner_check {
    verbose_log!("Disabling git repository ownership check");
    // SAFETY: This is safe to call as long as no git operations are in progress.
    // We call this early, before any Repository operations.
    unsafe {
      let _ = git2::opts::set_verify_owner_validation(false);
    }
  }

  // Set global ignore file if provided
  if let Some(ref global_ignore_file) = args.global_ignore_file {
    if let Some(path_str) = global_ignore_file.to_str() {
      // SAFETY:
      // This is safe because we control the lifetime of the program
      unsafe {
        std::env::set_var("GLOBAL_LICENSE_IGNORE", path_str);
      }
      verbose_log!("Setting GLOBAL_LICENSE_IGNORE to {}", global_ignore_file.display());
    } else {
      eprintln!("Warning: Could not convert global ignore file path to string");
    }
  }

  let workspace = resolve_workspace(&args.patterns)?;
  let workspace_root = workspace.root().to_path_buf();

  let git_only = args.git_only.unwrap_or(false);
  if git_only {
    if workspace.is_git() {
      verbose_log!("Git repository detected, only listing tracked files");
      verbose_log!("Using workspace root: {}", workspace_root.display());
    } else {
      eprintln!("ERROR: Git-only mode is enabled, but not in a git repository");
      eprintln!("When --git-only is enabled, you must run edlicense from inside a git repository");
      process::exit(1);
    }
  }

  // Create tree lister
  let mut lister = TreeLister::new(
    args.ignore,
    args.ratchet,
    git_only,
    workspace_root.clone(),
    workspace.is_git(),
  )?;

  // Collect and list files
  let files = lister.list_files(&args.patterns).await?;

  // Output the files
  for file in &files {
    // Try to make path relative to current directory for cleaner output
    let display_path = if let Ok(rel) = file.strip_prefix(&workspace_root) {
      rel.to_path_buf()
    } else {
      file.clone()
    };
    println!("{}", display_path.display());
  }

  // Show summary in verbose mode
  if !args.quiet {
    info_log!("Found {} files", files.len());
  }

  Ok(())
}

/// Helper struct for listing files that would be checked
struct TreeLister {
  workspace_root: PathBuf,
  file_filter: IgnoreFilter,
  ignore_manager: IgnoreManager,
  git_only: bool,
  ratchet_reference: Option<String>,
  ignore_manager_cache: std::collections::HashMap<PathBuf, IgnoreManager>,
}

impl TreeLister {
  fn new(
    ignore_patterns: Vec<String>,
    ratchet_reference: Option<String>,
    git_only: bool,
    workspace_root: PathBuf,
    workspace_is_git: bool,
  ) -> Result<Self> {
    if (git_only || ratchet_reference.is_some()) && !workspace_is_git {
      return Err(anyhow::anyhow!(
        "Git-only or ratchet mode requires a git-backed workspace"
      ));
    }

    let ignore_manager = IgnoreManager::new(ignore_patterns.clone())?;
    let file_filter = create_default_filter(ignore_patterns)?;

    Ok(Self {
      workspace_root,
      file_filter,
      ignore_manager,
      git_only,
      ratchet_reference,
      ignore_manager_cache: std::collections::HashMap::new(),
    })
  }

  async fn list_files(&mut self, patterns: &[String]) -> Result<Vec<PathBuf>> {
    if self.should_use_git_list() {
      let files = self.collect_git_files(patterns)?;
      return self.filter_files_with_ignore_context(files).await;
    }

    let mut all_files = Vec::new();

    // Process each pattern
    for pattern in patterns {
      let maybe_path = PathBuf::from(pattern);
      if maybe_path.is_file() {
        // Single file - check if it should be included
        if let Some(file) = self.check_single_file(&maybe_path).await? {
          all_files.push(file);
        }
      } else if maybe_path.is_dir() {
        // Directory - recursively collect files
        let dir_files = self.collect_directory_files(&maybe_path).await?;
        all_files.extend(dir_files);
      } else {
        // Try as glob pattern
        let entries = glob::glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        for entry in entries {
          match entry {
            Ok(path) => {
              if path.is_file() {
                if let Some(file) = self.check_single_file(&path).await? {
                  all_files.push(file);
                }
              } else if path.is_dir() {
                let dir_files = self.collect_directory_files(&path).await?;
                all_files.extend(dir_files);
              }
            }
            Err(e) => {
              eprintln!("Error with glob pattern: {}", e);
            }
          }
        }
      }
    }

    // Sort for consistent output
    all_files.sort();
    all_files.dedup();

    Ok(all_files)
  }

  const fn should_use_git_list(&self) -> bool {
    self.git_only || self.ratchet_reference.is_some()
  }

  fn collect_git_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let files: Vec<PathBuf> = if self.git_only {
      git::get_git_tracked_files(&self.workspace_root)?.into_iter().collect()
    } else if let Some(reference) = &self.ratchet_reference {
      git::get_changed_files_for_workspace(&self.workspace_root, reference)?
        .into_iter()
        .collect()
    } else {
      return Ok(Vec::new());
    };

    if files.is_empty() {
      return Ok(Vec::new());
    }

    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
    let matchers = build_pattern_matchers(patterns, &current_dir, &self.workspace_root)?;

    let selected: Vec<PathBuf> = files
      .into_iter()
      .filter_map(|file| {
        let normalized = normalize_relative_path(&file, &self.workspace_root);
        if matches_any_pattern(&normalized, &matchers) {
          Some(self.workspace_root.join(&normalized))
        } else {
          None
        }
      })
      .collect();

    Ok(selected)
  }

  async fn check_single_file(&mut self, path: &Path) -> Result<Option<PathBuf>> {
    let absolute_path = absolutize_path(path)?;

    // Skip symlinks
    match std::fs::symlink_metadata(&absolute_path) {
      Ok(metadata) => {
        if metadata.file_type().is_symlink() {
          verbose_log!("Skipping: {} (symlink)", path.display());
          return Ok(None);
        }
      }
      Err(_) => {
        return Ok(None);
      }
    }

    // Check ignore patterns from parent directory
    if let Some(parent_dir) = absolute_path.parent() {
      if parent_dir.exists() {
        let ignore_manager = self.get_or_create_ignore_manager(parent_dir)?;

        if ignore_manager.is_ignored(&absolute_path) {
          verbose_log!("Skipping: {} (matches .licenseignore pattern)", path.display());
          return Ok(None);
        }
      }
    }

    Ok(Some(absolute_path))
  }

  async fn collect_directory_files(&mut self, dir: &Path) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::with_capacity(1000);
    let mut dirs_to_process = VecDeque::with_capacity(100);
    dirs_to_process.push_back(dir.to_path_buf());

    verbose_log!("Scanning directory: {}", dir.display());

    while let Some(current_dir) = dirs_to_process.pop_front() {
      let read_dir_result = tokio::fs::read_dir(&current_dir).await;
      if let Err(e) = read_dir_result {
        eprintln!("Error reading directory {}: {}", current_dir.display(), e);
        continue;
      }

      let mut entries = read_dir_result.expect("Valid read_dir");
      while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if let Ok(file_type) = entry.file_type().await {
          if file_type.is_dir() {
            dirs_to_process.push_back(path);
          } else if file_type.is_file() {
            all_files.push(path);
          }
        }
      }
    }

    verbose_log!("Found {} files in directory", all_files.len());

    // Apply filters
    let filter_with_licenseignore = self.file_filter.with_licenseignore_files(dir, &self.workspace_root)?;
    let filtered = self.filter_files(&all_files, &filter_with_licenseignore);

    Ok(filtered)
  }

  fn filter_files(&self, files: &[PathBuf], filter: &dyn FileFilter) -> Vec<PathBuf> {
    files
      .iter()
      .filter(|p| {
        // Skip symlinks
        match std::fs::symlink_metadata(p) {
          Ok(metadata) => {
            if metadata.file_type().is_symlink() {
              verbose_log!("Skipping: {} (symlink)", p.display());
              return false;
            }
          }
          Err(_) => {
            return false;
          }
        }

        match filter.should_process(p) {
          Ok(result) => {
            if !result.should_process {
              let reason = result.reason.unwrap_or_else(|| "Unknown reason".to_string());
              verbose_log!("Skipping: {} ({})", p.display(), reason);
              false
            } else {
              true
            }
          }
          Err(_) => false,
        }
      })
      .cloned()
      .collect()
  }

  async fn filter_files_with_ignore_context(&mut self, files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut filtered = Vec::with_capacity(files.len());

    for path in files {
      // Skip symlinks
      match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
          if metadata.file_type().is_symlink() {
            verbose_log!("Skipping: {} (symlink)", path.display());
            continue;
          }
        }
        Err(_) => {
          continue;
        }
      }

      let absolute_path = absolutize_path(&path)?;
      let mut ignored = false;

      if let Some(parent_dir) = absolute_path.parent()
        && parent_dir.exists()
      {
        let ignore_manager = self.get_or_create_ignore_manager(parent_dir)?;

        if ignore_manager.is_ignored(&absolute_path) {
          verbose_log!("Skipping: {} (matches .licenseignore pattern)", path.display());
          ignored = true;
        }
      }

      if !ignored {
        filtered.push(path);
      }
    }

    // Sort for consistent output
    filtered.sort();
    filtered.dedup();

    Ok(filtered)
  }

  fn get_or_create_ignore_manager(&mut self, parent_dir: &Path) -> Result<IgnoreManager> {
    if let Some(cached_manager) = self.ignore_manager_cache.get(parent_dir) {
      verbose_log!("Using cached ignore manager for: {}", parent_dir.display());
      return Ok(cached_manager.clone());
    }

    verbose_log!("Creating new ignore manager for: {}", parent_dir.display());
    let mut new_manager = self.ignore_manager.clone();
    new_manager.load_licenseignore_files(parent_dir, &self.workspace_root)?;

    self
      .ignore_manager_cache
      .insert(parent_dir.to_path_buf(), new_manager.clone());
    Ok(new_manager)
  }
}

// Helper functions (duplicated from processor.rs to avoid coupling)

enum PatternMatcher {
  Any,
  File(PathBuf),
  Dir(PathBuf),
  Glob(glob::Pattern),
}

fn build_pattern_matchers(
  patterns: &[String],
  current_dir: &Path,
  workspace_root: &Path,
) -> Result<Vec<PatternMatcher>> {
  if patterns.is_empty() {
    return Ok(Vec::new());
  }

  let mut matchers = Vec::with_capacity(patterns.len());
  for pattern in patterns {
    let raw_path = PathBuf::from(pattern);
    if raw_path.exists() {
      let abs_path = if raw_path.is_absolute() {
        raw_path.clone()
      } else {
        current_dir.join(&raw_path)
      };
      let normalized = normalize_relative_path(&abs_path, workspace_root);
      let normalized = PathBuf::from(normalize_path_string(&normalized.to_string_lossy().replace('\\', "/")));
      if raw_path.is_dir() {
        if normalized.as_os_str() == "." {
          matchers.push(PatternMatcher::Any);
        } else {
          matchers.push(PatternMatcher::Dir(normalized));
        }
      } else if raw_path.is_file() {
        matchers.push(PatternMatcher::File(normalized));
      }
    } else {
      let mut glob_source = pattern.as_str().to_string();
      if raw_path.is_absolute() {
        if let Ok(rel_path) = raw_path.strip_prefix(workspace_root) {
          glob_source = rel_path.to_string_lossy().replace("\\", "/");
        }
      } else if let Ok(workspace_relative_cwd) = current_dir.strip_prefix(workspace_root)
        && !workspace_relative_cwd.as_os_str().is_empty()
        && workspace_relative_cwd.as_os_str() != "."
      {
        let cwd_prefix = workspace_relative_cwd.to_string_lossy().replace("\\", "/");
        glob_source = normalize_path_string(&format!("{}/{}", cwd_prefix, glob_source));
      }
      let glob_pattern =
        glob::Pattern::new(&glob_source).with_context(|| format!("Invalid glob pattern: {}", pattern))?;
      matchers.push(PatternMatcher::Glob(glob_pattern));
    }
  }

  Ok(matchers)
}

fn absolutize_path(path: &Path) -> Result<PathBuf> {
  if path.is_absolute() {
    Ok(path.to_path_buf())
  } else {
    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
    Ok(current_dir.join(path))
  }
}

fn matches_any_pattern(path: &Path, matchers: &[PatternMatcher]) -> bool {
  if matchers.is_empty() {
    return true;
  }

  matchers.iter().any(|matcher| match matcher {
    PatternMatcher::Any => true,
    PatternMatcher::File(file_path) => path == file_path,
    PatternMatcher::Dir(dir_path) => path.starts_with(dir_path),
    PatternMatcher::Glob(pattern) => pattern.matches_path(path),
  })
}

fn normalize_relative_path(path: &Path, current_dir: &Path) -> PathBuf {
  if path.is_absolute() {
    if let Ok(stripped) = path.strip_prefix(current_dir) {
      return stripped.to_path_buf();
    }

    if let Some(rel_path) = pathdiff::diff_paths(path, current_dir) {
      return rel_path;
    }
  }

  let mut normalized = PathBuf::new();
  for component in path.components() {
    if matches!(component, std::path::Component::CurDir) {
      continue;
    }
    normalized.push(component.as_os_str());
  }

  if normalized.as_os_str().is_empty() {
    PathBuf::from(".")
  } else {
    normalized
  }
}

fn normalize_path_string(path: &str) -> String {
  let mut components: Vec<&str> = Vec::new();

  for segment in path.split('/') {
    if segment == ".." {
      if let Some(last) = components.last()
        && *last != ".."
        && !last.is_empty()
      {
        components.pop();
        continue;
      }
      components.push(segment);
    } else if segment == "." {
      continue;
    } else {
      components.push(segment);
    }
  }

  components.join("/")
}
