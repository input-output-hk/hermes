//! Internal Mithril snapshot downloader task.
//!
//! This task is responsible for downloading Mithril snapshot files. It downloads the
//! latest snapshot file and then sleeps until the next snapshot is available.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use chrono::{TimeDelta, Utc};
use mithril_client::{
    common::CompressionAlgorithm, snapshot_downloader::SnapshotDownloader, Client, MessageBuilder,
    MithrilCertificate, MithrilResult, Snapshot, SnapshotListItem,
};
use tokio::time::{sleep, Duration};
use tracing::{debug, error};
use turbo_downloader::TurboDownloaderOptions;

use crate::{mithril_snapshot::update_tip, Network};

/// The minimum duration between checks for a new Mithril Snapshot. (Must be same as
/// `MINIMUM_MITHRIL_UPDATE_CHECK_DURATION`)
const MINIMUM_MITHRIL_UPDATE_CHECK_INTERVAL: TimeDelta = TimeDelta::minutes(10); // 10 Minutes
/// The minimum duration between checks for a new Mithril Snapshot. (Must be same as
/// `MINIMUM_MITHRIL_UPDATE_CHECK_INTERVAL`)
const MINIMUM_MITHRIL_UPDATE_CHECK_DURATION: Duration = Duration::from_secs(10 * 60); // 10 Minutes
/// Average Mithril Update is 6 Hrs, so don't wait longer than 7.
const MAXIMUM_MITHRIL_UPDATE_CHECK_INTERVAL: TimeDelta = TimeDelta::hours(7); // 7 Hours
/// Average Mithril Update is 6 Hrs, so don't wait longer than 7.
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
fn create_client(network: Network, aggregator_url: &str, genesis_vkey: &str) -> Option<Client> {
    let downloader = match TurboSnapshotDownloader::new() {
        Ok(downloader) => Arc::new(downloader),
        Err(err) => {
            error!("Unexpected Error [{}]: Unable to create Snapshot Downloader for {}. Mithril Snapshots can not update.", err,network);
            return None;
        },
    };

    // This can't fail, because we already tested it works. But just in case...
    let client = match mithril_client::ClientBuilder::aggregator(aggregator_url, genesis_vkey)
        //.add_feedback_receiver(receiver)
        .with_snapshot_downloader(downloader)
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
) -> Option<MithrilCertificate> {
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

/// This function creates a client based on the given network and genesis vkey.
///
/// # Arguments
///
/// * `network` - The network type for the client to connect to.
/// * `aggregator_url` - A reference to the URL of an aggregator that can be used to
///   create the client.
/// * `genesis_vkey` - The genesis verification key, which is needed to authenticate with
///   the server.
///
/// # Returns
///
/// This function returns a `Client` object if it successfully connects to the specified
/// URL and creates a client. If it fails, it waits for `DOWNLOAD_ERROR_RETRY_DURATION`
/// before attempting again. This never times out, as we can not attempt this if the
/// aggregator was not contactable when the parameters were defined.
async fn connect_client(network: Network, aggregator_url: &str, genesis_vkey: &str) -> Client {
    // Note: we pre-validated connection before we ran, so failure here should be transient.
    // Just wait if we fail, and try again later.
    loop {
        if let Some(client) = create_client(network, aggregator_url, genesis_vkey) {
            return client;
        }

        // If we couldn't create a client, then we don' t need to do anything.
        // Error already logged in create_client, no need to print anything here.
        sleep(DOWNLOAD_ERROR_RETRY_DURATION).await;
    }
}

/// Handle the background downloading of Mithril snapshots for a given network.
/// Note: There can ONLY be at most three of these running at any one time.
/// This is because there can ONLY be one snapshot for each of the three known Cardano
/// networks.
/// # Arguments
///
/// * `network` - The network type for the client to connect to.
/// * `aggregator_url` - A reference to the URL of an aggregator that can be used to
///   create the client.
/// * `genesis_vkey` - The genesis verification key, which is needed to authenticate with
///   the server.
///
/// # Returns
///
/// This does not return, it is a background task.
pub(crate) async fn background_mithril_update(
    network: Network, aggregator_url: String, genesis_vkey: String, mithril_path: PathBuf,
) {
    debug!("Mithril Snapshot background updater for: {network} from {aggregator_url} to {mithril_path:?} : Starting");
    let mut previous_snapshot_data: Option<SnapshotListItem> = None;
    let mut next_sleep = Duration::from_secs(0);

    // Note: we pre-validated connection before we ran, so failure here should be transient.
    // Just wait if we fail, and try again later.
    let client = connect_client(network, &aggregator_url, &genesis_vkey).await;

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
            continue;
        };

        // Download and Verify the certificate.
        let Some(certificate) =
            download_and_verify_snapshot_certificate(&client, &snapshot, network).await
        else {
            continue;
        };

        // Download and unpack the actual snapshot archive.
        if let Err(error) = client
            .snapshot()
            .download_unpack(&snapshot, &mithril_path)
            .await
        {
            // If we couldn't download and unpack, assume its a transient error,
            error!("Failed to Download and Unpack snapshot: {error}");
            continue;
        }

        if let Err(error) = client.snapshot().add_statistics(&snapshot).await {
            // Just log not fatal to anything.
            error!("Could not increment snapshot download statistics for {network}: {error}");
            // We can processing the download even after this fails.
        }

        match MessageBuilder::new()
            .compute_snapshot_message(&certificate, &mithril_path)
            .await
        {
            Ok(message) => {
                if !certificate.match_message(&message) {
                    // If we couldn't match then assume its a transient error.
                    error!(
                        "Failed to Match Certificate and Computed Snapshot Message for {network}!"
                    );
                    continue;
                }
            },
            Err(error) => {
                // If we couldn't build the message then assume its a transient error.
                error!("Failed to Compute Snapshot Message: {error}");
                continue;
            },
        }

        // Download was A-OK - Update the new immutable tip.
        if let Err(error) = update_tip(network) {
            // If we couldn't update the tip then assume its a transient error.
            error!("Failed to Update Tip for {network}: {error}");
            continue;
        }

        // Update the previous snapshot to the latest.
        previous_snapshot_data = Some(latest_snapshot.clone());
    }
}

/// A snapshot downloader that only handles download through HTTP.
pub struct TurboSnapshotDownloader {
    /// Handle to a HTTP client to use for downloading simply.
    http_client: reqwest::Client,
    /// Turbo Downloader options.
    turbo_downloader: Arc<TurboDownloaderOptions>,
}

impl TurboSnapshotDownloader {
    /// Constructs a new `HttpSnapshotDownloader`.
    pub fn new() -> MithrilResult<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .build()
            .with_context(|| "Building http client for TurboSnapshotDownloader failed")?;

        let turbo_downloader = TurboDownloaderOptions::default();

        Ok(Self {
            http_client,
            turbo_downloader: turbo_downloader.into(),
        })
    }
}

#[async_trait]
impl SnapshotDownloader for TurboSnapshotDownloader {
    async fn download_unpack(
        &self, location: &str, target_dir: &Path, _compression_algorithm: CompressionAlgorithm,
        _download_id: &str, _snapshot_size: u64,
    ) -> MithrilResult<()> {
        if !target_dir.is_dir() {
            Err(
                anyhow!("target path is not a directory or does not exist: `{target_dir:?}`")
                    .context("Download-Unpack: prerequisite error"),
            )?;
        }

        debug!("Download and Unpack started='{location}' to '{target_dir:?}'.");

        let downloader = self
            .turbo_downloader
            .start_download(location, target_dir.to_path_buf())
            .await?;

        while !downloader.is_finished() {
            let progress = downloader.get_progress();
            debug!("Download-Unpack: Downloading snapshot, progress={progress:?}");
            let progress = downloader.get_progress_human_line();
            debug!("Download-Unpack: Downloading snapshot, progress={progress:?}");
            sleep(Duration::from_secs(10)).await;
        }

        debug!("Download and Unpack finished='{location}' to '{target_dir:?}'.");

        Ok(())
    }

    async fn probe(&self, location: &str) -> MithrilResult<()> {
        debug!("HEAD Snapshot location='{location}'.");

        let request_builder = self.http_client.head(location);
        let response = request_builder.send().await.with_context(|| {
            format!("Cannot perform a HEAD for snapshot at location='{location}'")
        })?;

        let status = response.status();

        debug!("Probe for '{location}' completed: {status}");

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            reqwest::StatusCode::NOT_FOUND => {
                Err(anyhow!("Snapshot location='{location} not found"))
            },
            status_code => Err(anyhow!("Unhandled error {status_code}")),
        }
    }
}
