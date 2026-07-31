use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn build_value(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

fn main() {
    let git_sha = std::env::var("GITHUB_SHA").ok().or_else(|| {
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|value| value.trim().to_string())
    });
    println!(
        "cargo:rustc-env=AIP_BUILD_SHA={}",
        git_sha.unwrap_or_else(|| "unknown".into())
    );
    println!(
        "cargo:rustc-env=AIP_BUILD_TIMESTAMP={}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0)
    );
    println!(
        "cargo:rustc-env=AIP_RUNTIME_PACKAGING_MODE={}",
        build_value(
            "AIP_RUNTIME_PACKAGING_MODE",
            if std::env::var("PROFILE").as_deref() == Ok("release") {
                "packaged"
            } else {
                "development"
            }
        )
    );
    tauri_build::build()
}
