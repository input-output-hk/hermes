use cddl_parser::{self, cddl_test::Rule};

mod common;
use common::text_sequences::*;

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
