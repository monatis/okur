//! `okur-sr-no-uiaccess.exe`: the installed screen reader without UI Access
//! (`manifests/okur-sr-no-uiaccess.manifest`), which the installer puts in place of
//! `okur-sr.exe` where that one's signature does not verify. See [`okur::sr`].

#![cfg_attr(windows, windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    std::process::exit(okur::sr::main().await);
}
