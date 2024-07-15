//! Hermes virtual file system.

use hdf5 as hdf5_lib;

use crate::hdf5 as hermes_hdf5;

/// Hermes virtual file system type.
#[allow(dead_code)]
pub(crate) struct Vfs {
    /// HDFR5 root directory of the virtual file system.
    root: hermes_hdf5::Dir,
}

impl Vfs {
    /// Hermes virtual file system file extension.
    const FILE_EXTENSION: &'static str = "hfs";

    /// Bootstrap virtual file system and return a `Vfs` instance.
    /// `fs_file_path` is the path to the `Vfs` file's directory.
    /// `fs_file_name` is the name of the `Vfs` file.
    #[allow(dead_code)]
    pub(crate) fn bootstrap<P: AsRef<std::path::Path>>(
        fs_file_path: P, fs_file_name: &str,
    ) -> anyhow::Result<Self> {
        let fs_file_path = fs_file_path.as_ref().join(fs_file_name);
        fs_file_path.with_extension(Self::FILE_EXTENSION);

        let hdf5_file = if let Ok(hdf5_file) = hdf5_lib::File::open_rw(&fs_file_path) {
            hdf5_file
        } else {
            hdf5_lib::File::create(fs_file_path).map_err(|_| {
                anyhow::anyhow!("Failed to create Hermes virtual file system instance at.")
            })?
        };
        let root = hermes_hdf5::Dir::new(hdf5_file.as_group()?);

        Ok(Self { root })
    }
}
