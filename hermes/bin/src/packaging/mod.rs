//! Hermes packaging.

mod compression;
mod resources;
mod schema_validation;
pub(crate) mod wasm_module;

use std::io::Read;

use resources::ResourceTrait;

use self::compression::enable_compression;
use crate::errors::Errors;

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

/// Copy dir to hdf5 package recursively.
fn copy_dir_recursively_to_package(
    resource: &impl ResourceTrait, name: &str, package: &hdf5::Group,
) -> anyhow::Result<()> {
    let package = package.create_group(name)?;

    let mut errors = Errors::new();
    for resource in resource.get_directory_content()? {
        if resource.is_dir() {
            copy_dir_recursively_to_package(&resource, &resource.name()?, &package).unwrap_or_else(
                |err| {
                    match err.downcast::<Errors>() {
                        Ok(errs) => errors.merge(errs),
                        Err(err) => errors.add_err(err),
                    }
                },
            );
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

#[cfg(test)]
mod tests {
    use hdf5::File;
    use resources::fs_resource::FsResource;
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn copy_file_to_package_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");

        let metadata_json = "metadata.json";
        let metadata_json_data = r#"{ "name": "Alex", "age": 25"}"#;
        let metadata_json_path = dir.path().join(metadata_json);
        std::fs::write(&metadata_json_path, metadata_json_data)
            .expect("Cannot write data to metadata.json");

        copy_resource_to_package(
            &FsResource::new(metadata_json_path.clone()),
            metadata_json,
            &hdf5_file,
        )
        .expect("Cannot copy metadata.json to hdf5 package");

        let mut metadata_json_reader = get_package_file_reader(metadata_json, &hdf5_file)
            .unwrap_or_default()
            .expect("Cannot get metadata.json reader");

        let mut data = String::new();
        metadata_json_reader
            .read_to_string(&mut data)
            .expect("cannot parse metadata.json dataset");
        assert_eq!(data, metadata_json_data);
    }

    #[test]
    fn copy_dir_recursively_to_package_test() {
        let dir = TempDir::new().expect("cannot create temp dir");
        let dir_resource = FsResource::new(dir.path());
        let dir_name = dir_resource.name().expect("Cannot get root dir name");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");

        let file_1_name = "file_1";
        let file_1 = dir.child(file_1_name);
        std::fs::write(file_1, [0, 1, 2]).expect("Cannot create file_1 file");

        let file_2_name = "file_2_name";
        let file_2 = dir.child(file_2_name);
        std::fs::write(file_2, [0, 1, 2]).expect("Cannot create file_2 file");

        let child_dir_name = "child_dir";
        let child_dir = dir.child(child_dir_name);
        std::fs::create_dir(&child_dir).expect("Cannot create child_dir directory");

        let file_3_name = "file_3";
        let file_3 = child_dir.join(file_3_name);
        std::fs::write(file_3, [0, 1, 2]).expect("Cannot create file_3 file");

        copy_dir_recursively_to_package(&dir_resource, &dir_name, &hdf5_file)
            .expect("Cannot copy dir to hdf5 package");

        let root_group = hdf5_file.group(&dir_name).expect("Cannot open root group");
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
    }
}
