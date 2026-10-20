//! `okur-sr.exe`: the installed screen reader, with UI Access in its manifest
//! (`manifests/okur-sr.manifest`, embedded by `build.rs`). See [`okur::sr`].

#![cfg_attr(windows, windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    std::process::exit(okur::sr::main().await);
}
