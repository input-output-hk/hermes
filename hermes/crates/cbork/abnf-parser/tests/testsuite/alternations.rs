// cspell: words abnf

use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

pub(crate) const ALTERNATION_PASSES: &[&str] = &["foo / bar", "foo/bar", "foo/bar baz"];

pub(crate) const ALTERNATION_FAILS: &[&str] = &["foo /"];

pub(crate) const CONCATENATION_PASSES: &[&str] = &[
    "foo",
    "foo bar",
    "foo\tbar",
    "foo bar baz",
    "foo      bar      baz",
    "foo
    bar
    baz",
];

pub(crate) const CONCATENATION_FAILS: &[&str] = &["foo\nbar\nbaz", "foo bar\nbaz"];

#[test]
/// Test if the `alternation` rule passes properly.
fn check_alternation() {
    check_tests_rule(
        Rule::alternation_TEST,
        ALTERNATION_PASSES,
        ALTERNATION_FAILS,
    );
}

#[test]
/// Test if the `concatenation` rule passes properly.
fn check_concatenation() {
    check_tests_rule(
        Rule::concatenation_TEST,
        CONCATENATION_PASSES,
        CONCATENATION_FAILS,
    );
}
