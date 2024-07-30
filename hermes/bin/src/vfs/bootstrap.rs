//! Hermes virtual file system bootstrapper.

use std::path::PathBuf;

use hdf5 as hdf5_lib;

use super::{
    permission::{PermissionLevel, PermissionsTree},
    Vfs,
};
use crate::hdf5 as hermes_hdf5;

/// Hermes virtual file system builder.
pub(crate) struct VfsBootstrapper {
    /// Path to the VFS HDF5 file's directory.
    vfs_dir_path: PathBuf,
    /// VFS file name.
    vfs_file_name: String,
    /// Mounted module's files
    mounted_files: Vec<MountedFile>,
    /// Mounted module's directories.
    mounted_dirs: Vec<MountedDir>,
    /// HDF5 directories to create
    dirs_to_create: Vec<DitToCreate>,
    /// VFS permissions state.
    permissions: PermissionsTree,
}

/// Directory to create object.
struct DitToCreate {
    /// HDF5 directory path.
    path: hermes_hdf5::Path,
}

/// Mounted file object.
struct MountedFile {
    /// HDF5 file path.
    path: hermes_hdf5::Path,
    /// HDF5 file.
    file: hermes_hdf5::File,
}

/// Mounted directory object.
struct MountedDir {
    /// HDF5 directory path.
    path: hermes_hdf5::Path,
    /// HDF5 directory.
    dir: hermes_hdf5::Dir,
}

impl VfsBootstrapper {
    /// Create a new `VfsBootstrapper` instance.
    pub(crate) fn new<P: AsRef<std::path::Path>>(vfs_dir_path: P, vfs_file_name: String) -> Self {
        Self {
            vfs_dir_path: vfs_dir_path.as_ref().to_path_buf(),
            vfs_file_name,
            mounted_files: Vec::new(),
            mounted_dirs: Vec::new(),
            dirs_to_create: Vec::new(),
            permissions: PermissionsTree::new(),
        }
    }

    /// Add a `Dir` creation by the provided path during bootstrapping
    pub(crate) fn with_dir_to_create(&mut self, path: &str, permission: PermissionLevel) {
        self.permissions.add_permission(path, permission);
        self.dirs_to_create.push(DitToCreate { path: path.into() });
    }

    /// Add a mounted file
    pub(crate) fn with_mounted_file(
        &mut self, to: &str, file: hermes_hdf5::File, permission: PermissionLevel,
    ) {
        self.permissions.add_permission(to, permission);
        self.mounted_files.push(MountedFile {
            path: to.into(),
            file,
        });
    }

    /// Add a mounted dir
    pub(crate) fn with_mounted_dir(
        &mut self, to: &str, dir: hermes_hdf5::Dir, permission: PermissionLevel,
    ) {
        self.permissions.add_permission(to, permission);
        self.mounted_dirs.push(MountedDir {
            path: to.into(),
            dir,
        });
    }

    /// Bootstrap the virtual file system from the provided configuration.
    pub(crate) fn bootstrap(self) -> anyhow::Result<Vfs> {
        let mut vfs_file_path = self.vfs_dir_path.join(self.vfs_file_name.as_str());
        vfs_file_path.set_extension(Vfs::FILE_EXTENSION);

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

        Self::mount_hdf5_content(
            &root,
            self.dirs_to_create,
            &self.mounted_files,
            &self.mounted_dirs,
        )?;

        Ok(Vfs {
            root,
            permissions: self.permissions,
        })
    }

    /// Setup hdf5 VFS directories structure.
    fn setup_hdf5_vfs_structure(root: &hermes_hdf5::Dir) -> anyhow::Result<()> {
        root.create_dir(Vfs::TMP_DIR.into())?;
        root.create_dir(Vfs::ETC_DIR.into())?;
        root.create_dir(Vfs::SRV_DIR.into())?;
        root.create_dir(Vfs::USR_DIR.into())?;
        root.create_dir(Vfs::USR_LIB_DIR.into())?;
        root.create_dir(Vfs::LIB_DIR.into())?;
        Ok(())
    }

    /// Mount hdf5 content to the VFS.
    fn mount_hdf5_content(
        root: &hermes_hdf5::Dir, dirs_to_create: Vec<DitToCreate>, mounted_files: &[MountedFile],
        mounted_dirs: &[MountedDir],
    ) -> anyhow::Result<()> {
        for dir_to_create in dirs_to_create {
            let _unused = root.remove_dir(dir_to_create.path.clone());
            root.create_dir(dir_to_create.path)?;
        }
        for mounted in mounted_files {
            let to_dir = root.get_dir(&mounted.path)?;
            let file_path: hermes_hdf5::Path = mounted.file.name().into();
            let _unused = to_dir.remove_file(file_path.clone());
            to_dir.mount_file(&mounted.file, file_path)?;
        }

        for mounted in mounted_dirs {
            let to_dir = root.get_dir(&mounted.path)?;
            let dir_path: hermes_hdf5::Path = mounted.dir.name().into();
            let _unused = to_dir.remove_dir(dir_path.clone());
            to_dir.mount_dir(&mounted.dir, dir_path)?;
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
        assert!(vfs.root.get_dir(&Vfs::TMP_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::ETC_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::SRV_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::USR_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::USR_LIB_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::LIB_DIR.into()).is_ok());

        drop(vfs);
        let _vfs = VfsBootstrapper::new(tmp_dir.path(), vfs_name.clone())
            .bootstrap()
            .unwrap();
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vfs_bootstrap_with_mount_test() {
        let tmp_dir = TempDir::new().unwrap();

        let package = hdf5_lib::File::create(tmp_dir.child("test.hdf5")).unwrap();
        let package_dir = Dir::new(package.as_group().unwrap());

        // prepare mounted package content
        let dir_name = "dir1";
        let dir1 = package_dir.create_dir(dir_name.into()).unwrap();
        let file_name = "file.txt";
        let file = dir1.create_file(file_name.into()).unwrap();

        let vfs_name = "test_vfs".to_string();
        let mut bootstrapper = VfsBootstrapper::new(tmp_dir.path(), vfs_name.clone());

        bootstrapper.with_mounted_file("/", file.clone(), PermissionLevel::ReadAndWrite);

        let dir_to_create_name = "new_dir";
        bootstrapper.with_dir_to_create(
            format!("{}/{dir_to_create_name}", Vfs::LIB_DIR).as_str(),
            PermissionLevel::ReadAndWrite,
        );
        bootstrapper.with_mounted_file(
            format!("{}/{dir_to_create_name}", Vfs::LIB_DIR).as_str(),
            file.clone(),
            PermissionLevel::ReadAndWrite,
        );
        bootstrapper.with_mounted_dir(
            format!("{}/{dir_to_create_name}", Vfs::LIB_DIR).as_str(),
            dir1.clone(),
            PermissionLevel::ReadAndWrite,
        );

        let vfs = bootstrapper.bootstrap().unwrap();

        // check VFS hdf5 directories structure
        assert!(vfs.root.get_file(file_name.into()).is_ok());

        let lib_dir = vfs.root.get_dir(&Vfs::LIB_DIR.into()).unwrap();
        let new_dir = lib_dir.get_dir(&dir_to_create_name.into()).unwrap();
        assert!(new_dir.get_file(file_name.into()).is_ok());
        let dir = new_dir.get_dir(&dir_name.into()).unwrap();
        assert!(dir.get_file(file_name.into()).is_ok());

        // open existing vfs instance from disk with the same bootstrapping configuration

        drop(vfs);
        let mut bootstrapper = VfsBootstrapper::new(tmp_dir.path(), vfs_name.clone());
        bootstrapper.with_mounted_file("/", file.clone(), PermissionLevel::ReadAndWrite);
        let dir_to_create_name = "new_dir";
        bootstrapper.with_dir_to_create(
            format!("{}/{dir_to_create_name}", Vfs::LIB_DIR).as_str(),
            PermissionLevel::ReadAndWrite,
        );
        bootstrapper.with_mounted_file(
            format!("{}/{dir_to_create_name}", Vfs::LIB_DIR).as_str(),
            file.clone(),
            PermissionLevel::ReadAndWrite,
        );
        bootstrapper.with_mounted_dir(
            format!("{}/{dir_to_create_name}", Vfs::LIB_DIR).as_str(),
            dir1.clone(),
            PermissionLevel::ReadAndWrite,
        );

        let _vfs = bootstrapper.bootstrap().unwrap();
    }
}
