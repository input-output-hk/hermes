//! Hermes virtual file system bootstrapper.

use std::path::PathBuf;

use hdf5 as hdf5_lib;

use super::{
    permission::{PermissionLevel, PermissionsState},
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
    dirs_to_create: Vec<DirToCreate>,
}

/// Directory to create object.
struct DirToCreate {
    /// Path where create directory.
    path: String,
    /// Permission level.
    permission: PermissionLevel,
}

/// Mounted file object.
struct MountedFile {
    /// Path where to mount file.
    to_path: String,
    /// HDF5 file.
    file: hermes_hdf5::File,
    /// Permission level.
    permission: PermissionLevel,
}

/// Mounted directory object.
struct MountedDir {
    /// Path where to mount directory.
    to_path: String,
    /// HDF5 directory.
    dir: hermes_hdf5::Dir,
    /// Permission level.
    permission: PermissionLevel,
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
        }
    }

    /// Add a `Dir` creation by the provided path during bootstrapping
    pub(crate) fn with_dir_to_create(&mut self, path: String, permission: PermissionLevel) {
        self.dirs_to_create.push(DirToCreate { path, permission });
    }

    /// Add a mounted file
    pub(crate) fn with_mounted_file(
        &mut self, to_path: String, file: hermes_hdf5::File, permission: PermissionLevel,
    ) {
        self.mounted_files.push(MountedFile {
            to_path,
            file,
            permission,
        });
    }

    /// Add a mounted dir
    pub(crate) fn with_mounted_dir(
        &mut self, to_path: String, dir: hermes_hdf5::Dir, permission: PermissionLevel,
    ) {
        self.mounted_dirs.push(MountedDir {
            to_path,
            dir,
            permission,
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
            Self::setup_vfs_structure(&root)?;
            root
        };

        let mut permissions = PermissionsState::new();
        Self::setup_vfs_permissions(&mut permissions);
        Self::setup_vfs_content(
            &root,
            &self.dirs_to_create,
            &self.mounted_files,
            &self.mounted_dirs,
            &mut permissions,
        )?;

        Ok(Vfs { root, permissions })
    }

    /// Setup VFS directories structure.
    fn setup_vfs_structure(root: &hermes_hdf5::Dir) -> anyhow::Result<()> {
        root.create_dir(Vfs::TMP_DIR.into())?;
        root.create_dir(Vfs::ETC_DIR.into())?;
        root.create_dir(Vfs::SRV_DIR.into())?;
        root.create_dir(Vfs::USR_DIR.into())?;
        root.create_dir(Vfs::USR_LIB_DIR.into())?;
        root.create_dir(Vfs::LIB_DIR.into())?;
        Ok(())
    }

    /// Setup VFS directories permissions.
    fn setup_vfs_permissions(permissions: &mut PermissionsState) {
        permissions.add_permission(Vfs::TMP_DIR, PermissionLevel::ReadAndWrite);
        permissions.add_permission(Vfs::ETC_DIR, PermissionLevel::ReadAndWrite);
        permissions.add_permission(Vfs::SRV_DIR, PermissionLevel::Read);
        permissions.add_permission(Vfs::USR_DIR, PermissionLevel::Read);
        permissions.add_permission(Vfs::USR_LIB_DIR, PermissionLevel::Read);
        permissions.add_permission(Vfs::LIB_DIR, PermissionLevel::Read);
    }

    /// Setup initial content of the VFS.
    fn setup_vfs_content(
        root: &hermes_hdf5::Dir, dirs_to_create: &[DirToCreate], mounted_files: &[MountedFile],
        mounted_dirs: &[MountedDir], permissions: &mut PermissionsState,
    ) -> anyhow::Result<()> {
        for dir_to_create in dirs_to_create {
            Self::create_dir(root, dir_to_create, permissions)?;
        }
        for mounted in mounted_files {
            Self::mount_file(root, mounted, permissions)?;
        }

        for mounted in mounted_dirs {
            Self::mount_dir(root, mounted, permissions)?;
        }
        Ok(())
    }

    /// Create dir for the VFS.
    fn create_dir(
        root: &hermes_hdf5::Dir, dir_to_create: &DirToCreate, permissions: &mut PermissionsState,
    ) -> anyhow::Result<()> {
        let path_str = dir_to_create.path.as_str();
        permissions.add_permission(path_str, dir_to_create.permission);
        let path: hermes_hdf5::Path = path_str.into();
        let _unused = root.remove_dir(path.clone());
        root.create_dir(path)?;
        Ok(())
    }

    /// Mount file of the VFS.
    fn mount_file(
        root: &hermes_hdf5::Dir, mounted: &MountedFile, permissions: &mut PermissionsState,
    ) -> anyhow::Result<()> {
        let path_str = format!("{}/{}", mounted.to_path, mounted.file.name());
        permissions.add_permission(path_str.as_str(), mounted.permission);
        let path: hermes_hdf5::Path = path_str.into();
        let _unused = root.remove_file(path.clone());
        root.mount_file(&mounted.file, path)?;
        Ok(())
    }

    /// Mount dir of the VFS.
    fn mount_dir(
        root: &hermes_hdf5::Dir, mounted: &MountedDir, permissions: &mut PermissionsState,
    ) -> anyhow::Result<()> {
        let path_str = format!("{}/{}", mounted.to_path, mounted.dir.name());
        permissions.add_permission(path_str.as_str(), mounted.permission);
        let path: hermes_hdf5::Path = path_str.into();
        let _unused = root.remove_dir(path.clone());
        root.mount_dir(&mounted.dir, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hermes_hdf5::Dir;
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn vfs_bootstrap_test() {
        let tmp_dir = TempDir::new().unwrap();

        let vfs_name = "test_vfs".to_string();
        let vfs = VfsBootstrapper::new(tmp_dir.path(), vfs_name.clone())
            .bootstrap()
            .unwrap();

        // check VFS permissions
        assert_eq!(
            vfs.permissions.get_permission(Vfs::TMP_DIR),
            PermissionLevel::ReadAndWrite
        );
        assert_eq!(
            vfs.permissions.get_permission(Vfs::ETC_DIR),
            PermissionLevel::ReadAndWrite
        );
        assert_eq!(
            vfs.permissions.get_permission(Vfs::SRV_DIR),
            PermissionLevel::Read
        );
        assert_eq!(
            vfs.permissions.get_permission(Vfs::USR_DIR),
            PermissionLevel::Read
        );
        assert_eq!(
            vfs.permissions.get_permission(Vfs::USR_LIB_DIR),
            PermissionLevel::Read
        );
        assert_eq!(
            vfs.permissions.get_permission(Vfs::LIB_DIR),
            PermissionLevel::Read
        );

        // check VFS structure
        assert!(vfs.root.get_dir(&Vfs::TMP_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::ETC_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::SRV_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::USR_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::USR_LIB_DIR.into()).is_ok());
        assert!(vfs.root.get_dir(&Vfs::LIB_DIR.into()).is_ok());
    }

    #[test]
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

        bootstrapper.with_mounted_file("/".to_string(), file.clone(), PermissionLevel::Read);

        let dir_to_create_name = "new_dir";
        bootstrapper.with_dir_to_create(
            dir_to_create_name.to_string(),
            PermissionLevel::ReadAndWrite,
        );
        bootstrapper.with_mounted_file(
            dir_to_create_name.to_string(),
            file.clone(),
            PermissionLevel::Read,
        );
        bootstrapper.with_mounted_dir(
            dir_to_create_name.to_string(),
            dir1.clone(),
            PermissionLevel::Read,
        );

        let vfs = bootstrapper.bootstrap().unwrap();

        // check VFS permissions
        assert_eq!(
            vfs.permissions.get_permission(file_name),
            PermissionLevel::Read
        );
        assert_eq!(
            vfs.permissions.get_permission(dir_to_create_name),
            PermissionLevel::ReadAndWrite
        );
        assert_eq!(
            vfs.permissions
                .get_permission(format!("{dir_to_create_name}/{file_name}").as_str()),
            PermissionLevel::Read
        );
        assert_eq!(
            vfs.permissions
                .get_permission(format!("{dir_to_create_name}/{dir_name}").as_str()),
            PermissionLevel::Read
        );
        assert_eq!(
            vfs.permissions
                .get_permission(format!("{dir_to_create_name}/{dir_name}/{file_name}").as_str()),
            PermissionLevel::Read
        );

        // check VFS structure
        assert!(vfs.root.get_file(file_name.into()).is_ok());
        let new_dir = vfs.root.get_dir(&dir_to_create_name.into()).unwrap();
        assert!(new_dir.get_file(file_name.into()).is_ok());
        let dir = new_dir.get_dir(&dir_name.into()).unwrap();
        assert!(dir.get_file(file_name.into()).is_ok());
    }

    #[test]
    fn vfs_bootstrap_existing_test() {
        let tmp_dir = TempDir::new().unwrap();

        let package = hdf5_lib::File::create(tmp_dir.child("test.hdf5")).unwrap();
        let package_dir = Dir::new(package.as_group().unwrap());

        // prepare mounted package content
        let dir_name = "dir1";
        let dir1 = package_dir.create_dir(dir_name.into()).unwrap();
        let file_name = "file.txt";
        let file = dir1.create_file(file_name.into()).unwrap();

        let setup_bootstrapper = || {
            let mut bootstrapper = VfsBootstrapper::new(tmp_dir.path(), "test_vfs".to_string());
            bootstrapper.with_dir_to_create("new_dir".to_string(), PermissionLevel::ReadAndWrite);
            bootstrapper.with_mounted_file(
                "/".to_string(),
                file.clone(),
                PermissionLevel::ReadAndWrite,
            );
            bootstrapper.with_mounted_dir(
                "/".to_string(),
                dir1.clone(),
                PermissionLevel::ReadAndWrite,
            );
            bootstrapper
        };

        let vfs = setup_bootstrapper().bootstrap().unwrap();

        // open existing vfs instance from disk with the same bootstrapping configuration
        drop(vfs);
        let _vfs = setup_bootstrapper().bootstrap().unwrap();
    }
}
