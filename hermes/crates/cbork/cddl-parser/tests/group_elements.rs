// cspell: words OPTCOM MEMBERKEY bareword tstr GRPENT GRPCHOICE
// cspell: words optcom memberkey grpent grpchoice

use cddl_parser::{self, cddl_test::Rule};

mod common;
use common::{group_elements::*, identifiers::*};

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
