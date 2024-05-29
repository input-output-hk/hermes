//! Resources module functionality.

use std::{
    fmt::{Debug, Display},
    io::Read,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// Resource not found error.
#[derive(thiserror::Error, Debug)]
#[error("Resource not found at {0}")]
pub(crate) struct ResourceNotFoundError(String);

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum Resource {
    FsPath(PathBuf),
}

impl<P: AsRef<Path>> From<P> for Resource {
    fn from(path: P) -> Self {
        Self::FsPath(path.as_ref().to_path_buf())
    }
}

impl Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resource::FsPath(path) => write!(f, "{}", path.display()),
        }
    }
}

impl Resource {
    pub(crate) fn name(&self) -> anyhow::Result<String> {
        match self {
            Self::FsPath(path) => {
                Ok(path
                    .file_name()
                    .ok_or(anyhow::anyhow!("cannot get path name"))?
                    .to_str()
                    .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
                    .to_string())
            },
        }
    }

    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        match self {
            Resource::FsPath(path) => {
                if path.is_relative() {
                    *path = to.as_ref().join(&path);
                }
            },
        }
    }

    pub(crate) fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        match self {
            Resource::FsPath(path) => {
                std::fs::File::open(path).map_err(|err| {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        ResourceNotFoundError(self.to_string()).into()
                    } else {
                        err.into()
                    }
                })
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn resource_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = "file.txt";
        std::fs::write(dir.path().join(file_name), [0, 1, 2])
            .expect("Cannot write data to file.txt");

        let mut resource = serde_json::from_value::<Resource>(serde_json::json!(file_name))
            .expect("Cannot parse ResourceLocation json");

        let err = resource.get_reader().expect_err("Should return error");
        assert!(err.is::<ResourceNotFoundError>());

        resource.make_relative_to(dir.path());
        resource.get_reader().expect("Cannot get reader");
    }
}
