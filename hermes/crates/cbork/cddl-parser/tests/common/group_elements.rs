// cspell: words OPTCOM MEMBERKEY bareword tstr GRPENT GRPCHOICE

#![allow(dead_code)] // TODO: find a way to remove this.

pub(crate) const OCCUR_PASSES: &[&str] = &[
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

pub(crate) const OCCUR_FAILS: &[&str] = &[
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

pub(crate) const OPTCOM_PASSES: &[&str] = &["", ",", " ,", " , ", "\n,\n", "\n"];

pub(crate) const OPTCOM_FAILS: &[&str] = &[",,"];

pub(crate) const MEMBERKEY_PASSES: &[&str] = &[
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

pub(crate) const MEMBERKEY_FAILS: &[&str] = &["#:", "foo::"];

pub(crate) const GRPENT_PASSES: &[&str] = &[
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

pub(crate) const GRPENT_FAILS: &[&str] = &["tstr => (foo: bar)"];

pub(crate) const GRPCHOICE_PASSES: &[&str] = &[
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

pub(crate) const GRPCHOICE_FAILS: &[&str] = &["foo: ,", "foo:", "foo: bar: 2", "foo => bar: 2"];

pub(crate) const GROUP_PASSES: &[&str] = &[
    "(foo: 1)",
    "(foo: 1) // (bar: 2)",
    "(foo: 1) // (bar: 2)",
    "(street: tstr, ? number: uint, city // po-box: uint, city // per-pickup: true)",
    "(+ a // b / c)",
    "((+ a) // (b / c))",
];

pub(crate) const GROUP_FAILS: &[&str] = &["(foo: 1) / (bar: 2)"];
