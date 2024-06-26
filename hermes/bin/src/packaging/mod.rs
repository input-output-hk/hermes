//! Hermes packaging.

mod compression;
pub(crate) mod hash;
mod resources;
mod schema_validation;
pub(crate) mod sign;
pub(crate) mod wasm_module;

use std::{collections::BTreeSet, fmt::Display, io::Read, path::Path};

use resources::ResourceTrait;

use self::compression::enable_compression;
use crate::{
    errors::Errors,
    packaging::hash::{Blake2b256, Blake2b256Hasher},
};

/// File open and read error.
#[derive(thiserror::Error, Debug)]
struct FileError {
    /// File location.
    location: String,
    /// File open and read error.
    msg: Option<anyhow::Error>,
}
impl FileError {
    /// Create a new `FileError` instance from a string location.
    fn from_string(location: String, msg: Option<anyhow::Error>) -> Self {
        Self { location, msg }
    }

    /// Create a new `FileError` instance from a path location.
    fn from_path<P: AsRef<Path>>(path: P, msg: Option<anyhow::Error>) -> Self {
        Self {
            location: path.as_ref().display().to_string(),
            msg,
        }
    }
}
impl Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = format!("Cannot open or read file at {0}", self.location);
        let err = self
            .msg
            .as_ref()
            .map(|msg| format!(":\n{msg}"))
            .unwrap_or_default();
        writeln!(f, "{msg}{err}",)
    }
}

/// Copy resource to hdf5 package.
fn copy_resource_to_package(
    resource: &impl ResourceTrait, name: &str, package: &hdf5::Group,
) -> anyhow::Result<()> {
    let mut reader = resource.get_reader()?;
    let mut resource_data = Vec::new();
    reader.read_to_end(&mut resource_data)?;
    if resource_data.is_empty() {
        return Err(anyhow::anyhow!("Resource {} is empty", resource.name()?));
    }

    enable_compression(package.new_dataset_builder())
        .with_data(&resource_data)
        .create(name)?;

    Ok(())
}

/// Copy resource dir to hdf5 package recursively.
fn copy_resource_dir_recursively_to_package(
    resource: &impl ResourceTrait, name: &str, package: &hdf5::Group,
) -> anyhow::Result<()> {
    let package = package.create_group(name)?;

    let mut errors = Errors::new();
    for resource in resource.get_directory_content()? {
        if resource.is_dir() {
            copy_resource_dir_recursively_to_package(&resource, &resource.name()?, &package)
                .unwrap_or_else(|err| {
                    match err.downcast::<Errors>() {
                        Ok(errs) => errors.merge(errs),
                        Err(err) => errors.add_err(err),
                    }
                });
        }
        if resource.is_file() {
            copy_resource_to_package(&resource, &resource.name()?, &package)
                .unwrap_or_else(|err| errors.add_err(err));
        }
    }
    errors.return_result(())
}

/// Remove file from the package of.
fn remove_file_from_package(name: &str, package: &hdf5::Group) -> anyhow::Result<()> {
    if let Ok(_) = package.dataset(name) {
        package.unlink(name)?;
    }
    Ok(())
}

/// Remove directory from the package.
fn remove_dir_from_package(name: &str, package: &hdf5::Group) -> anyhow::Result<()> {
    if let Ok(_) = package.group(name) {
        package.unlink(name)?;
    }
    Ok(())
}

/// Get package file reader if present.
/// Return error if not possible get a byte reader.
fn get_package_file_reader(name: &str, package: &hdf5::Group) -> anyhow::Result<Option<impl Read>> {
    if let Ok(ds) = package.dataset(name) {
        Ok(Some(ds.as_byte_reader()?))
    } else {
        Ok(None)
    }
}

/// Buffer size for hash calculation.
/// 1024 * 1024 = 1MB.
const BUFFER_SIZE: usize = 1024 * 1024;

/// Calculates package file hash.
fn get_package_file_hash(name: &str, package: &hdf5::Group) -> anyhow::Result<Option<Blake2b256>> {
    let mut hasher = Blake2b256Hasher::new();
    Ok(calculate_package_file_hash(name, package, &mut hasher)?.then(|| hasher.finalize()))
}

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

/// Calculates recursively package directory contents hash including file contents and
/// file names.
fn get_package_dir_hash(name: &str, package: &hdf5::Group) -> anyhow::Result<Option<Blake2b256>> {
    let mut hasher = Blake2b256Hasher::new();
    Ok(calculate_package_dir_hash(name, package, &mut hasher)?.then(|| hasher.finalize()))
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
    use hdf5::File;
    use resources::fs_resource::FsResource;
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn copy_file_to_package_and_get_package_file_hash_test() {
        let tmp_dir = TempDir::new().expect("cannot create temp dir");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = File::create(package_name).expect("cannot create HDF5 file");

        let file_1_name = "file_1";
        let file_1 = tmp_dir.child(file_1_name);
        std::fs::write(&file_1, file_content).expect("Cannot create file_1 file");

        copy_resource_to_package(&FsResource::new(file_1), file_1_name, &package)
            .expect("Cannot copy file_1 to package");

        let mut file_1_reader = get_package_file_reader(file_1_name, &package)
            .unwrap_or_default()
            .expect("Cannot get metadata.json reader");

        let mut data = String::new();
        file_1_reader
            .read_to_string(&mut data)
            .expect("cannot parse metadata.json dataset");
        assert_eq!(data, "test");

        let hash = get_package_file_hash(file_1_name, &package)
            .expect("Package file hash calculation failed")
            .expect("Cannot get file_1 hash from package");

        let expected_hash = Blake2b256::hash(file_content);
        assert_eq!(expected_hash, hash);

        // Remove file from package
        remove_dir_from_package(file_1_name, &package).expect("Cannot remove file from package");
        assert!(get_package_file_hash(file_1_name, &package)
            .expect("Package file hash calculation failed")
            .is_some());
        remove_file_from_package(file_1_name, &package).expect("Cannot remove file from package");
        assert!(get_package_file_hash(file_1_name, &package)
            .expect("Package file hash calculation failed")
            .is_none());
    }

    #[test]
    fn copy_dir_recursively_to_package_and_get_package_file_hash_test() {
        let tmp_dir = TempDir::new().expect("cannot create temp dir");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = File::create(package_name).expect("cannot create HDF5 package");

        let dir_name = "dir";
        let dir = tmp_dir.child(dir_name);
        std::fs::create_dir(&dir).expect("Cannot create directory");

        let file_1_name = "file_1";
        let file_1 = dir.join(file_1_name);
        std::fs::write(file_1, file_content).expect("Cannot create file_1 file");

        let file_2_name = "file_2";
        let file_2 = dir.join(file_2_name);
        std::fs::write(file_2, file_content).expect("Cannot create file_2 file");

        let child_dir_name = "child_dir";
        let child_dir = dir.join(child_dir_name);
        std::fs::create_dir(&child_dir).expect("Cannot create child_dir directory");

        let file_3_name = "file_3";
        let file_3 = child_dir.join(file_3_name);
        std::fs::write(file_3, file_content).expect("Cannot create file_3 file");

        copy_resource_dir_recursively_to_package(&FsResource::new(dir), dir_name, &package)
            .expect("Cannot copy dir to package");

        let root_group = package.group(dir_name).expect("Cannot open root group");
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

        let hash = get_package_dir_hash(dir_name, &package)
            .expect("Package dir hash calculation failed")
            .expect("Cannot get dir hash from package");

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
        remove_file_from_package(dir_name, &package).expect("Cannot remove file from package");
        assert!(
            get_package_dir_hash(dir_name, &package)
                .expect("Package dir hash calculation failed")
                .is_some(),
            "remove_file_from_package"
        );
        remove_dir_from_package(dir_name, &package).expect("Cannot remove dir from package");
        assert!(get_package_dir_hash(dir_name, &package)
            .expect("Package dir hash calculation failed")
            .is_none());
    }
}
