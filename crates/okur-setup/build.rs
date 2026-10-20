//! Embeds `okur-setup.exe`'s manifest at link time (Windows, MSVC): `asInvoker`, so that
//! Windows' installer detection never elevates it by its name; it elevates its own worker
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.10).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        println!("cargo:warning=no CARGO_MANIFEST_DIR: okur-setup gets no manifest");
        return;
    };
    let manifest = PathBuf::from(dir)
        .join("manifests")
        .join("okur-setup.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if !(windows && msvc) {
        return;
    }
    println!("cargo:rustc-link-arg-bin=okur-setup=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bin=okur-setup=/MANIFESTINPUT:{}",
        manifest.display()
    );
    println!("cargo:rustc-link-arg-bin=okur-setup=/MANIFESTUAC:NO");
}
