//! This crate is the implementation of a benchmark to measure the overhead imposed

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

mod benchs;

/// Possible benchmarks to run.
#[derive(Copy, Clone, ValueEnum)]
enum BenchName {
    /// Benchmark using cardano-chain-follower crate to read blocks.
    CardanoChainFollower,
    /// Benchmark using standalone pallas crate to read blocks.
    Pallas,
}

/// Benchmark command line arguments.
#[derive(Parser)]
#[command(version, arg_required_else_help = true)]
struct Cli {
    /// Specifies which benchmark to run.
    #[arg(short, long, value_enum)]
    bench_name: BenchName,

    /// Path to the Mithril snapshot to use in the benchmark.
    #[arg(short, long)]
    mithril_snapshot_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let bench_params = benchs::BenchmarkParams {
        mithril_snapshot_path: cli.mithril_snapshot_path,
    };

    match cli.bench_name {
        BenchName::CardanoChainFollower => benchs::cardano_chain_follower::run(bench_params).await,
        BenchName::Pallas => benchs::pallas::run(&bench_params).await,
    }
}
