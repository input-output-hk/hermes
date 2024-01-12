use std::{fs, io::Result};

use cddl_parser::{parse_cddl, Extension};

#[test]
fn parse_cddl_files() -> Result<()> {
  let entries = fs::read_dir("tests/correct_cddl")?;

  for entry in entries {
    let file_path = entry?.path();

    if !file_path.is_file() {
      continue;
    }

    let mut content = fs::read_to_string(&file_path)?;

    parse_cddl(&mut content, &Extension::RFC8610Parser)
      .unwrap_or_else(|err| panic!("{file_path:?} {err}"));
  }

  Ok(())
}
