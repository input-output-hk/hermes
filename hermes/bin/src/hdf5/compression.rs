//! Enabling blosc compression for the hdf5 package.

// cspell: words nthreads decompressor

use hdf5::filters::Blosc;

/// Compression algorithm.
const COMPRESSION_ALGORITHM: Blosc = Blosc::ZStd;

/// Compression level.
/// zstd compressor of 9 to get a good balance of compression ration and decompression
/// speed.
const COMPRESSION_LEVEL: u8 = 9;

/// Minimum chunk size in kb, 8mb.
/// 1mb per compressor/decompressor thread
const MIN_CHUNK_SIZE: usize = 8000;

/// Minimum blosc threads.
/// 8 threads being a minimum likely standard across most computers.
const MIN_BLOSC_THREADS: u8 = 8;

/// Statically initialize blosc on the first call only once.
static BLOSC_THREADS_INIT: once_cell::sync::Lazy<()> = once_cell::sync::Lazy::new(blosc_init);

/// Initialize blosc.
fn blosc_init() {
    let num_cpus = num_cpus::get().try_into().unwrap_or(MIN_BLOSC_THREADS);

    let blosc_threads = std::cmp::min(MIN_BLOSC_THREADS, num_cpus);
    hdf5::filters::blosc_set_nthreads(blosc_threads);
}

/// Enable blosc compression.
pub(crate) fn enable_compression(ds_builder: hdf5::DatasetBuilder) -> hdf5::DatasetBuilder {
    // Calling `blosc_init()` only once on the fist call
    let () = *BLOSC_THREADS_INIT;

    ds_builder
        .chunk_min_kb(MIN_CHUNK_SIZE)
        .blosc(COMPRESSION_ALGORITHM, COMPRESSION_LEVEL, true)
}

#[cfg(all(test, debug_assertions))]
#[allow(unused_imports)]
mod tests {
    use std::path::Path;

    use hdf5::File;
    use temp_dir::TempDir;

    use super::*;

    fn copy_dir_recursively_to_package<P: AsRef<std::path::Path>>(
        dir: P,
        package: &hdf5::Group,
        with_compression: bool,
    ) -> anyhow::Result<()> {
        let dir_name = dir
            .as_ref()
            .file_name()
            .ok_or(anyhow::anyhow!("cannot get path name"))?
            .to_str()
            .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
            .to_string();
        let package = package.create_group(&dir_name)?;

        for dir_entry in std::fs::read_dir(dir)? {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            if path.is_dir() {
                copy_dir_recursively_to_package(&path, &package, with_compression)?;
            }
            if path.is_file() {
                let file_data = std::fs::read(&path)?;
                let file_name = path
                    .file_name()
                    .ok_or(anyhow::anyhow!("cannot get path name"))?
                    .to_str()
                    .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
                    .to_string();
                let ds = if with_compression {
                    //enable_compression(package.new_dataset_builder())
                    package.new_dataset_builder()
                } else {
                    package.new_dataset_builder()
                };
                ds.with_data(&file_data).create(file_name.as_str())?;
            }
        }
        Ok(())
    }

    /// Test of the blosc compression.
    /// Copies whole `src` directory into hdf5 packages with blosc compression and without
    /// it and checks the difference.
    #[test]
    fn blosc_compression_test() {
        let dir = TempDir::new().unwrap();

        let compressed_file_name = dir.child("compressed_test.hdf5");
        let compressed_hdf5_file = File::create(&compressed_file_name).unwrap();
        let uncompressed_file_name = dir.child("uncompressed_test.hdf5");
        let uncompressed_hdf5_file = File::create(&uncompressed_file_name).unwrap();

        let src_dir_path = Path::new("src");
        copy_dir_recursively_to_package(src_dir_path, &compressed_hdf5_file, true).unwrap();
        copy_dir_recursively_to_package(src_dir_path, &uncompressed_hdf5_file, false).unwrap();

        let compressed_size = std::fs::read(compressed_file_name).unwrap().len();
        let uncompressed_size = std::fs::read(uncompressed_file_name).unwrap().len();
        assert!(
            compressed_size < uncompressed_size,
            "compressed package size: {compressed_size}, uncompressed package size: {uncompressed_size}",
        );
    }
}
