// ─── why ────────────────────────────────────────────────────────
// The filler workflow: link an original PDF or XLSX, map its form fields or
// cells to names, and write filled copies. One folder per workflow, so the
// workflow being worked on is the folder being worked in — `trains` is its
// sibling and the two never reach into each other.
//
// What stayed OUTSIDE this folder is the point of the split. `doc/` is the
// document layer both workflows use, `model` the wire vocabulary it speaks,
// and `config`, `error` and `state` are the shell. Only the four modules here
// are filler's alone:
//
//   commands  the API surface, and nothing else
//   db        `data.db` / `profiles.db`, and the profile cascade
//   import    linking one file or a whole folder, no Tauri
//   export    one run of documents, no Tauri
// ────────────────────────────────────────────────────────────────

pub mod commands;
pub mod db;
pub mod export;
pub mod import;
