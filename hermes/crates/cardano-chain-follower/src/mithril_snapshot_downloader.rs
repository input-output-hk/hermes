//! Internal Mithril snapshot downloader task.
//!
//! This task is responsible for downloading Mithril snapshot files. It downloads the
//! latest snapshot file and then sleeps until the next snapshot is available.
use chrono::{TimeDelta, Utc};
use mithril_client::{Certificate, Client, Snapshot, SnapshotListItem};
use tokio::time::{sleep, Duration};
use tracing::error;

use crate::{
    mithril_snapshot::{read_aggregator_url, read_genesis_vkey},
    Network,
};

/// The minimum duration between checks for a new Mithril Snapshot. (Must be same as
/// `MINIMUM_MITHRIL_UPDATE_CHECK_DURATION`)
const MINIMUM_MITHRIL_UPDATE_CHECK_INTERVAL: TimeDelta = TimeDelta::minutes(10); // 10 Minutes
/// The minimum duration between checks for a new Mithril Snapshot. (Must be same as
/// `MINIMUM_MITHRIL_UPDATE_CHECK_INTERVAL`)
const MINIMUM_MITHRIL_UPDATE_CHECK_DURATION: Duration = Duration::from_secs(10 * 60); // 10 Minutes
/// Average Mithril Update is 6 Hrs, so don;t wait longer than 7.
const MAXIMUM_MITHRIL_UPDATE_CHECK_INTERVAL: TimeDelta = TimeDelta::hours(7); // 7 Hours
/// Average Mithril Update is 6 Hrs, so don;t wait longer than 7.
const EXPECTED_MITHRIL_UPDATE_CHECK_INTERVAL: TimeDelta = TimeDelta::hours(6); // 6 Hours
/// We shouldn't get errors that need to wait for this, but if we do wait this long.
/// These errors should be transient if they occur.
const DOWNLOAD_ERROR_RETRY_DURATION: Duration = Duration::from_secs(2 * 60); // 2 Minutes

/// Returns the Latest and chronologically previous snapshots data from the Aggregator.
/// Will return None if it can not get the Snapshot list, or there are no entries in it.
/// If there is only a single entry then the latest and chronologically next will be
/// identical.
async fn get_latest_snapshots(
    client: &Client, network: Network,
) -> Option<(SnapshotListItem, SnapshotListItem)> {
    // Get current latest snapshot from the aggregator
    let snapshots = match client.snapshot().list().await {
        Ok(s) => s,
        Err(e) => {
            error!("Unexpected Error [{}]: Unable to get Snapshot List from Aggregator for {}.  Mithril Snapshots can not update. Sleeping.", network, e);
            return None;
        },
    };

    // Get the current latest snapshot.
    let Some(latest_snapshot) = snapshots.first() else {
        error!("Unexpected Error: Empty Snapshot List from Aggregator for {}.  Mithril Snapshots can not update. Sleeping", network);
        return None;
    };

    let chronologically_previous = snapshots.get(1).unwrap_or(latest_snapshot);

    Some((latest_snapshot.clone(), chronologically_previous.clone()))
}

/// Create a client, should never fail, but return None if it does, because we can't
/// continue.
fn create_client(network: Network) -> Option<Client> {
    // This Can't fail, because we pre-validated the key exists. But just in case...
    let Some(aggregator_url) = read_aggregator_url(network) else {
        error!("Unexpected Error: No Aggregator URL Configured for {}.  Mithril Snapshots can not update.", network);
        return None;
    };

    // This Can't fail, because we pre-validated the key exists. But just in case...
    let Some(genesis_vkey) = read_genesis_vkey(network) else {
        error!("Unexpected Error: No Genesis VKEY Configured for {}.  Mithril Snapshots can not update.", network);
        return None;
    };

    // This can't fail, because we already tested it works. But just in case...
    let client = match mithril_client::ClientBuilder::aggregator(&aggregator_url, &genesis_vkey)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!("Unexpected Error [{}]: Unable to create Mithril Client for {}.  Mithril Snapshots can not update.", network, e);
            return None;
        },
    };

    Some(client)
}

/// Calculate how long we should wait before we check for another Mithril snapshot.
fn calculate_sleep_duration(
    latest_snapshot: &SnapshotListItem, previous_snapshot: &SnapshotListItem,
) -> Duration {
    // All times are relative to UTC.
    let now = Utc::now();
    let mut next_sleep = MINIMUM_MITHRIL_UPDATE_CHECK_INTERVAL;

    // How long between snapshots,
    let mut snapshot_interval = (latest_snapshot.created_at - previous_snapshot.created_at)
        .max(MAXIMUM_MITHRIL_UPDATE_CHECK_INTERVAL);

    // We should never be negative, but we CAN be zero if there was no chronologically
    // previous snapshot. In this case GUESS how long the interval should be based on
    // experience.
    if snapshot_interval <= TimeDelta::seconds(0) {
        snapshot_interval = EXPECTED_MITHRIL_UPDATE_CHECK_INTERVAL;
    }

    let next_expected_snapshot = latest_snapshot.created_at + snapshot_interval;

    if next_expected_snapshot > now {
        // We are behind schedule.  Sleep until the next expected snapshot should be published.
        next_sleep = next_expected_snapshot - now;
    }

    next_sleep
        .to_std()
        .unwrap_or(MINIMUM_MITHRIL_UPDATE_CHECK_DURATION)
}

/// Get the actual snapshot from the specified `snapshot_item` from the list of snapshots.
/// Returns None if there are any issues doing this, otherwise the Snapshot.
/// The only issues should be transient communications errors.
async fn get_snapshot(
    client: &Client, snapshot_item: &SnapshotListItem, network: Network,
) -> Option<Snapshot> {
    let latest_digest = snapshot_item.digest.as_ref();
    let snapshot = match client.snapshot().get(latest_digest).await {
        Ok(snapshot) => {
            if let Some(snapshot) = snapshot {
                snapshot
            } else {
                // Some kind of communications error has ocurred.
                error!("No snapshot returned for {} ???", network);
                return None;
            }
        },
        Err(err) => {
            // Some kind of communications error has ocurred.
            error!(
                "Failure to get the latest snapshot for {} with error: {}",
                network, err
            );
            return None;
        },
    };

    Some(snapshot)
}

/// Download and Verify the Snapshots certificate
async fn download_and_verify_snapshot_certificate(
    client: &Client, snapshot: &Snapshot, network: Network,
) -> Option<Certificate> {
    let certificate = match client
        .certificate()
        .verify_chain(&snapshot.certificate_hash)
        .await
    {
        Ok(certificate) => certificate,
        Err(err) => {
            // The certificate is invalid.
            error!("The certificate for {} is invalid: {}", network, err);
            return None;
        },
    };

    Some(certificate)
}

/// Handle the background downloading of Mithril snapshots for a given network.
/// Note: There can ONLY be at most three of these running at any one time.
/// This is because there can ONLY be one snapshot for each of the three known Cardano
/// networks.
pub(crate) async fn background_mithril_update(network: Network) {
    let mut previous_snapshot_data: Option<SnapshotListItem> = None;
    let mut next_sleep = Duration::from_secs(0);

    let Some(client) = create_client(network) else {
        // If we couldn't create a client, then we don' t need to do anything.
        return;
    };

    loop {
        // Wait until its likely we have a new snapshot ready to download.
        sleep(next_sleep).await;

        // Default sleep if we end up back at the top of this loop because of an error.
        next_sleep = DOWNLOAD_ERROR_RETRY_DURATION;

        // This should only fail if the Aggregator is offline.
        // Because we check we can talk to the aggregator before we create the downloader task.
        let Some((latest_snapshot, chronologically_previous_snapshot)) =
            get_latest_snapshots(&client, network).await
        else {
            // If we couldn't get the latest snapshot then we don't need to do anything else.
            continue;
        };

        // Check if the latest snapshot is different from our actual previous one.
        if let Some(ref previous_snapshot) = previous_snapshot_data {
            // We have a previous snapshot, check if the latest is different, and wait if it isn't.
            if previous_snapshot.digest == latest_snapshot.digest {
                next_sleep =
                    calculate_sleep_duration(&latest_snapshot, &chronologically_previous_snapshot);
                continue;
            }
        }

        // Download the snapshot from the aggregator.
        let Some(snapshot) = get_snapshot(&client, &latest_snapshot, network).await else {
            // If we couldn't get the snapshot then we don't need to do anything else, transient
            // error.
            next_sleep = DOWNLOAD_ERROR_RETRY_DURATION;
            continue;
        };

        // Download and Verify the certificate.
        let certificate =
            download_and_verify_snapshot_certificate(&client, &snapshot, network).await
        else {
            next_sleep = DOWNLOAD_ERROR_RETRY_DURATION;
            continue;
        };

        // Download and unpack the actual snapshot archive.

        // Update the previous snapshot to the latest.
        previous_snapshot_data = Some(latest_snapshot.clone());
    }
}
