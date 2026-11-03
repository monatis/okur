//! `okur.exe`: the screen reader's console program ([`okur::cli`]).

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    std::process::exit(okur::cli::main().await);
    #[cfg(not(windows))]
    std::process::exit(okur::not_on_this_system());
}
