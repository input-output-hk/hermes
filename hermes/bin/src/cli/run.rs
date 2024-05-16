//! Run cli command

use crate::{
    logger::{self, LoggerConfigBuilder},
    reactor::HermesReactor,
};

/// Run cli command
pub(crate) struct Run;

impl Run {
    /// Run the hermes
    pub(crate) fn exec() -> anyhow::Result<()> {
        let log_config = LoggerConfigBuilder::default().build();
        logger::init(&log_config)?;

        let mut reactor = HermesReactor::new(vec![])?;
        reactor.wait()?;

        Ok(())
    }
}
