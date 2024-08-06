//! Hermes virtual file system file object.

#![allow(dead_code, missing_docs, clippy::missing_docs_in_private_items)]

use super::PermissionLevel;
use crate::hdf5 as hermes_hdf5;

pub(crate) struct File {
    file: hermes_hdf5::File,
    permission: PermissionLevel,
}

impl File {
    /// Create a new `File` instance.
    pub(crate) fn new(file: hermes_hdf5::File, permission: PermissionLevel) -> Self {
        Self { file, permission }
    }

    /// Returns the name of the file.
    pub(crate) fn name(&self) -> String {
        self.file.name()
    }

    /// Returns the size of the file.
    pub(crate) fn size(&self) -> anyhow::Result<usize> {
        self.file.size()
    }
}

impl std::io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Write for File {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.permission == PermissionLevel::Read {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "File is read only, any write operation is denied.",
            ));
        }

        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.permission == PermissionLevel::Read {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "File is read only, any write operation is denied.",
            ));
        }

        self.file.flush()
    }
}

impl std::io::Seek for File {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.file.stream_position()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use hdf5 as hdf5_lib;
    use temp_dir::TempDir;

    use super::*;
    use crate::utils::tests::std_io_read_write_seek_test;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn file_test() {
        let dir = TempDir::new().unwrap();

        let package_name = dir.child("test.hdf5");
        let package = hdf5_lib::File::create(package_name).unwrap();
        let group = package
            .as_group()
            .expect("Failed to get a root group from package.");

        let file_name = "test.txt";
        let hdf5_file = hermes_hdf5::File::create(&group, file_name).unwrap();
        let file = File::new(hdf5_file, PermissionLevel::ReadAndWrite);

        std_io_read_write_seek_test(file);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn file_read_only_test() {
        let dir = TempDir::new().unwrap();

        let package_name = dir.child("test.hdf5");
        let package = hdf5_lib::File::create(package_name).unwrap();
        let group = package
            .as_group()
            .expect("Failed to get a root group from package.");

        let file_name = "test.txt";
        let hdf5_file = hermes_hdf5::File::create(&group, file_name).unwrap();
        let mut file = File::new(hdf5_file, PermissionLevel::Read);

        let content = b"content";
        assert!(file.write(content).is_err());
    }
}
