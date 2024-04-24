//! Run cli command

use clap::Args;

use crate::logger::{self, LoggerConfigBuilder};

/// Run cli command
#[derive(Args)]
pub(crate) struct Run {
    ///
    #[clap(flatten)]
    log_config: LoggerConfigBuilder,
}

impl Run {
    /// Run the hermes
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        logger::init(&self.log_config.build())?;
        Ok(())
    }
}
