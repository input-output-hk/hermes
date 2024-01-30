// cspell: words HEXPAIR rstuvw abcdefghijklmnopqrstuvwyz Xhhb Bhcm

#![allow(dead_code)] // TODO: find a way to remove this.

pub(crate) const HEXPAIR_PASSES: &[&str] = &["00", "ab", "de", "0f", "f0"];

pub(crate) const HEXPAIR_FAILS: &[&str] = &["0", " 0", "0 ", "az", "0p"];

pub(crate) const URL_BASE64_PASSES: &[&str] = &[
    "abcdefghijklmnopq   rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~",
    "abcdefghijklmnopqrstuvwyz0123456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ",
];

pub(crate) const URL_BASE64_FAILS: &[&str] = &[
    "abcdefghijklmnopq #  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~ ",
    "abcdefghijklmnopq $  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\t",
    "abcdefghijklmnopq %  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\n",
    "abcdefghijklmnopq ^  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\r",
    "abcdefghijklmnopq &  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~\r\n",
];

pub(crate) const BYTES_PASSES: &[&str] = &[
    "h''",
    "b64''",
    "''",
    "h'00'",
    "h'63666F6FF6'",
    "h'68656c6c6f20776f726c64'",
    "h'4 86 56c 6c6f'",
    "h' 20776 f726c64'",
    "h'00112233445566778899aabbccddeeff0123456789abcdef'",
    "h'0 1 2 3 4 5 6 7 8 9 a b c d e f'",
    "h' 0 1 2 3 4 5\r 6 7 \n 8 9 a\r\n\t b c d e f'",
    "h'0 \n\n\r f'",
    "b64'aHR0cHM6Ly93d3cuZXhhbXBsZS5jb20vcGFnZT9wYXJhbTE9dmFsdWUxJnBhcmFtMj12YWx1ZTI~'",
    "'text\n that gets converted \\\' into a byte string...'",
];

pub(crate) const BYTES_FAILS: &[&str] = &[
    "h64",
    "b64",
    "\"\"",
    "h ''",
    "b64 ''",
    "h'001'",
    "b64'abcdefghijklmnopq #  rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ~'",
    "b64'abcdefghijklmnopq   & rstuvw   yz01\t23456789-_ABCDEFGHIJKLMNOPQRSTUVWXYZ'",
    "'\u{7}'",
];
