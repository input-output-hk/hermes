use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use temp_dir::TempDir;

use crate::utils::LOG_FILE_NAME;

pub fn app_logs_contain(temp_dir: &TempDir, needle: &str) -> bool {
    let log_file_path = temp_dir.as_ref().join(LOG_FILE_NAME);
    file_contains_line_with(log_file_path, needle)
}

fn file_contains_line_with<P>(file_path: P, needle: &str) -> bool
where P: AsRef<Path> {
    let file = File::open(file_path).expect("cannot open file");
    let reader = BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result.expect("cannot read line from file");
        if line.contains(needle) {
            return true;
        }
    }

    false
}
