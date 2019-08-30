use chrono::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        std::process::Command::new("git")
            .args(&["describe", "--always"])
            .stdout(std::process::Stdio::piped())
            .output()
            .ok()
            .and_then(|out| std::str::from_utf8(&out.stdout).map(str::to_string).ok())
            .unwrap_or_else(|| "<unknown-revision>".to_owned())
    );
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        Utc::today().format("%Y-%m-%d")
    );
    Ok(())
}
