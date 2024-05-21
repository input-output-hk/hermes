//! Hermes packaging.

use crate::errors::Errors;

pub(crate) mod wasm_module;

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
#[error("File {0} not found")]
pub(crate) struct FileNotFoundError(String);

/// Copy file to hdf5 package.
fn copy_file_from_dir_to_package<P: AsRef<std::path::Path>>(
    dir: P, file_name: &str, package: &hdf5::Group,
) -> anyhow::Result<()> {
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
            let name = get_path_name(&path)?;
            copy_file_from_dir_to_package(&dir, &name, &package)
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
    fn copy_dir_recursively_to_package_test() {
        let dir = TempDir::new().expect("cannot create temp dir");
        let dir_name = get_path_name(dir.path()).expect("Cannot get root dir name");

        let file_name = dir.child("test.hdf5");
        let hdf5_file = File::create(file_name).expect("cannot create HDF5 file");

        let file_1_name = "file_1";
        let file_1 = dir.child(file_1_name);
        std::fs::File::create(file_1).expect("Cannot create file_1 file");

        let file_2_name = "file_2_name";
        let file_2 = dir.child(file_2_name);
        std::fs::File::create(file_2).expect("Cannot create file_2 file");

        let child_dir_name = "child_dir";
        let child_dir = dir.child(child_dir_name);
        std::fs::create_dir(&child_dir).expect("Cannot create child_dir directory");

        let file_3_name = "file_3";
        let file_3 = child_dir.join(file_3_name);
        std::fs::File::create(file_3).expect("Cannot create file_3 file");

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
