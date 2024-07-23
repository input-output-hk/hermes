//! HDF5 based resource implementation.

use std::{
    fmt::{Debug, Display},
    io::Read,
};

use super::{
    super::{Dir, File, Path},
    ResourceTrait,
};

/// HDF5 resource struct.
#[derive(Debug, Clone)]
pub(crate) enum Hdf5Resource {
    /// HDF5 group.
    Dir(Dir),
    /// HDF5 dataset.
    File(File),
}

impl Display for Hdf5Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dir(dir) => dir.fmt(f),
            Self::File(file) => file.fmt(f),
        }
    }
}

impl ResourceTrait for Hdf5Resource {
    fn name(&self) -> anyhow::Result<String> {
        match self {
            Self::Dir(dir) => Ok(dir.name()),
            Self::File(file) => Ok(file.name()),
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            Self::Dir(_) => true,
            Self::File(_) => false,
        }
    }

    fn is_file(&self) -> bool {
        match self {
            Self::Dir(_) => false,
            Self::File(_) => true,
        }
    }

    fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        match self {
            Self::Dir(_) => Err(anyhow::anyhow!("Hdf5Resource is not a file")),
            Self::File(file) => Ok(file.clone()),
        }
    }

    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>> {
        match self {
            Self::File(_) => anyhow::bail!("Hdf5Resource is not a directory"),
            Self::Dir(dir) => {
                let f_iter = dir.get_files(&Path::default())?.into_iter().map(Self::File);
                let d_iter = dir.get_dirs(&Path::default())?.into_iter().map(Self::Dir);
                Ok(f_iter.chain(d_iter).collect())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn hdf5_resource_test() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let package =
            hdf5::File::create(dir.child("test.hdf5")).expect("Failed to create a hdf5 package");
        let root_dir = Dir::new(package.as_group().expect("Failed to make a group"));

        let dir_1_name = "dir_1";
        let dir_1 = root_dir
            .create_dir(dir_1_name.into())
            .expect("Failed to create a dir");

        let dir_2_name = "dir_2";
        let dir_2 = dir_1
            .create_dir(dir_2_name.into())
            .expect("Failed to create a dir");

        let file_1_name = "file_1";
        dir_2
            .create_file(file_1_name.into())
            .expect("Failed to create a file");

        let resource = Hdf5Resource::Dir(root_dir);

        assert_eq!(resource.name().expect("Failed to get name"), String::new());
        assert!(resource.is_dir());
        assert!(!resource.is_file());
        assert!(resource.get_reader().is_err());

        let resources = resource
            .get_directory_content()
            .expect("Failed to get resources");
        assert_eq!(resources.len(), 1);
        for resource in resources {
            assert_eq!(
                resource.name().expect("Failed to get name"),
                dir_1_name.to_string()
            );
            assert!(resource.is_dir());
            assert!(!resource.is_file());
            assert!(resource.get_reader().is_err());

            let resources = resource
                .get_directory_content()
                .expect("Failed to get resources");
            assert_eq!(resources.len(), 1);
            for resource in resources {
                assert_eq!(
                    resource.name().expect("Failed to get name"),
                    dir_2_name.to_string()
                );
                assert!(resource.is_dir());
                assert!(!resource.is_file());
                assert!(resource.get_reader().is_err());

                let resources = resource
                    .get_directory_content()
                    .expect("Failed to get resources");
                assert_eq!(resources.len(), 1);
                for resource in resources {
                    assert_eq!(
                        resource.name().expect("Failed to get name"),
                        file_1_name.to_string()
                    );
                    assert!(!resource.is_dir());
                    assert!(resource.is_file());
                    assert!(resource.get_reader().is_ok());
                }
            }
        }
    }
}
