// ─── why ────────────────────────────────────────────────────────
// The trains workflow: read a sender's spreadsheet, map its columns onto our
// entities, and write sanitised data back out. One folder per workflow, the way
// `doc/` is one folder per format — the workflow being worked on is the folder
// being worked in.
//
// The pipeline runs in this order, and the order is the design:
//
//   sheet/    a file becomes a Grid, and a reader says WHERE the data is
//   recognise which saved template the header row belongs to, if exactly one
//   sanitise/ raw cell text becomes a typed value, or a German complaint
//   resolve/  a row's names and numbers become entity references
//   clean/    the original, read 1:1 and written back as a cleaned copy
//   dokument  the original and the copy, owned by the app as one record
//   stage     all of the above, into a preview that touches NO database
//   entities  that preview grouped per entity, and the answers back per row
//   commit    the preview plus the user's decisions, into one batched write
//   export/   the ERP artefact
//   master/   the customer's master workbook, exported to from filed documents
//   master_file/  the master workbook itself, copied in and cleaned, in versions
//
// Staging is deliberately severed from committing. Nothing between `stage` and
// `commit` can change a stored byte, so "the import went wrong halfway" is not a state this
// code can reach — which is why a bad cell is a line in a report rather than an
// aborted run. Cleaning has its own commit point — filing the document — and
// it touches no entity: a cleaned file is in the ledger, not in the Schattensystem.
// ────────────────────────────────────────────────────────────────

pub mod builtin;
pub mod clean;
pub mod clock;
pub mod commands;
pub mod commit;
pub mod db;
pub mod detail;
pub mod dokument;
pub mod entities;
pub mod export;
pub mod farbe;
pub mod hash;
pub mod master;
pub mod master_file;
pub mod model;
pub mod reading;
pub mod recognise;
pub mod resolve;
pub mod sanitise;
pub mod scan;
pub mod sheet;
pub mod stage;
pub mod telematik;
pub mod template;
pub mod zustand;
