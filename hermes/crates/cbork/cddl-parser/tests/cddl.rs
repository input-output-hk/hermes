use std::{fs, io::Result};

use cddl_parser::{parse_cddl, Extension};

#[test]
fn parse_cddl_files() -> Result<()> {
  let entries = fs::read_dir("tests/cddl")?;

  let mut file_paths: Vec<_> = entries
    .filter_map(Result::ok)
    .filter_map(|x| x.path().is_file().then_some(x.path()))
    .collect();

  file_paths.sort();

  let mut err_messages = vec![];
  for file_path in file_paths {
    let mut content = fs::read_to_string(&file_path)?;

    if let Err(e) = parse_cddl(&mut content, &Extension::RFC8610Parser) {
      err_messages.push(format!("{}) {file_path:?} {e}", err_messages.len() + 1));
    }
  }

  let err_msg = err_messages.join("\n\n");
  if !err_msg.is_empty() {
    panic!("{err_msg}")
  }

  Ok(())
}
