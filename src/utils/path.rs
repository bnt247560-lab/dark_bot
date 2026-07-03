use std::path::{Path, PathBuf};
use crate::errors::{AppError, AppResult};

pub fn sanitize_filename(filename: &str) -> String {
    filename.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

pub fn validate_path(base: &Path, target: &Path) -> AppResult<()> {
    let canonical_base = base.canonicalize()?;
    let canonical_target = target.canonicalize()?;
    
    if !canonical_target.starts_with(canonical_base) {
        return Err(AppError::Security("Path traversal attempt detected".into()));
    }
    Ok(())
}
