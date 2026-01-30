use std::process::Command;

fn main() {
  embed_build_info();
  set_rerun_conditions();
}

fn embed_build_info() {
  // Capture the current Git commit hash for version identification.
  // First check for environment variable (useful in Docker builds where .git isn't available),
  // then fall back to git command.
  let git_hash = std::env::var("GIT_HASH").ok().filter(|s| !s.is_empty()).or_else(|| {
    Command::new("git")
      .args(["rev-parse", "--short", "HEAD"])
      .output()
      .ok()
      .and_then(|o| String::from_utf8(o.stdout).ok())
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
  });
  if let Some(hash) = git_hash {
    println!("cargo:rustc-env=GIT_HASH={hash}");
  }

  // Capture the commit date in YYYY-MM-DD format.
  // First check for environment variable, then fall back to git command.
  let git_date = std::env::var("GIT_DATE").ok().filter(|s| !s.is_empty()).or_else(|| {
    Command::new("git")
      .args(["log", "-1", "--format=%cs"])
      .output()
      .ok()
      .and_then(|o| String::from_utf8(o.stdout).ok())
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
  });
  if let Some(date) = git_date {
    println!("cargo:rustc-env=GIT_DATE={date}");
  }
}

fn set_rerun_conditions() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=.git/HEAD");
}
