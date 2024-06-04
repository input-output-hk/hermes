//! Filesystem resource implementation.

use std::{
    fmt::{Debug, Display},
    io::Read,
    path::{Path, PathBuf},
};

use super::ResourceTrait;

/// File system resource.
/// A simple wrapper over `PathBuf`
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FsResource(PathBuf);

impl Display for FsResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FsResource {
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_path_buf())
    }

    /// Update current resource to make it relative to the given path.
    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        if self.0.is_relative() {
            self.0 = to.as_ref().join(&self.0);
        }
    }
}

impl ResourceTrait for FsResource {
    fn name(&self) -> anyhow::Result<String> {
        Ok(self
            .0
            .file_name()
            .ok_or(anyhow::anyhow!("cannot get path name"))?
            .to_str()
            .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
            .to_string())
    }

    fn is_dir(&self) -> bool {
        self.0.is_dir()
    }

    fn is_file(&self) -> bool {
        self.0.is_file()
    }

    fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        std::fs::File::open(&self.0).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("File not found at {}", self.0.display()).into()
            } else {
                err.into()
            }
        })
    }

    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>> {
        let mut res = Vec::new();
        let entries = std::fs::read_dir(&self.0).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("Cannot get directory content at {}", self.0.display())
            } else {
                err.into()
            }
        })?;
        for entry in entries {
            res.push(FsResource(entry?.path()));
        }
        Ok(res)
    }
}
