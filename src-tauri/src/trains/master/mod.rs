// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook, in both directions. It is a HUB: a dashboard
// of VLOOKUPs over sheets people fill by pasting portal exports into them, plus
// sheets they keep by hand. It is the CUSTOMER'S file, not one this program
// generates — formulas, colours, filters and columns this program invented no
// meaning for — which is why it has a module of its own and is not an export.
//
//   refresh   Schattensystem → master: paste targets refreshed from filed
//             documents, written into a DATED COPY; the original is never
//             written. The main goal while the customer still works in Excel.
//   mirror    master → Schattensystem: every bound sheet imported, the facts
//             rebuilt from scratch each run, for checking against the customer.
//
// What a sheet IS decides how it is read and whether it is written back, and
// that lives in `kinds/`, one module per kind. The customer's sheet NAMES live
// only in `master.json`, bound to a kind there — no name from the customer's
// file belongs in code.
//
//   book      opening the workbook the one safe way (lazy, one sheet at a time)
//   view      what the settings page needs from the workbook
//   sheet_view  one bound sheet as the mirror holds it, built for display
//   paste     writing a table into a paste-target sheet by header
//   source    typing a filed document's cells for the master
// ────────────────────────────────────────────────────────────────

mod book;
pub mod kinds;
pub mod mirror;
mod paste;
mod refresh;
mod sheet_view;
mod source;
mod view;

pub use refresh::refresh;
pub use sheet_view::sheet_view;
pub use view::view;
