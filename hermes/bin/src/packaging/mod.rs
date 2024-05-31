//! Hermes packaging.

mod compression;
pub(crate) mod wasm_module;

use std::path::PathBuf;

use self::compression::enable_compression;
use crate::errors::Errors;

/// Get path name.
fn get_path_name<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<String> {
    Ok(path
        .as_ref()
        .file_name()
        .ok_or(anyhow::anyhow!("cannot get path name"))?
        .to_str()
        .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
        .to_string())
}

/// File not found error.
#[derive(thiserror::Error, Debug)]
#[error("File not found at {0}")]
pub(crate) struct FileNotFoundError(PathBuf);

/// File not found error.
#[derive(thiserror::Error, Debug)]
#[error("File is empty: {0}")]
pub(crate) struct EmptyFileError(PathBuf);

/// Copy file to hdf5 package.
fn copy_file_from_dir_to_package<P: AsRef<std::path::Path>>(
    file_path: P, package: &hdf5::Group,
) -> anyhow::Result<()> {
    let file_data = std::fs::read(&file_path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            anyhow::Error::new(FileNotFoundError(file_path.as_ref().into()))
        } else {
            anyhow::Error::new(err)
        }
    })?;

    if file_data.is_empty() {
        return Err(EmptyFileError(file_path.as_ref().into()).into());
    }

    let file_name = get_path_name(&file_path)?;

    enable_compression(package.new_dataset_builder())
        .with_data(&file_data)
        .create(file_name.as_str())?;

    Ok(())
}

/// Dir not found error.
#[derive(thiserror::Error, Debug)]
#[error("Dir {0} not found")]
pub(crate) struct DirNotFoundError(String);

/// Copy dir to hdf5 package recursively.
pub(crate) fn copy_dir_recursively_to_package<P: AsRef<std::path::Path>>(
    dir: P, package: &hdf5::Group,
) -> anyhow::Result<()> {
    let dir_name = get_path_name(&dir)?;
    let package = package.create_group(&dir_name)?;

    let entries = std::fs::read_dir(&dir).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            anyhow::Error::new(DirNotFoundError(dir_name))
        } else {
            anyhow::Error::new(err)
        }
    })?;

    let mut errors = Errors::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            copy_dir_recursively_to_package(&path, &package).unwrap_or_else(|err| {
                match err.downcast::<Errors>() {
                    Ok(errs) => errors.merge(errs),
                    Err(err) => errors.add_err(err),
                }
            });
        }
        if path.is_file() {
            copy_file_from_dir_to_package(&path, &package)
                .unwrap_or_else(|err| errors.add_err(err));
        }
    }
    errors.return_result(())
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

        let metadata_json = "metadata.json";
        let metadata_json_data = r#"{ "name": "Alex", "age": 25"}"#;
        let metadata_json_path = dir.path().join(metadata_json);
        std::fs::write(&metadata_json_path, metadata_json_data)
            .expect("Cannot write data to metadata.json");

        copy_file_from_dir_to_package(&metadata_json_path, &hdf5_file)
            .expect("Cannot copy metadata.json to hdf5 package");

        let metadata_json_ds = hdf5_file
            .dataset(metadata_json)
            .expect("cannot open metadata.json dataset");
        let data = String::from_utf8(
            metadata_json_ds
                .read_raw()
                .expect("cannot read metadata.json dataset"),
        )
        .expect("cannot parse metadata.json dataset");
        assert_eq!(data, metadata_json_data);
    }

    #[test]
    fn copy_file_to_package_errors_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");

        let file_name = "file.json";
        let file_path = dir.path().join(file_name);
        let err =
            copy_file_from_dir_to_package(&file_path, &hdf5_file).expect_err("Should return error");
        assert!(err.is::<FileNotFoundError>());

        std::fs::File::create(&file_path).expect("Cannot create file.json");
        let err =
            copy_file_from_dir_to_package(file_path, &hdf5_file).expect_err("Should return error");
        assert!(err.is::<EmptyFileError>());
    }

    #[test]
    fn copy_dir_recursively_to_package_test() {
        let dir = TempDir::new().expect("cannot create temp dir");
        let dir_name = get_path_name(dir.path()).expect("Cannot get root dir name");

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

        copy_dir_recursively_to_package(dir.path(), &hdf5_file)
            .expect("Cannot copy dir to hdf5 package");

        let root_group = hdf5_file.group(&dir_name).expect("Cannot open root group");
        assert!(root_group.dataset(file_1_name).is_ok());
        assert!(root_group.dataset(file_2_name).is_ok());

        let child_group = root_group
            .group(child_dir_name)
            .expect("Cannot open child group");
        assert!(child_group.dataset(file_3_name).is_ok());
    }
}
