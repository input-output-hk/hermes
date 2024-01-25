use cddl_parser::{self, cddl_test::Rule};

#[path = "common/mod.rs"]
#[allow(clippy::duplicate_mod)]
mod common;

pub(crate) const COMMENT_PASSES: &[&str] = &["; A Comment \n", "; And another\r", ";more\r\n"];

pub(crate) const COMMENT_FAILS: &[&str] = &["not a comment\n"];

pub(crate) const WHITESPACE_COMMENT_PASSES: &[&str] = &[
    " ",
    "  ",
    " \t \t",
    " \t  \r \n \r\n   ",
    "; A Comment\r",
    " \t ; A Comment    \n",
    "; One Comment\n; Two Comments\n",
    "; One Comment  \n; Two Comments\r; Another Comment\r\n",
    "\t; One Comment \n\t; Two Comments\r; Another Comment\r\n",
    "\t; A Comment \n    ; Another Comment \t \r\n    \t  ; A Final Comment   \r\n",
];

pub(crate) const WHITESPACE_COMMENT_FAILS: &[&str] = &["not a comment"];

#[test]
/// Test if the `COMMENT` rule passes properly.
fn check_comment() {
    let passes = COMMENT_PASSES;
    let fails = COMMENT_FAILS;

    common::check_tests_rule(Rule::COMMENT_TEST, passes, fails);
}

#[test]
/// Test if the `COMMENT` rule passes properly with whitespace.
/// This uses a special rule in the Grammar to test whitespace exhaustively.
fn check_whitespace_comments() {
    let passes = WHITESPACE_COMMENT_PASSES;
    let fails = WHITESPACE_COMMENT_FAILS;

    common::check_tests_rule(Rule::COMMENT_TEST, passes, fails);
}
