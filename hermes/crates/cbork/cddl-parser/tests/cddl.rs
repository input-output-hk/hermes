use std::{fs, io::Result};

use cddl_parser::{parse_cddl, Extension};

#[test]
fn parse_cddl_files() -> Result<()> {
  let entries = fs::read_dir("tests/cddl")?;

  let mut error_results = vec![];

  for entry in entries {
    let file_path = entry?.path();

    if !file_path.is_file() {
      continue;
    }

    let mut content = fs::read_to_string(&file_path)?;

    if let Err(e) = parse_cddl(&mut content, &Extension::RFC8610Parser) {
      error_results.push(format!("{}) {file_path:?} {e}", error_results.len() + 1));
    }
  }

  let err_msg = error_results.join("\n\n");
  if !err_msg.is_empty() {
    panic!("{err_msg}")
  }

  Ok(())
}
