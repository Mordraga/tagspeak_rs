use anyhow::{Result, bail};
use std::path::{Component, Path, PathBuf};

// Walks up from `start` to locate a directory containing `red.tgsk`.
// Returns the path of that directory if found.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        if dir.join("red.tgsk").exists() {
            return Some(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

// Normalizes the `requested` path against `root`.
// Ensures the final path stays within `root`; otherwise errors with E_BOUNDARY_RED.
pub fn resolve(root: &Path, requested: &Path) -> Result<PathBuf> {
    let combined = root.join(requested);
    let mut normalized = PathBuf::new();
    for comp in combined.components() {
        match comp {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    if !normalized.starts_with(root) {
        bail!("E_BOUNDARY_RED");
    }
    Ok(normalized)
}
