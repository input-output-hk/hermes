//! HDF5 based resource implementation.

use std::{
    fmt::{Debug, Display},
    io::Read,
};

use super::{super::Path, ResourceTrait};

/// HDF5 resource struct.
#[derive(Debug, Clone)]
pub(crate) enum Hdf5Resource {
    /// HDF5 group.
    Group(hdf5::Group),
    /// HDF5 dataset.
    Dataset(hdf5::Dataset),
}

impl Display for Hdf5Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Group(g) => g.fmt(f),
            Self::Dataset(d) => d.fmt(f),
        }
    }
}

impl ResourceTrait for Hdf5Resource {
    fn name(&self) -> anyhow::Result<String> {
        match self {
            Self::Group(g) => Ok(Path::from_str(&g.name()).pop_elem()?),
            Self::Dataset(d) => Ok(Path::from_str(&d.name()).pop_elem()?),
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            Self::Group(_) => true,
            Self::Dataset(_) => false,
        }
    }

    fn is_file(&self) -> bool {
        match self {
            Self::Group(_) => false,
            Self::Dataset(_) => true,
        }
    }

    fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        match self {
            Self::Group(_) => Err(anyhow::anyhow!("Hdf5Resource is not a file")),
            Self::Dataset(d) => Ok(d.as_byte_reader()?),
        }
    }

    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>> {
        match self {
            Self::Dataset(_) => anyhow::bail!("Hdf5Resource is not a directory"),
            Self::Group(g) => {
                let d_iter = g.datasets()?.into_iter().map(Self::Dataset);
                let g_iter = g.groups()?.into_iter().map(Self::Group);
                Ok(d_iter.chain(g_iter).collect())
            },
        }
    }
}
