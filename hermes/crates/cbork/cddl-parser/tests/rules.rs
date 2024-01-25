// cspell: words GENERICARG bigfloat ASSIGNG GROUPNAME tstr genericarg GENERICARG
// cspell: words assigng assignt ASSIGNT GENERICPARM genericparm

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod identifiers;
use identifiers::{ID_FAILS, ID_PASSES};
mod type_declarations;
use type_declarations::{TYPE_FAILS, TYPE_PASSES};

pub(crate) const GENERICARG_PASSES: &[&str] = &[
    "<uint>",
    "<{ foo: bar }>",
    "<{ h'1234': uint }>",
    "<1...10>",
    "<\n1...10\t>",
    "<{ foo: bar }, { foo: baz }>",
    "<{ foo: bar }, 1..10>",
];

pub(crate) const GENERICARG_FAILS: &[&str] =
    &["", "<>", "<uint,>", "<( foo: bar )>", "<bigint / bigfloat>"];

pub(crate) const GENERICPARM_PASSES: &[&str] =
    &["<foo>", "<foo,bar>", "<foo, bar>", "<foo, bar, baz>"];

pub(crate) const GENERICPARM_FAILS: &[&str] = &[
    "",
    "<>",
    "<foo,>",
    "<{ foo: bar }>",
    "<{ h'1234': uint }>",
    "<1...10>",
    "<\n1...10\t>",
];

pub(crate) const ASSIGNG_PASSES: &[&str] = &["=", "//="];

pub(crate) const ASSIGNG_FAILS: &[&str] = &["==", "/="];

pub(crate) const ASSIGNT_PASSES: &[&str] = &["=", "/="];

pub(crate) const ASSIGNT_FAILS: &[&str] = &["==", "//="];

pub(crate) const TYPENAME_PASSES: &[&str] = ID_PASSES;

pub(crate) const TYPENAME_FAILS: &[&str] = ID_FAILS;

pub(crate) const GROUPNAME_PASSES: &[&str] = ID_PASSES;

pub(crate) const GROUPNAME_FAILS: &[&str] = ID_FAILS;

pub(crate) const RULE_GROUP_PASSES: &[&str] = &[
    "foo = (bar: baz)",
    "t //= (foo: bar)",
    "t //= foo",
    "t //= foo<bar>",
    "t //= foo: bar",
    "t //= 2*2 foo: bar",
    "delivery //= ( lat: float, long: float, drone-type: tstr )",
];

pub(crate) const RULE_GROUP_FAILS: &[&str] = &["foo = bar: baz", "t /= (foo: bar)"];

/// # Panics
pub(crate) fn passes_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_ok());
    }
}

/// # Panics
pub(crate) fn fails_tests_rule(rule_type: Rule, test_data: &[&str]) {
    for test in test_data {
        let parse = CDDLTestParser::parse(rule_type, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `genericarg` rule passes properly.
/// This uses a special rule in the Grammar to test `genericarg` exhaustively.
fn check_genericarg() {
    let passes = GENERICARG_PASSES;
    let fails = GENERICARG_FAILS;

    passes_tests_rule(Rule::genericarg_TEST, passes);
    fails_tests_rule(Rule::genericarg_TEST, fails);
}

#[test]
/// Test if the `genericparm` rule passes properly.
/// This uses a special rule in the Grammar to test `genericparm` exhaustively.
fn check_genericparm() {
    let passes = GENERICPARM_PASSES;
    let fails = GENERICPARM_FAILS;

    passes_tests_rule(Rule::genericparm_TEST, passes);
    fails_tests_rule(Rule::genericparm_TEST, fails);
}

#[test]
/// Test if the `assigng` rule passes properly.
/// This uses a special rule in the Grammar to test `assigng` exhaustively.
fn check_assigng() {
    let passes = ASSIGNG_PASSES;
    let fails = ASSIGNG_FAILS;

    passes_tests_rule(Rule::assigng_TEST, passes);
    fails_tests_rule(Rule::assigng_TEST, fails);
}

#[test]
/// Test if the `assignt` rule passes properly.
/// This uses a special rule in the Grammar to test `assignt` exhaustively.
fn check_assignt() {
    let passes = ASSIGNT_PASSES;
    let fails = ASSIGNT_FAILS;

    passes_tests_rule(Rule::assignt_TEST, passes);
    fails_tests_rule(Rule::assignt_TEST, fails);
}

#[test]
/// Test if the `typename` rule passes properly.
/// This uses a special rule in the Grammar to test `typename` exhaustively.
fn check_typename() {
    let passes = TYPENAME_PASSES;
    let fails = TYPENAME_FAILS;

    passes_tests_rule(Rule::typename_TEST, passes);
    fails_tests_rule(Rule::typename_TEST, fails);
}

#[test]
/// Test if the `groupname` rule passes properly.
/// This uses a special rule in the Grammar to test `groupname` exhaustively.
fn check_groupname() {
    let passes = GROUPNAME_PASSES;
    let fails = GROUPNAME_FAILS;

    passes_tests_rule(Rule::groupname_TEST, passes);
    fails_tests_rule(Rule::groupname_TEST, fails);
}

#[test]
/// Test if the `rule` rule passes properly for type variant.
fn check_rule_type_composition() {
    for (i, test_i) in [TYPENAME_PASSES, TYPENAME_FAILS]
        .into_iter()
        .flatten()
        .enumerate()
    {
        for (j, test_j) in [ASSIGNT_PASSES].into_iter().flatten().enumerate() {
            for (k, test_k) in [TYPE_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
                let input = [test_i.to_owned(), test_j.to_owned(), test_k.to_owned()].join(" ");
                let parse = CDDLTestParser::parse(Rule::rule_TEST, &input);
                if (0..TYPENAME_PASSES.len()).contains(&i)
                    && (0..ASSIGNT_PASSES.len()).contains(&j)
                    && (0..TYPE_PASSES.len()).contains(&k)
                {
                    assert!(parse.is_ok());
                } else {
                    assert!(parse.is_err());
                }
            }
        }
    }
}

#[test]
/// Test if the `rule` rule passes properly for group variant.
fn check_rule_group() {
    let passes = RULE_GROUP_PASSES;
    let fails = RULE_GROUP_FAILS;

    passes_tests_rule(Rule::rule_TEST, passes);
    fails_tests_rule(Rule::rule_TEST, fails);
}
