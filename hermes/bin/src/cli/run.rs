//! Run cli command

use std::path::PathBuf;

use clap::Args;

use crate::{
    app::{HermesApp, HermesAppName},
    logger::{self, LoggerConfigBuilder},
    reactor::HermesReactor,
};

/// Run cli command
#[derive(Args)]
pub(crate) struct Run {
    ///
    #[clap(flatten)]
    log_config: LoggerConfigBuilder,

    /// App name
    app_name: String,

    /// App directory
    app_dir: PathBuf,
}

impl Run {
    /// Run the hermes
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        logger::init(&self.log_config.build())?;

        let app = HermesApp::from_dir(HermesAppName(self.app_name), self.app_dir)?;
        let mut reactor = HermesReactor::new(vec![app])?;
        reactor.wait()?;

        Ok(())
    }
}
