use clap::{Parser, Subcommand};
use iwm_detector::{detect_input, load_package, selected_executable};
use iwm_parser::build_package;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "iwm-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Detect {
        #[arg(long)]
        input: PathBuf,
    },
    BuildPackage {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect { input } => match detect_input(&input) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        },
        Commands::BuildPackage { input, output } => {
            let package = match load_package(&input) {
                Ok(package) => package,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };

            let report = match detect_input(&input) {
                Ok(report) => report,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };

            if report.verdict != iwm_detector::DetectionVerdict::Gm8Likely {
                eprintln!("input is not classified as gm8-likely");
                std::process::exit(2);
            }

            let exe = match selected_executable(&package) {
                Ok(exe) => exe,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(2);
                }
            };

            if let Err(err) = build_package(exe, &output, &report.dlls) {
                eprintln!("{err:#}");
                std::process::exit(1);
            }
        }
    }
}
