use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, ValueEnum};

mod benchs;

#[derive(Copy, Clone, ValueEnum)]
enum BenchName {
    /// Benchmark using cardano-chain-follower crate to read blocks.
    CardanoChainFollower,
    /// Benchmark using standalone pallas crate to read blocks.
    Pallas,
}

#[derive(Parser)]
#[command(version, arg_required_else_help = true)]
struct Cli {
    /// Specifies which benchmark to run.
    #[arg(short, long, value_enum)]
    bench_name: BenchName,

    /// Path to the Mithril snapshot to use in the benchmark.
    #[arg(short, long)]
    mithril_snapshot_path: PathBuf,

    #[arg(long)]
    enable_logging: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.enable_logging {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_env_var("BENCHMARK_LOG_LEVEL")
                    .with_default_directive("info".parse().expect("valid tracing directive"))
                    .from_env_lossy(),
            )
            .init();
    }

    let bench_params = benchs::BenchmarkParams {
        mithril_snapshot_path: cli.mithril_snapshot_path,
    };

    let res = match cli.bench_name {
        BenchName::CardanoChainFollower => benchs::cardano_chain_follower::run(bench_params).await,
        BenchName::Pallas => benchs::pallas::run(&bench_params).await,
    };

    match res {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::FAILURE
        },
    }
}
