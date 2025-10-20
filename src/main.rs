//! Binary entry point that wires the CLI to the library application layer.

use clap::Parser;

use dotstrap::{Cli, run};

fn main() {
    let cli = Cli::parse();
    match run(cli) {
        Ok(report) => {
            if report.dry_run {
                println!(
                    "Dry run complete: {} templates evaluated.",
                    report.rendered.len()
                );
            }
        }
        Err(err) => {
            eprintln!("dotstrap failed: {err}");
            std::process::exit(1);
        }
    }
}
