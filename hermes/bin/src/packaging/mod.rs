//! Hermes packaging.

mod compression;
mod resources;
pub(crate) mod wasm_module;

use std::{collections::BTreeSet, io::Read};

use resources::ResourceTrait;

use self::compression::enable_compression;
use crate::{
    errors::Errors,
    sign::hash::{Blake2b256, Blake2b256Hasher},
};

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
    name: &str, package: &hdf5::Group, hasher: &mut Blake2b256Hasher,
) -> anyhow::Result<bool> {
    let dir = package.group(name).ok();

    if let Some(dir) = dir {
        // order all package directory content by names
        // to have consistent hash result not depending on the order.
        let content_names: BTreeSet<_> = dir.member_names()?.into_iter().collect();

        for name in content_names {
            hasher.update(name.as_bytes());

            // Returns false if file is not present, which means that it's a directory
            if !calculate_package_file_hash(&name, package, hasher)? {
                calculate_package_dir_hash(&name, package, hasher)?;
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

        let package_name = tmp_dir.child("test.hdf5");
        let package = File::create(package_name).expect("cannot create HDF5 file");

        let file_1_name = "file_1";
        let file_1 = tmp_dir.child(file_1_name);
        std::fs::write(&file_1, b"test").expect("Cannot create file_1 file");

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

        assert_eq!(
            "928b20366943e2afd11ebc0eae2e53a93bf177a4fcf35bcc64d503704e65e202",
            hash.to_hex()
        );
    }

    #[test]
    fn copy_dir_recursively_to_package_and_get_package_file_hash_test() {
        let tmp_dir = TempDir::new().expect("cannot create temp dir");

        let package_name = tmp_dir.child("test.hdf5");
        let package = File::create(package_name).expect("cannot create HDF5 package");

        let dir_name = "dir";
        let dir = tmp_dir.child(dir_name);

        let file_1_name = "file_1";
        let file_1 = dir.join(file_1_name);
        std::fs::write(file_1, [0, 1, 2]).expect("Cannot create file_1 file");

        let file_2_name = "file_2_name";
        let file_2 = dir.join(file_2_name);
        std::fs::write(file_2, [0, 1, 2]).expect("Cannot create file_2 file");

        let child_dir_name = "child_dir";
        let child_dir = dir.join(child_dir_name);
        std::fs::create_dir(&child_dir).expect("Cannot create child_dir directory");

        let file_3_name = "file_3";
        let file_3 = child_dir.join(file_3_name);
        std::fs::write(file_3, [0, 1, 2]).expect("Cannot create file_3 file");

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
            .expect("Package file hash calculation failed")
            .expect("Cannot get file_1 hash from package");

        assert_eq!(
            "e310b80433f172956d3df0c5cf53ed104eefc05aeaa5f9c1ea9202a8bbf471b1",
            hash.to_hex()
        );
    }
}
