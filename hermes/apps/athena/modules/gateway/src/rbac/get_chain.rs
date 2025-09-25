//! Utilities for obtaining a RBAC registration chain (`RegistrationChain`).

use anyhow::{bail, Context, Result};
use cardano_chain_follower::{ChainFollower, Network, Point, Slot, TxnIndex};
use catalyst_types::catalyst_id::CatalystId;
use futures::{future::try_join, TryFutureExt, TryStreamExt};
use rbac_registration::{cardano::cip509::Cip509, registration::cardano::RegistrationChain};

use crate::{rbac::ChainInfo, settings::Settings};

/// Returns the latest (including the volatile part) registration chain by the given
/// Catalyst ID.
pub async fn latest_rbac_chain(id: &CatalystId) -> Result<Option<ChainInfo>> {
    let id = id.as_short_id();

    // let volatile_session =
    //     CassandraSession::get(false).context("Failed to get volatile Cassandra session")?;

    pub(crate) struct Query {
        // /// Registration transaction id.
        // #[allow(dead_code)]
        // pub txn_id: DbTransactionId,
        // /// A block slot number.
        // pub slot_no: DbSlot,
        // /// A transaction index.
        // pub txn_index: DbTxnIndex,
        // /// A previous  transaction id.
        // #[allow(dead_code)]
        // pub prv_txn_id: Option<DbTransactionId>,
        // /// A set of removed stake addresses.
        // pub removed_stake_addresses: HashSet<DbStakeAddress>,
    }
    // Get the persistent part of the chain and volatile registrations. Both of these parts
    // can be non-existing.
    // TODO: update
    let (chain, volatile_regs): (Option<RegistrationChain>, Vec<Query>) = (None, vec![]);
    // try_join(
    //     persistent_rbac_chain(&id),
    //     indexed_regs(&volatile_session, &id),
    // )
    // .await?;

    let mut last_persistent_txn = None;
    let mut last_persistent_slot = 0.into();

    // Either update the persistent chain or build a new one.
    // TODO: link db
    let chain: Option<RegistrationChain> = None;
    // match chain {
    //     Some(c) => {
    //         last_persistent_txn = Some(c.current_tx_id_hash());
    //         last_persistent_slot = c.current_point().slot_or_default();
    //         Some(apply_regs(c, volatile_regs).await?)
    //     },
    //     None => build_rbac_chain(volatile_regs).await?,
    // };

    Ok(chain.map(|chain| {
        let last_txn = Some(chain.current_tx_id_hash());
        // If the last persistent transaction ID is the same as the last one, then there are no
        // volatile registrations in this chain.
        let last_volatile_txn = if last_persistent_txn == last_txn {
            None
        } else {
            last_txn
        };

        ChainInfo {
            chain,
            last_persistent_txn,
            last_volatile_txn,
            last_persistent_slot,
        }
    }))
}
