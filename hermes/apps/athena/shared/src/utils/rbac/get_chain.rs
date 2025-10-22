//! Utilities for obtaining a RBAC registration chain (`RegistrationChain`).

use anyhow::Result;
use catalyst_types::catalyst_id::CatalystId;

/// Returns the latest (including the volatile part) registration chain by the given
/// Catalyst ID.
pub async fn latest_rbac_chain(_id: &CatalystId) -> Result<Option<ChainInfo>> {
    Ok(None)
}
