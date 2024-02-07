// cspell: words abnf RULENAME rulename

use abnf_parser::{self, abnf_test::Rule};

use crate::common::*;

pub(crate) const RULENAME_PASSES: &[&str] = &[
    "ABCdef",
    "ABC123def456",
    "ABC-123-def-456",
    "A",
    "A1",
    "A12345",
];

pub(crate) const RULENAME_FAILS: &[&str] = &[
    "ABC@def", "ABC def", "ABC_def", "$A12345", "_A12345", "123ABC",
];

#[test]
/// Test if the `rulename` rule passes properly.
fn check_rulename() {
    check_tests_rule(Rule::rulename_TEST, RULENAME_PASSES, RULENAME_FAILS);
}
