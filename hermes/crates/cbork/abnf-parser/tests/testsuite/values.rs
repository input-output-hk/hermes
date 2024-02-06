use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const PROSE_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const PROSE_VAL_FAILS: &[&str] = &[
    
];

pub(crate) const HEX_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const HEX_VAL_FAILS: &[&str] = &[
    
];

pub(crate) const DEC_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const DEC_VAL_FAILS: &[&str] = &[
    
];

pub(crate) const BIN_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const BIN_VAL_FAILS: &[&str] = &[
    
];

pub(crate) const NUM_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const NUM_VAL_FAILS: &[&str] = &[
    
];

pub(crate) const CHAR_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const CHAR_VAL_FAILS: &[&str] = &[
    
];

#[test]
/// Test if the `prose_val` rule passes properly.
fn check_prose_val() {
    check_tests_rule(Rule::prose_val_TEST, PROSE_VAL_PASSES, PROSE_VAL_FAILS)
}

#[test]
/// Test if the `hex_val` rule passes properly.
fn check_hex_val() {
    check_tests_rule(Rule::hex_val_TEST, HEX_VAL_PASSES, HEX_VAL_FAILS)
}

#[test]
/// Test if the `dec_val` rule passes properly.
fn check_dec_val() {
    check_tests_rule(Rule::dec_val_TEST, DEC_VAL_PASSES, DEC_VAL_FAILS)
}

#[test]
/// Test if the `bin_val` rule passes properly.
fn check_bin_val() {
    check_tests_rule(Rule::bin_val_TEST, BIN_VAL_PASSES, BIN_VAL_FAILS)
}

#[test]
/// Test if the `num_val` rule passes properly.
fn check_num_val() {
    check_tests_rule(Rule::num_val_TEST, NUM_VAL_PASSES, NUM_VAL_FAILS)
}

#[test]
/// Test if the `char_val` rule passes properly.
fn check_char_val() {
    check_tests_rule(Rule::char_val_TEST, CHAR_VAL_PASSES, CHAR_VAL_FAILS)
}