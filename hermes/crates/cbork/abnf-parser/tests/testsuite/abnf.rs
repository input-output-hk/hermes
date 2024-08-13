use std::{ffi::OsStr, fs, io::Result};

use abnf_parser::parse_abnf;

#[test]
/// # Panics
fn parse_abnf_files() {
    let entries = fs::read_dir("tests/abnf").unwrap();

    let mut file_paths: Vec<_> = entries
        .filter_map(Result::ok)
        .filter_map(|x| x.path().is_file().then_some(x.path()))
        .collect();

    file_paths.sort();

    let valid_file_paths = file_paths.iter().filter(|p| {
        p.file_name()
            .and_then(OsStr::to_str)
            .map(|p| p.starts_with("valid"))
            .is_some_and(|p| p)
    });
    let invalid_file_paths = file_paths.iter().filter(|p| {
        p.file_name()
            .and_then(OsStr::to_str)
            .map(|p| p.starts_with("invalid"))
            .is_some_and(|p| p)
    });

    // test for valid files
    let mut err_messages = vec![];
    for file_path in valid_file_paths {
        let content = fs::read_to_string(file_path).unwrap();

        if let Err(e) = parse_abnf(&content) {
            err_messages.push(format!("{}) {file_path:?} {e}", err_messages.len() + 1));
        }
    }

    // test for invalid files
    for file_path in invalid_file_paths {
        let content = fs::read_to_string(file_path).unwrap();

        let result = parse_abnf(&content);

        assert!(result.is_err(), "{:?} is expected to fail", &file_path);
    }

    // summary
    let err_msg = err_messages.join("\n\n");
    assert!(err_msg.is_empty(), "{err_msg}");
}
