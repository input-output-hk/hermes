//! Run cli command

use std::{path::PathBuf, process::ExitCode, time::Duration};

use clap::Args;
use console::Emoji;

use crate::{
    app::set_no_parallel_event_execution,
    cli::{Cli, RuntimeConfig},
    event::queue::Exit,
    ipfs,
    packaging::{
        app::{ApplicationPackage, build_app},
        sign::certificate::{self, Certificate},
    },
    pool, reactor,
    runtime_extensions::hermes::sqlite,
};

/// Run cli command
#[derive(Args)]
pub(crate) struct Run {
    /// Path to the Hermes application package to run
    app_package: PathBuf,

    /// Path to the trusted certificate
    #[clap(name = "cert", short)]
    certificates: Vec<PathBuf>,

    /// Flag which disables package signature verification
    #[clap(long, action = clap::ArgAction::SetTrue)]
    untrusted: bool,

    /// See [`RuntimeConfig`] docs
    #[clap(flatten)]
    rt_config: RuntimeConfig,
}

impl Run {
    #[allow(unreachable_code)]
    /// Run the hermes application
    pub(crate) fn exec(self) -> anyhow::Result<Exit> {
        let exit_lock = reactor::init()?;

        for cert_path in self.certificates {
            let cert = Certificate::from_file(cert_path)?;
            certificate::storage::add_certificate(cert)?;
        }

        let package = ApplicationPackage::from_file(self.app_package)?;
        package.validate(self.untrusted)?;

        let hermes_home_dir = Cli::hermes_home()?;

        // Read custom bootstrap peers from environment variable
        // Format: comma-separated multiaddrs, e.g.:
        // /ip4/172.20.0.11/tcp/4001/p2p/12D3KooW...,/dns4/seed.example.com/tcp/4001/p2p/12D3KooW.
        // ..
        let custom_bootstrap_peers = std::env::var("IPFS_BOOTSTRAP_PEERS").ok().map(|peers_str| {
            peers_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        });

        // Only use public bootstrap if no custom peers provided
        let default_bootstrap = custom_bootstrap_peers.is_none();

        if let Some(ref peers) = custom_bootstrap_peers {
            tracing::info!(
                "{} Bootstrapping IPFS node with {} custom peer(s)",
                console::Emoji::new("🖧", ""),
                peers.len()
            );
        } else {
            tracing::info!(
                "{} Bootstrapping IPFS node with default public peers",
                console::Emoji::new("🖧", "")
            );
        }

        ipfs::bootstrap(ipfs::Config {
            base_dir: &hermes_home_dir,
            default_bootstrap,
            custom_peers: custom_bootstrap_peers,
        })?;
        let app = build_app(&package, hermes_home_dir)?;

        if self.rt_config.serialize_sqlite {
            sqlite::set_serialized();
        }

        if self.rt_config.no_parallel_event_execution {
            set_no_parallel_event_execution();
        } else {
            pool::init()?;
        }

        let app_name = app.name().clone();

        println!(
            "{} Loading application {}...",
            Emoji::new("🛠️", ""),
            app_name
        );
        // TODO[RC]: Prevent the app from receiving any events until it is fully initialized.
        // TODO[RC]: Currently, when a module fails to initialize, the whole app fails to run.
        reactor::load_app(app)?;

        let _ = reactor::initialize_smt(app_name);

        let exit = if let Some(timeout_ms) = self.rt_config.timeout_ms {
            exit_lock.wait_timeout(Duration::from_millis(timeout_ms))
        } else {
            exit_lock.wait()
        };

        // Stop accepting new events first to prevent event queue from continuing to process
        let exit_code = exit.get_exit_code().unwrap_or(ExitCode::SUCCESS);
        if let Err(e) = crate::event::queue::shutdown(exit_code) {
            tracing::warn!("Failed to shutdown event queue: {}", e);
        }

        // Shutdown chain sync tasks to prevent them from blocking process exit
        crate::runtime_extensions::hermes::cardano::shutdown_chain_sync();

        if !self.rt_config.no_parallel_event_execution {
            // Wait for scheduled tasks to be finished.
            pool::terminate();
        }
        Ok(exit)
    }
}
