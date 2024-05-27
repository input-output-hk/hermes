//! Run cli command

use crate::reactor::HermesReactor;

/// Run cli command
pub(crate) struct Run;

impl Run {
    /// Run the hermes
    pub(crate) fn exec() -> anyhow::Result<()> {
        let mut reactor = HermesReactor::new(vec![])?;
        reactor.wait()?;

        Ok(())
    }
}
