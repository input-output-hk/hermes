pub(crate) const CTLOP_PASSES: &[&str] = &[
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

pub(crate) const CTLOP_FAILS: &[&str] = &[
    "aname.", ".", "..", "aname.", "aname-", "aname%", "a%name4", "a^name5", "a name", "",
];

pub(crate) const RANGEOP_PASSES: &[&str] = &["..", "..."];

pub(crate) const RANGEOP_FAILS: &[&str] = &[".", "", "....", ".. .", ". .."];

pub(crate) const TYPE2_PASSES: &[&str] = &[
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

pub(crate) const TYPE2_FAILS: &[&str] = &[
    "",
    "##",
    "#1.",
    "#6.11 (tstr)",
    "#6.11(( foo: uint ))",
    "&",
    "& foo <bar>",
    "(foo bar)",
];

pub(crate) const TYPE1_PASSES: &[&str] = &[
    "1..2",
    "1 .. 2",
    "1\t..\n2",
    "1...2",
    "0..10.0", // BAD range 1
    "0.0..10", // BAD range 2
    "0..max-byte",
    "min-type..max-byte",
    "1.0..2.0",
    "1.0...2.0",
    "foo.bar",
];

pub(crate) const TYPE1_FAILS: &[&str] = &[""];

pub(crate) const TYPE_PASSES: &[&str] = &[
    "1 / 2",
    "1\n/\t2",
    "1 / 2 / 3 / 4",
    "1 / (2 / (3 / 4))",
    "# / #",
];

pub(crate) const TYPE_FAILS: &[&str] = &["", "1 \\ 2", "1 // 2", "1 2", "1 / 2 3"];
