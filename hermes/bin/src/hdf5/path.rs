//!  A Hermes HDF5 path abstraction.

use std::fmt::Display;

use crate::utils::parse_path;

/// Package path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Path(Vec<String>);

impl Path {
    /// Create new `PackagePath` from str.
    pub(crate) fn from_str(path: &str) -> Self {
        Self(parse_path(path))
    }

    /// Returns an iterator over the path elements.
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }

    /// Pop the last path element of the path from the path elements.
    /// If the path is empty, return empty string.
    pub(crate) fn pop_elem(&mut self) -> String {
        self.0.pop().unwrap_or_default()
    }

    /// Push a new path element to the path at the end.
    #[allow(dead_code)]
    pub(crate) fn push_elem(&mut self, value: String) {
        self.0.push(value);
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.0.join("/");
        write!(f, "{path}")
    }
}

impl IntoIterator for Path {
    type IntoIter = std::vec::IntoIter<String>;
    type Item = String;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<String> for Path {
    fn from(path: String) -> Self {
        Self::from_str(path.as_str())
    }
}

impl From<&str> for Path {
    fn from(path: &str) -> Self {
        Self::from_str(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_path_test() {
        // with '/' delimiter
        {
            let mut path = Path::from_str("/a/b/c");
            assert_eq!(path.pop_elem(), "c".to_string());
            assert_eq!(path.pop_elem(), "b".to_string());
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str("a/b/c");
            assert_eq!(path.pop_elem(), "c".to_string());
            assert_eq!(path.pop_elem(), "b".to_string());
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str("/a");
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str("/");
            assert_eq!(path.pop_elem(), String::new());
        }
        // with '\' delimiter
        {
            let mut path = Path::from_str(r"\a\b\c");
            assert_eq!(path.pop_elem(), "c".to_string());
            assert_eq!(path.pop_elem(), "b".to_string());
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str(r"a\b\c");
            assert_eq!(path.pop_elem(), "c".to_string());
            assert_eq!(path.pop_elem(), "b".to_string());
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str(r"\a");
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str(r"\");
            assert_eq!(path.pop_elem(), String::new());
        }

        {
            let mut path = Path::from_str("a");
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::from_str("");
            assert_eq!(path.pop_elem(), String::new());
        }
        {
            let mut path = Path::default();
            path.push_elem("a".to_string());
            path.push_elem("b".to_string());
            path.push_elem("c".to_string());
            assert_eq!(path.pop_elem(), "c".to_string());
            assert_eq!(path.pop_elem(), "b".to_string());
            assert_eq!(path.pop_elem(), "a".to_string());
            assert_eq!(path.pop_elem(), String::new());
        }
    }
}
