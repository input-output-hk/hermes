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

        let amt = std::cmp::min(buf.len(), remaining_len);
        let selection = hdf5::Selection::new(self.pos..self.pos + amt);

        let data = self
            .hdf5_ds
            .read_slice_1d::<u8, _>(selection)
            .map_err(map_to_io_error)?;

        let data_slice = data
            .as_slice()
            .ok_or(map_to_io_error("Failed to read data."))?;
        #[allow(clippy::indexing_slicing)]
        buf[..amt].copy_from_slice(data_slice);

        self.pos = self.pos.saturating_add(amt);
        Ok(amt)
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
    use std::io::Read;

    use temp_dir::TempDir;

    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn file_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");

        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).expect("Failed to create a new package.");
        let group = package
            .as_group()
            .expect("Failed to get a root group from package.");

        let file_name = "test.txt";
        let file_content = b"test1";

        assert!(group.dataset(file_name).is_err());
        let mut file =
            File::create(&group, file_name, file_content).expect("Failed to create a new file.");
        assert!(group.dataset(file_name).is_ok());

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read from file.");
        assert_eq!(buffer, file_content);

        // let dataspace = file.0.space().unwrap().extents().unwrap();
        // println!("{}", dataspace.is_resizable());

        // let writer = file.hdf5_ds.as_writer();
        // writer.write(b"test_2").expect("Failed to write to file.");

        // let mut buffer = Vec::new();
        // file.reader()
        //     .expect("Failed to read from file.")
        //     .read_to_end(&mut buffer)
        //     .expect("Failed to read from file.");
        // assert_eq!(buffer, b"test2");
    }
}
