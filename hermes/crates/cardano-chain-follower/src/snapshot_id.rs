//! Simple ID for a mithril snapshot path known by its largest immutable file number

use std::fmt::Display;
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};
/// A Representation of a Snapshot Path and its represented Immutable File Number.
#[derive(Clone, Debug)]
pub(crate) struct SnapshotId(PathBuf, u64);

impl SnapshotId {
    /// Try and create a new `SnapshotID` from a given path.
    pub(crate) fn try_new(path: &Path) -> Option<Self> {
        // Path must actually exist, and be a directory.
        if !path.is_dir() {
            None
        } else if let Ok(numeric_name) = path.to_string_lossy().parse::<u64>() {
            Some(SnapshotId(path.to_path_buf(), numeric_name))
        } else {
            None
        }
    }
}

impl std::convert::AsRef<std::path::Path> for SnapshotId {
    fn as_ref(&self) -> &std::path::Path {
        self.0.as_ref()
    }
}

impl Display for SnapshotId {
    /// Convert this `SnapshotID` to a `String`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}/{}", self.0.display(), self.1)
    }
}

// Normal Comparisons to simplify code.
impl PartialEq for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl PartialOrd for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
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
        self.1 == *other
    }
}

impl PartialOrd<u64> for SnapshotId {
    // Equality ONLY checks the Immutable File Number, not the path.
    // This is because the Filename is already the ImmutableFileNumber
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        self.1.partial_cmp(other)
    }
}
