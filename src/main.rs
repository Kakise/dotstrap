//! Binary entry point that wires the CLI to the library application layer.
use dotstrap::execute_cli;

fn main() {
    std::process::exit(execute_cli(std::env::args()));
}
