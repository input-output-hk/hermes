//! Regex Specificity Scoring and Sorting
//!
//! This module provides functionality to heuristically score and sort
//! regular expressions by their "specificity."
//!
//! ### Specificity Heuristics
//! - **More literal characters → more specific**
//! - **Anchors (`^`, `$`) → more specific**
//! - **Exact quantifiers (`{n}`) → more specific**
//! - **Wildcards (`.`, `.*`) and open quantifiers (`+`, `*`, `?`) → less specific**
//! - **Alternation (`|`) → less specific**
//!
//! ⚠️ Note: This is **heuristic-based**. Regex semantics can be arbitrarily
//! complex, so this does not guarantee mathematically correct ordering,
//! but is usually good enough for routing/matching use cases.

use std::sync::LazyLock;

use catalyst_types::conversion::from_saturating;
use regex::Regex;

use crate::runtime_extensions::utils::mul_add::SaturatingMulAdd;

// Scoring weights (tweakable constants).
/// Weight of an alphanumeric literal
const LITERAL_WEIGHT: i32 = 1;
/// Weight of an Anchor (`^` or `$`) in the Regex
const ANCHOR_WEIGHT: i32 = 5;
/// Weight of a single `.` in the Regex
const DOT_WEIGHT: i32 = -2;
/// Weight of a `.*` in the Regex
const DOTSTAR_WEIGHT: i32 = -10;
/// Weight of a `+` in the Regex
const PLUS_WEIGHT: i32 = -11;
/// Weight of a `*` in the Regex
const STAR_WEIGHT: i32 = -3;
/// Weight of a `?` in the Regex
const QMARK_WEIGHT: i32 = -2;
/// Weight of a quantifier (`{n}`) in the Regex
const EXACT_QUANT_WEIGHT: i32 = 4;

/// Regex used to match and count exact number of quantifiers in a regex.
static EXACT_QUANT: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"\{\d+\}").unwrap()
});

/// Compute a "specificity score" for a regex string.
/// Higher scores mean the regex is considered more specific.
/// `input` is anything that can be converted to a string reference.
#[allow(dead_code)]
pub(crate) fn regex_specificity_score<T: AsRef<str>>(input: T) -> i32 {
    let re = input.as_ref();

    let mut score: i32 = 0;

    // split on alternations, if present, and weight them distinctly, and use the total of
    // weights divided by the number of alternations.
    // Theory being the specificity is the average of all the alternatives specificity.
    //let alts = re.bytes().filter(|c| *c == b'|').count();
    let alts: Vec<&str> = re.split('|').collect();
    let alts_len = alts.len();
    if alts_len > 1 {
        for x in alts {
            score.mul_add(1, regex_specificity_score(x));
        }
        #[allow(clippy::arithmetic_side_effects)]
        return score.saturating_div(from_saturating(alts_len));
    }

    // Count literal (alphanumeric) characters
    score.mul_add(
        re.chars().filter(|c| c.is_alphanumeric()).count(),
        LITERAL_WEIGHT,
    );

    // Anchors
    if re.contains('^') {
        score.mul_add(1, ANCHOR_WEIGHT);
    }
    if re.contains('$') {
        score.mul_add(1, ANCHOR_WEIGHT);
    }

    // Wildcards
    let dot_stars = re.matches(".*").count();
    if dot_stars > 0 {
        score.mul_add(dot_stars, DOTSTAR_WEIGHT);
    } else {
        score.mul_add(re.matches('.').count(), DOT_WEIGHT);
    }

    // Quantifiers
    score.mul_add(re.matches('+').count(), PLUS_WEIGHT);
    score.mul_add(re.matches('*').count(), STAR_WEIGHT);
    score.mul_add(re.matches('?').count(), QMARK_WEIGHT);

    // Exact quantifiers `{n}`
    score.mul_add(EXACT_QUANT.find_iter(re).count(), EXACT_QUANT_WEIGHT);

    score
}

/// Sort a list of regex strings from most specific to least specific.
#[allow(dead_code)]
pub(crate) fn sort_regexes_by_specificity<T: AsRef<str>>(regexes: Vec<T>) -> Vec<T> {
    let mut scored: Vec<_> = regexes
        .into_iter()
        .map(|re| (regex_specificity_score(&re), re))
        .collect();

    // Sort descending by score (most specific first)
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    scored.into_iter().map(|(_score, re)| re).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_specificity() {
        let s1 = regex_specificity_score("hello");
        let s2 = regex_specificity_score(".*");
        assert!(s1 > s2, "Literal should be more specific than wildcard");
    }

    #[test]
    fn test_anchor_bonus() {
        let anchored = regex_specificity_score("^abc$");
        let unanchored = regex_specificity_score("abc");
        assert!(anchored > unanchored, "Anchored regex should score higher");
    }

    #[test]
    fn test_quantifiers() {
        let exact = regex_specificity_score(r"a{5}");
        let open = regex_specificity_score(r"a+");
        assert!(exact > open, "Exact quantifier should be more specific");
    }

    #[test]
    fn test_alternation_penalty() {
        let alt = regex_specificity_score("a|b|c");
        let literal = regex_specificity_score("abc");
        assert!(
            literal > alt,
            "Literal sequence should be more specific than alternation"
        );
    }

    #[test]
    fn test_sorting_order() {
        let regexes = vec![r".*", r"^hello$", r"abc\d+", r"^foo.*bar$", r"a|b|c"];
        let expected = vec![r"^hello$", r"^foo.*bar$", r"a|b|c", r"abc\d+", r".*"];

        let sorted = sort_regexes_by_specificity(regexes);

        assert_eq!(sorted, expected);
    }
}