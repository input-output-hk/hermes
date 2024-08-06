//! Generally used utility functions.

/// Parse a path string into a vector of path elements applying the `/` and `\`
/// delimiters.
pub(crate) fn parse_path(path: &str) -> Vec<String> {
    path.split(&['/', '\\'])
        .map(ToString::to_string)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    pub(crate) fn std_io_read_write_seek_test(
        mut obj: impl std::io::Read + std::io::Write + std::io::Seek,
    ) {
        let content = b"content";
        let written = obj.write(content).unwrap();
        assert_eq!(written, content.len());
        let written = obj.write(content).unwrap();
        assert_eq!(written, content.len());

        obj.seek(std::io::SeekFrom::Start(0))
            .expect("Failed to seek.");
        let mut buffer = [0; 12];
        assert_eq!(buffer.len(), content.len());
        let read = obj.read(&mut buffer).unwrap();
        assert_eq!(read, content.len());
        assert_eq!(buffer.as_slice(), content.as_slice());
        let read = obj.read(&mut buffer).unwrap();
        assert_eq!(read, content.len());
        assert_eq!(buffer.as_slice(), content.as_slice());

        obj.seek(std::io::SeekFrom::Start(0)).unwrap();
        let new_file_content = b"new_content";
        let written = obj.write(new_file_content).unwrap();
        assert_eq!(written, new_file_content.len());

        obj.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut buffer = [0; 16];
        assert_eq!(buffer.len(), new_file_content.len());
        let read = obj.read(&mut buffer).unwrap();
        assert_eq!(read, new_file_content.len());
        assert_eq!(buffer.as_slice(), new_file_content.as_slice());
    }

    #[test]
    fn parse_path_test() {
        assert!(parse_path("").is_empty());

        assert!(parse_path("/").is_empty());
        assert_eq!(parse_path("/a"), vec!["a".to_string()]);
        assert_eq!(parse_path("/a/b"), vec!["a".to_string(), "b".to_string()]);

        assert!(parse_path(r"\").is_empty());
        assert_eq!(parse_path(r"\a"), vec!["a".to_string()]);
        assert_eq!(parse_path(r"\a\b"), vec!["a".to_string(), "b".to_string()]);

        assert_eq!(parse_path("//a//b"), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(parse_path(r"\\a\\b"), vec![
            "a".to_string(),
            "b".to_string()
        ]);
    }
}
