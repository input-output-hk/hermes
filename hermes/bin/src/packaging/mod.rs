//! Hermes packaging.

use std::ops::Deref;

#[allow(dead_code, missing_docs, clippy::missing_docs_in_private_items)]
pub(crate) mod wasm_module;

/// File not found error.
#[derive(thiserror::Error, Debug)]
#[error("File {0} not found")]
pub(crate) struct FileNotFoundError(String);

/// Copy file to hdf5 package.
fn copy_file_from_dir_to_package<P, Package>(
    dir: P, file_name: &str, package: &Package,
) -> anyhow::Result<()>
where
    P: AsRef<std::path::Path>,
    Package: Deref<Target = hdf5::Group>,
{
    let file_path = dir.as_ref().join(file_name);

    let file_data = std::fs::read(file_path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            anyhow::Error::new(FileNotFoundError(file_name.to_string()))
        } else {
            anyhow::Error::new(err)
        }
    })?;

    package
        .new_dataset_builder()
        .with_data(&file_data)
        .create(file_name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use hdf5::File;
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn copy_file_to_package_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");

        let metada_json = "metadata.json";
        let metada_json_data = r#"{ "name": "Alex", "age": 25"}"#;
        std::fs::write(dir.child(metada_json), metada_json_data)
            .expect("Cannot write data to metadata.json");

        copy_file_from_dir_to_package(dir.path(), metada_json, &hdf5_file)
            .expect("Cannot copy metadata.json to hdf5 package");

        let metada_json = hdf5_file
            .dataset(metada_json)
            .expect("cannot open metadata.json dataset");
        let data = String::from_utf8(
            metada_json
                .read_raw()
                .expect("cannot read metadata.json dataset"),
        )
        .expect("cannot parse metadata.json dataset");
        assert_eq!(data, metada_json_data);
    }

    #[test]
    fn copy_file_to_package_not_found_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");

        let metada_json = "metadata.json";

        let err = copy_file_from_dir_to_package(dir.path(), metada_json, &hdf5_file)
            .expect_err("Should return error");

        assert!(err.is::<FileNotFoundError>());
    }

    #[test]
    fn hdf5_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = dir.child("test.hdf5");
        let root_group = File::create(file_name).expect("cannot create HDF5 file");

        println!("HDF5 size: {}", root_group.size());

        println!(
            "hdf5 group: {root_group:?}, members: {:?}",
            root_group.member_names().expect("cannot get member names")
        );

        let metadata_json = "metadata.json";
        let metada_json_data = r#"{ "name": "Alex", "age": 25"}"#;
        root_group
            .new_dataset_builder()
            .with_data(metada_json_data)
            .create(metadata_json)
            .expect("cannot create metadata.json");

        println!("HDF5 size: {}", root_group.size());
    }
}
