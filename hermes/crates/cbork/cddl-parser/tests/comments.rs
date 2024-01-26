use cddl_parser::{self, cddl_test::Rule};

mod common;
use common::comments::*;

#[test]
/// Test if the `COMMENT` rule passes properly.
fn check_comment() {
    common::check_tests_rule(Rule::COMMENT_TEST, COMMENT_PASSES, COMMENT_FAILS);
}

#[test]
/// Test if the `COMMENT` rule passes properly with whitespace.
/// This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_whitespace_comments() {
    common::check_tests_rule(
        Rule::COMMENT_TEST,
        WHITESPACE_COMMENT_PASSES,
        WHITESPACE_COMMENT_FAILS,
    );
}
