// ─── why ────────────────────────────────────────────────────────
// The fallback for every extension that is not `.pdf` or `.xlsx`. No fields and
// no mapping — it exists so an export folder can also carry the fixed
// attachments that belong with the filled forms.
// ────────────────────────────────────────────────────────────────

use std::path::Path;

use crate::error::{AppError, AppResult};

pub fn create(source: &Path, target: &Path) -> AppResult<()> {
    std::fs::copy(source, target).map_err(|error| AppError::io(target, error))?;
    Ok(())
}
