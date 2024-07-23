//! Implementation of the generalized Hermes package struct as a HDF5 package.

use std::{collections::BTreeMap, io::Read, ops::Deref};

use crate::{
    hdf5::{Dir, File, Path},
    packaging::hash::{Blake2b256, Blake2b256Hasher},
};

/// Hermes package object.
/// Wrapper over HDF5 file object.
pub(crate) struct Package(Dir);

impl Deref for Package {
    type Target = Dir;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Package {
    /// Create a new `Package` instance mounting a `dir` as a root for this package.
    pub(crate) fn mount(dir: Dir) -> Self {
        Self(dir)
    }

    /// Create new `Package` instance from path.
    pub(crate) fn create<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let hdf5_file = hdf5::File::create(&path).map_err(|_| {
            anyhow::anyhow!(
                "Failed to create package. Package at {0} could be already exists.",
                path.as_ref().display()
            )
        })?;
        Ok(Self(Dir::new(hdf5_file.as_group()?)))
    }

    /// Open existing `Package` instance from path.
    pub(crate) fn open<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let hdf5_file = hdf5::File::open_rw(&path).map_err(|_| {
            anyhow::anyhow!(
                "Failed to open package. Package at {0} could not be found.",
                path.as_ref().display()
            )
        })?;
        Ok(Self(Dir::new(hdf5_file.as_group()?)))
    }

    /// Calculates file hash, if present
    pub(crate) fn calculate_file_hash(&self, path: Path) -> anyhow::Result<Option<Blake2b256>> {
        let mut hasher = Blake2b256Hasher::new();
        let Ok(mut file) = self.0.get_file(path) else {
            return Ok(None);
        };
        calculate_file_hash(&mut file, &mut hasher)?;
        Ok(Some(hasher.finalize()))
    }

    /// Calculates recursively package directory contents hash including file contents and
    /// file names.
    pub(crate) fn calculate_dir_hash(&self, path: &Path) -> anyhow::Result<Option<Blake2b256>> {
        let mut hasher = Blake2b256Hasher::new();
        let Ok(dir) = self.0.get_dir(path) else {
            return Ok(None);
        };
        calculate_dir_hash(&dir, &mut hasher)?;
        Ok(Some(hasher.finalize()))
    }
}

/// Buffer size for hash calculation.
/// 1024 * 1024 = 1MB.
const BUFFER_SIZE: usize = 1024 * 1024;

/// Calculates file hash with the provided hasher.
#[allow(clippy::indexing_slicing)]
fn calculate_file_hash(file: &mut File, hasher: &mut Blake2b256Hasher) -> anyhow::Result<()> {
    let mut buf = vec![0; BUFFER_SIZE];

    loop {
        let len = file.read(&mut buf)?;
        if len == 0 {
            break;
        }
        hasher.update(&buf[..len]);
    }

    Ok(())
}

/// Calculates recursively directory contents hash with the provided hasher
/// including file contents.
fn calculate_dir_hash(dir: &Dir, hasher: &mut Blake2b256Hasher) -> anyhow::Result<()> {
    // order all package directory content by names
    // to have consistent hash result not depending on the order.
    let files: BTreeMap<_, _> = dir
        .get_files(&Path::default())?
        .into_iter()
        .map(|file| (file.name(), file))
        .collect();
    let dirs: BTreeMap<_, _> = dir
        .get_dirs(&Path::default())?
        .into_iter()
        .map(|dir| (dir.name(), dir))
        .collect();
    for (path_str, mut file) in files {
        hasher.update(path_str.as_bytes());
        calculate_file_hash(&mut file, hasher)?;
    }
    for (path_str, dir) in dirs {
        hasher.update(path_str.as_bytes());
        calculate_dir_hash(&dir, hasher)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::hdf5::resources::FsResource;

    #[test]
    fn calculate_file_hash_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = Package::create(package_name).expect("Failed to create a new package.");

        let file_1_name = "file_1";
        let file_1 = tmp_dir.child(file_1_name);
        std::fs::write(&file_1, file_content).expect("Failed to create a file.");

        package
            .copy_resource_file(&FsResource::new(file_1), file_1_name.into())
            .expect("Failed to copy file to package.");

        let hash = package
            .calculate_file_hash(file_1_name.into())
            .expect("Failed to calculate file hash.")
            .expect("Failed to get file hash from package.");

        let expected_hash = Blake2b256::hash(file_content);
        assert_eq!(expected_hash, hash);
    }

    #[test]
    fn calculate_dir_hash_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = Package::create(package_name).expect("Failed to create a new package.");

        let dir_name = "dir";
        let dir = tmp_dir.child(dir_name);
        std::fs::create_dir(&dir).expect("Failed to create directory.");

        let file_1_name = "file_1";
        let file_1 = dir.join(file_1_name);
        std::fs::write(file_1, file_content).expect("Failed to create file_1 file.");

        let file_2_name = "file_2";
        let file_2 = dir.join(file_2_name);
        std::fs::write(file_2, file_content).expect("Failed to create file_2 file.");

        let child_dir_name = "child_dir";
        let child_dir = dir.join(child_dir_name);
        std::fs::create_dir(&child_dir).expect("Failed to create child_dir directory.");

        let file_3_name = "file_3";
        let file_3 = child_dir.join(file_3_name);
        std::fs::write(file_3, file_content).expect("Failed to create file_3 file.");

        let root_dir_name = "root_dir";
        let root_dir = package
            .create_dir(root_dir_name.into())
            .expect("Failed to create dir.");
        root_dir
            .create_dir(dir_name.into())
            .expect("Failed to create dir.");
        root_dir
            .copy_resource_dir(&FsResource::new(dir), &dir_name.into())
            .expect("Failed to copy dir to package.");

        let hash = package
            .calculate_dir_hash(&format!("{root_dir_name}/{dir_name}").into())
            .expect("Failed to calculate dir hash.")
            .expect("Failed to get dir hash from package.");

        let mut hasher = Blake2b256Hasher::new();
        hasher.update(file_1_name.as_bytes());
        hasher.update(file_content);
        hasher.update(file_2_name.as_bytes());
        hasher.update(file_content);
        hasher.update(child_dir_name.as_bytes());
        hasher.update(file_3_name.as_bytes());
        hasher.update(file_content);
        let expected_hash = hasher.finalize();

        assert_eq!(expected_hash, hash);
    }
}
