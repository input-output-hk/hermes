// cspell: words OPTCOM MEMBERKEY bareword tstr GRPENT GRPCHOICE
// cspell: words optcom memberkey grpent grpchoice

use cddl_parser::{self, cddl_test::Rule};

#[path = "common/mod.rs"]
#[allow(clippy::duplicate_mod)]
mod common;

mod identifiers;
use identifiers::{ID_FAILS, ID_PASSES};

pub(crate) const OCCUR_PASSES: &[&str] = &[
    "*",
    "+",
    "?",
    "5*10",
    "0x1*0b110",
    "*20",
    "5*10",
    "0x1*0b110",
    "0*5",
    "5*",
    "*5",
    "0b110*",
    "0x1*",
];

pub(crate) const OCCUR_FAILS: &[&str] = &[
    "5**10",
    "5 * 10",
    "5\t\n*\n10",
    "++",
    "??",
    // Fail cases for uint
    "0123",  // Leading zero is not allowed for decimal
    "0xG",   // Invalid hex digit
    "0b123", // Invalid binary digit
    "0*5*",  // Multiple '*' not allowed
    "0x1*0b110*",
    "0x",
    "0b",
];

pub(crate) const OPTCOM_PASSES: &[&str] = &["", ",", " ,", " , ", "\n,\n", "\n"];

pub(crate) const OPTCOM_FAILS: &[&str] = &[",,"];

pub(crate) const MEMBERKEY_PASSES: &[&str] = &[
    // bareword
    "foo:",
    "foo-bar:",
    "foo_bar:",
    "foo :",
    // values
    "\"foo\":",
    "1:",
    "0x123:",
    "1.1:",
    "-1:",
    "b64'1234':",
    "h'1234':",
    "h'12 34\n':",
    // type1
    "tstr =>",
    "id =>",
    "# =>",
    "1..2 =>",
    "1...2 =>",
    "\"foo\" =>",
    "\"foo\" ^=>",
    "\"foo\"^ =>",
    "\"foo\" ^ =>",
    "1 =>",
    "0x123 =>",
    "1.1 =>",
    "-1 =>",
    "b64'1234' =>",
    "h'1234' =>",
    "h'12 34\n' =>",
];

pub(crate) const MEMBERKEY_FAILS: &[&str] = &["#:", "foo::"];

pub(crate) const GRPENT_PASSES: &[&str] = &[
    "foo: 1",
    "foo: 1",
    "foo-bar:\t\n1",
    "foo :\n1",
    "foo: #",
    "tstr => any",
    "tstr => { foo: bar }",
    "tstr => { foo: bar, baz }",
    "tstr => [foo: bar, baz]",
];

pub(crate) const GRPENT_FAILS: &[&str] = &["tstr => (foo: bar)"];

pub(crate) const GRPCHOICE_PASSES: &[&str] = &[
    "foo: 1",
    "foo: 1, bar: 2",
    "foo: 1, bar: 2,",
    "foo: 1\nbar: 2",
    "foo: 1 bar: 2",
    "foo => 1 bar: 2",
    "foo => 1, bar => 2",
    "foo => 1, bar: 2",
    "foo => 1bar: 2",
];

pub(crate) const GRPCHOICE_FAILS: &[&str] = &["foo: ,", "foo:", "foo: bar: 2", "foo => bar: 2"];

pub(crate) const GROUP_PASSES: &[&str] = &[
    "(foo: 1)",
    "(foo: 1) // (bar: 2)",
    "(foo: 1) // (bar: 2)",
    "(street: tstr, ? number: uint, city // po-box: uint, city // per-pickup: true)",
    "(+ a // b / c)",
    "((+ a) // (b / c))",
];

pub(crate) const GROUP_FAILS: &[&str] = &["(foo: 1) / (bar: 2)"];

#[test]
/// Test if the `occur` rule passes properly.
/// This uses a special rule in the Grammar to test `occur` exhaustively.
fn check_occur() {
    common::check_tests_rule(Rule::occur_TEST, OCCUR_PASSES, OCCUR_FAILS);
}

#[test]
/// Test if the `bareword` rule passes properly.
/// This uses a special rule in the Grammar to test `bareword` exhaustively.
fn check_bareword() {
    common::check_tests_rule(Rule::bareword_TEST, ID_PASSES, ID_FAILS);
}

#[test]
/// Test if the `optcom` rule passes properly.
/// This uses a special rule in the Grammar to test `optcom` exhaustively.
fn check_optcom() {
    common::check_tests_rule(Rule::optcom_TEST, OPTCOM_PASSES, OPTCOM_FAILS);
}

#[test]
/// Test if the `memberkey` rule passes properly.
/// This uses a special rule in the Grammar to test `memberkey` exhaustively.
fn check_memberkey() {
    common::check_tests_rule(Rule::memberkey_TEST, MEMBERKEY_PASSES, MEMBERKEY_FAILS);
}

#[test]
/// Test if the `grpent` rule passes properly.
/// This uses a special rule in the Grammar to test `grpent` exhaustively.
fn check_grpent() {
    common::check_tests_rule(Rule::grpent_TEST, GRPENT_PASSES, GRPENT_FAILS);
}

#[test]
/// Test if the `grpchoice` rule passes properly.
/// This uses a special rule in the Grammar to test `grpchoice` exhaustively.
fn check_grpchoice() {
    common::check_tests_rule(Rule::grpchoice_TEST, GRPCHOICE_PASSES, GRPCHOICE_FAILS);
}

#[test]
/// Test if the `group` rule passes properly.
/// This uses a special rule in the Grammar to test `group` exhaustively.
fn check_group() {
    common::check_tests_rule(Rule::group_TEST, GROUP_PASSES, GROUP_FAILS);
}
