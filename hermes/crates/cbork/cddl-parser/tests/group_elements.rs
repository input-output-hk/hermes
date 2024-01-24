use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod identifiers;
use identifiers::{ID_FAILS, ID_PASSES};

pub const OCCUR_PASSES: &[&str] = &[
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

pub const OCCUR_FAILS: &[&str] = &[
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

pub const OPTCOM_PASSES: &[&str] = &["", ",", " ,", " , ", "\n,\n", "\n"];

pub const OPTCOM_FAILS: &[&str] = &[",,"];

pub const MEMBERKEY_PASSES: &[&str] = &[
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

pub const MEMBERKEY_FAILS: &[&str] = &["#:", "foo::"];

pub const GRPENT_PASSES: &[&str] = &[
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

pub const GRPENT_FAILS: &[&str] = &["tstr => (foo: bar)"];

pub const GRPCHOICE_PASSES: &[&str] = &[
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

pub const GRPCHOICE_FAILS: &[&str] = &["foo: ,", "foo:", "foo: bar: 2", "foo => bar: 2"];

pub const GROUP_PASSES: &[&str] = &[
    "(foo: 1)",
    "(foo: 1) // (bar: 2)",
    "(foo: 1) // (bar: 2)",
    "(street: tstr, ? number: uint, city // po-box: uint, city // per-pickup: true)",
    "(+ a // b / c)",
    "((+ a) // (b / c))",
];

pub const GROUP_FAILS: &[&str] = &["(foo: 1) / (bar: 2)"];

#[test]
/// Test if the `occur` rule passes properly.
/// This uses a special rule in the Grammar to test `occur` exhaustively.
fn check_occur() {
    let tests = OCCUR_PASSES;
    let fails = OCCUR_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::occur_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::occur_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `bareword` rule passes properly.
/// This uses a special rule in the Grammar to test `bareword` exhaustively.
fn check_bareword() {
    let tests: &[&str] = ID_PASSES;
    let fails = ID_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::bareword_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::bareword_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `optcom` rule passes properly.
/// This uses a special rule in the Grammar to test `optcom` exhaustively.
fn check_optcom() {
    let tests = OPTCOM_PASSES;
    let fails = OPTCOM_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::optcom_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::optcom_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `memberkey` rule passes properly.
/// This uses a special rule in the Grammar to test `memberkey` exhaustively.
fn check_memberkey() {
    let tests = MEMBERKEY_PASSES;
    let fails = MEMBERKEY_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::memberkey_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::memberkey_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `grpent` rule passes properly.
/// This uses a special rule in the Grammar to test `grpent` exhaustively.
fn check_grpent() {
    let tests = GRPENT_PASSES;
    let fails = GRPENT_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::grpent_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::grpent_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `grpchoice` rule passes properly.
/// This uses a special rule in the Grammar to test `grpchoice` exhaustively.
fn check_grpchoice() {
    let tests = GRPCHOICE_PASSES;
    let fails = GRPCHOICE_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::grpchoice_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::grpchoice_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `group` rule passes properly.
/// This uses a special rule in the Grammar to test `group` exhaustively.
fn check_group() {
    let tests = GROUP_PASSES;
    let fails = GROUP_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::group_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::group_TEST, test);
        assert!(parse.is_err());
    }
}
