use std::{ffi::OsStr, fs, io::Result};

use cddl_parser::{parse_cddl, Extension};

#[test]
/// # Panics
fn parse_cddl_files() {
    let entries = fs::read_dir("tests/cddl").expect("`tests/cddl` directory must exist");

    let mut file_paths: Vec<_> = entries
        .filter_map(Result::ok)
        .filter_map(|x| x.path().is_file().then_some(x.path()))
        .collect();

    file_paths.sort();

    let valid_file_paths = file_paths
        .iter()
        .filter_map(|p| p.file_name().and_then(OsStr::to_str))
        .filter(|p| p.starts_with("valid"));
    let invalid_file_paths = file_paths
        .iter()
        .filter_map(|p| p.file_name().and_then(OsStr::to_str))
        .filter(|p| p.starts_with("invalid"));

    // test for valid files
    let mut err_messages = vec![];
    for file_path in valid_file_paths {
        let mut content = fs::read_to_string(file_path).expect("failed to read a file");

        if let Err(e) = parse_cddl(&mut content, &Extension::RFC8610Parser) {
            err_messages.push(format!("{}) {file_path:?} {e}", err_messages.len() + 1));
        }
    }

    // test for invalid files
    for file_path in invalid_file_paths {
        let mut content = fs::read_to_string(file_path).expect("failed to read a file");

        let result = parse_cddl(&mut content, &Extension::RFC8610Parser);

        assert!(result.is_err(), "{:?} is expected to fail", &file_path);
    }

    // summary
    let err_msg = err_messages.join("\n\n");
    assert!(err_msg.is_empty(), "{err_msg}");
}
