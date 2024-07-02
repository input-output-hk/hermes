//! Implementation of the generalized Hermes package struct as a HDF5 package.

mod compression;
pub(crate) mod path;

use std::{collections::BTreeSet, io::Read, path::Path};

use path::PackagePath;

use self::compression::enable_compression;
use crate::{
    errors::Errors,
    packaging::{
        hash::{Blake2b256, Blake2b256Hasher},
        resources::ResourceTrait,
    },
};

/// Hermes package object.
/// Wrapper over HDF5 file object.
#[allow(dead_code)]
pub(crate) struct Package(hdf5::File);

#[allow(dead_code)]
impl Package {
    /// Create new `Package` instance from path.
    pub(crate) fn create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = hdf5::File::create(&path).map_err(|_| {
            anyhow::anyhow!(
                "Failed to create package. Package at {0} could be already exists.",
                path.as_ref().display()
            )
        })?;
        Ok(Self(package))
    }

    /// Open existing `Package` instance from path.
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = hdf5::File::open_rw(&path).map_err(|_| {
            anyhow::anyhow!(
                "Failed to open package. Package at {0} could not be found.",
                path.as_ref().display()
            )
        })?;
        Ok(Self(package))
    }

    /// Copy file to `Package`
    pub(crate) fn copy_file(&self, file: &impl ResourceTrait, path: &str) -> anyhow::Result<()> {
        copy_file_to_root(file, path, &self.0)
    }

    /// Copy dir to `Package` recursively.
    pub(crate) fn copy_dir_recursively(
        &self, dir: &impl ResourceTrait, name: &str,
    ) -> anyhow::Result<()> {
        copy_dir_recursively_to_package(dir, name, &self.0)
    }

    /// Remove file from `Package`.
    pub(crate) fn remove_file(&self, name: &str) -> anyhow::Result<()> {
        if self.0.dataset(name).is_ok() {
            self.0
                .unlink(name)
                .map_err(|_| anyhow::anyhow!("Failed to remove file '{name}' from package"))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("File '{name}' not found"))
        }
    }

    /// Remove directory from `Package`.
    #[allow(dead_code)]
    pub(crate) fn remove_dir(&self, name: &str) -> anyhow::Result<()> {
        if self.0.group(name).is_ok() {
            self.0
                .unlink(name)
                .map_err(|_| anyhow::anyhow!("Failed to remove directory '{name}' from package"))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Directory '{name}' not found"))
        }
    }

    /// Get package file reader if present.
    /// Return error if not possible get a byte reader.
    pub(crate) fn get_file_reader(&self, name: &str) -> anyhow::Result<Option<impl Read>> {
        get_package_file_reader(name, &self.0)
    }

    /// Calculates file hash, if present
    pub(crate) fn get_file_hash(&self, name: &str) -> anyhow::Result<Option<Blake2b256>> {
        let mut hasher = Blake2b256Hasher::new();

        Ok(calculate_package_file_hash(name, &self.0, &mut hasher)?.then(|| hasher.finalize()))
    }

    /// Calculates recursively package directory contents hash including file contents and
    /// file names.
    pub(crate) fn get_dir_hash(&self, name: &str) -> anyhow::Result<Option<Blake2b256>> {
        let mut hasher = Blake2b256Hasher::new();
        Ok(calculate_package_dir_hash(name, &self.0, &mut hasher)?.then(|| hasher.finalize()))
    }
}

/// Create package dirs recursively from path in provided root dir.
/// If some dir already exists it will be skipped.
fn create_dir_in_root(
    path: &PackagePath, root: &hdf5::Group,
) -> anyhow::Result<Option<hdf5::Group>> {
    let mut dir: Option<hdf5::Group> = None;

    let get_or_create = |dir: &hdf5::Group, name| {
        if let Ok(dir) = dir.group(name) {
            Ok(dir)
        } else {
            dir.create_group(name)
        }
    };

    for path_element in path.iter() {
        if let Some(dir) = &mut dir {
            *dir = get_or_create(dir, path_element)?;
        } else {
            dir = Some(get_or_create(root, path_element)?);
        }
    }

    Ok(dir)
}

/// Copy resource to hdf5 package into provided root dir.
fn copy_file_to_root(
    resource: &impl ResourceTrait, name: &str, root: &hdf5::Group,
) -> anyhow::Result<()> {
    let mut reader = resource.get_reader()?;
    let mut resource_data = Vec::new();
    reader.read_to_end(&mut resource_data)?;
    if resource_data.is_empty() {
        return Err(anyhow::anyhow!("Resource {} is empty", resource.name()?));
    }

    let ds_builder = root.new_dataset_builder();
    enable_compression(ds_builder)
        .with_data(&resource_data)
        .create(name)?;

    Ok(())
}

/// Copy resource dir to hdf5 package recursively.
fn copy_dir_recursively_to_package(
    resource: &impl ResourceTrait, name: &str, package: &hdf5::Group,
) -> anyhow::Result<()> {
    let package = package.create_group(name)?;

    let mut errors = Errors::new();
    for resource in resource.get_directory_content()? {
        if resource.is_dir() {
            copy_dir_recursively_to_package(&resource, &resource.name()?, &package)
                .unwrap_or_else(errors.get_add_err_fn());
        }
        if resource.is_file() {
            copy_file_to_root(&resource, &resource.name()?, &package)
                .unwrap_or_else(errors.get_add_err_fn());
        }
    }
    errors.return_result(())
}

/// Get package file reader if present.
/// Return error if not possible get a byte reader.
fn get_package_file_reader(name: &str, package: &hdf5::Group) -> anyhow::Result<Option<impl Read>> {
    let reader = package
        .dataset(name)
        .ok()
        .map(|ds| ds.as_byte_reader())
        .transpose()?;
    Ok(reader)
}

/// Buffer size for hash calculation.
/// 1024 * 1024 = 1MB.
const BUFFER_SIZE: usize = 1024 * 1024;

/// Calculates package file hash with the provided hasher.
/// Returns true if hash was calculated successfully and file is present.
#[allow(clippy::indexing_slicing)]
fn calculate_package_file_hash(
    name: &str, package: &hdf5::Group, hasher: &mut Blake2b256Hasher,
) -> anyhow::Result<bool> {
    if let Some(mut reader) = get_package_file_reader(name, package)? {
        let mut buf = vec![0; BUFFER_SIZE];

        loop {
            let len = reader.read(&mut buf)?;
            if len == 0 {
                break;
            }
            hasher.update(&buf[..len]);
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Calculates recursively package directory contents hash with the provided hasher
/// including file contents.
/// Returns true if hash was calculated successfully and file is present.
fn calculate_package_dir_hash(
    dir_name: &str, package: &hdf5::Group, hasher: &mut Blake2b256Hasher,
) -> anyhow::Result<bool> {
    let dir = package.group(dir_name).ok();

    if let Some(dir) = dir {
        // order all package directory content by names
        // to have consistent hash result not depending on the order.
        let content_names: BTreeSet<_> = dir.member_names()?.into_iter().collect();

        for name in content_names {
            hasher.update(name.as_bytes());

            // Returns false if file is not present, which means that it's a directory
            if !calculate_package_file_hash(&name, &dir, hasher)? {
                calculate_package_dir_hash(&name, &dir, hasher)?;
            }
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::packaging::resources::fs_resource::FsResource;

    #[test]
    fn create_dir_in_root_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).expect("Failed to create a new package.");

        let path = PackagePath::new("dir_1/dir_2/dir_3/dir_4");
        assert!(create_dir_in_root(&path, &package)
            .expect("Failed to create directories in package.")
            .is_some());

        let dir_1 = package.group("dir_1").expect("Failed to get group.");
        let dir_2 = dir_1.group("dir_2").expect("Failed to get group.");
        let dir_3 = dir_2.group("dir_3").expect("Failed to get group.");
        let _dir_4 = dir_3.group("dir_4").expect("Failed to get group.");

        assert!(create_dir_in_root(&path, &package)
            .expect("Failed to create directories in package.")
            .is_some());
    }

    #[test]
    fn copy_file_to_package_and_get_package_file_hash_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = Package::create(package_name).expect("Failed to create a new package.");

        let file_1_name = "file_1";
        let file_1 = tmp_dir.child(file_1_name);
        std::fs::write(&file_1, file_content).expect("Failed to create a file.");

        package
            .copy_file(&FsResource::new(file_1), file_1_name)
            .expect("Failed to copy file to package.");

        let mut file_1_reader = package
            .get_file_reader(file_1_name)
            .unwrap_or_default()
            .expect("Failed to get file reader.");

        let mut data = Vec::new();
        file_1_reader
            .read_to_end(&mut data)
            .expect("Failed to read file's data.");
        assert_eq!(data.as_slice(), file_content);

        let hash = package
            .get_file_hash(file_1_name)
            .expect("Failed to calculate file hash.")
            .expect("Failed to get file hash from package.");

        let expected_hash = Blake2b256::hash(file_content);
        assert_eq!(expected_hash, hash);

        // Remove file from package
        assert!(
            package.remove_dir(file_1_name).is_err(),
            "Cannot remove file from package using remove_dir."
        );
        assert!(package
            .get_file_hash(file_1_name)
            .expect("Failed to calculate file hash.")
            .is_some());
        package
            .remove_file(file_1_name)
            .expect("Failed to remove file from package.");
        assert!(package
            .get_file_hash(file_1_name)
            .expect("Failed to calculate file hash.")
            .is_none());
    }

    #[test]
    fn copy_dir_recursively_to_package_and_get_package_file_hash_test() {
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

        package
            .copy_dir_recursively(&FsResource::new(dir), dir_name)
            .expect("Failed to copy dir to package.");

        let root_group = package
            .0
            .group(dir_name)
            .expect("Failed to open root group.");
        assert!(get_package_file_reader(file_1_name, &root_group)
            .unwrap_or_default()
            .is_some());
        assert!(get_package_file_reader(file_2_name, &root_group)
            .unwrap_or_default()
            .is_some());

        let child_group = root_group
            .group(child_dir_name)
            .expect("Cannot open child group");
        assert!(get_package_file_reader(file_3_name, &child_group)
            .unwrap_or_default()
            .is_some());

        let hash = package
            .get_dir_hash(dir_name)
            .expect("Failed to calculate dir hash.")
            .expect("Failed to get dir hash from package.");

        let mut hasher = Blake2b256Hasher::new();
        hasher.update(child_dir_name.as_bytes());
        hasher.update(file_3_name.as_bytes());
        hasher.update(file_content);
        hasher.update(file_1_name.as_bytes());
        hasher.update(file_content);
        hasher.update(file_2_name.as_bytes());
        hasher.update(file_content);
        let expected_hash = hasher.finalize();

        assert_eq!(expected_hash, hash);

        // Remove directory from package
        assert!(
            package.remove_file(dir_name).is_err(),
            "Cannot remove dir from package using remove_file."
        );
        assert!(package
            .get_dir_hash(dir_name)
            .expect("Failed to calculate dir hash.")
            .is_some());
        package
            .remove_dir(dir_name)
            .expect("Failed to remove dir from package.");
        assert!(package
            .get_dir_hash(dir_name)
            .expect("Failed to calculate dir hash.")
            .is_none());
    }
}
