//! # Templates Module
//!
//! This module provides functionality for managing license templates, rendering
//! them with specific data (like the copyright year), and formatting them
//! appropriately for different file types.
//!
//! The module includes:
//! - [`TemplateManager`] for loading and rendering license templates
//! - [`LicenseData`] for providing data to fill in templates
//! - [`CommentStyle`] for defining how comments should be formatted in
//!   different file types
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::path::Path;
//!
//! use edlicense::templates::{LicenseData, TemplateManager};
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create license data with the current year
//! let license_data = LicenseData {
//!   year: "2025".to_string(),
//! };
//!
//! // Create and initialize template manager
//! let mut template_manager = TemplateManager::new();
//! template_manager.load_template(Path::new("LICENSE.txt"))?;
//!
//! // Render the template with the license data
//! let license_text = template_manager.render(&license_data)?;
//!
//! // Format the license for a specific file type
//! let formatted_license =
//!   template_manager.format_for_file_type(&license_text, Path::new("main.rs"));
//! # Ok(())
//! # }
//! ```

use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::config::{CommentStyleConfig, Config};
use crate::verbose_log;

/// Data used to fill out a license template.
///
/// # Fields
///
/// * `year` - The copyright year to use in the license
pub struct LicenseData {
  /// The copyright year to use in the license
  pub year: String,
}

/// Manager for loading, rendering, and formatting license templates.
///
/// The `TemplateManager` is responsible for:
/// - Loading license templates from files
/// - Rendering templates with specific data (like the year)
/// - Formatting license text with appropriate comment styles for different file
///   types
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
///
/// use edlicense::templates::{LicenseData, TemplateManager};
///
/// # fn main() -> anyhow::Result<()> {
/// let mut manager = TemplateManager::new();
/// manager.load_template(Path::new("LICENSE.txt"))?;
///
/// let data = LicenseData {
///   year: "2025".to_string(),
/// };
/// let license = manager.render(&data)?;
///
/// // Format for a Rust file
/// let formatted = manager.format_for_file_type(&license, Path::new("main.rs"));
/// # Ok(())
/// # }
/// ```
pub struct TemplateManager {
  /// The loaded license template content
  template: String,
  /// The comment style resolver to use
  resolver: Box<dyn CommentStyleResolver>,
}

impl Default for TemplateManager {
  fn default() -> Self {
    Self::new()
  }
}

impl TemplateManager {
  /// Creates a new empty template manager with the default builtin resolver.
  ///
  /// The manager is initialized with an empty template string.
  /// You must call [`load_template`](Self::load_template) before using it.
  ///
  /// # Returns
  ///
  /// A new `TemplateManager` instance with an empty template.
  pub fn new() -> Self {
    Self {
      template: String::new(),
      resolver: Box::new(BuiltinResolver),
    }
  }

  /// Creates a new template manager with a custom comment style resolver.
  ///
  /// # Arguments
  ///
  /// * `resolver` - The comment style resolver to use for formatting licenses
  ///
  /// # Returns
  ///
  /// A new `TemplateManager` instance with an empty template and the specified
  /// resolver.
  pub fn with_resolver(resolver: Box<dyn CommentStyleResolver>) -> Self {
    Self {
      template: String::new(),
      resolver,
    }
  }

  /// Loads a custom license template from a file.
  ///
  /// This method reads the template content from the specified file path
  /// and stores it in the template manager for later use.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the license template file
  ///
  /// # Returns
  ///
  /// `Ok(())` if the template was loaded successfully, or an error if the file
  /// could not be read.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The file does not exist
  /// - The file cannot be read
  /// - The file content is not valid UTF-8
  pub fn load_template(&mut self, path: &Path) -> Result<()> {
    verbose_log!("Loading template from: {}", path.display());

    let template_content =
      fs::read_to_string(path).with_context(|| format!("Failed to read license template file: {}", path.display()))?;

    verbose_log!("Template content:\n{}", template_content);

    self.template = template_content;

    Ok(())
  }

  /// Renders a license template with the given data.
  ///
  /// This method replaces template variables with actual values from the
  /// provided license data. Currently, it supports the `{{year}}` variable
  /// which is replaced with the year from the license data.
  ///
  /// # Parameters
  ///
  /// * `data` - License data containing values to substitute into the template
  ///
  /// # Returns
  ///
  /// The rendered license text with variables replaced, or an error if
  /// rendering fails.
  pub fn render(&self, data: &LicenseData) -> Result<String> {
    verbose_log!("Rendering template with year: {}", data.year);

    // Simple string replacement
    let rendered = self.template.replace("{{year}}", &data.year);

    Ok(rendered)
  }

  /// Formats the license text with the appropriate comment style for the given
  /// file type.
  ///
  /// This method determines the appropriate comment style based on the file
  /// extension and formats the license text accordingly. If a custom resolver
  /// was provided, it will be used to determine the comment style.
  ///
  /// # Parameters
  ///
  /// * `license_text` - The rendered license text to format
  /// * `file_path` - Path to the file, used to determine the appropriate
  ///   comment style
  ///
  /// # Returns
  ///
  /// The formatted license text with appropriate comment markers.
  pub fn format_for_file_type(&self, license_text: &str, file_path: &Path) -> String {
    let comment_style = self.resolver.resolve(file_path);
    format_with_comment_style(license_text, &comment_style)
  }
}

/// Defines the comment style for different file types.
///
/// This struct represents how comments should be formatted for different
/// programming languages and file types. It includes markers for the top,
/// middle, and bottom of a comment block.
///
/// # Fields
///
/// * `top` - The string to use at the top of a comment block (e.g., "/*")
/// * `middle` - The string to use at the beginning of each line in the comment
///   block (e.g., " * ")
/// * `bottom` - The string to use at the bottom of a comment block (e.g., "
///   */")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentStyle {
  /// The string to use at the top of a comment block
  pub top: String,

  /// The string to use at the beginning of each line in the comment block
  pub middle: String,

  /// The string to use at the bottom of a comment block
  pub bottom: String,
}

impl CommentStyle {
  /// Create a line-comment style (no top/bottom markers).
  ///
  /// # Arguments
  ///
  /// * `prefix` - The prefix to use for each line (e.g., "// " or "# ")
  pub fn line(prefix: &str) -> Self {
    Self {
      top: String::new(),
      middle: prefix.to_string(),
      bottom: String::new(),
    }
  }

  /// Create a block-comment style.
  ///
  /// # Arguments
  ///
  /// * `top` - The string to start the comment block (e.g., "/*")
  /// * `middle` - The prefix for each line (e.g., " * ")
  /// * `bottom` - The string to end the comment block (e.g., " */")
  pub fn block(top: &str, middle: &str, bottom: &str) -> Self {
    Self {
      top: top.to_string(),
      middle: middle.to_string(),
      bottom: bottom.to_string(),
    }
  }
}

impl From<CommentStyleConfig> for CommentStyle {
  fn from(config: CommentStyleConfig) -> Self {
    Self {
      top: config.top,
      middle: config.middle,
      bottom: config.bottom,
    }
  }
}

impl From<&CommentStyleConfig> for CommentStyle {
  fn from(config: &CommentStyleConfig) -> Self {
    Self {
      top: config.top.clone(),
      middle: config.middle.clone(),
      bottom: config.bottom.clone(),
    }
  }
}

/// Trait for resolving comment styles for file paths.
///
/// This trait allows different strategies for determining the appropriate
/// comment style for a given file path. Implementations can use built-in
/// mappings, user configuration, or both.
pub trait CommentStyleResolver: Send + Sync {
  /// Resolve the comment style for the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file
  ///
  /// # Returns
  ///
  /// The appropriate `CommentStyle` for the file.
  fn resolve(&self, path: &Path) -> CommentStyle;
}

/// Default resolver using built-in mappings.
///
/// This resolver uses the hardcoded mappings from file extensions to comment
/// styles. It's used when no configuration file is present.
#[derive(Debug, Default)]
pub struct BuiltinResolver;

impl CommentStyleResolver for BuiltinResolver {
  fn resolve(&self, path: &Path) -> CommentStyle {
    get_comment_style_for_file(path)
  }
}

/// Configurable resolver that checks user config first, then falls back to
/// builtin.
///
/// This resolver first checks for user-defined comment styles in the config
/// file, then falls back to the built-in mappings if no override is found.
pub struct ConfigurableResolver {
  config: Arc<Config>,
}

impl std::fmt::Debug for ConfigurableResolver {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ConfigurableResolver")
      .field("config", &"<config>")
      .finish()
  }
}

impl ConfigurableResolver {
  /// Create a new configurable resolver with the given config.
  ///
  /// # Arguments
  ///
  /// * `config` - The loaded configuration
  pub fn new(config: Config) -> Self {
    Self {
      config: Arc::new(config),
    }
  }

  /// Create a new configurable resolver from an Arc'd config.
  ///
  /// This is useful when you want to share the config across multiple
  /// resolvers.
  #[allow(dead_code)]
  pub const fn from_arc(config: Arc<Config>) -> Self {
    Self { config }
  }
}

impl CommentStyleResolver for ConfigurableResolver {
  fn resolve(&self, path: &Path) -> CommentStyle {
    let file_name = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("")
      .to_lowercase();

    // 1. Check filename patterns in config (exact match first)
    if let Some(style) = self.config.filenames.get(&file_name) {
      verbose_log!("Using config filename override for: {}", file_name);
      return CommentStyle::from(style);
    }

    // 2. Check filename patterns with glob matching
    for (pattern, style) in &self.config.filenames {
      if pattern.contains('*')
        && let Ok(glob_pattern) = glob::Pattern::new(&pattern.to_lowercase())
        && glob_pattern.matches(&file_name)
      {
        verbose_log!("Using config filename glob override '{}' for: {}", pattern, file_name);
        return CommentStyle::from(style);
      }
    }

    // 3. Check extension overrides in config
    let extension = path
      .extension()
      .and_then(|ext| ext.to_str())
      .unwrap_or("")
      .to_lowercase();

    if let Some(style) = self.config.comment_styles.get(&extension) {
      verbose_log!("Using config extension override for: .{}", extension);
      return CommentStyle::from(style);
    }

    // 4. Fall back to builtin resolver
    get_comment_style_for_file(path)
  }
}

/// Create a comment style resolver based on the provided configuration.
///
/// If a configuration is provided, returns a `ConfigurableResolver` that
/// checks user overrides first. Otherwise, returns a `BuiltinResolver`.
pub fn create_resolver(config: Option<Config>) -> Box<dyn CommentStyleResolver> {
  match config {
    Some(cfg) => Box::new(ConfigurableResolver::new(cfg)),
    None => Box::new(BuiltinResolver),
  }
}

/// Determines the appropriate comment style for a file based on its extension.
///
/// This function examines the file extension (and in some cases the filename)
/// to determine the appropriate comment style for the given file type.
///
/// # Parameters
///
/// * `path` - Path to the file
///
/// # Returns
///
/// A `CommentStyle` instance appropriate for the file type.
///
/// # Supported File Types
///
/// The function supports many common file types including:
/// - C/C++/C#/Go/Rust/Swift/Dart: `// comment style`
/// - Java/Scala/Kotlin: `/* comment style */`
/// - JavaScript/TypeScript/CSS: `/** comment style */`
/// - Python/Shell/YAML/Ruby: `# comment style`
/// - HTML/XML/Vue: `<!-- comment style -->`
/// - And many more...
///
/// If the file type cannot be determined, it defaults to C-style line comments
/// (`// `).
fn get_comment_style_for_file(path: &Path) -> CommentStyle {
  let file_name = path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("")
    .to_lowercase();

  let extension = path
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap_or("")
    .to_lowercase();

  match extension.as_str() {
    "c" | "h" | "gv" | "java" | "scala" | "kt" | "kts" => CommentStyle::block("/*", " * ", " */"),
    "js" | "mjs" | "cjs" | "jsx" | "tsx" | "css" | "scss" | "sass" | "ts" => CommentStyle::block("/**", " * ", " */"),
    "cc" | "cpp" | "cs" | "go" | "hcl" | "hh" | "hpp" | "m" | "mm" | "proto" | "rs" | "swift" | "dart" | "groovy"
    | "v" | "sv" => CommentStyle::line("// "),
    "py" | "sh" | "yaml" | "yml" | "rb" | "tcl" | "tf" | "bzl" | "pl" | "pp" | "toml" => CommentStyle::line("# "),
    "el" | "lisp" => CommentStyle::line(";; "),
    "erl" => CommentStyle::line("% "),
    "hs" | "sql" | "sdl" => CommentStyle::line("-- "),
    "html" | "xml" | "vue" | "wxi" | "wxl" | "wxs" => CommentStyle::block("<!--", " ", "-->"),
    "php" => CommentStyle::line("// "),
    "j2" => CommentStyle::block("{#", "", "#}"),
    "ml" | "mli" | "mll" | "mly" => CommentStyle::block("(**", "   ", "*)"),
    _ => {
      // Handle special cases based on filename
      if file_name == "cmakelists.txt"
        || file_name.ends_with(".cmake.in")
        || file_name.ends_with(".cmake")
        || file_name == "dockerfile"
        || file_name.ends_with(".dockerfile")
      {
        CommentStyle::line("# ")
      } else {
        // Default to C-style comments if we can't determine the file type
        CommentStyle::line("// ")
      }
    }
  }
}

/// Formats license text with the given comment style.
///
/// This function takes a license text and formats it with the appropriate
/// comment markers based on the provided comment style. It handles:
/// - Adding top comment markers (if any)
/// - Prefixing each line with the middle comment marker
/// - Adding bottom comment markers (if any)
/// - Ensuring proper spacing and newlines
///
/// # Parameters
///
/// * `license_text` - The license text to format
/// * `style` - The comment style to use for formatting
///
/// # Returns
///
/// The formatted license text with appropriate comment markers.
pub fn format_with_comment_style(license_text: &str, style: &CommentStyle) -> String {
  let mut result = String::new();

  // Add top comment marker if present
  if !style.top.is_empty() {
    result.push_str(&style.top);
    result.push('\n');
  }

  // Add each line with the middle comment marker
  for line in license_text.lines() {
    if line.is_empty() {
      result.push_str(style.middle.trim_end());
    } else {
      result.push_str(&style.middle);
      result.push_str(line);
    }
    result.push('\n');
  }

  // Add bottom comment marker if present
  if !style.bottom.is_empty() {
    result.push_str(&style.bottom);
    result.push('\n');
  }

  // Add an extra newline at the end
  result.push('\n');

  result
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::Path;

  use super::*;
  use crate::config::{CommentStyleConfig, Config, ExtensionConfig};

  #[test]
  fn test_builtin_resolver_rust() {
    let resolver = BuiltinResolver;
    let style = resolver.resolve(Path::new("main.rs"));

    assert_eq!(style.top, "");
    assert_eq!(style.middle, "// ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_builtin_resolver_python() {
    let resolver = BuiltinResolver;
    let style = resolver.resolve(Path::new("script.py"));

    assert_eq!(style.top, "");
    assert_eq!(style.middle, "# ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_builtin_resolver_java() {
    let resolver = BuiltinResolver;
    let style = resolver.resolve(Path::new("Main.java"));

    assert_eq!(style.top, "/*");
    assert_eq!(style.middle, " * ");
    assert_eq!(style.bottom, " */");
  }

  #[test]
  fn test_builtin_resolver_javascript() {
    let resolver = BuiltinResolver;
    let style = resolver.resolve(Path::new("app.js"));

    assert_eq!(style.top, "/**");
    assert_eq!(style.middle, " * ");
    assert_eq!(style.bottom, " */");
  }

  #[test]
  fn test_builtin_resolver_unknown_defaults_to_line_comment() {
    let resolver = BuiltinResolver;
    let style = resolver.resolve(Path::new("unknown.xyz"));

    assert_eq!(style.top, "");
    assert_eq!(style.middle, "// ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_configurable_resolver_extension_override() {
    let mut comment_styles = HashMap::new();
    comment_styles.insert("java".to_string(), CommentStyleConfig::line("// "));

    let config = Config {
      comment_styles,
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let resolver = ConfigurableResolver::new(config);
    let style = resolver.resolve(Path::new("Main.java"));

    // Should use the config override (line style) instead of builtin (block style)
    assert_eq!(style.top, "");
    assert_eq!(style.middle, "// ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_configurable_resolver_custom_extension() {
    let mut comment_styles = HashMap::new();
    comment_styles.insert("xyz".to_string(), CommentStyleConfig::line("## "));

    let config = Config {
      comment_styles,
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let resolver = ConfigurableResolver::new(config);
    let style = resolver.resolve(Path::new("custom.xyz"));

    assert_eq!(style.top, "");
    assert_eq!(style.middle, "## ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_configurable_resolver_filename_override() {
    let mut filenames = HashMap::new();
    filenames.insert("justfile".to_string(), CommentStyleConfig::line("# "));

    let config = Config {
      comment_styles: HashMap::new(),
      filenames,
      extensions: ExtensionConfig::default(),
    };

    let resolver = ConfigurableResolver::new(config);
    let style = resolver.resolve(Path::new("Justfile"));

    assert_eq!(style.top, "");
    assert_eq!(style.middle, "# ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_configurable_resolver_filename_glob() {
    let mut filenames = HashMap::new();
    filenames.insert("*.cmake.in".to_string(), CommentStyleConfig::line("# "));

    let config = Config {
      comment_styles: HashMap::new(),
      filenames,
      extensions: ExtensionConfig::default(),
    };

    let resolver = ConfigurableResolver::new(config);
    let style = resolver.resolve(Path::new("config.cmake.in"));

    assert_eq!(style.top, "");
    assert_eq!(style.middle, "# ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_configurable_resolver_falls_back_to_builtin() {
    let config = Config {
      comment_styles: HashMap::new(),
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let resolver = ConfigurableResolver::new(config);

    // Should fall back to builtin for Rust files
    let style = resolver.resolve(Path::new("main.rs"));
    assert_eq!(style.top, "");
    assert_eq!(style.middle, "// ");
    assert_eq!(style.bottom, "");

    // Should fall back to builtin for Python files
    let style = resolver.resolve(Path::new("script.py"));
    assert_eq!(style.top, "");
    assert_eq!(style.middle, "# ");
    assert_eq!(style.bottom, "");
  }

  #[test]
  fn test_create_resolver_with_config() {
    let mut comment_styles = HashMap::new();
    comment_styles.insert("rs".to_string(), CommentStyleConfig::line("## "));

    let config = Config {
      comment_styles,
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let resolver = create_resolver(Some(config));
    let style = resolver.resolve(Path::new("main.rs"));

    // Should use the config override
    assert_eq!(style.middle, "## ");
  }

  #[test]
  fn test_create_resolver_without_config() {
    let resolver = create_resolver(None);
    let style = resolver.resolve(Path::new("main.rs"));

    // Should use the builtin style
    assert_eq!(style.middle, "// ");
  }

  #[test]
  fn test_comment_style_helpers() {
    let line_style = CommentStyle::line("// ");
    assert_eq!(line_style.top, "");
    assert_eq!(line_style.middle, "// ");
    assert_eq!(line_style.bottom, "");

    let block_style = CommentStyle::block("/*", " * ", " */");
    assert_eq!(block_style.top, "/*");
    assert_eq!(block_style.middle, " * ");
    assert_eq!(block_style.bottom, " */");
  }

  #[test]
  fn test_format_with_line_comment_style() {
    let style = CommentStyle::line("// ");
    let formatted = format_with_comment_style("Copyright 2025\nAll rights reserved.", &style);

    assert!(formatted.starts_with("// Copyright 2025\n"));
    assert!(formatted.contains("// All rights reserved."));
  }

  #[test]
  fn test_format_with_block_comment_style() {
    let style = CommentStyle::block("/*", " * ", " */");
    let formatted = format_with_comment_style("Copyright 2025", &style);

    assert!(formatted.starts_with("/*\n"));
    assert!(formatted.contains(" * Copyright 2025"));
    assert!(formatted.contains(" */\n"));
  }

  #[test]
  fn test_template_manager_with_resolver() {
    let mut comment_styles = HashMap::new();
    comment_styles.insert("rs".to_string(), CommentStyleConfig::line("## "));

    let config = Config {
      comment_styles,
      filenames: HashMap::new(),
      extensions: ExtensionConfig::default(),
    };

    let resolver = create_resolver(Some(config));
    let manager = TemplateManager::with_resolver(resolver);

    let formatted = manager.format_for_file_type("Copyright 2025", Path::new("main.rs"));
    assert!(formatted.starts_with("## Copyright 2025"));
  }
}
