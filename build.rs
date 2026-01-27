use std::process::Command;

fn main() {
  // Get the short commit hash
  let hash = Command::new("git")
    .args(["rev-parse", "--short", "HEAD"])
    .output()
    .ok()
    .filter(|o| o.status.success())
    .and_then(|o| String::from_utf8(o.stdout).ok())
    .map(|s| s.trim().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  // Get the commit date in YYYY-MM-DD format
  let date = Command::new("git")
    .args(["log", "-1", "--format=%cs"])
    .output()
    .ok()
    .filter(|o| o.status.success())
    .and_then(|o| String::from_utf8(o.stdout).ok())
    .map(|s| s.trim().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  println!("cargo:rustc-env=GIT_HASH={hash}");
  println!("cargo:rustc-env=GIT_DATE={date}");
}
