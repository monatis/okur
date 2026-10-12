use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "sr", about = "Open-Source Rust Screen Reader CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Test {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,
    },
    Validate {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,
    },
}

fn main() {
    let cli = Cli::parse();
    println!("screen-reader CLI: {:?}", cli);
    sr_core::init();
    sr_platform::init();
    sr_corpus::init();
}
