// ─── why ────────────────────────────────────────────────────────
// Partner names are the only key there is, and they are filthy. "Fa. Müller
// GmbH & Co. KG", "Mueller GmbH" and "MÜLLER" are one werkstatt, and no rule
// derives that with confidence.
//
// So there are two tiers and ONLY THE FIRST EVER DECIDES.
//
// Tier 1 is the alias table: `match_key` normalises a name to a comparable form,
// and a partner carries every raw spelling the user has ever confirmed for it.
// An exact hit is `Known` — no scoring, no threshold, no surprise. That table is
// what makes the feature usable at all: without it the user answers the same
// question about the same werkstatt every month.
//
// Tier 2 is similarity, and it is a SUGGESTION. Token-set overlap for multi-word
// names, bounded edit distance for single words, and the result is `Likely` or
// `Ambiguous` — never a silent match. It exists to put the right candidate in
// front of the user, not to decide for them.
//
// `match_key` strips legal forms because they carry no identity: every second
// German company is a GmbH, and leaving them in makes two unrelated firms look
// 60% alike to the token overlap. Umlauts expand rather than being stripped,
// since `Müller` and `Mueller` are the same name typed by two people and
// `Mller` is neither.
//
// Both metrics are hand-written. `strsim` would remove fifty lines and with them
// the understanding of what the threshold means, which is the one thing that has
// to be tuned against real sender data.
// ────────────────────────────────────────────────────────────────

use crate::trains::sanitise::text;

const LEGAL_FORMS: [&str; 16] = [
    "gmbh", "mbh", "ag", "kg", "ohg", "gbr", "se", "ek", "eg", "co", "und", "and", "fa", "sa",
    "srl", "bv",
];

const LIKELY: f64 = 0.80;

pub fn match_key(name: &str) -> String {
    let expanded: String = text::normalise(name)
        .to_lowercase()
        .chars()
        .flat_map(|character| match character {
            'ä' => "ae".chars().collect::<Vec<_>>(),
            'ö' => "oe".chars().collect(),
            'ü' => "ue".chars().collect(),
            'ß' => "ss".chars().collect(),
            other if other.is_alphanumeric() => vec![other],
            _ => vec![' '],
        })
        .collect();

    expanded
        .split_whitespace()
        .filter(|token| !LEGAL_FORMS.contains(token))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn tokens(key: &str) -> Vec<&str> {
    key.split_whitespace().collect()
}

pub fn similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }

    let left_tokens = tokens(left);
    let right_tokens = tokens(right);
    if left_tokens.len() == 1 && right_tokens.len() == 1 {
        let longest = left.chars().count().max(right.chars().count()) as f64;
        return 1.0 - (distance(left, right) as f64 / longest);
    }

    let shared = left_tokens
        .iter()
        .filter(|token| right_tokens.contains(token))
        .count() as f64;
    let union = left_tokens
        .iter()
        .chain(right_tokens.iter())
        .collect::<std::collections::BTreeSet<_>>()
        .len() as f64;
    if union == 0.0 {
        0.0
    } else {
        shared / union
    }
}

pub fn is_likely(score: f64) -> bool {
    score >= LIKELY
}

fn distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (row, left_char) in left.chars().enumerate() {
        current[0] = row + 1;
        for (column, right_char) in right_chars.iter().enumerate() {
            let cost = usize::from(left_char != *right_char);
            current[column + 1] = (current[column] + 1)
                .min(previous[column + 1] + 1)
                .min(previous[column] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_workshop_written_four_ways_normalises_to_one_key() {
        let expected = "mueller";
        for written in [
            "Müller",
            "MÜLLER GmbH",
            "Fa. Mueller GmbH & Co. KG",
            "  müller   ",
        ] {
            assert_eq!(match_key(written), expected, "{written}");
        }
    }

    /// Umlauts expand rather than being stripped: `Müller` and `Mueller` are one
    /// name typed by two people, and `Mller` is neither.
    #[test]
    fn umlauts_expand_the_way_a_german_keyboard_would() {
        assert_eq!(match_key("Schröder"), "schroeder");
        assert_eq!(match_key("Grüße"), "gruesse");
        assert_eq!(match_key("Bahn Äpfel"), "bahn aepfel");
    }

    /// Legal forms carry no identity — every second firm is a GmbH, and leaving
    /// them in makes two unrelated ones look alike to the token overlap.
    #[test]
    fn legal_forms_carry_no_identity() {
        assert_eq!(match_key("Bahnwerk GmbH"), "bahnwerk");
        assert_eq!(match_key("Nord AG"), "nord");
        assert_ne!(match_key("Bahnwerk GmbH"), match_key("Nordwerk GmbH"));
    }

    #[test]
    fn punctuation_becomes_a_separator_rather_than_disappearing() {
        assert_eq!(match_key("Nord-West Bahn"), "nord west bahn");
        assert_eq!(match_key("A.B.C."), "a b c");
    }

    #[test]
    fn an_identical_key_scores_one() {
        assert_eq!(similarity("mueller", "mueller"), 1.0);
    }

    #[test]
    fn a_typo_in_a_single_word_stays_likely() {
        assert!(is_likely(similarity("mueller", "muellr")));
        assert!(is_likely(similarity("bahnwerk", "bahnwerkk")));
    }

    #[test]
    fn two_unrelated_names_are_not_likely() {
        assert!(!is_likely(similarity("mueller", "schmidt")));
        assert!(!is_likely(similarity("nord bahn", "sued werk")));
    }

    #[test]
    fn multi_word_names_compare_by_the_words_they_share() {
        assert!(is_likely(similarity("nord west bahn", "nord west bahn")));
        assert!(!is_likely(similarity("nord west bahn", "nord")));
    }

    #[test]
    fn an_empty_key_matches_nothing() {
        assert_eq!(similarity("", "mueller"), 0.0);
        assert_eq!(similarity("mueller", ""), 0.0);
    }
}
