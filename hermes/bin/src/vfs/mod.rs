//! Hermes virtual file system.

mod bootstrap;

use std::io::{Read, Write};

pub(crate) use bootstrap::VfsBootstrapper;

use crate::hdf5::{self as hermes_hdf5, Path};

/// Hermes virtual file system type.
pub(crate) struct Vfs {
    /// HDF5 root directory of the virtual file system.
    #[allow(dead_code)]
    root: hermes_hdf5::Dir,
}

impl Vfs {
    /// Reads in data in bytes, the number of which is specified by the caller,
    /// from the hdf5 file and stores then into a buffer supplied by the calling process.
    #[allow(dead_code)]
    pub(crate) fn read(&self, path: Path) -> anyhow::Result<Vec<u8>, anyhow::Error> {
        let mut file = self.root.get_file(path)?;

        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    /// Writes data from a buffer declared by the user to a hdf5 file.
    #[allow(dead_code)]
    pub(crate) fn write(&self, path: &Path, buffer: &[u8]) -> anyhow::Result<(), anyhow::Error> {
        let mut file = match self.root.get_file(path.clone()) {
            Ok(file) => file,
            Err(_) => self.root.create_file(path.clone())?,
        };

        let _unused = file.write(&buffer)?;

        Ok(())
    }
}
#[cfg(test)]
mod tests {

    use temp_dir::TempDir;

    use crate::hdf5::{Dir, Path};

    use super::VfsBootstrapper;

    #[test]
    fn read_write_file_test() {
        // bootstrap
        let dir = TempDir::new().unwrap();

        let vfs_name = "test_vfs".to_string();

        let tmp_dir_www = TempDir::new().expect("Failed to create temp dir.");
        let www = tmp_dir_www.child("www.hdf5");

        let www = hdf5::File::create(www).expect("Failed to create a new package.");
        let www_dir = Dir::new(www.as_group().expect("Failed to create a root group."));

        let mut bootstrapper = VfsBootstrapper::new(dir.path(), vfs_name.clone());

        bootstrapper.with_mounted_www(www_dir);
        let vfs = bootstrapper.bootstrap().unwrap();

        let www_file_path = Path::from_str("/www");
        vfs.write(&www_file_path, b"web_server").unwrap();

        let written_data = vfs.read(www_file_path).unwrap();
        assert_eq!(10, written_data.len());

        let written = String::from_utf8_lossy(&written_data).to_string();
        assert_eq!(written, "web_server".to_string());
    }
}
