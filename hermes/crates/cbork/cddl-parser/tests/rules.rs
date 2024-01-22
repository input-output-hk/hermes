use cddl_parser::{
  self,
  cddl_test::{CDDLTestParser, Parser, Rule},
};

pub const GENERICARG_PASSES: &[&str] = &[

];

pub const GENERICARG_FAILS: &[&str] = &[

];

pub const GENERICPARM_PASSES: &[&str] = &[

];

pub const GENERICPARM_FAILS: &[&str] = &[

];

pub const ASSIGNG_PASSES: &[&str] = &[

];

pub const ASSIGNG_FAILS: &[&str] = &[

];

pub const ASSIGNT_PASSES: &[&str] = &[

];

pub const ASSIGNT_FAILS: &[&str] = &[

];

pub const TYPENAME_PASSES: &[&str] = &[

];

pub const TYPENAME_FAILS: &[&str] = &[

];

pub const GROUPNAME_PASSES: &[&str] = &[

];

pub const GROUPNAME_FAILS: &[&str] = &[

];

pub const RULE_PASSES: &[&str] = &[

];

pub const RULE_FAILS: &[&str] = &[

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
/// Test if the `rule` rule passes properly.
/// This uses a special rule in the Grammar to test `rule` exhaustively.
fn check_rule() {
  let tests = RULE_PASSES;
  let fails = RULE_FAILS;

  for test in tests {
      let parse = CDDLTestParser::parse(Rule::rule_TEST, test);
      assert!(parse.is_ok());
  }

  for test in fails {
      let parse = CDDLTestParser::parse(Rule::rule_TEST, test);
      assert!(parse.is_err());
  }
}