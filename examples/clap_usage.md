# Clap Command-Line Argument Guide

This guide explains how to use the Clap crate for command-line argument handling in Rust applications, with examples from the edlicense project.

## Table of Contents

- [Basic Setup](#basic-setup)
- [Short Options & Aliases](#short-options--aliases)
- [Required vs Optional Arguments](#required-vs-optional-arguments)
- [Mutually Exclusive Options](#mutually-exclusive-options)
- [Subcommands](#subcommands)
- [Argument Groups](#argument-groups)
- [Value Enums](#value-enums)
- [Default Values](#default-values)
- [Advanced Help Documentation](#advanced-help-documentation)
- [Best Practices](#best-practices)

## Basic Setup

Clap can be used with the `derive` feature to create CLI applications with minimal boilerplate:

```rust
use clap::Parser;

/// A tool that ensures source code files have copyright license headers
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File or directory patterns to process. Directories are processed recursively.
    #[arg(required = true)]
    patterns: Vec<String>,
    
    /// Verbose mode: print names of modified files
    #[arg(long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    println!("Processing patterns: {:?}", args.patterns);
    if args.verbose {
        println!("Verbose mode enabled");
    }
}
```

In this basic example:
- `#[derive(Parser)]` enables Clap's derive feature for argument parsing
- `#[command(author, version, about)]` automatically populates metadata from Cargo.toml
- Doc comments (`///`) are used to generate help text
- Arguments are defined as fields in the struct

## Short Options & Aliases

To add short options or aliases to your commands:

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Verbose mode: print names of modified files
    #[arg(short, long)]
    verbose: bool,

    /// Output format to use
    #[arg(short, long, alias = "fmt", value_name = "FORMAT")]
    format: Option<String>,
    
    /// Skip error checking
    #[arg(short = 'S', long = "skip-errors")]
    skip_errors: bool,
}
```

- `short` adds a single-letter flag (`-v` for verbose)
- `short = 'S'` specifies a custom short flag character
- `alias = "fmt"` provides an alternative name for the option
- `value_name = "FORMAT"` customizes the placeholder text in help output

## Required vs Optional Arguments

Clap supports both required and optional arguments:

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Custom license file to use (required)
    #[arg(long, required = true)]
    license_file: PathBuf,
    
    /// Copyright year(s) (optional)
    #[arg(long)]
    year: Option<String>,
    
    /// File patterns to ignore (supports glob patterns, can be specified multiple times)
    #[arg(long)]
    ignore: Vec<String>,
}
```

- `required = true` marks an argument as mandatory
- `Option<T>` makes an argument optional
- `Vec<T>` allows an argument to be specified multiple times

## Mutually Exclusive Options

For options that cannot be used together:

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Dry run mode: only check for license headers without modifying files (default)
    #[arg(long, group = "mode")]
    dry_run: bool,
    
    /// Modify mode: add or update license headers in files
    #[arg(long, group = "mode")]
    modify: bool,
}
```

- `group = "mode"` places these options in a mutually exclusive group
- Only one option in the group can be specified at a time

## Subcommands

For more complex CLI applications with subcommands:

```rust
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check license headers without modifying files
    Check {
        /// Files to check
        #[arg(required = true)]
        files: Vec<PathBuf>,
        
        /// Show diff of missing headers
        #[arg(long)]
        show_diff: bool,
    },
    
    /// Add or update license headers
    Update {
        /// Files to update
        #[arg(required = true)]
        files: Vec<PathBuf>,
        
        /// Custom license template
        #[arg(long, required = true)]
        license: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Check { files, show_diff } => {
            println!("Checking files: {:?}", files);
            if *show_diff {
                println!("Will show diff");
            }
        }
        Commands::Update { files, license } => {
            println!("Updating files: {:?}", files);
            println!("Using license: {:?}", license);
        }
    }
}
```

- `#[command(subcommand)]` indicates this field will hold the subcommand
- `#[derive(Subcommand)]` for the enum of possible subcommands
- Each variant can have its own set of arguments

## Argument Groups

Group related arguments together:

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Files to process
    #[arg(required = true)]
    files: Vec<PathBuf>,
    
    /// Generate an HTML report of license status
    #[arg(long, group = "report")]
    report_html: Option<PathBuf>,
    
    /// Generate a JSON report of license status
    #[arg(long, group = "report")]
    report_json: Option<PathBuf>,
    
    /// Generate a CSV report of license status
    #[arg(long, group = "report")]
    report_csv: Option<PathBuf>,
}
```

- All options in the "report" group can be used independently
- Unlike mutual exclusion, grouping just helps with organization

## Value Enums

For arguments that should only accept specific values:

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorMode {
    /// Automatically determine based on TTY
    Auto,
    /// Never use colors
    Never,
    /// Always use colors
    Always,
}

#[derive(Parser, Debug)]
struct Args {
    /// Control when to use colored output
    #[arg(long, value_enum, default_value = "auto")]
    colors: ColorMode,
}
```

- `#[derive(ValueEnum)]` creates a type-safe enum for command-line values
- The variants become the allowed values for the argument
- Doc comments on variants are included in help text
- `default_value` specifies the default option

## Default Values

Several ways to specify default values:

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Number of threads to use
    #[arg(long, default_value_t = 4)]
    threads: usize,
    
    /// Output format
    #[arg(long, default_value = "text")]
    format: String,
    
    /// Use current working directory if not specified
    #[arg(long, default_value = ".")]
    directory: PathBuf,
}
```

- `default_value_t = 4` uses type inference for numeric types
- `default_value = "text"` for string defaults
- Default values are shown in help output

## Advanced Help Documentation

Enhance your help text for better user experience:

```rust
/// A tool for license header management
///
/// This tool scans source code files and ensures they have 
/// proper copyright license headers. It can check files,
/// add missing headers, or update existing ones.
#[derive(Parser, Debug)]
#[command(
    author, 
    version, 
    about, 
    long_about = None,
    after_help = "Examples:
  edlicense --dry-run --license-file LICENSE.txt src/
  edlicense --modify --license-file custom.txt --year 2025 include/ src/
"
)]
struct Args {
    /// File or directory patterns to process
    ///
    /// Directories are processed recursively.
    /// Multiple patterns can be specified.
    #[arg(required = true, value_name = "PATTERNS")]
    patterns: Vec<String>,
}
```

- Multi-line doc comments for detailed help
- `#[command(after_help = "...")]` adds content after the main help
- `value_name` customizes argument names in help output

## Best Practices

1. **Use long options by default**
   
   ```rust
   // Good: Easy to understand in scripts and pipelines
   #[arg(long)]
   verbose: bool,
   
   // Avoid unless it's a very common flag
   #[arg(short)]
   verbose: bool,
   ```

2. **Provide detailed help text**
   
   ```rust
   /// Generate a report in the specified format
   ///
   /// The report contains details about license compliance
   /// across all processed files.
   #[arg(long, value_name = "FILE")]
   report: Option<PathBuf>,
   ```

3. **Use kebab-case for multi-word options**
   
   ```rust
   // Good: Follows CLI conventions
   #[arg(long = "global-ignore-file")]
   global_ignore_file: Option<PathBuf>,
   
   // Avoid: Inconsistent with common CLI patterns
   #[arg(long = "global_ignore_file")]
   global_ignore_file: Option<PathBuf>,
   ```

4. **Implement validation for complex arguments**
   
   ```rust
   fn main() -> Result<(), Box<dyn std::error::Error>> {
       let args = Args::parse();
       
       // Validate arguments after parsing
       if let Some(ref year) = args.year {
           if !is_valid_year_format(year) {
               return Err("Invalid year format. Use YYYY or YYYY-YYYY".into());
           }
       }
       
       // Continue with the program
       Ok(())
   }
   ```

5. **Consider using `ArgAction` for more control**
   
   ```rust
   use clap::ArgAction;
   
   #[derive(Parser, Debug)]
   struct Args {
       /// Increase verbosity (-v, -vv, -vvv)
       #[arg(short, long, action = ArgAction::Count)]
       verbose: u8,
   }
   ```

## Practical Examples from edlicense

### Example 1: Mutually Exclusive Mode Options

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Dry run mode: only check for license headers without modifying files (default)
    #[arg(long, group = "mode")]
    dry_run: bool,

    /// Modify mode: add or update license headers in files
    #[arg(long, group = "mode")]
    modify: bool,
}

fn main() {
    let args = Args::parse();
    
    // Determine mode (dry run is default if neither is specified or if dry_run is explicitly set)
    let check_only = args.dry_run || !args.modify;
    
    if check_only {
        println!("Running in check-only mode");
    } else {
        println!("Running in modify mode");
    }
}
```

### Example 2: Working with Value Enums

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReportFormat {
    Html,
    Json,
    Csv,
}

#[derive(Parser, Debug)]
struct Args {
    /// Report format
    #[arg(long, value_enum)]
    format: Option<ReportFormat>,
    
    /// Output path for report
    #[arg(long, requires = "format")]
    output: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    
    if let Some(format) = args.format {
        let output = args.output.unwrap(); // Safe because of requires
        
        match format {
            ReportFormat::Html => generate_html_report(&output),
            ReportFormat::Json => generate_json_report(&output),
            ReportFormat::Csv => generate_csv_report(&output),
        }
    }
}
```

### Example 3: Git Integration Options

```rust
#[derive(Parser, Debug)]
struct Args {
    /// Files to process
    #[arg(required = true)]
    files: Vec<PathBuf>,
    
    /// Ratchet mode: only check files changed relative to a git reference
    #[arg(long)]
    ratchet: Option<String>,
    
    /// Only consider files in the current git repository
    #[arg(long)]
    git_only: Option<bool>,
}

fn main() {
    let args = Args::parse();
    
    if args.git_only.unwrap_or(false) {
        if !is_git_repository() {
            eprintln!("Error: git-only mode enabled but not in a git repository");
            std::process::exit(1);
        }
        
        println!("Processing only tracked git files");
    }
    
    if let Some(ref reference) = args.ratchet {
        println!("Ratchet mode enabled, comparing to: {}", reference);
        // Only process files that changed relative to the reference
    }
}
```

## Conclusion

Clap provides a powerful, type-safe way to handle command-line arguments in Rust. By using the derive approach, you can create complex CLI applications with minimal boilerplate while maintaining excellent help documentation and user experience.

For further information, see the [official Clap documentation](https://docs.rs/clap/latest/clap/).