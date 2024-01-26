use super::identifiers::{ID_PASSES, ID_FAILS};

pub(crate) const GENERICARG_PASSES: &[&str] = &[
    "<uint>",
    "<{ foo: bar }>",
    "<{ h'1234': uint }>",
    "<1...10>",
    "<\n1...10\t>",
    "<{ foo: bar }, { foo: baz }>",
    "<{ foo: bar }, 1..10>",
];

pub(crate) const GENERICARG_FAILS: &[&str] =
    &["", "<>", "<uint,>", "<( foo: bar )>", "<bigint / bigfloat>"];

pub(crate) const GENERICPARM_PASSES: &[&str] =
    &["<foo>", "<foo,bar>", "<foo, bar>", "<foo, bar, baz>"];

pub(crate) const GENERICPARM_FAILS: &[&str] = &[
    "",
    "<>",
    "<foo,>",
    "<{ foo: bar }>",
    "<{ h'1234': uint }>",
    "<1...10>",
    "<\n1...10\t>",
];

pub(crate) const ASSIGNG_PASSES: &[&str] = &["=", "//="];

pub(crate) const ASSIGNG_FAILS: &[&str] = &["==", "/="];

pub(crate) const ASSIGNT_PASSES: &[&str] = &["=", "/="];

pub(crate) const ASSIGNT_FAILS: &[&str] = &["==", "//="];

pub(crate) const TYPENAME_PASSES: &[&str] = ID_PASSES;

pub(crate) const TYPENAME_FAILS: &[&str] = ID_FAILS;

pub(crate) const GROUPNAME_PASSES: &[&str] = ID_PASSES;

pub(crate) const GROUPNAME_FAILS: &[&str] = ID_FAILS;

pub(crate) const RULE_GROUP_PASSES: &[&str] = &[
    "foo = (bar: baz)",
    "t //= (foo: bar)",
    "t //= foo",
    "t //= foo<bar>",
    "t //= foo: bar",
    "t //= 2*2 foo: bar",
    "delivery //= ( lat: float, long: float, drone-type: tstr )",
];

pub(crate) const RULE_GROUP_FAILS: &[&str] = &["foo = bar: baz", "t /= (foo: bar)"];
