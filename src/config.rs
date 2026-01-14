//! # Configuration Module
//!
//! This module provides configuration support for edlicense, allowing users to
//! customize comment styles for file extensions and control extension
//! filtering.
//!
//! Configuration can be specified in a `.edlicense.toml` file or via the
//! `EDLICENSE_CONFIG` environment variable.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::verbose_log;

/// The default config file name.
pub const DEFAULT_CONFIG_FILENAME: &str = ".edlicense.toml";

/// Environment variable for specifying config file path.
pub const CONFIG_ENV_VAR: &str = "EDLICENSE_CONFIG";

/// User-defined comment style configuration.
///
/// This struct represents a custom comment style that can be specified in the
/// configuration file. It defines how license comments should be formatted for
/// a specific file extension.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CommentStyleConfig {
  /// The string to use at the top of a comment block (e.g., "/*").
  /// Optional for line-style comments.
  #[serde(default)]
  pub top: String,

  /// The string to use at the beginning of each line in the comment block
  /// (e.g., " * " or "// ").
  pub middle: String,

  /// The string to use at the bottom of a comment block (e.g., " */").
  /// Optional for line-style comments.
  #[serde(default)]
  pub bottom: String,
}

impl CommentStyleConfig {
  /// Create a new line-comment style (no top/bottom markers).
  #[allow(dead_code)]
  pub fn line(prefix: &str) -> Self {
    Self {
      top: String::new(),
      middle: prefix.to_string(),
      bottom: String::new(),
    }
  }

  /// Create a new block-comment style.
  #[allow(dead_code)]
  pub fn block(top: &str, middle: &str, bottom: &str) -> Self {
    Self {
      top: top.to_string(),
      middle: middle.to_string(),
      bottom: bottom.to_string(),
    }
  }
}

/// Configuration for extension-based file filtering.
///
/// This allows users to include or exclude specific file extensions from
/// processing. If `include` is specified, only files with those extensions
/// will be processed. If only `exclude` is specified, all files except those
/// with the excluded extensions will be processed.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
pub struct ExtensionConfig {
  /// If specified, only process files with these extensions.
  /// All other extensions will be excluded.
  #[serde(default)]
  pub include: Option<Vec<String>>,

  /// Extensions to exclude from processing.
  /// Ignored if `include` is specified.
  #[serde(default)]
  pub exclude: Vec<String>,
}

/// Main configuration struct for edlicense.
///
/// This struct is loaded from a `.edlicense.toml` file and contains all
/// user-configurable options for comment styles and extension filtering.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
  /// Custom comment styles for file extensions.
  /// Keys are file extensions without the leading dot (e.g., "java", "xyz").
  #[serde(default, rename = "comment-styles")]
  pub comment_styles: HashMap<String, CommentStyleConfig>,

  /// Filename-specific comment style overrides.
  /// Keys are exact filenames or glob patterns (e.g., "Justfile",
  /// "*.cmake.in").
  #[serde(default)]
  pub filenames: HashMap<String, CommentStyleConfig>,

  /// Extension-based file filtering configuration.
  #[serde(default)]
  pub extensions: ExtensionConfig,
}

/// Error type for configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
  /// The config file could not be read.
  #[error("Failed to read config file '{path}': {source}")]
  ReadError { path: PathBuf, source: std::io::Error },

  /// The config file contains invalid TOML.
  #[error("Failed to parse config file '{path}': {source}")]
  ParseError { path: PathBuf, source: toml::de::Error },

  /// A comment style configuration is invalid.
  #[error("Invalid comment style for '{extension}': {message}")]
  InvalidCommentStyle { extension: String, message: String },
}

impl Config {
  /// Load configuration from a file.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the configuration file
  ///
  /// # Returns
  ///
  /// The loaded configuration, or an error if the file cannot be read or
  /// parsed.
  pub fn load(path: &Path) -> Result<Self, ConfigError> {
    verbose_log!("Loading config from: {}", path.display());

    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
      path: path.to_path_buf(),
      source: e,
    })?;

    let config: Config = toml::from_str(&content).map_err(|e| ConfigError::ParseError {
      path: path.to_path_buf(),
      source: e,
    })?;

    config.validate()?;

    // Normalize keys to lowercase for case-insensitive matching
    let config = config.normalize();

    verbose_log!("Loaded {} comment style overrides", config.comment_styles.len());

    Ok(config)
  }

  /// Validate the configuration.
  ///
  /// Checks that:
  /// - All `middle` fields are non-empty
  /// - Extension names don't include the leading dot
  /// - Extension filter entries don't include the leading dot
  fn validate(&self) -> Result<(), ConfigError> {
    for (ext, style) in &self.comment_styles {
      if style.middle.is_empty() {
        return Err(ConfigError::InvalidCommentStyle {
          extension: ext.clone(),
          message: "middle field cannot be empty".to_string(),
        });
      }

      if ext.starts_with('.') {
        return Err(ConfigError::InvalidCommentStyle {
          extension: ext.clone(),
          message: "extension should not include leading dot".to_string(),
        });
      }
    }

    for (filename, style) in &self.filenames {
      if style.middle.is_empty() {
        return Err(ConfigError::InvalidCommentStyle {
          extension: filename.clone(),
          message: "middle field cannot be empty".to_string(),
        });
      }
    }

    // Validate extension filter entries
    if let Some(ref include) = self.extensions.include {
      for ext in include {
        if ext.starts_with('.') {
          return Err(ConfigError::InvalidCommentStyle {
            extension: ext.clone(),
            message: "extension in include list should not include leading dot".to_string(),
          });
        }
      }
    }

    for ext in &self.extensions.exclude {
      if ext.starts_with('.') {
        return Err(ConfigError::InvalidCommentStyle {
          extension: ext.clone(),
          message: "extension in exclude list should not include leading dot".to_string(),
        });
      }
    }

    Ok(())
  }

  /// Check if the configuration has any comment style overrides.
  #[allow(dead_code)]
  pub fn has_overrides(&self) -> bool {
    !self.comment_styles.is_empty() || !self.filenames.is_empty()
  }

  /// Check if the configuration has any extension filtering.
  #[allow(dead_code)]
  pub const fn has_extension_filter(&self) -> bool {
    self.extensions.include.is_some() || !self.extensions.exclude.is_empty()
  }

  /// Normalize configuration keys to lowercase for case-insensitive matching.
  ///
  /// This ensures that config keys like "Justfile" or "CMakeLists.txt" will
  /// match the lowercased filenames used during lookup.
  fn normalize(self) -> Self {
    let comment_styles = self
      .comment_styles
      .into_iter()
      .map(|(k, v)| (k.to_lowercase(), v))
      .collect();

    let filenames = self.filenames.into_iter().map(|(k, v)| (k.to_lowercase(), v)).collect();

    Self {
      comment_styles,
      filenames,
      extensions: self.extensions,
    }
  }
}

/// Discover the configuration file path.
///
/// The configuration file is discovered in the following order:
/// 1. Path specified via `--config` flag (passed as `explicit_path`)
/// 2. Path specified via `EDLICENSE_CONFIG` environment variable
/// 3. `.edlicense.toml` in the workspace root
///
/// # Arguments
///
/// * `explicit_path` - Optional explicit path from CLI flag
/// * `workspace_root` - The workspace root directory
///
/// # Returns
///
/// The path to the configuration file, or `None` if no config file is found.
pub fn discover_config_path(explicit_path: Option<&Path>, workspace_root: &Path) -> Option<PathBuf> {
  // 1. Explicit path from CLI takes highest priority
  if let Some(path) = explicit_path {
    if path.exists() {
      verbose_log!("Using explicit config path: {}", path.display());
      return Some(path.to_path_buf());
    }
    verbose_log!("Explicit config path does not exist: {}", path.display());
    return None;
  }

  // 2. Check environment variable
  if let Ok(env_path) = std::env::var(CONFIG_ENV_VAR) {
    let path = PathBuf::from(&env_path);
    if path.exists() {
      verbose_log!("Using config from {}: {}", CONFIG_ENV_VAR, path.display());
      return Some(path);
    }
    verbose_log!("{} path does not exist: {}", CONFIG_ENV_VAR, env_path);
  }

  // 3. Check workspace root
  let workspace_config = workspace_root.join(DEFAULT_CONFIG_FILENAME);
  if workspace_config.exists() {
    verbose_log!("Using workspace config: {}", workspace_config.display());
    return Some(workspace_config);
  }

  verbose_log!("No config file found");
  None
}

/// Load configuration from the discovered path, or return a default config.
///
/// # Arguments
///
/// * `explicit_path` - Optional explicit path from CLI flag
/// * `workspace_root` - The workspace root directory
/// * `no_config` - If true, skip config file discovery and use defaults
///
/// # Returns
///
/// The loaded configuration, or a default configuration if no config file is
/// found.
pub fn load_config(explicit_path: Option<&Path>, workspace_root: &Path, no_config: bool) -> Result<Option<Config>> {
  if no_config {
    verbose_log!("Config file discovery disabled (--no-config)");
    return Ok(None);
  }

  match discover_config_path(explicit_path, workspace_root) {
    Some(path) => {
      let config = Config::load(&path).with_context(|| format!("Failed to load config from {}", path.display()))?;
      Ok(Some(config))
    }
    None => Ok(None),
  }
}

#[cfg(test)]
mod tests {
  use tempfile::TempDir;

  use super::*;

  #[test]
  fn test_parse_valid_config() {
    let config_content = concat!(
      "[comment-styles]\n",
      "java = { middle = \"// \" }\n",
      "xyz = { top = \"/*\", middle = \" * \", bottom = \" */\" }\n",
      "acme = { middle = \"## \" }\n",
      "\n",
      "[filenames]\n",
      "\"Justfile\" = { middle = \"# \" }\n",
    );

    let config: Config = toml::from_str(config_content).expect("valid config should parse");

    assert_eq!(config.comment_styles.len(), 3);
    assert_eq!(config.filenames.len(), 1);

    let java_style = config.comment_styles.get("java").expect("java should exist");
    assert_eq!(java_style.top, "");
    assert_eq!(java_style.middle, "// ");
    assert_eq!(java_style.bottom, "");

    let xyz_style = config.comment_styles.get("xyz").expect("xyz should exist");
    assert_eq!(xyz_style.top, "/*");
    assert_eq!(xyz_style.middle, " * ");
    assert_eq!(xyz_style.bottom, " */");
  }

  #[test]
  fn test_parse_empty_config() {
    let config_content = "";
    let config: Config = toml::from_str(config_content).expect("empty config should parse");

    assert!(config.comment_styles.is_empty());
    assert!(config.filenames.is_empty());
  }

  #[test]
  fn test_validate_empty_middle() {
    let config = Config {
      comment_styles: {
        let mut map = HashMap::new();
        map.insert(
          "bad".to_string(),
          CommentStyleConfig {
            top: String::new(),
            middle: String::new(),
            bottom: String::new(),
          },
        );
        map
      },
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let result = config.validate();
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, ConfigError::InvalidCommentStyle { .. }));
  }

  #[test]
  fn test_validate_leading_dot() {
    let config = Config {
      comment_styles: {
        let mut map = HashMap::new();
        map.insert(".bad".to_string(), CommentStyleConfig::line("// "));
        map
      },
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let result = config.validate();
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, ConfigError::InvalidCommentStyle { .. }));
  }

  #[test]
  fn test_load_config_from_file() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let config_path = temp_dir.path().join(".edlicense.toml");

    std::fs::write(
      &config_path,
      concat!("[comment-styles]\n", "custom = { middle = \"## \" }\n",),
    )
    .expect("write config");

    let config = Config::load(&config_path).expect("load should succeed");
    assert_eq!(config.comment_styles.len(), 1);
    assert!(config.comment_styles.contains_key("custom"));
  }

  #[test]
  fn test_load_config_file_not_found() {
    let result = Config::load(Path::new("/nonexistent/path/.edlicense.toml"));
    assert!(result.is_err());
    assert!(matches!(
      result.expect_err("should fail"),
      ConfigError::ReadError { .. }
    ));
  }

  #[test]
  fn test_discover_config_explicit_path() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let config_path = temp_dir.path().join("custom-config.toml");
    std::fs::write(&config_path, "").expect("write config");

    let workspace_root = temp_dir.path();
    let result = discover_config_path(Some(&config_path), workspace_root);

    assert_eq!(result, Some(config_path));
  }

  #[test]
  fn test_discover_config_workspace_root() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let config_path = temp_dir.path().join(DEFAULT_CONFIG_FILENAME);
    std::fs::write(&config_path, "").expect("write config");

    let result = discover_config_path(None, temp_dir.path());

    assert_eq!(result, Some(config_path));
  }

  #[test]
  fn test_discover_config_none_found() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let result = discover_config_path(None, temp_dir.path());

    assert!(result.is_none());
  }

  #[test]
  fn test_comment_style_config_helpers() {
    let line_style = CommentStyleConfig::line("// ");
    assert_eq!(line_style.top, "");
    assert_eq!(line_style.middle, "// ");
    assert_eq!(line_style.bottom, "");

    let block_style = CommentStyleConfig::block("/*", " * ", " */");
    assert_eq!(block_style.top, "/*");
    assert_eq!(block_style.middle, " * ");
    assert_eq!(block_style.bottom, " */");
  }

  #[test]
  fn test_has_overrides() {
    let empty_config = Config::default();
    assert!(!empty_config.has_overrides());

    let config_with_styles = Config {
      comment_styles: {
        let mut map = HashMap::new();
        map.insert("rs".to_string(), CommentStyleConfig::line("// "));
        map
      },
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };
    assert!(config_with_styles.has_overrides());

    let config_with_filenames = Config {
      comment_styles: HashMap::new(),
      filenames: {
        let mut map = HashMap::new();
        map.insert("Makefile".to_string(), CommentStyleConfig::line("# "));
        map
      },
      extensions: ExtensionConfig::default(),
    };
    assert!(config_with_filenames.has_overrides());
  }

  #[test]
  fn test_load_normalizes_keys_to_lowercase() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let config_path = temp_dir.path().join(".edlicense.toml");

    // Config with mixed-case keys
    std::fs::write(
      &config_path,
      concat!(
        "[comment-styles]\n",
        "RS = { middle = \"// \" }\n",
        "Java = { middle = \"// \" }\n",
        "\n",
        "[filenames]\n",
        "\"Justfile\" = { middle = \"# \" }\n",
        "\"CMakeLists.txt\" = { middle = \"# \" }\n",
      ),
    )
    .expect("write config");

    let config = Config::load(&config_path).expect("load should succeed");

    // Extension keys should be lowercased
    assert!(config.comment_styles.contains_key("rs"));
    assert!(config.comment_styles.contains_key("java"));
    assert!(!config.comment_styles.contains_key("RS"));
    assert!(!config.comment_styles.contains_key("Java"));

    // Filename keys should be lowercased
    assert!(config.filenames.contains_key("justfile"));
    assert!(config.filenames.contains_key("cmakelists.txt"));
    assert!(!config.filenames.contains_key("Justfile"));
    assert!(!config.filenames.contains_key("CMakeLists.txt"));
  }

  #[test]
  fn test_parse_config_with_extensions() {
    let config_content = concat!(
      "[comment-styles]\n",
      "rs = { middle = \"// \" }\n",
      "\n",
      "[extensions]\n",
      "include = [\"rs\", \"go\"]\n",
    );

    let config: Config = toml::from_str(config_content).expect("config should parse");

    assert!(config.extensions.include.is_some());
    let include = config.extensions.include.as_ref().expect("include should exist");
    assert_eq!(include.len(), 2);
    assert!(include.contains(&"rs".to_string()));
    assert!(include.contains(&"go".to_string()));
  }

  #[test]
  fn test_parse_config_with_exclude() {
    let config_content = concat!("[extensions]\n", "exclude = [\"min.js\", \"pb.go\"]\n",);

    let config: Config = toml::from_str(config_content).expect("config should parse");

    assert!(config.extensions.include.is_none());
    assert_eq!(config.extensions.exclude.len(), 2);
    assert!(config.extensions.exclude.contains(&"min.js".to_string()));
    assert!(config.extensions.exclude.contains(&"pb.go".to_string()));
  }

  #[test]
  fn test_validate_extension_include_leading_dot() {
    let config = Config {
      comment_styles: HashMap::new(),
      filenames: HashMap::new(),
      extensions: ExtensionConfig {
        include: Some(vec![".rs".to_string()]),
        exclude: Vec::new(),
      },
    };

    let result = config.validate();
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, ConfigError::InvalidCommentStyle { .. }));
  }

  #[test]
  fn test_validate_extension_exclude_leading_dot() {
    let config = Config {
      comment_styles: HashMap::new(),
      filenames: HashMap::new(),
      extensions: ExtensionConfig {
        include: None,
        exclude: vec![".js".to_string()],
      },
    };

    let result = config.validate();
    assert!(result.is_err());
    let err = result.expect_err("should fail");
    assert!(matches!(err, ConfigError::InvalidCommentStyle { .. }));
  }

  #[test]
  fn test_has_extension_filter() {
    let empty_config = Config::default();
    assert!(!empty_config.has_extension_filter());

    let config_with_include = Config {
      comment_styles: HashMap::new(),
      filenames: HashMap::new(),
      extensions: ExtensionConfig {
        include: Some(vec!["rs".to_string()]),
        exclude: Vec::new(),
      },
    };
    assert!(config_with_include.has_extension_filter());

    let config_with_exclude = Config {
      comment_styles: HashMap::new(),
      filenames: HashMap::new(),
      extensions: ExtensionConfig {
        include: None,
        exclude: vec!["js".to_string()],
      },
    };
    assert!(config_with_exclude.has_extension_filter());
  }
}
