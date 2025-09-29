use anyhow::Result;
use std::path::Path;

use crate::kernel::fs_guard::resolve;
use crate::kernel::{Arg, Packet, Runtime, Value};

// [cd@/path] or [cd@relative/path] -> change runtime cwd within red root
// Returns the new cwd as a string starting with '/'.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("E_BOX_REQUIRED: no red.tgsk"))?;

    let raw = match &p.arg {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        _ => String::new(),
    };
    if raw.is_empty() {
        // no-op; return current cwd
        let disp = format!("/{}", rt.cwd.display());
        return Ok(Value::Str(disp));
    }

    let rel = if raw.starts_with('/') {
        &raw[1..]
    } else {
        raw.as_str()
    };
    let candidate = if raw.starts_with('/') {
        Path::new(rel).to_path_buf()
    } else {
        rt.cwd.join(rel)
    };
    let full = resolve(root, &candidate)?;
    let new_rel = full
        .strip_prefix(root)
        .unwrap_or(Path::new(""))
        .to_path_buf();
    rt.cwd = new_rel;
    let disp = format!("/{}", rt.cwd.display());
    Ok(Value::Str(disp))
}
