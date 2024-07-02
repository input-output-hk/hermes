//! Implementation of the package path object.

use std::fmt::Display;

/// Package path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackagePath(Vec<String>);

impl PackagePath {
    /// Create new `PackagePath` from path components.
    pub(crate) fn new(path: &str) -> Self {
        let path_components = path
            .split('/')
            .map(ToString::to_string)
            .filter(|s| !s.is_empty())
            .collect();
        Self(path_components)
    }

    /// Returns an iterator over the path elements.
    pub(crate) fn iter(&self) -> std::slice::Iter<String> {
        self.0.iter()
    }

    /// Pop the last path element of the path from the path elements.
    pub(crate) fn pop_last_elem(&mut self) -> anyhow::Result<String> {
        self.0
            .pop()
            .ok_or(anyhow::anyhow!("Empty last path element."))
    }
}

impl Display for PackagePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.0.join("/");
        write!(f, "{path}")
    }
}

impl IntoIterator for PackagePath {
    type IntoIter = std::vec::IntoIter<String>;
    type Item = String;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<String> for PackagePath {
    fn from(path: String) -> Self {
        Self::new(path.as_str())
    }
}

impl From<&str> for PackagePath {
    fn from(path: &str) -> Self {
        Self::new(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_path_test() {
        let mut path = PackagePath::new("/a/b/c");
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "c".to_string()
        );
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "b".to_string()
        );
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "a".to_string()
        );
        assert!(path.pop_last_elem().is_err());

        let mut path = PackagePath::new("a/b/c");
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "c".to_string()
        );
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "b".to_string()
        );
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "a".to_string()
        );
        assert!(path.pop_last_elem().is_err());

        let mut path = PackagePath::new("/a");
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "a".to_string()
        );
        assert!(path.pop_last_elem().is_err());

        let mut path = PackagePath::new("a");
        assert_eq!(
            path.pop_last_elem().expect("Failed to pop last element"),
            "a".to_string()
        );
        assert!(path.pop_last_elem().is_err());

        let mut path = PackagePath::new("/");
        assert!(path.pop_last_elem().is_err());

        let mut path = PackagePath::new("");
        assert!(path.pop_last_elem().is_err());
    }
}
