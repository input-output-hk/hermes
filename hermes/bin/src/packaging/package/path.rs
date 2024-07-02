//! Implementation of the package path object.

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

    /// Pop the last element of the path from the path elements.
    pub(crate) fn pop_last_element(&mut self) -> Option<String> {
        self.0.pop()
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
        assert_eq!(path.pop_last_element(), Some("c".to_string()));
        assert_eq!(path.pop_last_element(), Some("b".to_string()));
        assert_eq!(path.pop_last_element(), Some("a".to_string()));
        assert_eq!(path.pop_last_element(), None);

        let mut path = PackagePath::new("a/b/c");
        assert_eq!(path.pop_last_element(), Some("c".to_string()));
        assert_eq!(path.pop_last_element(), Some("b".to_string()));
        assert_eq!(path.pop_last_element(), Some("a".to_string()));
        assert_eq!(path.pop_last_element(), None);

        let mut path = PackagePath::new("/a");
        assert_eq!(path.pop_last_element(), Some("a".to_string()));
        assert_eq!(path.pop_last_element(), None);

        let mut path = PackagePath::new("a");
        assert_eq!(path.pop_last_element(), Some("a".to_string()));
        assert_eq!(path.pop_last_element(), None);

        let mut path = PackagePath::new("/");
        assert_eq!(path.pop_last_element(), None);

        let mut path = PackagePath::new("");
        assert_eq!(path.pop_last_element(), None);
    }
}
