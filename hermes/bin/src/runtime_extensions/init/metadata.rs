//! Runtime Init metadata

use std::{fmt::Display, sync::Arc};

use traitreg::RegisteredImplWrapper;

/// Runtime Extension Metadata (clean of trait registry baggage.)
/// We can end up with many multiple copies of this data,  using an Arc
/// means clones are cheap, and memory allocation is minimized.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct RteMetadata(Arc<RteMetadataInner>);

impl Display for RteMetadata {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl RteMetadata {
    /// Create new `RteMetadata`.
    ///
    /// Not normally used outside test, use `.into()` instead.
    pub(crate) fn new(inner: RteMetadataInner) -> Self {
        RteMetadata(Arc::new(inner))
    }

    /// Create an RTE Metadata when there is no specific RTE associated with an error.
    pub(crate) fn none() -> Self {
        Self::new(RteMetadataInner {
            has_constructor: false,
            file: "None",
            name: "None",
            path: "None",
            module_path: "None",
            trait_name: "None",
        })
    }
}

impl<T> From<&RegisteredImplWrapper<T>> for RteMetadata {
    fn from(orig: &RegisteredImplWrapper<T>) -> Self {
        Self::new(RteMetadataInner {
            has_constructor: orig.has_constructor(),
            name: orig.name(),
            path: orig.path(),
            file: orig.file(),
            module_path: orig.module_path(),
            trait_name: orig.trait_name(),
        })
    }
}

/// Runtime Extension Metadata (clean of trait registry baggage.)
#[derive(PartialEq, Debug)]
pub(crate) struct RteMetadataInner {
    /// Does it have a constructor?
    pub has_constructor: bool,
    /// Trait name
    pub name: &'static str,
    /// Trait path
    pub path: &'static str,
    /// File name
    pub file: &'static str,
    /// Modules path
    pub module_path: &'static str,
    /// Name of the trait
    pub trait_name: &'static str,
}

impl Display for RteMetadataInner {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "RTE-> name:{}, path:{}, file:{}, module_path:{}, trait_name:{}, constructed: {}",
            self.name,
            self.path,
            self.file,
            self.module_path,
            self.trait_name,
            self.has_constructor
        )
    }
}
