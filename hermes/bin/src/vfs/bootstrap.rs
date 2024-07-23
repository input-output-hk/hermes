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
    /// HDF5 mounted content.
    hdf5_mount: Hdf5Mount,
}

/// HDF5 mounted content struct.
#[derive(Default)]
pub(crate) struct Hdf5Mount {
    /// Mounted files to the `/` dir
    root_files: Vec<hermes_hdf5::File>,
    /// Mounted `srv/share` directory.
    share: Option<hermes_hdf5::Dir>,
    /// Mounted `srv/www` directory.
    www: Option<hermes_hdf5::Dir>,
}

impl Hdf5Mount {
    /// Add a mounted root files
    pub(crate) fn with_root_files(&mut self, root_files: Vec<hermes_hdf5::File>) {
        self.root_files = root_files;
    }

    /// Add a mounted share directory.
    pub(crate) fn with_share_dir(&mut self, share: hermes_hdf5::Dir) {
        self.share = Some(share);
    }

    /// Add a mounted www directory.
    pub(crate) fn with_www_dir(&mut self, www: hermes_hdf5::Dir) {
        self.www = Some(www);
    }
}

impl VfsBootstrapper {
    /// Virtual file system `etc` directory name.
    const ETC_DIR: &'static str = "etc";
    /// Virtual file system file extension.
    const FILE_EXTENSION: &'static str = "hfs";
    /// Virtual file system `lib` directory name.
    const LIB_DIR: &'static str = "lib";
    /// Virtual file system `srv` directory name.
    const SRV_DIR: &'static str = "srv";
    /// Virtual file system `srv/share` directory name.
    const SRV_SHARE_DIR: &'static str = "srv/share";
    /// Virtual file system `srv/www` directory name.
    const SRV_WWW_DIR: &'static str = "srv/www";
    /// Virtual file system `tmp` directory name.
    const TMP_DIR: &'static str = "tmp";
    /// Virtual file system `usr` directory name.
    const USR_DIR: &'static str = "usr";
    /// Virtual file system `usr/lib` directory name.
    const USR_LIB_DIR: &'static str = "usr/lib";

    /// Create a new `VfsBootstrapper` instance.
    pub(crate) fn new<P: AsRef<std::path::Path>>(vfs_dir_path: P, vfs_file_name: String) -> Self {
        Self {
            vfs_dir_path: vfs_dir_path.as_ref().to_path_buf(),
            vfs_file_name,
            hdf5_mount: Hdf5Mount::default(),
        }
    }

    /// Set `Hdf5Mount` object
    pub(crate) fn set_hdf5_mount(&mut self, hdf5_mount: Hdf5Mount) {
        self.hdf5_mount = hdf5_mount;
    }

    /// Bootstrap the virtual file system from the provided configuration.
    pub(crate) fn bootstrap(self) -> anyhow::Result<Vfs> {
        let mut vfs_file_path = self.vfs_dir_path.join(self.vfs_file_name.as_str());
        vfs_file_path.set_extension(Self::FILE_EXTENSION);

        let root = if let Ok(hdf5_file) = hdf5_lib::File::open_rw(&vfs_file_path) {
            hermes_hdf5::Dir::new(hdf5_file.as_group()?)
        } else {
            let hdf5_file = hdf5_lib::File::create(&vfs_file_path).map_err(|_| {
                anyhow::anyhow!(
                    "Failed to create Hermes virtual file system instance at `{}`.",
                    vfs_file_path.display()
                )
            })?;
            let root = hermes_hdf5::Dir::new(hdf5_file.as_group()?);
            Self::setup_hdf5_vfs_structure(&root)?;
            root
        };

        Self::mount_hdf5_content(&root, &self.hdf5_mount)?;

        Ok(Vfs { root })
    }

    /// Setup hdf5 VFS directories structure.
    fn setup_hdf5_vfs_structure(root: &hermes_hdf5::Dir) -> anyhow::Result<()> {
        root.create_dir(Self::TMP_DIR.into())?;
        root.create_dir(Self::ETC_DIR.into())?;
        root.create_dir(Self::SRV_DIR.into())?;
        root.create_dir(Self::USR_DIR.into())?;
        root.create_dir(Self::USR_LIB_DIR.into())?;
        root.create_dir(Self::LIB_DIR.into())?;

        Ok(())
    }

    /// Mount hdf5 content to the VFS.
    fn mount_hdf5_content(root: &hermes_hdf5::Dir, mount: &Hdf5Mount) -> anyhow::Result<()> {
        for root_file in &mount.root_files {
            root.mount_file(&root_file, root_file.name().into())?;
        }

        if let Some(www) = mount.www.as_ref() {
            root.mount_dir(www, Self::SRV_WWW_DIR.into())?;
        }
        if let Some(share) = mount.share.as_ref() {
            root.mount_dir(share, Self::SRV_SHARE_DIR.into())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hermes_hdf5::Dir;
    use temp_dir::TempDir;

    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]

    fn vfs_bootstrap_test() {
        let tmp_dir = TempDir::new().unwrap();

        let vfs_name = "test_vfs".to_string();
        let vfs = VfsBootstrapper::new(tmp_dir.path(), vfs_name.clone())
            .bootstrap()
            .unwrap();

        // check VFS hdf5 directories structure
        assert!(vfs.root.get_dir(&VfsBootstrapper::TMP_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&VfsBootstrapper::ETC_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&VfsBootstrapper::SRV_DIR.into()).is_ok());
        assert!(vfs
            .root
            .get_dir(&VfsBootstrapper::SRV_WWW_DIR.into())
            .is_err());
        assert!(vfs
            .root
            .get_dir(&VfsBootstrapper::SRV_SHARE_DIR.into())
            .is_err());
        assert!(vfs.root.get_dir(&VfsBootstrapper::USR_DIR.into()).is_ok());
        assert!(vfs
            .root
            .get_dir(&VfsBootstrapper::USR_LIB_DIR.into())
            .is_ok());
        assert!(vfs.root.get_dir(&VfsBootstrapper::LIB_DIR.into()).is_ok());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vfs_bootstrap_with_mount_test() {
        let tmp_dir = TempDir::new().unwrap();

        let package = hdf5_lib::File::create(tmp_dir.child("test.hdf5")).unwrap();
        let dir = Dir::new(package.as_group().unwrap());

        // prepare mounted package content
        let dir1 = dir.create_dir("dir1".into()).unwrap();
        let file_name = "file.txt";
        let file = dir1.create_file(file_name.into()).unwrap();

        let mut mount = Hdf5Mount::default();
        mount.with_root_files(vec![file]);
        mount.with_www_dir(dir1.clone());
        mount.with_share_dir(dir1.clone());

        let vfs_name = "test_vfs".to_string();
        let mut bootstrapper = VfsBootstrapper::new(tmp_dir.path(), vfs_name);
        bootstrapper.set_hdf5_mount(mount);

        let vfs = bootstrapper.bootstrap().unwrap();

        // check VFS hdf5 directories structure
        assert!(vfs.root.get_file(file_name.into()).is_ok());
        let www_dir = vfs
            .root
            .get_dir(&VfsBootstrapper::SRV_WWW_DIR.into())
            .unwrap();
        assert!(www_dir.get_file(file_name.into()).is_ok());
        let share_dir = vfs
            .root
            .get_dir(&VfsBootstrapper::SRV_SHARE_DIR.into())
            .unwrap();
        assert!(share_dir.get_file(file_name.into()).is_ok());
    }
}
