//! # Workspace Module
//!
//! This module defines the workspace root that edlicense operates on.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::git;

/// Workspace root selection.
pub enum Workspace {
  /// Workspace rooted at a git repository.
  Git { root: PathBuf },
  /// Workspace rooted at a directory.
  Directory { root: PathBuf },
}

impl Workspace {
  pub fn root(&self) -> &Path {
    match self {
      Self::Git { root } | Self::Directory { root } => root.as_path(),
    }
  }

  pub const fn is_git(&self) -> bool {
    matches!(self, Self::Git { .. })
  }
}

/// Resolve the current workspace based on the current directory and patterns.
pub fn resolve_workspace(patterns: &[String]) -> Result<Workspace> {
  let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;

  if let Some(root) = git::discover_repo_root(&current_dir)? {
    return Ok(Workspace::Git { root });
  }

  if let Some(root) = resolve_workspace_from_patterns(patterns, &current_dir) {
    return Ok(Workspace::Directory { root });
  }

  Ok(Workspace::Directory { root: current_dir })
}

fn resolve_workspace_from_patterns(patterns: &[String], current_dir: &Path) -> Option<PathBuf> {
  for pattern in patterns {
    let candidate = PathBuf::from(pattern);
    if candidate.exists() {
      if candidate.is_dir() {
        return Some(abs_path_or_current(&candidate, current_dir));
      }

      if candidate.is_file()
        && let Some(parent) = candidate.parent()
      {
        return Some(abs_path_or_current(parent, current_dir));
      }
    }
  }

  None
}

fn abs_path_or_current(path: &Path, current_dir: &Path) -> PathBuf {
  if path.is_absolute() {
    path.to_path_buf()
  } else {
    current_dir.join(path)
  }
}
