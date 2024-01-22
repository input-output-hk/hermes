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

];

pub const TYPE2_FAILS: &[&str] = &[

];

pub const TYPE1_PASSES: &[&str] = &[

];

pub const TYPE1_FAILS: &[&str] = &[

];

pub const TYPE_PASSES: &[&str] = &[

];

pub const TYPE_FAILS: &[&str] = &[

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