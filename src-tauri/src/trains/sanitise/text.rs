// ─── why ────────────────────────────────────────────────────────
// The whitespace rules everything else leans on, in one place.
//
// The four space characters in `SPACES` are not pedantry — they are NBSP,
// narrow NBSP and thin space beside the ASCII one. Excel and the tools that
// export from it emit NBSP as a thousands separator and narrow NBSP inside
// German number formats, so a column that looks like plain digits routinely
// carries code points that are invisible on screen. Getting this wrong makes
// `1 234,56` unparseable for a reason nobody can see.
//
// `normalise` trims and collapses every run of internal whitespace to one ASCII
// space; the space is deferred rather than pushed, so a trailing run leaves
// nothing behind. What comes back is what gets matched, displayed and stored.
//
// `parse` is the free-text field, and it cannot fail: any run of characters is a
// bemerkung. A blank cell becomes `Value::Empty` rather than an empty string,
// so "nothing here" is one thing in the model and not two.
// ────────────────────────────────────────────────────────────────

use super::{Parse, Parsed, Value};

pub const SPACES: [char; 4] = [' ', '\u{00a0}', '\u{202f}', '\u{2009}'];

pub fn is_space(character: char) -> bool {
    SPACES.contains(&character) || character.is_whitespace()
}

pub fn normalise(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut pending_space = false;
    for character in raw.chars() {
        if is_space(character) {
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(character);
    }
    out
}

pub fn parse(raw: &str) -> Parse {
    let text = normalise(raw);
    Ok(Parsed::plain(if text.is_empty() {
        Value::Empty
    } else {
        Value::Text(text)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_every_space_character_it_knows() {
        assert_eq!(
            normalise("  Fa.\u{00a0}Müller   GmbH \u{202f}"),
            "Fa. Müller GmbH"
        );
    }

    #[test]
    fn a_run_of_spaces_becomes_exactly_one() {
        assert_eq!(normalise("a \t\n  b"), "a b");
    }

    #[test]
    fn a_blank_cell_parses_to_empty_not_to_an_empty_string() {
        assert_eq!(parse("\u{00a0} ").unwrap().value, Value::Empty);
        assert_eq!(normalise("\u{00a0} \t"), "");
    }

    #[test]
    fn text_never_fails() {
        assert_eq!(
            parse(" Bremsprobe ").unwrap().value,
            Value::Text("Bremsprobe".into())
        );
    }
}
