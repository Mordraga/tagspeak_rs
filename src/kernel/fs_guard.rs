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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn find_root_locates_red_tgsk() -> Result<()> {
        let dir = tempdir()?;
        println!("created temp dir {:?}", dir.path());
        fs::write(dir.path().join("red.tgsk"), "")?;
        println!("placed red.tgsk at {:?}", dir.path().join("red.tgsk"));
        let nested = dir.path().join("a/b/c");
        fs::create_dir_all(&nested)?;
        println!("created nested path {:?}", nested);
        let found = find_root(&nested);
        println!("find_root returned {:?}", found);
        assert_eq!(found, Some(dir.path().to_path_buf()));
        Ok(())
    }

    #[test]
    fn resolve_keeps_paths_inside_root() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        println!("root path {:?}", root);
        fs::create_dir_all(root.join("safe"))?;
        println!("created safe dir {:?}", root.join("safe"));
        let resolved = resolve(root, Path::new("safe/file"))?;
        println!("resolve returned {:?}", resolved);
        assert_eq!(resolved, root.join("safe/file"));
        let err = resolve(root, Path::new("../escape")).unwrap_err();
        println!("resolve error {:?}", err);
        assert_eq!(err.to_string(), "E_BOUNDARY_RED");
        Ok(())
    }
}
