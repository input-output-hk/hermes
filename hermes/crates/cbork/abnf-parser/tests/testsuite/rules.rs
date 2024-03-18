use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

pub(crate) const RULE_PASSES: &[&str] = &[
    "a = b\n",
    "a=b\n",
    "CRLF        =  %d13.10 \n",
    "command     =  \"command string\"\n",
    "rulename     =  \"abc\"\n",
    "rulename    =  %d97 %d98 %d99\n",
    "rulelist       =  1*( rule / (*c-wsp c-nl) )\n",
];

pub(crate) const RULE_FAILS: &[&str] = &["rulename     =  \"abc\"", "rulename     =  abc"];

pub(crate) const DEFINED_AS_PASSES: &[&str] =
    &["= ", "=/ ", "=   ", "=/   ", "   =/   ", "   =   "];

pub(crate) const DEFINED_AS_FAILS: &[&str] = &["==", "=\\"];

pub(crate) const ELEMENTS_PASSES: &[&str] = &[
    "foo",
    "foo bar baz",
    "foo / bar",
    "(foo / bar) baz",
    "*(1*1foo / 2*bar) 012*baz",
    "%b1",
    "*%b1",
];

pub(crate) const ELEMENTS_FAILS: &[&str] = &["%", "=", "()", "[]"];

#[test]
/// Test if the `rule` rule passes properly.
fn check_rule() {
    check_tests_rule(Rule::rule_TEST, RULE_PASSES, RULE_FAILS);
}

#[test]
/// Test if the `defined_as` rule passes properly.
fn check_defined_as() {
    check_tests_rule(Rule::defined_as_TEST, DEFINED_AS_PASSES, DEFINED_AS_FAILS);
}

#[test]
/// Test if the `elements` rule passes properly.
fn check_elements() {
    check_tests_rule(Rule::elements_TEST, ELEMENTS_PASSES, ELEMENTS_FAILS);
}
