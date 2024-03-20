use std::ops::Deref;

use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

pub(crate) const COMMENT_PASSES: &[&str] = &[
    ";\n",
    "; a\n",
    "; a\r\n",
    ";;\n",
    "; a87246h\t\t\r\n",
    "; And another\r",
];

pub(crate) const COMMENT_FAILS: &[&str] = &[";", ";abc", "not a comment"];

pub(crate) const C_NL_PASSES: &[&str] = &["\r\n", "\r", "\n"];

pub(crate) const C_NL_FAILS: &[&str] = &[" "];

pub(crate) const C_WSP_PASSES: &[&str] = &[
    " ",
    ";\n ",
    "; a\n ",
    "; a\r\n ",
    ";;\n ",
    "; a87246h\t\t\r\n ",
    "; And another\r ",
];

pub(crate) const C_WSP_FAILS: &[&str] = &[";", ";abc", "not a comment", "; a\n     "];

#[test]
/// Test if the `COMMENT` rule passes properly.
fn check_comment() {
    check_tests_rule(Rule::COMMENT_TEST, COMMENT_PASSES, COMMENT_FAILS);
}

#[test]
/// Test if the `c_nl` rule passes properly.
fn check_c_nl() {
    let passes: Vec<_> = C_NL_PASSES
        .iter()
        .chain(COMMENT_PASSES)
        .map(Deref::deref)
        .collect();
    let fails: Vec<_> = C_NL_FAILS
        .iter()
        .chain(COMMENT_FAILS)
        .map(Deref::deref)
        .collect();

    check_tests_rule(Rule::c_nl_TEST, &passes, &fails);
}

#[test]
/// Test if the `c_wsp` rule passes properly.
fn check_c_wsp() {
    check_tests_rule(Rule::c_wsp_TEST, C_WSP_PASSES, C_WSP_FAILS);
}
