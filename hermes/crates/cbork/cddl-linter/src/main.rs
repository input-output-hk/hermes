//! CDDL linter cli tool

mod cli;
mod errors;

fn main() {
    use clap::Parser;

    cli::Cli::parse().exec();
}
