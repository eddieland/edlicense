use std::process::Command;

fn main() {
  embed_build_info();
  set_rerun_conditions();
}

fn embed_build_info() {
  // Capture the current Git commit hash for version identification.
  // Falls back gracefully if Git is unavailable or not in a repository.
  if let Ok(output) = Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
    let git_hash = String::from_utf8(output.stdout).unwrap_or_default().trim().to_string();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
  }

  // Capture the commit date in YYYY-MM-DD format.
  // Falls back gracefully if Git is unavailable.
  if let Ok(output) = Command::new("git").args(["log", "-1", "--format=%cs"]).output() {
    let git_date = String::from_utf8(output.stdout).unwrap_or_default().trim().to_string();
    println!("cargo:rustc-env=GIT_DATE={git_date}");
  }
}

fn set_rerun_conditions() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=.git/HEAD");
}
