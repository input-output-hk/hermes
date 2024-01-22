use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};  

pub const CTLOP_PASSES: &[&str] = &[
    ".$",
    ".@",
    "._",
    ".a",
    ".z",
    ".A",
    ".Z",
    ".$$",
    ".@@",
    ".__",
    ".a$",
    ".a@",
    ".a_",
    ".$0",
    ".@9",
    "._a",
    ".abc",
    ".aname",
    ".@aname",
    "._aname",
    ".$aname",
    ".a$name",
    ".a.name",
    ".@a.name",
    ".$a.name",
    "._a.name",
    ".$$",
    ".$$groupsocket",
    ".$",
    ".$typesocket",
];

pub const CTLOP_FAILS: &[&str] = &[
    "aname.", ".", "..", "aname.", "aname-", "aname%", "a%name4", "a^name5", "a name", "",
];

pub const RANGEOP_PASSES: &[&str] = &[
    "..", "..."
];

pub const RANGEOP_FAILS: &[&str] = &[
    ".", "", "....", ".. .", ". .."
];

pub const TYPE2_PASSES: &[&str] = &[
    "#",
    "#1",
    "#1.1",
    "#1.1",
    "#6",
    "#6.11",
    "#6.11(tstr)",
    "#6.11(\tstr\n)",
    "#6.11({ foo })",
    "#6.11([ foo ])",
    "#6.11(#3.1)",
    "&foo",
    "& foo<bar>",
    "&((+ a) // (b / c))",
    "&\t( foo: bar )",
    "~foo",
    "~ foo<bar>",
    "foo<bar>",
    "[ foo bar ]",
    "{ foo bar }",
    "(a)",
    "(a / b)",
    "(#)",
    "((a))",
    "1",
    "h'1111'",
    "true",
    "foo",
];

pub const TYPE2_FAILS: &[&str] = &[
    "",
    "##",
    "#1.",
    "#6.11 (tstr)",
    "#6.11(( foo: uint ))",
    "&",
    "& foo <bar>",
    "(foo bar)",
];

pub const TYPE1_PASSES: &[&str] = &[
    "1..2",
    "1...2",
    "0..10.0", // BAD range 1
    "0.0..10", // BAD range 2
    "0..max-byte",
    "1.0..2.0",
    "1.0...2.0",
    "foo.bar",
];

pub const TYPE1_FAILS: &[&str] = &[
    ""
];

pub const TYPE_PASSES: &[&str] = &[
    "1 / 2",
    "1\n/\t2",
    "1 / 2 / 3 / 4",
    "1 / (2 / (3 / 4))",
    "# / #",
];

pub const TYPE_FAILS: &[&str] = &[
    "",
    "1 \\ 2",
    "1 // 2",
    "1 2",
    "1 / 2 3",
];

#[test]
/// Test if the `ctlop` rule passes properly.
/// This uses a special rule in the Grammar to test `ctlop` exhaustively.
fn check_ctlop() {
    let passes = CTLOP_PASSES;
    let fails = CTLOP_FAILS;

    for test in passes {
        let parse = CDDLTestParser::parse(Rule::ctlop_TEST, test);
        assert!(parse.is_ok());
    }
  
    for test in fails {
        let parse = CDDLTestParser::parse(Rule::ctlop_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `rangeop` rule passes properly.
/// This uses a special rule in the Grammar to test `rangeop` exhaustively.
fn check_rangeop() {
    let passes = RANGEOP_PASSES;
    let fails = RANGEOP_FAILS;

    for test in passes {
        let parse = CDDLTestParser::parse(Rule::rangeop_TEST, test);
        assert!(parse.is_ok());
    }
  
    for test in fails {
        let parse = CDDLTestParser::parse(Rule::rangeop_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `type2` rule passes properly.
/// This uses a special rule in the Grammar to test `type2` exhaustively.
fn check_type2() {
    let passes = TYPE2_PASSES;
    let fails = TYPE2_FAILS;

    for test in passes {
        let parse = CDDLTestParser::parse(Rule::type2_TEST, test);
        assert!(parse.is_ok());
    }
  
    for test in fails {
        let parse = CDDLTestParser::parse(Rule::type2_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `type1` rule passes properly.
/// This uses a special rule in the Grammar to test `type1` exhaustively.
fn check_type1() {
    let passes = TYPE1_PASSES;
    let fails = TYPE1_FAILS;

    for test in passes {
        let parse = CDDLTestParser::parse(Rule::type1_TEST, test);
        assert!(parse.is_ok());
    }
  
    for test in fails {
        let parse = CDDLTestParser::parse(Rule::type1_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `type1` rule passes properly based on composition of type2 test cases.
fn check_type1_composition() {
    // type2 composition testing
    for (i, test_i) in [TYPE2_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
        for (_, test_j) in [CTLOP_PASSES, RANGEOP_PASSES].into_iter().flatten().enumerate() {
            for (k, test_k) in [TYPE2_PASSES, TYPE_FAILS].into_iter().flatten().enumerate() {
                let input = [test_i.to_owned(), test_j.to_owned(), test_k.to_owned()].join(" ");
                let parse = CDDLTestParser::parse(Rule::type1_TEST, &input);
                if (0..TYPE2_PASSES.len()).contains(&i)
                && (0..TYPE2_PASSES.len()).contains(&k) {
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
    let passes = TYPE_PASSES;
    let fails = TYPE_FAILS;

    for test in passes {
        let parse = CDDLTestParser::parse(Rule::type_TEST, test);
        assert!(parse.is_ok());
    }
  
    for test in fails {
        let parse = CDDLTestParser::parse(Rule::type_TEST, test);
        assert!(parse.is_err());
    }
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