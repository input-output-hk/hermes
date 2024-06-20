//! Internal Mithril snapshot downloader task.
//!
//! This task is responsible for downloading Mithril snapshot files. It downloads the
//! latest snapshot file and then sleeps until the next snapshot is available.
use std::{path::Path, process::Stdio, sync::Arc};

use crate::{
    error::{Error, Result},
    mithril_snapshot_config::MithrilSnapshotConfig,
    network::Network,
    snapshot_id::SnapshotId,
};

use anyhow::{anyhow, bail, Context};
use async_compression::tokio::bufread::ZstdDecoder;
use async_trait::async_trait;
use chrono::{TimeDelta, Utc};
use dashmap::DashMap;
use humantime::format_duration;
use mithril_client::{
    common::CompressionAlgorithm, snapshot_downloader::SnapshotDownloader, Client, MessageBuilder,
    MithrilCertificate, MithrilResult, Snapshot, SnapshotListItem,
};
use once_cell::sync::Lazy;
use pallas_hardano::storage::immutable::Point;
use tokio::{
    fs::{create_dir_all, remove_dir_all, File},
    io::BufReader,
    process::Command,
    sync::mpsc::Sender,
    time::{sleep, Duration},
};
use tokio_stream::StreamExt;
use tokio_tar::Archive;
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{debug, error};

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

/// Current TIP of a network.
static CURRENT_TIPS: Lazy<DashMap<Network, Point>> = Lazy::new(DashMap::new);

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

/// Given a particular snapshot ID, find the Actual Snapshot for it.
async fn get_snapshot_by_id(
    client: &Client, network: Network, snapshot_id: &SnapshotId,
) -> Option<SnapshotListItem> {
    let snapshots = match client.snapshot().list().await {
        Ok(s) => s,
        Err(e) => {
            error!("Unexpected Error [{}]: Unable to get Snapshot List from Aggregator for {}.  Mithril Snapshots can not update. Sleeping.", network, e);
            return None;
        },
    };

    // Try and find the current snapshot in the list of available snapshots.
    for snapshot in snapshots {
        if *snapshot_id == snapshot.beacon.immutable_file_number {
            return Some(snapshot);
        }
    }

    None
}

/// Create a client, should never fail, but return None if it does, because we can't
/// continue.
fn create_client(network: Network, cfg: &MithrilSnapshotConfig) -> Option<Client> {
    let downloader = match TurboSnapshotDownloader::new(cfg.clone()) {
        Ok(downloader) => Arc::new(downloader),
        Err(err) => {
            error!("Unexpected Error [{}]: Unable to create Snapshot Downloader for {}. Mithril Snapshots can not update.", err,network);
            return None;
        },
    };

    // This can't fail, because we already tested it works. But just in case...
    let client = match mithril_client::ClientBuilder::aggregator(
        &cfg.aggregator_url,
        &cfg.genesis_key,
    )
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
async fn connect_client(network: Network, cfg: &MithrilSnapshotConfig) -> Client {
    // Note: we pre-validated connection before we ran, so failure here should be transient.
    // Just wait if we fail, and try again later.
    loop {
        if let Some(client) = create_client(network, cfg) {
            return client;
        }

        // If we couldn't create a client, then we don' t need to do anything.
        // Error already logged in create_client, no need to print anything here.
        sleep(DOWNLOAD_ERROR_RETRY_DURATION).await;
    }
}

/// Get the tip from the given path.
fn get_mithril_tip(path: &Path) -> Result<Point> {
    let mut path = path.to_path_buf();
    path.push("immutable");

    debug!(
        "Calculating TIP from Immutable storage @ {}",
        path.to_string_lossy()
    );

    // Read the Tip, and if we don;t get one, or we error, its an error.
    let Some(tip) = pallas_hardano::storage::immutable::get_tip(&path)
        .map_err(|error| Error::MithrilSnapshot(Some(error)))?
    else {
        return Err(Error::MithrilSnapshot(None));
    };

    // Yay, we got a tip, so return it.
    Ok(tip)
}

/// Get the Snapshot Data itself from the Aggregator, and a validate Certificate.
async fn get_mithril_snapshot_and_certificate(
    chain: Network, client: &Client, item: &SnapshotListItem,
) -> Option<(Snapshot, MithrilCertificate)> {
    debug!("Mithril Snapshot background updater for: {chain} : Download snapshot from aggregator.");

    // Download the snapshot from the aggregator.
    let Some(snapshot) = get_snapshot(client, item, chain).await else {
        // If we couldn't get the snapshot then we don't need to do anything else, transient
        // error.
        return None;
    };

    debug!("Mithril Snapshot background updater for: {chain} : Download/Verify certificate.");

    // Download and Verify the certificate.
    let certificate = download_and_verify_snapshot_certificate(client, &snapshot, chain).await?;

    Some((snapshot, certificate))
}

/// Validate that a Mithril Snapshot downloaded matches its certificate.
async fn validate_mithril_snapshot(
    chain: Network, certificate: &MithrilCertificate, path: &Path,
) -> bool {
    let cert = certificate.clone();
    let mithril_path = path.to_path_buf();
    match tokio::spawn(async move {
        // This can be long running and CPU Intensive.
        // So we spawn it off to a background task.
        MessageBuilder::new()
            .compute_snapshot_message(&cert, &mithril_path)
            .await
    })
    .await
    {
        Ok(Ok(result)) => {
            if certificate.match_message(&result) {
                true
            } else {
                // If we couldn't match then assume its a transient error.
                error!("Failed to Match Certificate and Computed Snapshot Message for {chain}!");
                false
            }
        },
        Ok(Err(error)) => {
            // If we got an error then it must be false.
            error!("Failed to Compute Snapshot Message: {error}");
            false
        },
        Err(error) => {
            error!("Snapshot Certificate computation failed: {error}");
            false
        },
    }
}

/// See if we have a latest snapshot already, and if so, validate it.
/// 
/// For a existing mithril snapshot to be valid it has to be:
/// 1. The actual latest mithril snapshot; AND
/// 2. It must 
async fn get_latest_validated_mithril_snapshot(
    chain: Network, client: &Client, cfg: &MithrilSnapshotConfig,
) -> (Option<SnapshotId>, Option<Point>) {
    /// Purge a bad mithril snapshot from disk.
    async fn purge_bad_mithril_snapshot(chain: Network, latest_mithril: &SnapshotId) {
        if let Err(error) = remove_dir_all(&latest_mithril).await {
            // This should NOT happen because we already checked the Mithril path is fully writable.
            error!("Mithril Snapshot background updater for: {chain}: Failed to remove old snapshot {latest_mithril}: {error}");
        }
    }

    // Check if we already have a Mithril snapshot downloaded, and IF we do validate it is intact.
    let Some(latest_mithril) = cfg.latest_snapshot_path().await else {
        return (None, None);
    };

    // Get the actual latest snapshot, shouldn't fail, but say the current is invalid if it does.
    let Some((actual_latest, _)) = get_latest_snapshots(client, chain).await else {
        return (None, None);
    };

    // IF the mithril data we have is NOT the current latest, it may as well be invalid.
    if latest_mithril != actual_latest.beacon.immutable_file_number {
        return (None, None);
    }

    let Some(snapshot) = get_snapshot_by_id(client, chain, &latest_mithril).await else {
        // We have a latest snapshot, but the Aggregator does not know it.
        error!("Mithril Snapshot background updater for: {chain}: Latest snapshot {latest_mithril} does not exist on the Aggregator.");
        purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return (None, None);
    };

    // Download the snapshot/certificate from the aggregator.
    let Some((_, certificate)) =
        get_mithril_snapshot_and_certificate(chain, client, &snapshot).await
    else {
        // If we couldn't get the snapshot then we don't need to do anything else, transient
        // error.
        purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return (None, None);
    };

    if !validate_mithril_snapshot(chain, &certificate, latest_mithril.as_ref()).await {
        purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return (None, None);
    }

    // Make sure we can actually get a TIP from it
    let Ok(tip) = get_mithril_tip(latest_mithril.as_ref()) else {
        purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return (None, None);
    };

    (Some(latest_mithril), Some(tip))
}

/// Get the Mithril client and recover out existing mithril snapshot data, if any.
async fn recover_existing_snapshot(
    chain: Network, cfg: &MithrilSnapshotConfig, tx: &Sender<Point>,
) -> (Client, Option<SnapshotId>, Option<Point>) {
    // Note: we pre-validated connection before we ran, so failure here should be transient.
    // Just wait if we fail, and try again later.
    let client = connect_client(chain, cfg).await;

    debug!("Mithril Snapshot background updater for: {chain} : Client connected.");

    // Check if we already have a Mithril snapshot downloaded, and IF we do validate it is intact.
    let (current_snapshot, current_tip) =
        get_latest_validated_mithril_snapshot(chain, &client, cfg).await;

    if let Some(current_tip) = current_tip.clone() {
        // Save the current TIP
        CURRENT_TIPS.insert(chain, current_tip.clone());

        // Tell the live sync service the current Mithril TIP.
        if let Err(error) = tx.send(current_tip).await {
            error!("Failed to send new tip to the live updater for: {chain}:  {error}");
        };
    }

    (client, current_snapshot, current_tip)
}

/// Status of checking if we have a new snapshot to get or not.
enum SnapshotStatus {
    /// No update, sleep for this long before checking again
    Sleep(Duration),
    /// Snapshot has updated, here are the details.
    Updated((Snapshot, MithrilCertificate)),
}

/// Check if we have a new snapshot to download, and if so, return its details.
async fn check_snapshot_to_download(
    chain: Network, client: &Client, current_snapshot: &Option<SnapshotId>,
) -> SnapshotStatus {
    debug!("Mithril Snapshot background updater for: {chain} : Getting Latest Snapshot.");

    // This should only fail if the Aggregator is offline.
    // Because we check we can talk to the aggregator before we create the downloader task.
    let Some((latest_snapshot, chronologically_previous_snapshot)) =
        get_latest_snapshots(client, chain).await
    else {
        return SnapshotStatus::Sleep(DOWNLOAD_ERROR_RETRY_DURATION);
    };

    debug!("Mithril Snapshot background updater for: {chain} : Checking if we are up-to-date.");

    // Check if the latest snapshot is different from our actual previous one.
    if let Some(current_mithril_snapshot) = &current_snapshot {
        let latest_immutable_file_number = latest_snapshot.beacon.immutable_file_number;
        debug!("We have a current snapshot: {current_mithril_snapshot} == {latest_immutable_file_number} ??");
        if *current_mithril_snapshot == latest_immutable_file_number {
            debug!("Current Snapshot and latest are the same, so wait for it to likely to have changed.");
            let next_sleep =
                calculate_sleep_duration(&latest_snapshot, &chronologically_previous_snapshot);
            return SnapshotStatus::Sleep(next_sleep);
        }
    }

    // Download the snapshot/certificate from the aggregator.
    let Some((snapshot, certificate)) =
        get_mithril_snapshot_and_certificate(chain, client, &latest_snapshot).await
    else {
        // If we couldn't get the snapshot then we don't need to do anything else, transient
        // error.
        debug!("Failed to retrieve the snapshot and certificate from aggregator.");
        return SnapshotStatus::Sleep(DOWNLOAD_ERROR_RETRY_DURATION);
    };

    SnapshotStatus::Updated((snapshot, certificate))
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
    chain: Network, cfg: MithrilSnapshotConfig, tx: Sender<Point>,
) {
    debug!(
        "Mithril Snapshot background updater for: {} from {} to {} : Starting",
        chain,
        cfg.aggregator_url,
        cfg.path.to_string_lossy()
    );
    let mut next_sleep = Duration::from_secs(0);

    let (client, mut current_snapshot, mut current_tip) =
        recover_existing_snapshot(chain, &cfg, &tx).await;

    loop {
        // We can accumulate junk depending on errors or when we terminate, make sure we are always clean.
        if let Err(error) = cfg.cleanup().await {
            error!(
                "Mithril Snapshot background updater for:  {} : Error cleaning up: {:?}",
                chain, error
            );
        }

        debug!(
            "Mithril Snapshot background updater for: {chain} : Sleeping for {}.",
            format_duration(next_sleep)
        );
        // Wait until its likely we have a new snapshot ready to download.
        sleep(next_sleep).await;

        // Default sleep if we end up back at the top of this loop because of an error.
        next_sleep = DOWNLOAD_ERROR_RETRY_DURATION;

        let (snapshot, certificate) =
            match check_snapshot_to_download(chain, &client, &current_snapshot).await {
                SnapshotStatus::Sleep(sleep) => {
                    next_sleep = sleep;
                    continue;
                },
                SnapshotStatus::Updated(update) => update,
            };

        debug!("Mithril Snapshot background updater for: {chain} : Download and unpack the Mithril snapshot.");

        // Download and unpack the actual snapshot archive.
        if let Err(error) = client
            .snapshot()
            .download_unpack(&snapshot, &cfg.tmp_path())
            .await
        {
            // If we couldn't download and unpack, assume its a transient error,
            error!("Failed to Download and Unpack snapshot: {error}");
            continue;
        }

        debug!("Mithril Snapshot background updater for: {chain} : Add statistics for download.");

        if let Err(error) = client.snapshot().add_statistics(&snapshot).await {
            // Just log not fatal to anything.
            error!("Could not increment snapshot download statistics for {chain}: {error}");
            // We can process the download even after this fails.
        }

        debug!("Mithril Snapshot background updater for: {chain} : Check Certificate.");

        match MessageBuilder::new()
            .compute_snapshot_message(&certificate, &cfg.tmp_path())
            .await
        {
            Ok(message) => {
                if !certificate.match_message(&message) {
                    // If we couldn't match then assume its a transient error.
                    error!(
                        "Failed to Match Certificate and Computed Snapshot Message for {chain}!"
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

        debug!("Mithril Snapshot background updater for: {chain} : Updating TIP.");

        // Download was A-OK - Update the new immutable tip.
        let tip = match get_mithril_tip(&cfg.tmp_path()) {
            Ok(tip) => tip,
            Err(error) => {
                // If we couldn't get the tip then assume its a transient error.
                error!("Failed to Get Tip from Snapshot for {chain}:  {error}");
                continue;
            },
        };

        // Check that the new tip is more advanced than the OLD tip.
        if let Some(current_tip) = current_tip.clone() {
            if tip.slot_or_default() <= current_tip.slot_or_default() {
                error!("New Tip is not more advanced than the old tip for: {chain}");
                continue;
            }
        }

        // Got a good new tip, so switch to the new mithril image.
        match cfg.activate(snapshot.beacon.immutable_file_number).await {
            Ok(new_path) => {
                debug!("Mithril Snapshot background updater for: {chain} : Updated TIP.");
                current_tip = Some(tip.clone());
                current_snapshot = SnapshotId::try_new(&new_path);
                // Tell the live updater that the Immutable TIP has updated.
                CURRENT_TIPS.insert(chain, tip.clone());
                if let Err(error) = tx.send(tip).await {
                    error!("Failed to send new tip to the live updater for: {chain}:  {error}");
                };
            },
            Err(err) => {
                error!("Failed to activate new snapshot for: {chain}: {err}");
            },
        }
    }
}

/// A snapshot downloader that accelerates Download using `aria2`.
pub struct TurboSnapshotDownloader {
    /// Handle to a HTTP client to use for downloading simply.
    http_client: reqwest::Client,
    /// Configuration for the snapshot sync.
    cfg: MithrilSnapshotConfig,
}

impl TurboSnapshotDownloader {
    /// Constructs a new `HttpSnapshotDownloader`.
    pub fn new(cfg: MithrilSnapshotConfig) -> MithrilResult<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .build()
            .with_context(|| "Building http client for TurboSnapshotDownloader failed")?;

        Ok(Self { http_client, cfg })
    }
}

/// Use a consistent name for a download archive to simplify processing.
const DOWNLOAD_FILE_NAME: &str = "latest-mithril.tar.zst";

/// Download a file using `aria2` tool, with maximum number of simultaneous connections.
async fn aria2_download(dest: &Path, url: &str) -> MithrilResult<()> {
    let dest = format!("--dir={}", dest.to_string_lossy());
    let dest_file = format!("--out={DOWNLOAD_FILE_NAME}");

    let mut process = Command::new("aria2c")
        .args(["-x", "16", "-s", "16", &dest, &dest_file, url])
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let Some(stdout) = process.stdout.take() else {
        bail!("aria2c stdout channel was not readable.");
    };
    let Some(stderr) = process.stderr.take() else {
        bail!("aria2c stdout channel was not readable.");
    };

    let stdout = FramedRead::with_capacity(stdout, LinesCodec::new(), 32)
        .map(std::result::Result::unwrap_or_default);

    let stderr = FramedRead::with_capacity(stderr, LinesCodec::new(), 32)
        .map(std::result::Result::unwrap_or_default);

    let mut stream = stdout.chain(stderr);

    while let Some(msg) = stream.next().await {
        debug!("{:?}", msg);
    }

    // wait for the process to complete
    let result = process.wait().await?;
    if !result.success() {
        bail!("aria2c exited with error code {}", result);
    }

    Ok(())
}

#[async_trait]
impl SnapshotDownloader for TurboSnapshotDownloader {
    async fn download_unpack(
        &self, location: &str, target_dir: &Path, _compression_algorithm: CompressionAlgorithm,
        _download_id: &str, _snapshot_size: u64,
    ) -> MithrilResult<()> {
        if let Err(error) = create_dir_all(self.cfg.dl_path()).await {
            let msg = format!(
                "Download directory {} could not be created: {}",
                self.cfg.dl_path().to_string_lossy(),
                error
            );
            Err(anyhow!(msg.clone()).context(msg))?;
        }

        if let Err(error) = create_dir_all(target_dir).await {
            let msg = format!(
                "Target directory {} could not be created: {}",
                target_dir.to_string_lossy(),
                error
            );
            Err(anyhow!(msg.clone()).context(msg))?;
        }

        debug!("Download and Unpack started='{location}' to '{target_dir:?}'.");

        // First Download the Archive using Aria2 to the `dl` directory.
        // TODO(SJ): Using `aria2` as a convenience, need to change to a rust native
        // multi-connection download crate, which needs to be written.
        aria2_download(&self.cfg.dl_path(), location).await?;

        // Decompress and extract and de-dupe each file in the archive.
        let mut dst_archive = self.cfg.dl_path();
        dst_archive.push(DOWNLOAD_FILE_NAME);

        debug!(
            "Unpacking and extracting '{}' to '{}'.",
            dst_archive.to_string_lossy(),
            target_dir.to_string_lossy()
        );

        let mut archive = Archive::new(ZstdDecoder::new(BufReader::new(
            File::open(dst_archive).await?,
        )));

        debug!("Extracting files from compressed archive.");

        let latest_snapshot = self.cfg.latest_snapshot_path().await;

        let mut entries = archive.entries()?;
        let tmp_dir = self.cfg.tmp_path();
        while let Some(file) = entries.next().await {
            let mut file = file?;

            // Unpack the raw file first.
            file.unpack_in(tmp_dir.clone()).await?;

            // Now attempt to dedup it with the current snapshot.
            if let Some(latest_snapshot) = &latest_snapshot {
                let tmp_file = tmp_dir.join(&file.path()?);
                if !self.cfg.dedup_tmp(&tmp_file, latest_snapshot).await {
                    debug!("Unique File '{}'.", tmp_file.to_string_lossy());
                }
            }
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
