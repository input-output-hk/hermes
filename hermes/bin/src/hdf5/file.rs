//! A Hermes HDF5 file abstraction over the HDF5 dataset object.

use super::{compression::enable_compression, Path};

/// Hermes HDF5 file object, wrapper of `hdf5::Dataset`
#[derive(Clone, Debug)]
pub(crate) struct File {
    /// HDF5 dataset object.
    hdf5_ds: hdf5::Dataset,
    /// Reading/Writing position of the `hdf5_ds`.
    pos: usize,
}

impl File {
    /// Create a new file.
    pub(crate) fn create(
        group: &hdf5::Group, file_name: &str, data: &[u8],
    ) -> anyhow::Result<Self> {
        let builder = group.new_dataset_builder();
        let shape = hdf5::SimpleExtents::resizable([data.len()]);
        let hdf5_ds = enable_compression(builder)
            .empty::<u8>()
            .shape(shape)
            .create(file_name)?;
        hdf5_ds.write(data)?;
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

    /// Return file size.
    fn size(&self) -> anyhow::Result<usize> {
        let dataspace = self.hdf5_ds.space()?;
        let shape = dataspace.shape();
        let size = shape
            .first()
            .ok_or(anyhow::anyhow!("Failed to get file size.",))?;
        Ok(*size)
    }
}

/// Convert arbitrary error to `std::io::Error`.
#[allow(clippy::needless_pass_by_value)]
fn map_to_io_error(err: impl ToString) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
}

impl std::io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let file_size = self.size().map_err(map_to_io_error)?;
        let remaining_len = file_size.saturating_sub(self.pos);

        let reading_len = std::cmp::min(buf.len(), remaining_len);
        let selection = hdf5::Selection::new(self.pos..self.pos + reading_len);

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
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let file_size = self.size().map_err(map_to_io_error)?;
        let remaining_len = file_size.saturating_sub(self.pos);
        let increasing_len = buf.len().saturating_sub(remaining_len);

        let new_shape = [file_size.saturating_add(increasing_len)];
        self.hdf5_ds.resize(new_shape).map_err(map_to_io_error)?;

        let selection = hdf5::Selection::new(self.pos..self.pos + buf.len());

        self.hdf5_ds
            .write_slice(buf, selection)
            .map_err(map_to_io_error)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for File {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
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
            None => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid seek to a negative or overflowing position",
                ))
            },
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.pos.try_into().map_err(map_to_io_error)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, Write};

    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn file_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");

        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).expect("Failed to create a new package.");
        let group = package
            .as_group()
            .expect("Failed to get a root group from package.");

        let file_name = "test.txt";
        let file_content = b"file_content";

        assert!(group.dataset(file_name).is_err());
        let mut file =
            File::create(&group, file_name, file_content).expect("Failed to create a new file.");
        assert!(group.dataset(file_name).is_ok());

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read from file.");
        assert_eq!(buffer, file_content);

        file.seek(std::io::SeekFrom::Start(0))
            .expect("Failed to seek.");
        let new_file_content = b"new_file_content";
        file.write_all(new_file_content)
            .expect("Failed to write to file.");

        file.seek(std::io::SeekFrom::Start(0))
            .expect("Failed to seek.");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read from file.");
        assert_eq!(buffer, new_file_content);
    }
}
