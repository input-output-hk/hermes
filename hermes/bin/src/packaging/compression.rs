//! Enabling blosc compression for the hdf5 package.

use hdf5::filters::Blosc;

/// Compression algorithm.
const COMPRESSION_ALOGORITHM: Blosc = Blosc::ZStd;

/// Compression level.
const COMPRESSION_LEVEL: u8 = 9;

/// Minumum chunk size in kb, 8mb.
const MIN_CHUNK_SIZE: usize = 8000;

/// Minumum blosc threads.
const MIN_BLOSC_THREADS: u8 = 8;

/// Statically initialize blosc on the first call only once.
static BLOSC_THREADS_INIT: once_cell::sync::Lazy<()> = once_cell::sync::Lazy::new(blosc_init);

/// Initialize blosc.
fn blosc_init() {
    /// Default system core amount, any machine has at least 1 core.
    const DEFAULT_SYSTEM_CORE_NUM: u8 = 1;

    let core_num = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(DEFAULT_SYSTEM_CORE_NUM.into())
        .try_into()
        .unwrap_or(DEFAULT_SYSTEM_CORE_NUM);
    let blosc_threads = std::cmp::min(MIN_BLOSC_THREADS, core_num);
    hdf5::filters::blosc_set_nthreads(blosc_threads);
}

/// Enable blosc compression.
pub(crate) fn enable_compression(ds_builder: hdf5::DatasetBuilder) -> hdf5::DatasetBuilder {
    // Calling `blosc_init()` only once on the fist call
    let () = *BLOSC_THREADS_INIT;

    ds_builder
        .chunk_min_kb(MIN_CHUNK_SIZE)
        .blosc(COMPRESSION_ALOGORITHM, COMPRESSION_LEVEL, true)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use hdf5::File;
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn blosc_compression_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let compressed_file_name = dir.child("compressed_test.hdf5");
        let compressed_hdf5_file =
            File::create(&compressed_file_name).expect("cannot create HDF5 file");
        let uncompressed_file_name = dir.child("uncompressed_test.hdf5");
        let uncompressed_hdf5_file =
            File::create(&uncompressed_file_name).expect("cannot create HDF5 file");

        let large_json = "large.json";
        let large_json_data = std::fs::read(Path::new("src/packaging").join(large_json))
            .expect("cannot read large.json file");

        enable_compression(compressed_hdf5_file.new_dataset_builder())
            .with_data(&large_json_data)
            .create(large_json)
            .expect("Cannot create dataset for compressed hdf5 package");

        uncompressed_hdf5_file
            .new_dataset_builder()
            .with_data(&large_json_data)
            .create(large_json)
            .expect("Cannot create dataset for uncompressed hdf5 package");

        let compressed_size = std::fs::read(compressed_file_name)
            .expect("Cannot read hdf5 package bytes")
            .len();
        let uncompressed_size = std::fs::read(uncompressed_file_name)
            .expect("Cannot read hdf5 package bytes")
            .len();
        assert!(
            compressed_size < uncompressed_size,
            "compressed package size: {compressed_size}, uncompressed package size: {uncompressed_size}",
        );
    }
}
