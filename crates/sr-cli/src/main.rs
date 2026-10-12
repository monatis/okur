use clap::{Parser, Subcommand};
use sr_corpus::DifferentialRunner;
use std::path::Path;

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

        /// Scenario range or specific ID (e.g. "01..10", "1..10", or "01-doc-say-all")
        #[arg(short, long)]
        scenarios: Option<String>,
    },
    Validate {
        #[arg(short, long, default_value = "corpus")]
        corpus_dir: String,
    },
}

fn parse_range(range_str: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = range_str.split("..").collect();
    if parts.len() == 2 {
        let start = parts[0].trim().parse::<u32>().ok()?;
        let end = parts[1].trim().parse::<u32>().ok()?;
        Some((start, end))
    } else {
        None
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Test {
            corpus_dir,
            scenarios,
        }) => {
            let corpus_path = Path::new(&corpus_dir);
            if !corpus_path.exists() {
                eprintln!("Error: corpus directory not found at {:?}", corpus_path);
                std::process::exit(1);
            }

            let runner = DifferentialRunner::new(corpus_path);
            let mut total = 0;
            let mut passed = 0;
            let mut failed = 0;

            println!("============================================================");
            println!("  Differential Screen Reader Test Suite (JAWS Parity)        ");
            println!("============================================================");

            let filter = scenarios.as_deref().unwrap_or("01..10");
            let results = if let Some((start, end)) = parse_range(filter) {
                runner.run_range(start, end)
            } else {
                vec![runner.run_scenario_by_id(filter)]
            };

            for res in results {
                total += 1;
                match res {
                    Ok(scenario_res) => {
                        let status_str = if scenario_res.passed {
                            passed += 1;
                            "PASS"
                        } else {
                            failed += 1;
                            "FAIL"
                        };

                        println!(
                            "[{}] {} - {} ({} steps)",
                            status_str,
                            scenario_res.scenario_id,
                            scenario_res.scenario_name,
                            scenario_res.steps.len()
                        );

                        for step in &scenario_res.steps {
                            if !step.success {
                                eprintln!(
                                    "   Step [{}]: command='{}', speech='{}'",
                                    step.step_id, step.command, step.emitted_speech
                                );
                                for fail in &step.failed_assertions {
                                    eprintln!("     * {}", fail);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        eprintln!("[ERROR] Execution failure: {}", e);
                    }
                }
            }

            println!("------------------------------------------------------------");
            println!(
                "Summary: {} total, {} passed, {} failed ({:.1}% parity)",
                total,
                passed,
                failed,
                if total > 0 {
                    (passed as f64 / total as f64) * 100.0
                } else {
                    0.0
                }
            );
            println!("============================================================");

            if failed > 0 {
                std::process::exit(1);
            }
        }
        Some(Commands::Validate { corpus_dir }) => {
            println!("Validating corpus manifest at {}...", corpus_dir);
            sr_corpus::init();
        }
        None => {
            println!("No command specified. Use `sr-cli test --help` for available options.");
        }
    }
}
