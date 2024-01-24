use cddl_parser::{
    self,
    cddl_test::{CDDLTestParser, Parser, Rule},
};

mod identifiers;
use identifiers::{ID_FAILS, ID_PASSES};
mod type_declarations;
use type_declarations::{TYPE_FAILS, TYPE_PASSES};

pub const GENERICARG_PASSES: &[&str] = &[
    "<uint>",
    "<{ foo: bar }>",
    "<{ h'1234': uint }>",
    "<1...10>",
    "<\n1...10\t>",
    "<{ foo: bar }, { foo: baz }>",
    "<{ foo: bar }, 1..10>",
];

pub const GENERICARG_FAILS: &[&str] =
    &["", "<>", "<uint,>", "<( foo: bar )>", "<bigint / bigfloat>"];

pub const GENERICPARM_PASSES: &[&str] = &["<foo>", "<foo,bar>", "<foo, bar>", "<foo, bar, baz>"];

pub const GENERICPARM_FAILS: &[&str] = &[
    "",
    "<>",
    "<foo,>",
    "<{ foo: bar }>",
    "<{ h'1234': uint }>",
    "<1...10>",
    "<\n1...10\t>",
];

pub const ASSIGNG_PASSES: &[&str] = &["=", "//="];

pub const ASSIGNG_FAILS: &[&str] = &["==", "/="];

pub const ASSIGNT_PASSES: &[&str] = &["=", "/="];

pub const ASSIGNT_FAILS: &[&str] = &["==", "//="];

pub const TYPENAME_PASSES: &[&str] = &ID_PASSES;

pub const TYPENAME_FAILS: &[&str] = &ID_FAILS;

pub const GROUPNAME_PASSES: &[&str] = &ID_PASSES;

pub const GROUPNAME_FAILS: &[&str] = &ID_FAILS;

pub const RULE_GROUP_PASSES: &[&str] = &[
    "foo = (bar: baz)",
    "t //= (foo: bar)",
    "t //= foo",
    "t //= foo<bar>",
    "t //= foo: bar",
    "t //= 2*2 foo: bar",
    "delivery //= ( lat: float, long: float, drone-type: tstr )"
];

pub const RULE_GROUP_FAILS: &[&str] = &[
  "foo = bar: baz",
  "t /= (foo: bar)"
];

#[test]
/// Test if the `genericarg` rule passes properly.
/// This uses a special rule in the Grammar to test `genericarg` exhaustively.
fn check_genericarg() {
    let tests = GENERICARG_PASSES;
    let fails = GENERICARG_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::genericarg_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::genericarg_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `genericparm` rule passes properly.
/// This uses a special rule in the Grammar to test `genericparm` exhaustively.
fn check_genericparm() {
    let tests = GENERICPARM_PASSES;
    let fails = GENERICPARM_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::genericparm_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::occur_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `assigng` rule passes properly.
/// This uses a special rule in the Grammar to test `assigng` exhaustively.
fn check_assigng() {
    let tests = ASSIGNG_PASSES;
    let fails = ASSIGNG_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::assigng_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::assigng_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `assignt` rule passes properly.
/// This uses a special rule in the Grammar to test `assignt` exhaustively.
fn check_assignt() {
    let tests = ASSIGNT_PASSES;
    let fails = ASSIGNT_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::assignt_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::assignt_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `typename` rule passes properly.
/// This uses a special rule in the Grammar to test `typename` exhaustively.
fn check_typename() {
    let tests = TYPENAME_PASSES;
    let fails = TYPENAME_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::typename_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::typename_TEST, test);
        assert!(parse.is_err());
    }
}

#[test]
/// Test if the `groupname` rule passes properly.
/// This uses a special rule in the Grammar to test `groupname` exhaustively.
fn check_groupname() {
    let tests = GROUPNAME_PASSES;
    let fails = GROUPNAME_FAILS;

    for test in tests {
        let parse = CDDLTestParser::parse(Rule::groupname_TEST, test);
        assert!(parse.is_ok());
    }

    for test in fails {
        let parse = CDDLTestParser::parse(Rule::groupname_TEST, test);
        assert!(parse.is_err());
    }
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
  let tests = RULE_GROUP_PASSES;
  let fails = RULE_GROUP_FAILS;

  for test in tests {
      let parse = CDDLTestParser::parse(Rule::rule_TEST, test);

      assert!(parse.is_ok());
  }

  for test in fails {
      let parse = CDDLTestParser::parse(Rule::rule_TEST, test);
      assert!(parse.is_err());
  }
}
