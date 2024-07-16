//! A Hermes HDF5 directory abstraction over the HDF5 Group object.

use std::io::Read;

use super::{
    compression::enable_compression,
    resources::{Hdf5Resource, ResourceTrait},
    File, Path,
};
use crate::errors::Errors;

/// Hermes HDF5 directory object, wrapper of `hdf5::Group`
#[derive(Clone, Debug)]
pub(crate) struct Dir(hdf5::Group);

impl Dir {
    /// Create new `Dir`.
    pub(crate) fn new(group: hdf5::Group) -> Self {
        Self(group)
    }

    /// Return dir `Path`.
    pub(crate) fn path(&self) -> Path {
        Path::from_str(&self.0.name())
    }

    /// Mount directory from the another HDF5 package to the provided path.
    pub(crate) fn mount_dir(&self, mounted_dir: &Dir, mut path: Path) -> anyhow::Result<()> {
        let link_name = path.pop_elem()?;
        let dir = self.get_dir(&path)?;

        let target_file_name = mounted_dir.0.filename();
        let target = mounted_dir.0.name();
        dir.0.link_external(
            target_file_name.as_str(),
            target.as_str(),
            link_name.as_str(),
        )?;
        Ok(())
    }

    /// Copy resource file to the provided path.
    pub(crate) fn copy_resource_file(
        &self, resource: &impl ResourceTrait, mut path: Path,
    ) -> anyhow::Result<()> {
        let file_name = path.pop_elem()?;

        let mut reader = resource.get_reader()?;
        let mut resource_data = Vec::new();
        reader.read_to_end(&mut resource_data)?;
        if resource_data.is_empty() {
            anyhow::bail!("Resource {} is empty", resource.to_string());
        }

        let dir = self.create_dir(&path)?;
        let ds_builder = dir.0.new_dataset_builder();
        enable_compression(ds_builder)
            .with_data(&resource_data)
            .create(file_name.as_str())?;

        Ok(())
    }

    /// Copy resource dir recursively to the provided path.
    pub(crate) fn copy_resource_dir(
        &self, resource: &impl ResourceTrait, path: &Path,
    ) -> anyhow::Result<()> {
        let dir = self.create_dir(path)?;

        let mut errors = Errors::new();
        for resource in resource.get_directory_content()? {
            if resource.is_dir() {
                dir.copy_resource_dir(&resource, &resource.name()?.into())
                    .unwrap_or_else(errors.get_add_err_fn());
            }
            if resource.is_file() {
                dir.copy_resource_file(&resource, resource.name()?.into())
                    .unwrap_or_else(errors.get_add_err_fn());
            }
        }
        errors.return_result(())
    }

    /// Copy other `Dir` recursively content to the current one.
    pub(crate) fn copy_dir(&self, dir: &Dir, path: &Path) -> anyhow::Result<()> {
        let resource = Hdf5Resource::Group(dir.0.clone());
        self.copy_resource_dir(&resource, path)?;
        Ok(())
    }

    /// Create dir recursively from path related to current dir.
    /// If some dir already exists it will be skipped, if some dir does not exist it will
    /// be created.
    /// If path is empty it will return cloned `Dir`.
    pub(crate) fn create_dir(&self, path: &Path) -> anyhow::Result<Self> {
        let mut dir = self.0.clone();
        for path_element in path.iter() {
            if let Ok(known_dir) = dir.group(path_element) {
                dir = known_dir;
            } else {
                dir = dir.create_group(path_element)?;
            }
        }
        Ok(Self(dir))
    }

    /// Remove file by the provided path.
    pub(crate) fn remove_file(&self, mut path: Path) -> anyhow::Result<()> {
        let file_name = path.pop_elem()?;
        let dir = self.get_dir(&path)?;

        if dir.0.dataset(file_name.as_str()).is_ok() {
            dir.0.unlink(file_name.as_str()).map_err(|_| {
                anyhow::anyhow!("Failed to remove file '{path}/{file_name}' from package")
            })?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("File '{path}/{file_name}' not found"))
        }
    }

    /// Remove directory by the provided path.
    #[allow(dead_code)]
    pub(crate) fn remove_dir(&self, mut path: Path) -> anyhow::Result<()> {
        let dir_name = path.pop_elem()?;
        let dir = self.get_dir(&path)?;

        if dir.0.group(dir_name.as_str()).is_ok() {
            dir.0.unlink(dir_name.as_str()).map_err(|_| {
                anyhow::anyhow!("Failed to remove directory '{path}/{dir_name}' from package")
            })?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Directory '{path}/{dir_name}' not found"))
        }
    }

    /// Get file if present from path.
    /// Return error if file does not exist by the provided path.
    pub(crate) fn get_file(&self, mut path: Path) -> anyhow::Result<File> {
        let file_name = path.pop_elem()?;
        let dir = self.get_dir(&path)?;
        dir.0
            .dataset(file_name.as_str())
            .map(File::new)
            .map_err(|_| anyhow::anyhow!("File {file_name}/{path} not found"))
    }

    /// Get all files from the provided path.
    /// If path is empty it will return all child files of the current one.
    pub(crate) fn get_files(&self, path: &Path) -> anyhow::Result<Vec<File>> {
        let dir = self.get_dir(path)?;
        Ok(dir.0.datasets()?.into_iter().map(File::new).collect())
    }

    /// Get dir by the provided path.
    /// Return error if dir does not exist by the provided path.
    /// If path is empty it will return cloned `Dir`.
    pub(crate) fn get_dir(&self, path: &Path) -> anyhow::Result<Self> {
        let mut dir = self.0.clone();
        for path_element in path.iter() {
            dir = dir
                .group(path_element)
                .map_err(|_| anyhow::anyhow!("Dir {path} not found"))?;
        }
        Ok(Self(dir))
    }

    /// Get all dirs from the provided path.
    /// If path is empty it will return all child dirs of the current one.
    pub(crate) fn get_dirs(&self, path: &Path) -> anyhow::Result<Vec<Self>> {
        let dir = self.get_dir(path)?;
        Ok(dir.0.groups()?.into_iter().map(Self).collect())
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::hdf5::resources::{BytesResource, FsResource};

    #[test]
    fn create_dir_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).expect("Failed to create a new package.");
        let dir = Dir::new(package.as_group().expect("Failed to create a root group."));

        let path = Path::from_str("dir_1/dir_2/dir_3/dir_4");
        dir.create_dir(&path)
            .expect("Failed to create directories in package.");

        assert!(dir.get_dir(&Path::from_str("dir_1")).is_ok());
        assert!(dir.get_dir(&Path::from_str("dir_1/dir_2")).is_ok());
        assert!(dir.get_dir(&Path::from_str("dir_1/dir_2/dir_3")).is_ok());
        assert!(dir.get_dir(&path).is_ok());
        assert!(dir.get_dir(&Path::from_str("not_created_dir")).is_err());

        dir.create_dir(&path)
            .expect("Failed to create directories in package.");
    }

    #[test]
    fn mount_external_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let package1 = hdf5::File::create(tmp_dir.child("test1.hdf5"))
            .expect("Failed to create a new package.");
        let dir1 = Dir::new(package1.as_group().expect("Failed to create a root group."));

        let package2 = hdf5::File::create(tmp_dir.child("test2.hdf5"))
            .expect("Failed to create a new package.");
        let dir2 = Dir::new(package2.as_group().expect("Failed to create a root group."));

        let child_dir_name = "child_dir";
        let child_dir_path = Path::from_str(child_dir_name);

        let child_dir = dir2
            .create_dir(&child_dir_path)
            .expect("Failed to create dir.");
        child_dir
            .create_dir(&child_dir_path)
            .expect("Failed to create dir.");

        let mounted_dir_name = "mounted_dir";
        assert!(dir1.get_dir(&mounted_dir_name.into()).is_err());
        assert_eq!(
            dir1.get_dirs(&"".into()).expect("Failed to get dirs").len(),
            0
        );

        dir1.mount_dir(&dir2, mounted_dir_name.into())
            .expect("Failed to mount external.");

        assert!(dir1.get_dir(&mounted_dir_name.into()).is_ok());
        assert_eq!(
            dir1.get_dirs(&"".into()).expect("Failed to get dirs").len(),
            1
        );
        assert!(dir1
            .get_dir(&format!("{mounted_dir_name}/{child_dir_name}").into())
            .is_ok());
        assert_eq!(
            dir1.get_dirs(&format!("{mounted_dir_name}/{child_dir_name}").into())
                .expect("Failed to get dirs")
                .len(),
            1
        );
        assert!(dir1
            .get_dir(&format!("{mounted_dir_name}/{child_dir_name}/{child_dir_name}").into())
            .is_ok());
    }

    #[test]
    fn copy_resource_file() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).expect("Failed to create a new package.");
        let dir = Dir::new(package.as_group().expect("Failed to create a root group."));

        let file_1_name = "file_1";
        let file_1 = tmp_dir.child(file_1_name);
        std::fs::write(&file_1, file_content).expect("Failed to create a file.");

        dir.copy_resource_file(&FsResource::new(file_1), file_1_name.into())
            .expect("Failed to copy file to package.");

        let file_1 = dir
            .get_file(file_1_name.into())
            .expect("Failed to get file.");

        let mut data = Vec::new();
        file_1
            .reader()
            .expect("Failed to get file reader.")
            .read_to_end(&mut data)
            .expect("Failed to read file's data.");
        assert_eq!(data.as_slice(), file_content);

        // Remove file from package
        assert!(
            dir.remove_dir(file_1_name.into()).is_err(),
            "Failed to remove file from package using remove_dir."
        );
        assert!(dir.remove_file(file_1_name.into()).is_ok());
        assert!(dir.get_file(file_1_name.into()).is_err());
    }

    #[test]
    fn copy_resource_dir_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");
        let file_content = "test".as_bytes();

        let package_name = tmp_dir.child("test.hdf5");
        let package = hdf5::File::create(package_name).expect("Failed to create a new package.");
        let dir = Dir::new(package.as_group().expect("Failed to create a root group."));

        let base_dir_name = "base_dir";
        let fs_base_dir = tmp_dir.child(base_dir_name);
        std::fs::create_dir(&fs_base_dir).expect("Failed to create directory.");

        let file_1_name = "file_1";
        let file_1 = fs_base_dir.join(file_1_name);
        std::fs::write(file_1, file_content).expect("Failed to create file_1 file.");

        let file_2_name = "file_2";
        let file_2 = fs_base_dir.join(file_2_name);
        std::fs::write(file_2, file_content).expect("Failed to create file_2 file.");

        let child_dir_name = "child_dir";
        let fs_child_dir = fs_base_dir.join(child_dir_name);
        std::fs::create_dir(&fs_child_dir).expect(
            "Failed to create child_dir
    directory.",
        );

        let file_3_name = "file_3";
        let file_3 = fs_child_dir.join(file_3_name);
        std::fs::write(file_3, file_content).expect("Failed to create file_3 file.");

        dir.copy_resource_dir(&FsResource::new(fs_base_dir), &base_dir_name.into())
            .expect("Failed to copy dir to package.");

        assert!(dir.get_dir(&base_dir_name.into()).is_ok());
        assert!(dir
            .get_file(Path::new(vec![base_dir_name.into(), file_1_name.into()]))
            .is_ok());
        assert!(dir
            .get_file(Path::new(vec![base_dir_name.into(), file_2_name.into()]))
            .is_ok());

        assert!(dir
            .get_dir(&Path::new(vec![
                base_dir_name.into(),
                child_dir_name.into()
            ]))
            .is_ok());
        assert!(dir
            .get_file(Path::new(vec![
                base_dir_name.into(),
                child_dir_name.into(),
                file_3_name.into()
            ]))
            .is_ok());

        // Remove directory from package
        assert!(
            dir.remove_file(base_dir_name.into()).is_err(),
            "Failed to remove dir from package using remove_file."
        );
        assert!(dir.remove_dir(base_dir_name.into()).is_ok());
        assert!(dir.get_dir(&base_dir_name.into()).is_err());
        assert!(dir
            .get_file(Path::new(vec![base_dir_name.into(), file_1_name.into()]))
            .is_err());
        assert!(dir
            .get_file(Path::new(vec![base_dir_name.into(), file_2_name.into()]))
            .is_err());
        assert!(dir
            .get_dir(&Path::new(vec![
                base_dir_name.into(),
                child_dir_name.into()
            ]))
            .is_err());
        assert!(dir
            .get_file(Path::new(vec![
                base_dir_name.into(),
                child_dir_name.into(),
                file_3_name.into()
            ]))
            .is_err());
    }

    #[test]
    fn copy_dir_test() {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir.");

        // prepare fist dir
        let content_name = "file_1";
        let content_data = b"test_content".to_vec();
        let content = BytesResource::new(content_name.to_string(), content_data);

        let package_1_name = tmp_dir.child("test1.hdf5");
        let package_1 =
            hdf5::File::create(package_1_name).expect("Failed to create a new package.");
        let dir_1 = Dir::new(
            package_1
                .as_group()
                .expect("Failed to create a root group."),
        );

        dir_1
            .copy_resource_file(&content, content_name.into())
            .expect("Failed to copy file to dir.");

        // prepare second dir even from another package
        let package_2_name = tmp_dir.child("test2.hdf5");
        let package_2 =
            hdf5::File::create(package_2_name).expect("Failed to create a new package.");
        let dir_2 = Dir::new(
            package_2
                .as_group()
                .expect("Failed to create a root group."),
        );

        // copy content from first dir from first package to second dir in second package
        assert!(dir_2.get_file(content_name.into()).is_err());
        dir_2
            .copy_dir(&dir_1, &"".into())
            .expect("Failed to copy package to package.");
        assert!(dir_2.get_file(content_name.into()).is_ok());
    }
}
