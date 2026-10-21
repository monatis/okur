//! Embeds each program's application manifest at link time (Windows, MSVC):
//! - `okur-lab.exe`, the verification harness's console program: `asInvoker`, no UI Access;
//! - `okur-lab-keys.exe`, the reference machine's key sender: `asInvoker` with `uiAccess="true"`.
//!
//! Each manifest is whole, its `trustInfo` included, so the linker is told to add no UAC
//! fragment of its own (`/MANIFESTUAC:NO`) that could collide with it
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.2). Check the result with
//! `mt.exe -inputresource:okur-lab-keys.exe;#1 -out:con`, or `okur doctor`.

use std::path::PathBuf;

/// Each program and its manifest file under `manifests/`.
const MANIFESTS: [(&str, &str); 2] = [
    ("okur-lab", "okur-lab.manifest"),
    ("okur-lab-keys", "okur-lab-keys.manifest"),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        println!("cargo:warning=no CARGO_MANIFEST_DIR: the programs get no manifest");
        return;
    };
    let manifests = PathBuf::from(dir).join("manifests");
    for (_, file) in MANIFESTS {
        println!("cargo:rerun-if-changed={}", manifests.join(file).display());
    }
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if !(windows && msvc) {
        return;
    }
    for (program, file) in MANIFESTS {
        let path = manifests.join(file);
        println!("cargo:rustc-link-arg-bin={program}=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin={program}=/MANIFESTINPUT:{}",
            path.display()
        );
        println!("cargo:rustc-link-arg-bin={program}=/MANIFESTUAC:NO");
    }
}
