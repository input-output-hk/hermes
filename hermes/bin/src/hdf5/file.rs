//! A Hermes HDF5 file abstraction over the HDF5 dataset object.

use super::Path;
use crate::hdf5::compression::enable_compression;

/// Hermes HDF5 file object, wrapper of `hdf5::Dataset`
pub(crate) struct File {
    /// HDF5 dataset object.
    pub(super) hdf5_ds: hdf5::Dataset,
    /// Reading/Writing position of the `hdf5_ds`.
    pos: usize,
}

impl std::fmt::Display for File {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.path())
    }
}

impl std::fmt::Debug for File {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.path())
    }
}

impl std::clone::Clone for File {
    fn clone(&self) -> Self {
        Self {
            hdf5_ds: self.hdf5_ds.clone(),
            pos: self.pos,
        }
    }
}

impl File {
    /// Create a new file.
    pub(crate) fn create(
        group: &hdf5::Group,
        file_name: &str,
    ) -> anyhow::Result<Self> {
        let builder = group.new_dataset_builder();
        let shape = hdf5::SimpleExtents::resizable([0]);
        let hdf5_ds = enable_compression(builder)
            .empty::<u8>()
            .shape(shape)
            .create(file_name)?;
        Ok(Self { hdf5_ds, pos: 0 })
    }

    /// Open an existing `File` from the provided dataset.
    pub(crate) fn open(hdf5_ds: hdf5::Dataset) -> Self {
        Self { hdf5_ds, pos: 0 }
    }

    /// Return file `Path`.
    pub(crate) fn path(&self) -> Path {
        Path::from_str(&self.hdf5_ds.name())
    }

    /// Return file name.
    pub(crate) fn name(&self) -> String {
        self.path().pop_elem()
    }

    /// Return file size.
    pub(crate) fn size(&self) -> anyhow::Result<usize> {
        let shape = self.hdf5_ds.space()?.shape();
        let size = shape
            .first()
            .ok_or(anyhow::anyhow!("Failed to get file size.",))?;
        Ok(*size)
    }
}

/// Convert arbitrary error to `std::io::Error`.
#[allow(clippy::needless_pass_by_value)]
fn map_to_io_error(err: impl ToString) -> std::io::Error {
    std::io::Error::other(err.to_string())
}

impl std::io::Read for File {
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> std::io::Result<usize> {
        let file_size = self.size().map_err(map_to_io_error)?;
        let remaining_len = file_size.saturating_sub(self.pos);

        let reading_len = std::cmp::min(buf.len(), remaining_len);
        let selection = hdf5::Selection::new(self.pos..self.pos.saturating_add(reading_len));

        let data = self
            .hdf5_ds
            .read_slice_1d::<u8, _>(selection)
            .map_err(map_to_io_error)?;

        let data_slice = data
            .as_slice()
            .ok_or(map_to_io_error("Failed to read data."))?;
        #[allow(clippy::indexing_slicing)]
        buf[..reading_len].copy_from_slice(data_slice);

        self.pos = self.pos.saturating_add(reading_len);
        Ok(reading_len)
    }
}

impl std::io::Write for File {
    fn write(
        &mut self,
        buf: &[u8],
    ) -> std::io::Result<usize> {
        let file_size = self.size().map_err(map_to_io_error)?;
        let remaining_len = file_size.saturating_sub(self.pos);
        let increasing_len = buf.len().saturating_sub(remaining_len);

        let new_shape = [file_size.saturating_add(increasing_len)];
        self.hdf5_ds.resize(new_shape).map_err(map_to_io_error)?;

        let selection = hdf5::Selection::new(self.pos..self.pos.saturating_add(buf.len()));

        self.hdf5_ds
            .write_slice(buf, selection)
            .map_err(map_to_io_error)?;

        self.pos = self.pos.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let package = self.hdf5_ds.file().map_err(map_to_io_error)?;
        package.flush().map_err(map_to_io_error)?;
        Ok(())
    }
}

impl std::io::Seek for File {
    fn seek(
        &mut self,
        pos: std::io::SeekFrom,
    ) -> std::io::Result<u64> {
        let (base_pos, offset) = match pos {
            std::io::SeekFrom::Start(n) => {
                self.pos = n.try_into().map_err(map_to_io_error)?;
                return Ok(n);
            },
            std::io::SeekFrom::End(n) => (self.size().map_err(map_to_io_error)?, n),
            std::io::SeekFrom::Current(n) => (self.pos, n),
        };
        let new_pos = if offset.is_negative() {
            base_pos.checked_sub(offset.wrapping_abs().try_into().map_err(map_to_io_error)?)
        } else {
            base_pos.checked_add(offset.try_into().map_err(map_to_io_error)?)
        };
        match new_pos {
            Some(n) => {
                self.pos = n;
                Ok(self.pos.try_into().map_err(map_to_io_error)?)
            },
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid seek to a negative or overflowing position",
            )),
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.pos.try_into().map_err(map_to_io_error)
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use std::io::{Read, Seek, Write};

    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn file_test() {
        let tmp_dir = TempDir::new().unwrap();

        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).unwrap();
        let group = package.as_group().unwrap();

        let file_name = "test.txt";

        assert!(group.dataset(file_name).is_err());
        let mut file = File::create(&group, file_name).unwrap();
        assert!(group.dataset(file_name).is_ok());

        let file_content = b"file_content";
        let written = file.write(file_content).unwrap();
        assert_eq!(written, file_content.len());
        let written = file.write(file_content).unwrap();
        assert_eq!(written, file_content.len());

        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut buffer = [0; 12];
        assert_eq!(buffer.len(), file_content.len());
        let read = file.read(&mut buffer).unwrap();
        assert_eq!(read, file_content.len());
        assert_eq!(buffer.as_slice(), file_content.as_slice());
        let read = file.read(&mut buffer).unwrap();
        assert_eq!(read, file_content.len());
        assert_eq!(buffer.as_slice(), file_content.as_slice());

        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let new_file_content = b"new_file_content";
        let written = file.write(new_file_content).unwrap();
        assert_eq!(written, new_file_content.len());

        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut buffer = [0; 16];
        assert_eq!(buffer.len(), new_file_content.len());
        let read = file.read(&mut buffer).unwrap();
        assert_eq!(read, new_file_content.len());
        assert_eq!(buffer.as_slice(), new_file_content.as_slice());
    }
}
