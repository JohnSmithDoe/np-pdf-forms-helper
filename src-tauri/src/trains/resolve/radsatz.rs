// ─── why ────────────────────────────────────────────────────────
// A RADSATZNUMMER IS NOT A KEY, and this module is where that costs something.
//
// A Wagennummer is twelve digits with a check digit: unique, validatable, and so
// allowed to decide on its own. A Radsatznummer has none of that — it is assigned
// by the Halter or by the Werkstatt, in house format, so two workshops can
// legitimately send ONE string for TWO different radsaetze, and one radsatz
// arrives under several spellings. See `docs/fachdomaene.md`.
//
// So the alias table is SCOPED BY SENDER. An alias does not mean "this radsatz is
// also spelled X", it means "THIS SENDER calls this radsatz X". A spelling the
// user confirmed for sender A decides for sender A and for nobody else; the same
// number from sender B is a question, not a merge. Confirming it learns a second
// alias, and choosing "neu" instead leaves two radsaetze sharing a `match_key`
// under different senders — which is exactly the real situation, and why the
// index maps one key to SEVERAL ids.
//
// Scoping the alias rather than storing one sender ON the radsatz is what makes
// it correct: the relation is many senders per radsatz and it grows as merges are
// confirmed. `Provenance` was the obvious place for a sender and is deliberately
// not used — it records only whoever created the record, and `Radsatz.source` is
// optional anyway, so a hand-made radsatz would have no sender and no way to
// gain one.
//
// THERE IS NO FUZZY TIER HERE, unlike `partner`. Two radsaetze differing by a
// digit ARE two radsaetze, and suggesting otherwise puts the wrong history under
// a wagen. The one exception is LEADING ZEROS, which are a formatting difference
// and not a digit: `0012345` and `12345` are one number typed by two people, and
// Excel eats leading zeros on its own. That is a `Likely` — a suggestion — never
// a match, and it is why `match_key` does NOT strip them itself: tier one must
// never surprise.
// ────────────────────────────────────────────────────────────────

/// Upper case, every separator dropped. `RS-12 345/A` and `rs12345a` are one
/// spelling; `RS4711` and `RS4712` are two numbers and stay two.
pub fn match_key(raw: &str) -> String {
    crate::trains::sanitise::text::normalise(raw)
        .to_uppercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

pub fn without_leading_zeros(key: &str) -> &str {
    let trimmed = key.trim_start_matches('0');
    if trimmed.is_empty() {
        key
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_number_written_four_ways_normalises_to_one_key() {
        for written in ["RS-12 345/A", "rs12345a", "  RS 12345 A  ", "rs.12.345.a"] {
            assert_eq!(match_key(written), "RS12345A", "{written}");
        }
    }

    /// The line this module exists to hold: a differing DIGIT is a different
    /// radsatz, and no normalisation may hide that.
    #[test]
    fn two_numbers_differing_by_a_digit_stay_two_keys() {
        assert_ne!(match_key("RS4711"), match_key("RS4712"));
    }

    /// Tier one must never surprise, so the key keeps its zeros. Offering the
    /// zero-stripped form is tier two's job.
    #[test]
    fn leading_zeros_survive_the_key() {
        assert_eq!(match_key("0012345"), "0012345");
        assert_ne!(match_key("0012345"), match_key("12345"));
    }

    #[test]
    fn stripping_zeros_is_what_makes_the_two_comparable() {
        assert_eq!(
            without_leading_zeros(&match_key("0012345")),
            without_leading_zeros(&match_key("12345"))
        );
    }

    /// A number that is all zeros must not strip down to nothing, or every such
    /// row would collide with every other.
    #[test]
    fn a_number_of_only_zeros_keeps_itself() {
        assert_eq!(without_leading_zeros("000"), "000");
    }

    #[test]
    fn a_cell_with_no_alphanumerics_yields_no_key() {
        assert_eq!(match_key("  -- / --  "), "");
    }
}
