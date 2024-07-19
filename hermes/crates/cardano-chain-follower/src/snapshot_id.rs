//! Simple ID for a mithril snapshot path known by its largest immutable file number

use std::{
    cmp::Ordering,
    default,
    fmt::Display,
    path::{Path, PathBuf},
};

use tracing::debug;

use crate::{
    mithril_snapshot_sync::{get_mithril_tip, MITHRIL_IMMUTABLE_SUB_DIRECTORY},
    point::UNKNOWN_POINT,
    Network, Point,
};
/// A Representation of a Snapshot Path and its represented Immutable File Number.
#[derive(Clone, Debug)]
pub(crate) struct SnapshotId {
    /// The Snapshot Path
    path: PathBuf,
    /// The largest Immutable File Number
    file: u64,
    /// The Tip of the Snapshot
    tip: Point,
}

impl SnapshotId {
    /// See if we can Parse the path into an immutable file number.
    pub(crate) fn parse_path(path: &Path) -> Option<u64> {
        // Path must actually exist, and be a directory.
        if !path.is_dir() {
            None
        } else if let Ok(numeric_name) = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .parse::<u64>()
        {
            Some(numeric_name)
        } else {
            // If we couldn't parse the file name as a number, then it's not an immutable file.
            None
        }
    }

    /// Try and create a new `SnapshotID` from a given path.
    /// Immutable TIP must be provided.
    pub(crate) fn new(path: &Path, tip: Point) -> Option<Self> {
        debug!("Trying to Get SnapshotID of: {}", path.to_string_lossy());
        let immutable_file = SnapshotId::parse_path(path)?;
        debug!("Immutable File#: {}", immutable_file);

        Some(SnapshotId {
            path: path.to_path_buf(),
            file: immutable_file,
            tip,
        })
    }

    /// Try and create a new `SnapshotID` from a given path.
    /// Includes properly getting the Immutable TIP.
    pub(crate) async fn try_new(chain: Network, path: &Path) -> Option<Self> {
        let Ok(tip) = get_mithril_tip(chain, path).await else {
            return None;
        };

        SnapshotId::new(path, tip.point())
    }

    /// Get the Immutable Blockchain path from this `SnapshotId`
    pub(crate) fn immutable_path(&self) -> PathBuf {
        let mut immutable = self.path.clone();
        immutable.push(MITHRIL_IMMUTABLE_SUB_DIRECTORY);

        immutable
    }

    /// Get the Blockchain path from this `SnapshotId`
    pub(crate) fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the Tip of the Immutable Blockchain from this `SnapshotId`
    pub(crate) fn tip(&self) -> Point {
        self.tip.clone()
    }
}

impl default::Default for SnapshotId {
    /// Create an empty `SnapshotID`.
    fn default() -> Self {
        SnapshotId {
            path: PathBuf::new(),
            file: 0,
            tip: UNKNOWN_POINT,
        }
    }
}

impl std::convert::AsRef<std::path::Path> for SnapshotId {
    fn as_ref(&self) -> &std::path::Path {
        self.path.as_ref()
    }
}

impl Display for SnapshotId {
    /// Convert this `SnapshotID` to a `String`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{} @ Tip [{} / {:?}]",
            self.path.display(),
            self.file,
            self.tip
        )
    }
}

// Normal Comparisons to simplify code.
impl PartialEq for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
    }
}

impl PartialOrd for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.file.partial_cmp(&other.file)
    }
}

// Allows us to compare a SnapshotID against Some(SnapshotID).
impl PartialEq<Option<SnapshotId>> for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn eq(&self, other: &Option<Self>) -> bool {
        match other {
            None => false,
            Some(other) => self == other,
        }
    }
}

impl PartialOrd<Option<SnapshotId>> for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn partial_cmp(&self, other: &Option<Self>) -> Option<Ordering> {
        match other {
            None => Some(Ordering::Greater), // Anything is always greater than None.
            Some(other) => self.partial_cmp(other),
        }
    }
}

// Allows us to compare a SnapshotID against u64 (Just the Immutable File Number).
impl PartialEq<u64> for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn eq(&self, other: &u64) -> bool {
        self.file == *other
    }
}

impl PartialOrd<u64> for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        self.file.partial_cmp(other)
    }
}

#[cfg(test)]
mod tests {
    use crate::point::*;
    use super::*;

    const TEST_DIR: &str = "test_snapshot_id";

    #[test]
    fn test_parse_path() {
        let dir_path_1 = &[TEST_DIR, "12345"].join("/");
        let dir_path_2 = &[TEST_DIR, "12346"].join("/");
        let dir_path_3 = &[TEST_DIR, "12347"].join("/");
        let dir_path_4 = &[TEST_DIR, "not_found"].join("/");
        let dir_path_5 = &[TEST_DIR, "123abc"].join("/");

        assert_eq!(SnapshotId::parse_path(&PathBuf::from(dir_path_1)), Some(12345));
        assert_eq!(SnapshotId::parse_path(&PathBuf::from(dir_path_2)), Some(12346));
        assert_eq!(SnapshotId::parse_path(&PathBuf::from(dir_path_3)), Some(12347));
        assert_eq!(SnapshotId::parse_path(&PathBuf::from(dir_path_4)), None);
        assert_eq!(SnapshotId::parse_path(&PathBuf::from(dir_path_5)), None);
    }

    #[test]
    fn test_new() {
        let dir_path_1 = &[TEST_DIR, "12345"].join("/");
        let dir_path_2 = &[TEST_DIR, "12346"].join("/");
        let dir_path_3 = &[TEST_DIR, "12347"].join("/");

        let point_1 = Point::fuzzy(999);
        let point_2 = Point::new(999, vec![0; 32]);
        let point_3 = Point::new(12345, vec![8; 32]);

        assert!(SnapshotId::new(&PathBuf::from(dir_path_1), point_1).is_some());
        assert!(SnapshotId::new(&PathBuf::from(dir_path_2), point_2).is_some());
        assert!(SnapshotId::new(&PathBuf::from(dir_path_3), point_3).is_some());
    }

    #[tokio::test]
    async fn test_try_new() {
        let dir_path_1 = &[TEST_DIR, "12345"].join("/");
        let dir_path_1 = PathBuf::from(dir_path_1).canonicalize().expect("cannot get absolute path");

        let tmp = std::fs::read_dir(&dir_path_1);

        println!("{:?}", tmp);

        assert_eq!(SnapshotId::try_new(Network::Preprod, &dir_path_1).await, None);
    }

    #[test]
    fn test_immutable_path() {
        let dir_path_1 = &[TEST_DIR, "12345"].join("/");

        let point_1 = Point::fuzzy(999);

        let snapshot_id_1 = SnapshotId::new(&PathBuf::from(dir_path_1), point_1).expect("cannot create snapshot id");

        assert_eq!(snapshot_id_1.immutable_path(), PathBuf::from([dir_path_1.as_str(), "immutable"].join("/")));
    }

    #[test]
    fn test_compare() {
        let dir_path_1 = &[TEST_DIR, "12345"].join("/");
        let dir_path_2 = &[TEST_DIR, "12345"].join("/");
        let dir_path_3 = &[TEST_DIR, "12346"].join("/");
        let dir_path_4 = &[TEST_DIR, "12347"].join("/");

        let point_1 = Point::fuzzy(999);
        let point_2 = Point::new(999, vec![0; 32]);
        let point_3 = Point::new(12345, vec![8; 32]);

        let snapshot_id_1 = SnapshotId::new(&PathBuf::from(dir_path_1), point_1.clone());
        let snapshot_id_2 = SnapshotId::new(&PathBuf::from(dir_path_2), point_1);
        let snapshot_id_3 = SnapshotId::new(&PathBuf::from(dir_path_3), point_2);
        let snapshot_id_4 = SnapshotId::new(&PathBuf::from(dir_path_4), point_3);

        assert!(snapshot_id_1 == snapshot_id_1);
        assert!(snapshot_id_1 == snapshot_id_2);
        assert!(snapshot_id_1 != snapshot_id_3);
        assert!(snapshot_id_1 < snapshot_id_3);
        assert!(snapshot_id_1 != snapshot_id_4);
        assert!(snapshot_id_1 < snapshot_id_4);

        assert!(snapshot_id_2 == snapshot_id_1);
        assert!(snapshot_id_2 != snapshot_id_3);
        assert!(snapshot_id_2 < snapshot_id_3);
        assert!(snapshot_id_2 != snapshot_id_4);
        assert!(snapshot_id_2 < snapshot_id_4);

        assert!(snapshot_id_4 != snapshot_id_1);
        assert!(snapshot_id_4 > snapshot_id_1);
        assert!(snapshot_id_4 != snapshot_id_2);
        assert!(snapshot_id_4 > snapshot_id_2);
        assert!(snapshot_id_4 != snapshot_id_3);
        assert!(snapshot_id_4 > snapshot_id_3);
    }
}
