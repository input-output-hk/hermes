//! Hermes virtual file system bootstrapper.

use std::path::PathBuf;

use hdf5 as hdf5_lib;

use super::Vfs;
use crate::hdf5::{self as hermes_hdf5};

/// Hermes virtual file system builder.
pub(crate) struct VfsBootstrapper {
    /// Path to the VFS HDF5 file's directory.
    vfs_dir_path: PathBuf,
    /// VFS file name.
    vfs_file_name: String,
    /// Mounted `srv/share` directory.
    mounted_share: Option<hermes_hdf5::Dir>,
    /// Mounted `srv/www` directory.
    mounted_www: Option<hermes_hdf5::Dir>,
}

impl VfsBootstrapper {
    /// Virtual file system file extension.
    const FILE_EXTENSION: &'static str = "hfs";
    /// Virtual file system `share` directory name.
    const SHARE_DIR: &'static str = "share";
    /// Virtual file system `srv` directory name.
    const SRV_DIR: &'static str = "srv";
    /// Virtual file system `www` directory name.
    const WWW_DIR: &'static str = "www";

    /// Create a new `VfsBootstrapper` instance.
    pub(crate) fn new<P: AsRef<std::path::Path>>(vfs_dir_path: P, vfs_file_name: String) -> Self {
        Self {
            vfs_dir_path: vfs_dir_path.as_ref().to_path_buf(),
            vfs_file_name,
            mounted_share: None,
            mounted_www: None,
        }
    }

    /// Add a mounted share directory.
    #[allow(dead_code)]
    pub(crate) fn with_mounted_share(&mut self, mounted_share: hermes_hdf5::Dir) {
        self.mounted_share = Some(mounted_share);
    }

    /// Add a mounted www directory.
    #[allow(dead_code)]
    pub(crate) fn with_mounted_www(&mut self, mounted_www: hermes_hdf5::Dir) {
        self.mounted_share = Some(mounted_www);
    }

    /// Bootstrap the virtual file system from the provided configuration.
    pub(crate) fn bootstrap(self) -> anyhow::Result<Vfs> {
        let mut vfs_file_path = self.vfs_dir_path.join(self.vfs_file_name);
        vfs_file_path.set_extension(Self::FILE_EXTENSION);

        let hdf5_file = if let Ok(hdf5_file) = hdf5_lib::File::open_rw(&vfs_file_path) {
            hdf5_file
        } else {
            hdf5_lib::File::create(&vfs_file_path).map_err(|_| {
                anyhow::anyhow!(
                    "Failed to create Hermes virtual file system instance at {}.",
                    vfs_file_path.display()
                )
            })?
        };
        let root = hermes_hdf5::Dir::new(hdf5_file.as_group()?);

        let srv_dir = root.create_dir(&Self::SRV_DIR.into())?;
        if let Some(www) = self.mounted_www.as_ref() {
            srv_dir.mount_dir(www, Self::WWW_DIR)?;
        }
        if let Some(share) = self.mounted_share.as_ref() {
            srv_dir.mount_dir(share, Self::SHARE_DIR)?;
        }

        Ok(Vfs { root })
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn vfs_bootstrap_test() -> anyhow::Result<()> {
        let dir = TempDir::new()?;

        let vfs_name = "test_vfs".to_string();

        let vfs = VfsBootstrapper::new(dir.path(), vfs_name.clone()).bootstrap()?;
        drop(vfs);

        let _vfs = VfsBootstrapper::new(dir.path(), vfs_name).bootstrap()?;
        Ok(())
    }
}
