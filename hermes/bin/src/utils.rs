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
        const CONTENT: &[u8] = b"content";
        const NEW_CONTENT: &[u8] = b"new_content";

        let written = obj.write(CONTENT).unwrap();
        assert_eq!(written, CONTENT.len());
        let written = obj.write(CONTENT).unwrap();
        assert_eq!(written, CONTENT.len());

        obj.seek(std::io::SeekFrom::Start(0))
            .expect("Failed to seek.");
        let mut buffer = [0; CONTENT.len()];
        assert_eq!(buffer.len(), CONTENT.len());
        let read = obj.read(&mut buffer).unwrap();
        assert_eq!(read, CONTENT.len());
        assert_eq!(buffer.as_slice(), CONTENT);
        let read = obj.read(&mut buffer).unwrap();
        assert_eq!(read, CONTENT.len());
        assert_eq!(buffer.as_slice(), CONTENT);

        obj.seek(std::io::SeekFrom::Start(0)).unwrap();
        let written = obj.write(NEW_CONTENT).unwrap();
        assert_eq!(written, NEW_CONTENT.len());

        obj.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut buffer = [0; NEW_CONTENT.len()];
        assert_eq!(buffer.len(), NEW_CONTENT.len());
        let read = obj.read(&mut buffer).unwrap();
        assert_eq!(read, NEW_CONTENT.len());
        assert_eq!(buffer.as_slice(), NEW_CONTENT);
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
