// cspell: words aname groupsocket typesocket

#![allow(dead_code)] // TODO: find a way to remove this.

pub(crate) const ID_PASSES: &[&str] = &[
    "$",
    "@",
    "_",
    "a",
    "z",
    "A",
    "Z",
    "$$",
    "@@",
    "__",
    "a$",
    "a@",
    "a_",
    "$0",
    "@9",
    "_a",
    "abc",
    "aname",
    "@aname",
    "_aname",
    "$aname",
    "a$name",
    "a.name",
    "@a.name",
    "$a.name",
    "_a.name",
    "$$",
    "$$groupsocket",
    "$",
    "$typesocket",
];

pub(crate) const ID_FAILS: &[&str] = &[
    "aname.", "aname-", "aname%", "a%name4", "a^name5", "a name", "",
];
