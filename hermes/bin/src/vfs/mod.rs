//! Hermes virtual file system.

mod bootstrap;
mod permission;

use std::io::{Read, Write};

pub(crate) use bootstrap::VfsBootstrapper;
pub(crate) use permission::PermissionLevel;
use permission::PermissionsState;

use crate::hdf5 as hermes_hdf5;

/// Hermes virtual file system type.
#[derive(Debug)]
pub(crate) struct Vfs {
    /// HDF5 root directory of the virtual file system.
    root: hermes_hdf5::Dir,
    /// VFS permissions state.
    permissions: PermissionsState,
}

impl Vfs {
    /// Virtual file system `etc` directory name.
    pub(crate) const ETC_DIR: &'static str = "etc";
    /// Virtual file system file extension.
    pub(crate) const FILE_EXTENSION: &'static str = "hfs";
    /// Virtual file system `lib` directory name.
    pub(crate) const LIB_DIR: &'static str = "lib";
    /// Virtual file system `srv` directory name.
    pub(crate) const SRV_DIR: &'static str = "srv";
    /// Virtual file system `tmp` directory name.
    pub(crate) const TMP_DIR: &'static str = "tmp";
    /// Virtual file system `usr` directory name.
    pub(crate) const USR_DIR: &'static str = "usr";
    /// Virtual file system `usr/lib` directory name.
    pub(crate) const USR_LIB_DIR: &'static str = "usr/lib";
}

impl Vfs {
    /// Reads in data in bytes, the number of which is specified by the caller,
    /// from the hdf5 file and stores then into a buffer supplied by the calling process.
    #[allow(dead_code)]
    pub(crate) fn read(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let mut file = self.root.get_file(path.into())?;

        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    /// Writes data from a buffer declared by the user to a hdf5 file.
    #[allow(dead_code)]
    pub(crate) fn write(&self, path: &str, buffer: &[u8]) -> anyhow::Result<()> {
        let permission = self.permissions.get_permission(path);
        anyhow::ensure!(
            permission == PermissionLevel::ReadAndWrite,
            "Permission denied, file does not has write permission."
        );

        let path: hermes_hdf5::Path = path.into();
        let mut file = match self.root.get_file(path.clone()) {
            Ok(file) => file,
            Err(_) => self.root.create_file(path)?,
        };

        let _unused = file.write(buffer)?;
        file.flush()?;

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn read_write_file_test() {
        // bootstrap
        let dir = TempDir::new().expect("Failed to create temp dir");

        let vfs_name = "test_vfs".to_string();

        let bootstrapper = VfsBootstrapper::new(dir.path(), vfs_name.clone());

        let vfs = bootstrapper.bootstrap().expect("Cannot bootstrap");

        let file_path = format!("{}/www.txt", Vfs::SRV_DIR);
        let file_content = b"web_server";
        vfs.write(file_path.as_str(), file_content)
            .expect("Cannot write to VFS");

        let written_data = vfs.read(file_path.as_str()).expect("Cannot read from VFS");
        assert_eq!(written_data.as_slice(), file_content);
    }
}
