//! A Cardano Point on the Blockchain, or Tip.

pub use pallas::network::miniprotocols::Point;

/// A point in the chain or the tip.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PointOrTip {
    /// Represents a specific point of the chain.
    Point(Point),
    /// Represents the tip of the chain.
    Tip,
}

impl From<Point> for PointOrTip {
    fn from(point: Point) -> Self {
        Self::Point(point)
    }
}
