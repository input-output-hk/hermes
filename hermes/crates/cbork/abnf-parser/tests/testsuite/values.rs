use abnf_parser::{
    self,
    abnf_test::Rule,
};

use crate::common::*;

pub(crate) const PROSE_VAL_PASSES: &[&str] = &[
    "<Hello>",
    "<This is a valid string>",
    "<12345>",
    "<!@#$%^&*>",
    "<A_B-C.D/E?F>",
    "<Valid string with spaces >",
    "<Valid<string>",
    "<>",
];

pub(crate) const PROSE_VAL_FAILS: &[&str] = &[
    "<>>",
    "<<>>",
    "<This is a string with a newline\ncharacter>",
    "<This string contains a tab\tcharacter>",
    "<This string has a control character at the end\u{1F}>",
];

pub(crate) const HEX_VAL_PASSES: &[&str] = &[
    "x1A",
    "xFF",
    "x0",
    "x1A.B2",
    "xFF.FF.FF",
    "x1A-FF",
];

pub(crate) const HEX_VAL_FAILS: &[&str] = &[
    "x",
    "X1A",
    "x1A-FF-0D",
    "xFF.FF.FF-FF",
    "x.F",
    "x-F",
    "xG",
    "x-FF",
    "x1A-FF-0D.",
    "x1A.0D-",
    "x1A-FF-0D-",
];

pub(crate) const DEC_VAL_PASSES: &[&str] = &[
    "d123",
    "d456.789",
    "d0",
    "d987.654.321",
    "d123-456",
];

pub(crate) const DEC_VAL_FAILS: &[&str] = &[
    "d",
    "D123",
    "d123-456-789",
    "d123.",
    "d.456",
    "d-789",
    "dG",
    "d-123",
    "d123.",
    "d456.789.",
    "d123-",
];

pub(crate) const BIN_VAL_PASSES: &[&str] = &[
    "b101",
    "b1110.0101",
    "b0",
    "b1010.1010.1010",
    "b1101-1001",
];

pub(crate) const BIN_VAL_FAILS: &[&str] = &[
    "b",
    "B101",
    "b101.",
    "b.111",
    "b-000",
    "bG",
    "b-101",
    "b101.",
    "b1110.0101.",
    "b101-",
];

pub(crate) const NUM_VAL_PASSES: &[&str] = &[
    
];

pub(crate) const NUM_VAL_FAILS: &[&str] = &[
    "%",
    "%%"
];

pub(crate) const CHAR_VAL_PASSES: &[&str] = &[
    "\"Valid string\"",
    "\"Another valid string\"",
    "\"1234\"",
    "\"!@#$%^&*()\"",
    "\"Quoted string with spaces\"",
];

pub(crate) const CHAR_VAL_FAILS: &[&str] = &[
    "\"Invalid string with newline\ncharacter\"",
    "\"Invalid string with tab\tcharacter\"",
    "\"Invalid string with control character at the end\u{1F}\"",
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
    let passes: Vec<_> = NUM_VAL_PASSES
        .iter()
        .map(|x| format!("{x}"))
        .chain(BIN_VAL_PASSES.into_iter().map(|x| format!("%{x}")))
        .chain(DEC_VAL_PASSES.into_iter().map(|x| format!("%{x}")))
        .chain(HEX_VAL_PASSES.into_iter().map(|x| format!("%{x}")))
        .collect();
    let fails: Vec<_> = NUM_VAL_FAILS
        .iter()
        .map(|x| format!("{x}"))
        .chain(BIN_VAL_FAILS.into_iter().map(|x| format!("%{x}")))
        .chain(DEC_VAL_FAILS.into_iter().map(|x| format!("%{x}")))
        .chain(HEX_VAL_FAILS.into_iter().map(|x| format!("%{x}")))
        .collect();

    check_tests_rule(
        Rule::num_val_TEST,
        &passes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        &fails.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    )
}

#[test]
/// Test if the `char_val` rule passes properly.
fn check_char_val() {
    check_tests_rule(Rule::char_val_TEST, CHAR_VAL_PASSES, CHAR_VAL_FAILS)
}