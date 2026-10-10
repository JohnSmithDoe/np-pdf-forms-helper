// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook, in both directions. It is a HUB: a dashboard
// of VLOOKUPs over sheets people fill by pasting portal exports into them, plus
// sheets they keep by hand. It is the CUSTOMER'S file, not one this program
// generates — formulas, colours, filters and columns this program invented no
// meaning for — which is why it has a module of its own and is not an export.
//
//   export    Schattensystem → master: one filed document into the sheets the
//             user ticks, previewed cell by cell, written as a NEW VERSION of
//             the client master (`master_file`); no version is ever written
//             over. The main goal while the customer still works in Excel.
//
// The workbook is always the client master's current version — `master_file`
// takes it in, `bindings::follow` points everything here at it. There is no
// second master picked anywhere else.
//   mirror    master → Schattensystem: every bound sheet imported, the facts
//             rebuilt from scratch each run, for checking against the customer.
//   import_all  the same run in one go, every question answered the way
//             „alle neuen anlegen“ does, reported afterwards.
//
// What a sheet IS decides how it is read and whether it is written back, and
// that lives in `kinds/`, one module per kind. The customer's sheet NAMES live
// only in `master.json`, bound to a kind there — no name from the customer's
// file belongs in code.
//
//   bindings  every sheet bound by its headers; the header scan, cached per file version
//   book      opening the workbook the one safe way (lazy, one sheet at a time)
//   prepare   the trimmed read copy every reader opens instead of the original
//   view      what the settings page needs from the workbook
//   sheet_view  one bound sheet as the mirror holds it, built for display
//   paste     writing a table into a paste-target sheet by header
//   source    typing a filed document's cells for the master
// ────────────────────────────────────────────────────────────────

pub mod bindings;
mod book;
pub mod export;
pub mod import_all;
pub mod kinds;
pub mod mirror;
mod paste;
pub mod prepare;
mod sheet_view;
mod source;
mod view;

pub use sheet_view::sheet_view;
pub use view::view;
