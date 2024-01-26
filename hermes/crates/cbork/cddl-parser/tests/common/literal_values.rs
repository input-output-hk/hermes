// cspell: words xdog INTFLOAT HEXFLOAT xabcp defp

#![allow(dead_code)] // TODO: find a way to remove this.

pub(crate) const UINT_PASSES: &[&str] = &[
    "10",
    "101",
    "2034",
    "30456",
    "123456789",
    "0x123456789abcdefABCDEF",
    "0b0001110101010101",
    "0",
];

pub(crate) const UINT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const INT_PASSES: &[&str] = &[
    "10",
    "101",
    "2034",
    "30456",
    "123456789",
    "0x123456789abcdefABCDEF",
    "0b0001110101010101",
    "0",
    "-10",
    "-101",
    "-2034",
    "-30456",
    "-123456789",
    "-0x123456789abcdefABCDEF",
    "-0b0001110101010101",
    "-0",
];

pub(crate) const INT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const INTFLOAT_PASSES: &[&str] = &[
    "10",
    "101",
    "2034",
    "30456",
    "123456789",
    "0",
    "-10",
    "-101",
    "-2034",
    "-30456",
    "-123456789",
    "123.456",
    "123.456",
    "123e+789",
    "123e-789",
    "123.456e+789",
    "123.456e-789",
];

pub(crate) const INTFLOAT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const HEXFLOAT_PASSES: &[&str] = &[
    "0xabcp+123",
    "-0xabcp+123",
    "0xabcp-123",
    "-0xabcp-123",
    "0xabc.defp+123",
    "-0xabc.defp+123",
    "0xabc.defp-123",
    "-0xabc.defp-123",
];

pub(crate) const HEXFLOAT_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const NUMBER_PASSES: &[&str] = &[
    "0xabcp+123",
    "-0xabcp+123",
    "0xabcp-123",
    "-0xabcp-123",
    "0xabc.defp+123",
    "-0xabc.defp+123",
    "0xabc.defp-123",
    "-0xabc.defp-123",
    "10",
    "101",
    "2034",
    "30456",
    "123456789",
    "0",
    "-10",
    "-101",
    "-2034",
    "-30456",
    "-123456789",
    "123.456",
    "123.456",
    "123e+789",
    "123e-789",
    "123.456e+789",
    "123.456e-789",
];

pub(crate) const NUMBER_FAILS: &[&str] = &[" a ", "zz", "0123zzz", "0xdog", "0b777"];

pub(crate) const VALUE_PASSES: &[&str] = &[];

pub(crate) const VALUE_FAILS: &[&str] = &[];
