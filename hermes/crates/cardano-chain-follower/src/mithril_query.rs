//! Functions which query or interact with the Immutable blockchain on disk.

use std::path::Path;

use pallas_hardano::storage::immutable::FallibleBlock;
use tokio::task;

use crate::{
    error::{Error, Result},
    Point,
};

/// Synchronous Immutable block iterator.
pub(crate) type ImmutableBlockIterator = Box<dyn Iterator<Item = FallibleBlock> + Send + Sync>;

/// Get a mithril snapshot iterator.
pub(crate) async fn make_mithril_iterator(
    path: &Path, start: &Point,
) -> Result<ImmutableBlockIterator> {
    let path = path.to_path_buf();
    let start = start.clone();
    // Initial input
    let res = task::spawn_blocking(move || {
        pallas_hardano::storage::immutable::read_blocks_from_point(&path, start.clone().into())
            .map_err(|error| Error::MithrilSnapshot(Some(error)))
    })
    .await;

    match res {
        Ok(res) => res,
        Err(_error) => Err(Error::MithrilSnapshot(None)),
    }
}

/// Get latest TIP of the Mithril Immutable Chain.
pub(crate) async fn get_mithril_tip_point(path: &Path) -> Result<Point> {
    let path = path.to_path_buf();
    let res =
        task::spawn_blocking(move || pallas_hardano::storage::immutable::get_tip(&path)).await;

    match res {
        Ok(Ok(Some(res))) => Ok(res.into()),
        Ok(Ok(None)) => Err(Error::MithrilSnapshot(None)),
        Ok(Err(error)) => Err(Error::MithrilSnapshot(Some(error))),
        Err(_error) => Err(Error::MithrilSnapshot(None)),
    }
}
