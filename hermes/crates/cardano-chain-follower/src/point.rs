//! A Cardano Point on the Blockchain.
//!
//! Wrapped version of the Pallas primitive.
//! We only use this version unless talking to Pallas.

use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
};

use pallas::crypto::hash::Hash;

/// A specific point in the blockchain. It can be used to
/// identify a particular location within the blockchain, such as the tip (the
/// most recent block) or any other block. It has special kinds of `Point`,
/// available as constants: `TIP_POINT`, and `ORIGIN_POINT`.
///
/// # Attributes
///
/// * `Point` - The inner type is a `Point` from the `pallas::network::miniprotocols`
///   module. This inner `Point` type encapsulates the specific details required to
///   identify a point in the blockchain.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Point(pallas::network::miniprotocols::Point);

/// A truly unknown point in the blockchain. It is used
/// when the previous point is completely unknown and does not correspond to the
/// origin of the blockchain.
///
/// # Usage
///
/// `UNKNOWN_POINT` can be used in scenarios where the previous point in the blockchain
/// is not known and should not be assumed to be the origin. It serves as a marker
/// for an indeterminate or unspecified point.
///
/// The inner `Point` is created with `u64::MIN` and an empty `Vec<u8>`, indicating
/// that this is a special marker for an unknown point, rather than a specific
/// point in the blockchain.
pub(crate) const UNKNOWN_POINT: Point = Point(pallas::network::miniprotocols::Point::Specific(
    u64::MIN,
    Vec::new(),
));

/// The tip of the blockchain at the current moment.
/// It is used when the specific point in the blockchain is not known, but the
/// interest is in the most recent block (the tip). The tip is the point where
/// new blocks are being added.
///
/// # Usage
///
/// `TIP_POINT` can be used in scenarios where the most up-to-date point in the
/// blockchain is required. It signifies that the exact point is not important
/// as long as it is the latest available point in the chain.
///
/// The inner `Point` is created with `u64::MAX` and an empty `Vec<u8>`, indicating
/// that this is a special marker rather than a specific point in the blockchain.
pub const TIP_POINT: Point = Point(pallas::network::miniprotocols::Point::Specific(
    u64::MAX,
    Vec::new(),
));

/// The origin of the blockchain. It is used when the
/// interest is in the very first point of the blockchain, regardless of its
/// specific details.
///
/// # Usage
///
/// `ORIGIN_POINT` can be used in scenarios where the starting point of the
/// blockchain is required. It signifies the genesis block or the initial state
/// of the blockchain.
///
/// The inner `Point` is created with the `Origin` variant from
/// `pallas::network::miniprotocols::Point`, indicating that this is a marker
/// for the blockchain's origin.
pub const ORIGIN_POINT: Point = Point(pallas::network::miniprotocols::Point::Origin);

impl Point {
    /// Creates a new `Point` instance representing a specific
    /// point in the blockchain, identified by a given slot and hash.
    ///
    /// # Parameters
    ///
    /// * `slot` - A `u64` value representing the slot number in the blockchain.
    /// * `hash` - A `Vec<u8>` containing the hash of the block at the specified slot.
    ///
    /// # Returns
    ///
    /// A new `Point` instance encapsulating the given slot and hash.
    ///
    /// # Examples
    ///
    /// ```rs
    /// use cardano_chain_follower::Point;
    ///
    /// let slot = 42;
    /// let hash = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// let point = Point::new(slot, hash);
    /// ```
    #[must_use]
    pub fn new(slot: u64, hash: Vec<u8>) -> Self {
        Self(pallas::network::miniprotocols::Point::Specific(slot, hash))
    }

    /// Creates a new `Point` instance representing a specific
    /// point in the blockchain, identified by a given slot, but with an
    /// unknown hash. This can be useful in scenarios where the slot is known
    /// but the hash is either unavailable or irrelevant.
    ///
    /// # Parameters
    ///
    /// * `slot` - A `u64` value representing the slot number in the blockchain.
    ///
    /// # Returns
    ///
    /// A new `Point` instance encapsulating the given slot with an empty hash.
    ///
    /// # Examples
    ///
    /// ```rs
    /// use cardano_chain_follower::Point;
    ///
    /// let slot = 42;
    /// let point = Point::fuzzy(slot);
    /// ```
    #[must_use]
    pub fn fuzzy(slot: u64) -> Self {
        Self(pallas::network::miniprotocols::Point::Specific(
            slot,
            Vec::new(),
        ))
    }

    /// Compares the hash stored in the `Point` with a known hash.
    /// It returns `true` if the hashes match and `false` otherwise. If the
    /// provided hash is `None`, the function checks if the `Point` has an
    /// empty hash.
    ///
    /// # Parameters
    ///
    /// * `hash` - An `Option<Hash<32>>` containing the hash to compare against. If
    ///   `Some`, the contained hash is compared with the `Point`'s hash. If `None`, the
    ///   function checks if the `Point`'s hash is empty.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the hashes match.
    ///
    /// # Examples
    ///
    /// ```rs
    /// use cardano_chain_follower::Point;
    ///
    /// use pallas::crypto::hash::Hash;
    ///
    /// let point = Point::new(42, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    /// let hash = Some(Hash::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]));
    /// assert!(point.cmp_hash(&hash));
    ///
    /// let empty_point = Point::fuzzy(42);
    /// assert!(empty_point.cmp_hash(&None));
    /// ```
    #[must_use]
    pub fn cmp_hash(&self, hash: &Option<Hash<32>>) -> bool {
        match hash {
            Some(cmp_hash) => {
                match self.0 {
                    pallas::network::miniprotocols::Point::Specific(_, ref hash) => {
                        **hash == **cmp_hash
                    },
                    pallas::network::miniprotocols::Point::Origin => false,
                }
            },
            None => {
                match self.0 {
                    pallas::network::miniprotocols::Point::Specific(_, ref hash) => hash.is_empty(),
                    pallas::network::miniprotocols::Point::Origin => true,
                }
            },
        }
    }

    /// Retrieves the slot number from the `Point`. If the `Point`
    /// is the origin, it returns a default slot number.
    ///
    /// # Returns
    ///
    /// A `u64` representing the slot number. If the `Point` is the origin,
    /// it returns a default slot value, typically `0`.
    ///
    /// # Examples
    ///
    /// ```rs
    /// use cardano_chain_follower::{Point, ORIGIN_POINT};
    ///
    /// let specific_point = Point::new(42, vec![1, 2, 3]);
    /// assert_eq!(specific_point.slot_or_default(), 42);
    ///
    /// let origin_point = ORIGIN_POINT;
    /// assert_eq!(origin_point.slot_or_default(), 0); // assuming 0 is the default
    /// ```
    #[must_use]
    pub fn slot_or_default(&self) -> u64 {
        self.0.slot_or_default()
    }

    /// Retrieves the hash from the `Point`. If the `Point` is
    /// the origin, it returns a default hash value, which is an empty `Vec<u8>`.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` representing the hash. If the `Point` is the `Origin`, it
    /// returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```rs
    /// use cardano_chain_follower::{Point, ORIGIN_POINT};
    ///
    /// let specific_point = Point::new(42, vec![1, 2, 3]);
    /// assert_eq!(specific_point.hash_or_default(), vec![1, 2, 3]);
    ///
    /// let origin_point = ORIGIN_POINT;
    /// assert_eq!(origin_point.hash_or_default(), Vec::new());
    /// ```
    #[must_use]
    pub fn hash_or_default(&self) -> Vec<u8> {
        match &self.0 {
            pallas::network::miniprotocols::Point::Specific(_, hash) => hash.clone(),
            pallas::network::miniprotocols::Point::Origin => Vec::new(),
        }
    }

    /// Checks if two `Point` instances are strictly equal.
    /// Strict equality means both the slot number and hash must be identical.
    ///
    /// # Parameters
    ///
    /// * `b` - Another `Point` instance to compare against.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the two `Point` instances are strictly equal.
    ///
    /// # Examples
    ///
    /// ```rs
    /// use cardano_chain_follower::Point;
    ///
    /// let point1 = Point::new(42, vec![1, 2, 3]);
    /// let point2 = Point::new(42, vec![1, 2, 3]);
    /// assert!(point1.strict_eq(&point2));
    ///
    /// let point3 = Point::new(42, vec![1, 2, 3]);
    /// let point4 = Point::new(43, vec![1, 2, 3]);
    /// assert!(!point3.strict_eq(&point4));
    /// ```
    #[must_use]
    pub fn strict_eq(&self, b: &Self) -> bool {
        self.0 == b.0
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if *self == ORIGIN_POINT {
            return write!(f, "Point @ Origin");
        } else if *self == TIP_POINT {
            return write!(f, "Point @ Tip");
        } else if *self == UNKNOWN_POINT {
            return write!(f, "Point @ Unknown");
        }

        let slot = self.slot_or_default();
        let hash = self.hash_or_default();
        if hash.is_empty() {
            return write!(f, "Point @ Probe:{slot}");
        }
        write!(f, "Point @ {slot}:{}", hex::encode(hash))
    }
}

impl From<pallas::network::miniprotocols::Point> for Point {
    fn from(point: pallas::network::miniprotocols::Point) -> Self {
        Self(point)
    }
}

impl From<Point> for pallas::network::miniprotocols::Point {
    fn from(point: Point) -> pallas::network::miniprotocols::Point {
        point.0
    }
}

impl PartialOrd for Point {
    /// Implements a partial ordering based on the slot number
    /// of two `Point` instances. It only checks the slot number for ordering.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    /// Implements a total ordering based on the slot number
    /// of two `Point` instances. It only checks the slot number for ordering.
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_point(&self.0, &other.0)
    }
}

impl PartialEq<u64> for Point {
    /// Allows to compare a `SnapshotID` against `u64` (Just the Immutable File Number).
    ///
    /// Equality ONLY checks the Immutable File Number, not the path.
    /// This is because the Filename is already the Immutable File Number.
    fn eq(&self, other: &u64) -> bool {
        self.0.slot_or_default() == *other
    }
}

impl PartialOrd<u64> for Point {
    /// Allows to compare a `Point` against a `u64` (Just the Immutable File Number).
    ///
    /// Equality ONLY checks the Immutable File Number, not the path.
    /// This is because the Filename is already the Immutable File Number.
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        self.0.slot_or_default().partial_cmp(other)
    }
}

impl PartialEq<Option<Point>> for Point {
    /// Allows to compare a `SnapshotID` against `u64` (Just the Immutable File Number).
    ///
    /// Equality ONLY checks the Immutable File Number, not the path.
    /// This is because the Filename is already the Immutable File Number.
    fn eq(&self, other: &Option<Point>) -> bool {
        if let Some(other) = other {
            *self == *other
        } else {
            false
        }
    }
}

impl PartialOrd<Option<Point>> for Point {
    /// Allows to compare a `Point` against a `u64` (Just the Immutable File Number).
    ///
    /// Equality ONLY checks the Immutable File Number, not the path.
    /// This is because the Filename is already the Immutable File Number.
    /// Any point is greater than None.
    fn partial_cmp(&self, other: &Option<Point>) -> Option<Ordering> {
        if let Some(other) = other {
            self.partial_cmp(other)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl Default for Point {
    /// Returns the default value for `Point`, which is `UNKNOWN_POINT`.
    fn default() -> Self {
        UNKNOWN_POINT
    }
}

/// Compare Points, because Pallas does not impl `Ord` for Point.
pub(crate) fn cmp_point(
    a: &pallas::network::miniprotocols::Point, b: &pallas::network::miniprotocols::Point,
) -> Ordering {
    match a {
        pallas::network::miniprotocols::Point::Origin => {
            match b {
                pallas::network::miniprotocols::Point::Origin => Ordering::Equal,
                pallas::network::miniprotocols::Point::Specific(..) => Ordering::Less,
            }
        },
        pallas::network::miniprotocols::Point::Specific(slot, _) => {
            match b {
                pallas::network::miniprotocols::Point::Origin => Ordering::Greater,
                pallas::network::miniprotocols::Point::Specific(other_slot, _) => {
                    slot.cmp(other_slot)
                },
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use pallas::crypto::hash::Hash;

    use crate::*;

    #[test]
    fn test_create_points() {
        let point1 = Point::new(100u64, vec![]);
        let fuzzy1 = Point::fuzzy(100u64);

        assert!(point1 == fuzzy1);
    }

    #[test]
    fn test_cmp_hash_simple() {
        let origin1 = ORIGIN_POINT;
        let point1 = Point::new(100u64, vec![8; 32]);

        assert!(!origin1.cmp_hash(&Some(Hash::new([0; 32]))));
        assert!(origin1.cmp_hash(&None));

        assert!(point1.cmp_hash(&Some(Hash::new([8; 32]))));
        assert!(!point1.cmp_hash(&None));
    }

    #[test]
    fn test_get_hash_simple() {
        let point1 = Point::new(100u64, vec![8; 32]);

        assert_eq!(point1.hash_or_default(), vec![8; 32]);
    }

    #[test]
    fn test_identical_compare() {
        let point1 = Point::new(100u64, vec![8; 32]);
        let point2 = Point::new(100u64, vec![8; 32]);
        let point3 = Point::new(999u64, vec![8; 32]);

        assert!(point1.strict_eq(&point2));
        assert!(!point1.strict_eq(&point3));
    }

    #[test]
    fn test_comparisons() {
        let origin1 = ORIGIN_POINT;
        let origin2 = ORIGIN_POINT;
        let tip1 = TIP_POINT;
        let tip2 = TIP_POINT;
        let early_block = Point::new(100u64, vec![]);
        let late_block1 = Point::new(5000u64, vec![]);
        let late_block2 = Point::new(5000u64, vec![]);

        assert!(origin1 == origin2);
        assert!(origin1 < early_block);
        assert!(origin1 <= early_block);
        assert!(origin1 != early_block);
        assert!(origin1 < late_block1);
        assert!(origin1 <= late_block1);
        assert!(origin1 != late_block1);
        assert!(origin1 < tip1);
        assert!(origin1 <= tip1);
        assert!(origin1 != tip1);

        assert!(tip1 > origin1);
        assert!(tip1 >= origin1);
        assert!(tip1 != origin1);
        assert!(tip1 > early_block);
        assert!(tip1 >= late_block1);
        assert!(tip1 != late_block1);
        assert!(tip1 == tip2);

        assert!(early_block > origin1);
        assert!(early_block >= origin1);
        assert!(early_block != origin1);
        assert!(early_block < late_block1);
        assert!(early_block <= late_block1);
        assert!(early_block != late_block1);
        assert!(early_block < tip1);
        assert!(early_block <= tip1);
        assert!(early_block != tip1);

        assert!(late_block1 == late_block2);
        assert!(late_block1 > origin1);
        assert!(late_block1 >= origin1);
        assert!(late_block1 != origin1);
        assert!(late_block1 > early_block);
        assert!(late_block1 >= early_block);
        assert!(late_block1 != early_block);
        assert!(late_block1 < tip1);
        assert!(late_block1 <= tip1);
        assert!(late_block1 != tip1);
    }
}
