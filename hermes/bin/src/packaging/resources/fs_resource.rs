//! Filesystem resource implementation.

use std::{
    fmt::Debug,
    io::Read,
    path::{Path, PathBuf},
};

use super::ResourceTrait;

impl ResourceTrait for PathBuf {
    fn name(&self) -> anyhow::Result<String> {
        Ok(self
            .file_name()
            .ok_or(anyhow::anyhow!("cannot get path name"))?
            .to_str()
            .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
            .to_string())
    }

    fn is_dir(&self) -> bool {
        self.as_path().is_dir()
    }

    fn is_file(&self) -> bool {
        self.as_path().is_file()
    }

    fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        if self.is_relative() {
            *self = to.as_ref().join(&self);
        }
    }

    fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        std::fs::File::open(self).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("File not found at {}", self.display()).into()
            } else {
                err.into()
            }
        })
    }

    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>> {
        let mut res = Vec::new();
        let entries = std::fs::read_dir(self).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!("Cannot get directory content at {}", self.display())
            } else {
                err.into()
            }
        })?;
        for entry in entries {
            res.push(entry?.path());
        }
        Ok(res)
    }
}
