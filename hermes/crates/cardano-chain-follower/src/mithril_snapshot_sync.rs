//! Internal Mithril snapshot downloader task.
//!
//! This task is responsible for downloading Mithril snapshot files. It downloads the
//! latest snapshot file and then sleeps until the next snapshot is available.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{TimeDelta, Utc};
use dashmap::DashSet;
use humantime::format_duration;
use logcall::logcall;
use mithril_client::{Client, MessageBuilder, MithrilCertificate, Snapshot, SnapshotListItem};
use tokio::{
    fs::remove_dir_all,
    sync::mpsc::Sender,
    time::{sleep, Duration},
};
use tracing::{debug, error};
use tracing_log::log;

use crate::{
    error::{Error, Result},
    mithril_query::get_mithril_tip_point,
    mithril_snapshot_config::{MithrilSnapshotConfig, MithrilUpdateMessage},
    mithril_snapshot_data::update_latest_mithril_snapshot,
    mithril_snapshot_iterator::MithrilSnapshotIterator,
    mithril_turbo_downloader::MithrilTurboDownloader,
    network::Network,
    snapshot_id::SnapshotId,
    stats::{self, mithril_sync_failure, mithril_validation_state},
    MultiEraBlock,
};

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
        // debug!("Checking Snapshot : {:?}", snapshot);
        if *snapshot_id == snapshot.beacon.immutable_file_number {
            return Some(snapshot);
        }
    }

    None
}

/// Create a client, should never fail, but return None if it does, because we can't
/// continue.
fn create_client(cfg: &MithrilSnapshotConfig) -> Option<(Client, Arc<MithrilTurboDownloader>)> {
    let downloader = Arc::new(MithrilTurboDownloader::new(cfg.clone()));

    // This can't fail, because we already tested it works. But just in case...
    let client = match mithril_client::ClientBuilder::aggregator(
        &cfg.aggregator_url,
        &cfg.genesis_key,
    )
    //.add_feedback_receiver(receiver)
    .with_snapshot_downloader(downloader.clone())
    .build()
    {
        Ok(c) => c,
        Err(err) => {
            error!(chain=cfg.chain.to_string(),"Unexpected Error [{}]: Unable to create Mithril Client.  Mithril Snapshots can not update.", err);
            return None;
        },
    };

    Some((client, downloader))
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
async fn connect_client(cfg: &MithrilSnapshotConfig) -> (Client, Arc<MithrilTurboDownloader>) {
    // Note: we pre-validated connection before we ran, so failure here should be transient.
    // Just wait if we fail, and try again later.
    loop {
        if let Some(client) = create_client(cfg) {
            return client;
        }

        // If we couldn't create a client, then we don' t need to do anything.
        // Error already logged in create_client, no need to print anything here.
        sleep(DOWNLOAD_ERROR_RETRY_DURATION).await;
    }
}

/// Relative Directory for Immutable data within a full mithril snapshot.
pub(crate) const MITHRIL_IMMUTABLE_SUB_DIRECTORY: &str = "immutable";

/// Get the tip block from the Immutable chain, and the block immediately proceeding it.
///
/// # Arguments
///
/// * `path` - The path where the immutable chain is stored.
///
/// # Returns
///
/// This function returns the tip block point, and the block point immediately proceeding
/// it in a tuple.
#[allow(clippy::indexing_slicing)]
#[logcall(ok = "debug", err = "error")]
pub(crate) async fn get_mithril_tip(chain: Network, path: &Path) -> Result<MultiEraBlock> {
    let mut path = path.to_path_buf();
    path.push(MITHRIL_IMMUTABLE_SUB_DIRECTORY);

    debug!(
        "Calculating TIP from Immutable storage @ {}",
        path.to_string_lossy()
    );

    // Read the Tip (fuzzy), and if we don't get one, or we error, its an error.
    // has to be Fuzzy, because we intend to iterate and don't know the previous.
    // Nor is there a subsequent block we can use as next.
    let tip = get_mithril_tip_point(&path).await?.as_fuzzy();
    debug!("Mithril Tip: {tip}");

    // Decode and read the tip from the Immutable chain.
    let tip_iterator = MithrilSnapshotIterator::new(chain, &path, &tip, None).await?;
    let Some(tip_block) = tip_iterator.next().await else {
        error!("Failed to fetch the TIP block from the immutable chain.");

        return Err(Error::MithrilSnapshot(None));
    };

    // Yay, we got a tip, so return it.
    Ok(tip_block)
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
) -> Option<SnapshotId> {
    /// Purge a bad mithril snapshot from disk.
    async fn purge_bad_mithril_snapshot(chain: Network, latest_mithril: &SnapshotId) {
        debug!("Purging Bad Mithril Snapshot: {latest_mithril}");
        if let Err(error) = remove_dir_all(&latest_mithril).await {
            // This should NOT happen because we already checked the Mithril path is fully writable.
            error!("Mithril Snapshot background updater for: {chain}: Failed to remove old snapshot {latest_mithril}: {error}");
        }
    }

    // Check if we already have a Mithril snapshot downloaded, and IF we do validate it is
    // intact.
    let latest_mithril = cfg.recover_latest_snapshot_id().await?;

    debug!("Latest Recovered Mithril ID = {latest_mithril}");

    // Get the actual latest snapshot, shouldn't fail, but say the current is invalid if it
    // does.
    let (actual_latest, _) = get_latest_snapshots(client, chain).await?;

    // IF the mithril data we have is NOT the current latest (or the immediately previous), it
    // may as well be invalid.
    if latest_mithril < actual_latest.beacon.immutable_file_number - 1 {
        return None;
    }

    let Some(snapshot) = get_snapshot_by_id(client, chain, &latest_mithril).await else {
        // We have a latest snapshot, but the Aggregator does not know it.
        error!("Mithril Snapshot background updater for: {chain}: Latest snapshot {latest_mithril} does not exist on the Aggregator.");
        purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return None;
    };

    // Download the snapshot/certificate from the aggregator.
    let Some((_, certificate)) =
        get_mithril_snapshot_and_certificate(chain, client, &snapshot).await
    else {
        error!("Mithril Snapshot : Failed to get Snapshot and certificate (Transient Error).");

        // If we couldn't get the snapshot then we don't need to do anything else, transient
        // error.
        // purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return None;
    };

    let path = latest_mithril.as_ref();
    let valid = validate_mithril_snapshot(chain, &certificate, path).await;

    if !valid {
        error!("Mithril Snapshot : Snapshot fails to validate, can not be recovered.");
        purge_bad_mithril_snapshot(chain, &latest_mithril).await;
        return None;
    }

    Some(latest_mithril)
}

/// Get the Mithril client and recover our existing mithril snapshot data, if any.
async fn recover_existing_snapshot(
    cfg: &MithrilSnapshotConfig, tx: &Sender<MithrilUpdateMessage>,
) -> Option<SnapshotId> {
    // This is a Mithril Validation, so record it.
    mithril_validation_state(cfg.chain, stats::MithrilValidationState::Start);

    // Note: we pre-validated connection before we ran, so failure here should be transient.
    // Just wait if we fail, and try again later.
    let (client, downloader) = connect_client(cfg).await;

    debug!(
        "Mithril Snapshot background updater for: {} : Client connected.",
        cfg.chain
    );

    let mut current_snapshot = None;

    // Check if we already have a Mithril snapshot downloaded, and IF we do validate it is
    // intact.
    if let Some(active_snapshot) =
        get_latest_validated_mithril_snapshot(cfg.chain, &client, cfg).await
    {
        // Read the actual TIP block from the Mithril chain.
        match get_mithril_tip(cfg.chain, &active_snapshot.path()).await {
            Ok(tip_block) => {
                // Validate the Snapshot ID matches the true TIP.
                if active_snapshot.tip() == tip_block.point() {
                    current_snapshot = Some(active_snapshot.clone());
                    update_latest_mithril_snapshot(cfg.chain, active_snapshot);

                    // Tell the live sync service the current Mithril TIP.
                    let update = MithrilUpdateMessage {
                        tip: tip_block.point(),
                        previous: tip_block.previous(),
                    };
                    if let Err(error) = tx.send(update).await {
                        error!(
                            "Failed to send new tip to the live updater for: {}:  {error}",
                            cfg.chain
                        );
                    };
                } else {
                    error!(
                        "Actual Tip Block and Active SnapshotID Point Mismatch. {:?} != {:?}",
                        active_snapshot.tip(),
                        tip_block.point()
                    );
                }
            },
            Err(error) => {
                error!("Mithril snapshot validation failed for: {}.  Could not read the TIP Block : {}.", cfg.chain, error);
            },
        }
    } else {
        debug!("No latest validated snapshot for: {}", cfg.chain);
    }

    if current_snapshot.is_none() {
        mithril_validation_state(cfg.chain, stats::MithrilValidationState::Failed);
    } else {
        mithril_validation_state(cfg.chain, stats::MithrilValidationState::Finish);
    }

    // Explicitly free the resources claimed by the Mithril Client and Downloader.
    drop(client);
    drop(downloader);

    current_snapshot
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

    debug!("Mithril Snapshot background updater for: {chain} : Checking if we are up-to-date {current_snapshot:?}.");

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

/// Start Mithril Validation in the background, and return a handle so we can check when
/// it finishes.
fn background_validate_mithril_snapshot(
    chain: Network, certificate: MithrilCertificate, tmp_path: PathBuf,
) -> tokio::task::JoinHandle<bool> {
    tokio::spawn(async move {
        debug!(
            "Mithril Snapshot background updater for: {} : Check Certificate.",
            chain
        );

        stats::mithril_validation_state(chain, stats::MithrilValidationState::Start);

        if !validate_mithril_snapshot(chain, &certificate, &tmp_path).await {
            stats::mithril_validation_state(chain, stats::MithrilValidationState::Failed);
            // If we couldn't build the message then assume its a transient error.
            error!(
                chain = %chain,
                "Failed to Compute Snapshot Message"
            );
            return false;
        }
        stats::mithril_validation_state(chain, stats::MithrilValidationState::Finish);

        debug!(
            "Mithril Snapshot background updater for: {} : Certificate Validated OK.",
            chain
        );

        true
    })
}

/// Convert a chunk filename into its numeric equivalent.
fn chunk_filename_to_chunk_number(chunk: &Path) -> Option<u64> {
    if let Some(stem) = chunk.file_stem().map(Path::new) {
        if let Some(base) = stem.file_name().map(|s| s.to_string_lossy().to_string()) {
            if let Ok(num) = base.parse::<u64>() {
                return Some(num);
            }
        }
    }
    None
}

/// Remove any chunks from the chunk list which exceed the `max_chunk`.
fn trim_chunk_list(chunk_list: &Arc<DashSet<PathBuf>>, max_chunks: u64) {
    chunk_list.retain(|entry| {
        if let Some(chunk_index) = chunk_filename_to_chunk_number(entry) {
            if chunk_index > max_chunks {
                debug!("Removing Non immutable Chunk: {:?}", entry);
                false
            } else {
                true
            }
        } else {
            // Huh, not a valid filename, so purge it.
            error!("Found an invalid chunk name: {:?}", entry);
            false
        }
    });
}

/// Downloads and validates a snapshot from the aggregator.
async fn download_and_validate_snapshot(
    client: &Client, downloader: Arc<MithrilTurboDownloader>, cfg: &MithrilSnapshotConfig,
    snapshot: &Snapshot, certificate: MithrilCertificate,
) -> bool {
    debug!(
        "Mithril Snapshot background updater for: {} : Download and unpack the Mithril snapshot.",
        cfg.chain
    );

    // Download and unpack the actual snapshot archive.
    if let Err(error) = client
        .snapshot()
        .download_unpack(snapshot, &cfg.tmp_path())
        .await
    {
        // If we couldn't download and unpack, assume its a transient error,
        error!("Failed to Download and Unpack snapshot: {error}");
        return false;
    }

    debug!(
        "Mithril Snapshot background updater for: {} : Add statistics for download.",
        cfg.chain
    );

    if let Err(error) = client.snapshot().add_statistics(snapshot).await {
        // Just log not fatal to anything.
        error!(
            "Could not increment snapshot download statistics for {}: {error}",
            cfg.chain
        );
        // We can process the download even after this fails.
    }

    debug!(
        "Mithril Snapshot background updater for: {} : Index and Check Certificate.",
        cfg.chain
    );

    let chunk_list = downloader.get_new_chunks();
    // Remove the last chunks from the list, if they are > the max_chunk thats immutable.
    let max_chunk = snapshot.beacon.immutable_file_number;
    trim_chunk_list(&chunk_list, max_chunk);

    let validate_handle =
        background_validate_mithril_snapshot(cfg.chain, certificate, cfg.tmp_path());

    if !validate_handle.await.unwrap_or(false) {
        error!("Failed to validate for {}", cfg.chain);
        return false;
    }

    true
}

/// We can accumulate junk depending on errors or when we terminate, make sure we are
/// always clean.
async fn cleanup(cfg: &MithrilSnapshotConfig) {
    if let Err(error) = cfg.cleanup().await {
        error!(
            "Mithril Snapshot background updater for:  {} : Error cleaning up: {:?}",
            cfg.chain, error
        );
    }
}

/// Sleep until its likely there has been another mithril snapshot update.
async fn sleep_until_next_probable_update(
    cfg: &MithrilSnapshotConfig, next_sleep: &Duration,
) -> Duration {
    debug!(
        "Mithril Snapshot background updater for: {} : Sleeping for {}.",
        cfg.chain,
        format_duration(*next_sleep)
    );
    // Wait until its likely we have a new snapshot ready to download.
    sleep(*next_sleep).await;

    // Default sleep if we end up back at the top of this loop because of an error.
    DOWNLOAD_ERROR_RETRY_DURATION
}

/// Cleanup the client explicitly and do a new iteration of the loop.
macro_rules! next_iteration {
    ($client:ident, $downloader:ident) => {
        drop($client);
        drop($downloader);

        continue;
    };
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
#[allow(clippy::too_many_lines)]
pub(crate) async fn background_mithril_update(
    cfg: MithrilSnapshotConfig, tx: Sender<MithrilUpdateMessage>,
) {
    debug!(
        "Mithril Snapshot background updater for: {} from {} to {} : Starting",
        cfg.chain,
        cfg.aggregator_url,
        cfg.path.to_string_lossy()
    );
    let mut next_sleep = Duration::from_secs(0);

    let mut current_snapshot = recover_existing_snapshot(&cfg, &tx).await;

    loop {
        debug!("Background Mithril Updater - New Loop");

        cleanup(&cfg).await;

        next_sleep = sleep_until_next_probable_update(&cfg, &next_sleep).await;

        let (client, downloader) = connect_client(&cfg).await;

        let (snapshot, certificate) =
            match check_snapshot_to_download(cfg.chain, &client, &current_snapshot).await {
                SnapshotStatus::Sleep(sleep) => {
                    next_sleep = sleep;
                    next_iteration!(client, downloader);
                },
                SnapshotStatus::Updated(update) => update,
            };

        if !download_and_validate_snapshot(
            &client,
            downloader.clone(),
            &cfg,
            &snapshot,
            certificate,
        )
        .await
        {
            error!("Failed to Download or Validate a snapshot.");
            mithril_sync_failure(cfg.chain, stats::MithrilSyncFailures::DownloadOrValidation);

            next_iteration!(client, downloader);
        }

        // Download was A-OK - Update the new immutable tip.
        let tip = match get_mithril_tip(cfg.chain, &cfg.tmp_path()).await {
            Ok(tip) => tip,
            Err(error) => {
                // If we couldn't get the tip then assume its a transient error.
                error!(
                    "Failed to Get Tip from Snapshot for {}:  {error}",
                    cfg.chain
                );
                mithril_sync_failure(cfg.chain, stats::MithrilSyncFailures::FailedToGetTip);

                next_iteration!(client, downloader);
            },
        };

        debug!("New Immutable TIP = {}", tip);

        // Check that the new tip is more advanced than the OLD tip.
        if let Some(active_snapshot) = current_snapshot.clone() {
            if tip <= active_snapshot.tip() {
                error!(
                    "New Tip is not more advanced than the old tip for: {}",
                    cfg.chain
                );
                mithril_sync_failure(cfg.chain, stats::MithrilSyncFailures::TipDidNotAdvance);
                next_iteration!(client, downloader);
            }
        }

        // Got a good new tip, so switch to the new mithril image.
        match cfg.activate(snapshot.beacon.immutable_file_number).await {
            Ok(new_path) => {
                debug!(
                    "Mithril Snapshot background updater for: {} : Updated TIP.",
                    cfg.chain
                );
                current_snapshot = SnapshotId::new(&new_path, tip.point());

                if let Some(latest_snapshot) = current_snapshot.clone() {
                    // Update the latest snapshot data record
                    update_latest_mithril_snapshot(cfg.chain, latest_snapshot);

                    // Tell the live updater that the Immutable TIP has updated.
                    if let Err(error) = tx
                        .send(MithrilUpdateMessage {
                            tip: tip.point(),
                            previous: tip.previous(),
                        })
                        .await
                    {
                        error!(
                            "Failed to send new tip to the live updater for: {}:  {error}",
                            cfg.chain
                        );
                        mithril_sync_failure(
                            cfg.chain,
                            stats::MithrilSyncFailures::TipFailedToSendToUpdater,
                        );
                        next_iteration!(client, downloader);
                    };
                }
            },
            Err(err) => {
                error!(
                    chain = cfg.chain.to_string(),
                    "Failed to activate new snapshot : {err}"
                );
                mithril_sync_failure(
                    cfg.chain,
                    stats::MithrilSyncFailures::FailedToActivateNewSnapshot,
                );
                next_iteration!(client, downloader);
            },
        }
        next_iteration!(client, downloader);
    }
}
