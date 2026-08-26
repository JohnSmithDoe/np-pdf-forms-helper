// ─── why ────────────────────────────────────────────────────────
// The trains workflow: read a sender's spreadsheet, map its columns onto our
// entities, and write sanitised data back out. One folder per workflow, the way
// `doc/` is one folder per format — the workflow being worked on is the folder
// being worked in.
//
// The pipeline runs in this order, and the order is the design:
//
//   sheet/    a file becomes a Grid, and a reader says WHERE the data is
//   sanitise/ raw cell text becomes a typed value, or a German complaint
//   resolve/  a row's names and numbers become entity references
//   stage     all of the above, into a preview that touches NO database
//   commit    the preview plus the user's decisions, into one batched write
//   export/   the master workbook and the ERP artefact
//
// Staging is deliberately severed from committing. Nothing before `commit` can
// change a stored byte, so "the import went wrong halfway" is not a state this
// code can reach — which is why a bad cell is a line in a report rather than an
// aborted run.
// ────────────────────────────────────────────────────────────────

pub mod clock;
pub mod commands;
pub mod commit;
pub mod db;
pub mod export;
pub mod fingerprint;
pub mod hash;
pub mod model;
pub mod resolve;
pub mod sanitise;
pub mod sheet;
pub mod stage;
