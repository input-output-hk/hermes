use cddl_parser::{self, cddl_test::Rule};

#[path = "common/mod.rs"]
#[allow(clippy::duplicate_mod)]
mod common;

pub(crate) const S_PASSES: &[&str] = &[" ", "  ", " \t \t", " \t  \r \n \r\n   "];
pub(crate) const S_FAILS: &[&str] = &[" a ", "zz", " \t d \t", " \t  \r \n \t \r\n  x"];
pub(crate) const TEXT_PASSES: &[&str] = &[r#""""#, r#""abc""#, "\"abc\\n\""];
pub(crate) const TEXT_FAILS: &[&str] = &["", "''", "\"abc\n\""];

#[test]
/// Test if the `S` rule passes properly.
/// This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_s() {
    common::check_tests_rule(Rule::S_TEST, S_PASSES, S_FAILS);
}

#[test]
/// Test if the `text` rule passes properly.
fn check_text() {
    common::check_tests_rule(Rule::text_TEST, TEXT_PASSES, TEXT_FAILS);
}
