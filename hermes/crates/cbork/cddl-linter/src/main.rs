//! CDDL linter cli tool

mod cli;

fn main() {
    use clap::Parser;

    cli::Cli::parse().exec();
}
