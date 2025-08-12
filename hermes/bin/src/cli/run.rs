//! Run cli command

use std::{path::PathBuf, time::Duration};

use clap::Args;
use console::Emoji;

use crate::{
    cli::Cli,
    event::queue::Exit,
    ipfs,
    packaging::{
        app::{build_app, ApplicationPackage},
        sign::certificate::{self, Certificate},
    },
    pool, reactor,
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

    /// Shutdown an application after the timeout (milliseconds)
    #[arg(long)]
    timeout_ms: Option<u64>,
}

impl Run {
    #[allow(unreachable_code)]
    /// Run the hermes application
    pub(crate) fn exec(self) -> anyhow::Result<Exit> {
        for cert_path in self.certificates {
            let cert = Certificate::from_file(cert_path)?;
            certificate::storage::add_certificate(cert)?;
        }

        let package = ApplicationPackage::from_file(self.app_package)?;
        package.validate(self.untrusted)?;

        let hermes_home_dir = Cli::hermes_home()?;

        // enable bootstrapping the IPFS node to default addresses
        let default_bootstrap = true;
        tracing::info!("{} Bootstrapping IPFS node", console::Emoji::new("🖧", ""),);
        ipfs::bootstrap(hermes_home_dir.as_path(), default_bootstrap)?;
        let app = build_app(&package, hermes_home_dir)?;

        let exit_lock = reactor::init()?;
        pool::init()?;
        println!(
            "{} Loading application {}...",
            Emoji::new("🛠️", ""),
            app.name()
        );
        reactor::load_app(app)?;

        let exit = if let Some(timeout_ms) = self.timeout_ms {
            exit_lock.wait_timeout(Duration::from_millis(timeout_ms))
        } else {
            exit_lock.wait()
        };

        Ok(exit)
    }
}
