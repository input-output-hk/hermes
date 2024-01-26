// cspell: words CTLOP aname groupsocket typesocket RANGEOP tstr ctlop
// cspell: words rangeop RANGEOP

use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod common;
use common::type_declarations::*;

#[test]
/// Test if the `ctlop` rule passes properly.
/// This uses a special rule in the Grammar to test `ctlop` exhaustively.
fn check_ctlop() {
    common::check_tests_rule(Rule::ctlop_TEST, CTLOP_PASSES, CTLOP_FAILS);
}

#[test]
/// Test if the `rangeop` rule passes properly.
/// This uses a special rule in the Grammar to test `rangeop` exhaustively.
fn check_rangeop() {
    common::check_tests_rule(Rule::rangeop_TEST, RANGEOP_PASSES, RANGEOP_FAILS);
}

#[test]
/// Test if the `type2` rule passes properly.
/// This uses a special rule in the Grammar to test `type2` exhaustively.
fn check_type2() {
    common::check_tests_rule(Rule::type2_TEST, TYPE2_PASSES, TYPE2_FAILS);
}

#[test]
/// Test if the `type1` rule passes properly.
/// This uses a special rule in the Grammar to test `type1` exhaustively.
fn check_type1() {
    common::check_tests_rule(Rule::type1_TEST, TYPE1_PASSES, TYPE1_FAILS);
}

#[test]
/// Test if the `type1` rule passes properly based on composition of type2 test cases.
fn check_type1_composition() {
    let j_len = CTLOP_PASSES.len() + RANGEOP_PASSES.len();
    for (i, test_i) in [TYPE2_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
        for (j, test_j) in [CTLOP_PASSES, RANGEOP_PASSES]
            .into_iter()
            .flatten()
            .enumerate()
        {
            for (k, test_k) in [TYPE2_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
                let input = [test_i.to_owned(), test_j.to_owned(), test_k.to_owned()].join(" ");
                let parse = CDDLTestParser::parse(Rule::type1_TEST, &input);
                if (0..TYPE2_PASSES.len()).contains(&i)
                    && (0..j_len).contains(&j)
                    && (0..TYPE2_PASSES.len()).contains(&k)
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
/// Test if the `type` rule passes properly.
/// This uses a special rule in the Grammar to test `type` exhaustively.
fn check_type() {
    common::check_tests_rule(Rule::type_TEST, TYPE_PASSES, TYPE_FAILS);
}

#[test]
/// Test if the `type` rule passes properly based on composition of type2 test cases.
fn check_type_composition() {
    // type2 composition testing
    for (i, test_i) in [TYPE2_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
        for (j, test_j) in [TYPE2_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
            let input = [test_i.to_owned(), "/", test_j.to_owned()].join(" ");
            let parse = CDDLTestParser::parse(Rule::type_TEST, &input);

            if (0..TYPE2_PASSES.len()).contains(&i) && (0..TYPE2_PASSES.len()).contains(&j) {
                assert!(parse.is_ok());
            } else {
                assert!(parse.is_err());
            }
        }
    }
}
