// ─── why ────────────────────────────────────────────────────────
// Raw cell text in, a typed value or a German sentence out. This is the layer
// where being wrong is INVISIBLE — nothing on screen tells you that `1.234`
// became `1234` rather than `1.234`, because both look completely plausible in
// a preview. That is why it is the one part of trains with unit tests, and why
// it is written before anything that calls it.
//
// Two shapes carry the whole vocabulary:
//
//   Err(String)                     a Fehler — no value came out
//   Ok(Parsed { value, warning })   a value, and possibly something to say
//
// Severity is therefore a property of the RESULT SHAPE, not a field somebody has
// to keep in step with it. A parser cannot accidentally return a value marked
// fatal, or an error marked harmless.
//
// `Zeitpunkt`, `Zahl` and `Flag` exist for the Wagen-Zustand (telematics,
// Schadensmeldungen): a reading's moment, a whole number (km, percent) and a
// JA/NEIN column. `Zahl` is whole on purpose, for the same reason as `Money`.
//
// `Value::Empty` is a VALUE, not a failure: a blank cell in an optional column
// is the most ordinary thing in a spreadsheet. `Value::Money` is CENTS, never an
// `f64` — `0.1 + 0.2` in a master workbook reads as `1234,5600000000001` to the
// person who owns the file.
//
// Nothing in here reads a system locale, a timezone or a clock.
// ────────────────────────────────────────────────────────────────

pub mod column;
pub mod date;
pub mod format;
pub mod number;
pub mod text;
pub mod wagen;

pub use date::{Date, Zeitpunkt};
pub use wagen::Uic;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Empty,
    Text(String),
    Money(i64),
    Date(Date),
    Zeitpunkt(Zeitpunkt),
    Zahl(i64),
    Flag(bool),
    Uic(Uic),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub value: Value,
    pub warning: Option<String>,
}

impl Parsed {
    pub fn plain(value: Value) -> Self {
        Self {
            value,
            warning: None,
        }
    }

    pub fn warned(value: Value, warning: impl Into<String>) -> Self {
        Self {
            value,
            warning: Some(warning.into()),
        }
    }
}

pub type Parse = Result<Parsed, String>;
