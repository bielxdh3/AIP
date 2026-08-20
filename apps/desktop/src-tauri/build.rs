use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn build_value(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

#[cfg(windows)]
const WINDOWS_TEST_MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

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
    #[cfg(windows)]
    let attributes = if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest())
    } else {
        tauri_build::Attributes::new()
    };
    #[cfg(not(windows))]
    let attributes = tauri_build::Attributes::new();
    tauri_build::try_build(attributes).expect("failed to run tauri-build");

    #[cfg(windows)]
    embed_windows_test_manifest();
}

#[cfg(windows)]
fn embed_windows_test_manifest() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }

    let manifest = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("OUT_DIR must be set for the Windows test manifest"),
    )
    .join("windows-test-manifest.xml");
    std::fs::write(&manifest, WINDOWS_TEST_MANIFEST)
        .expect("failed to write the Windows test manifest");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    println!("cargo:rustc-link-arg=/WX");
}
