use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::kernel::ast::Arg;
use crate::kernel::config;
use crate::kernel::fs_guard::resolve;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if rt.effective_root.is_none() {
        anyhow::bail!("E_NO_RED: [run] disabled without a red.tgsk root");
    }
    let raw = match &p.arg {
        Some(Arg::Str(s)) => s,
        _ => bail!("run needs @<path>"),
    };

    let cfg = config::load(rt.effective_root.as_deref());

    // Optional yellow requirement for run (configurable; default off)
    if cfg.require_yellow_run {
        let y = rt.get_num("__yellow_depth").unwrap_or(0.0);
        if y <= 0.0 {
            anyhow::bail!(
                "E_YELLOW_REQUIRED: wrap [run] in [yellow]{{...}} or use [yellow:run@...] to enable"
            );
        }
    }

    // Depth guard (configurable via .tagspeak.toml or env)
    let cur_depth = rt.get_num("__run_depth").unwrap_or(0.0) as usize;
    let max_depth = std::env::var("TAGSPEAK_MAX_RUN_DEPTH")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(cfg.run_max_depth);
    if cur_depth >= max_depth {
        bail!("E_RUN_DEPTH: exceeded max depth {max_depth}");
    }

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

    // root-relative if starts with '/', else relative to current rt.cwd
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
    let path = resolve(root, &candidate)?;

    // Only allow .tgsk files
    if path.extension().and_then(|e| e.to_str()).unwrap_or("") != "tgsk" {
        bail!("run expects a .tgsk file");
    }

    let src = fs::read_to_string(&path)?;
    let ast = crate::router::parse(&src)?;

    // Temporarily switch working dir to the directory of the target script
    let prev_cwd = rt.cwd.clone();
    let new_cwd: PathBuf = match path.parent() {
        Some(parent_abs) => {
            // convert to root-relative
            if let Ok(rel) = parent_abs.strip_prefix(root) {
                rel.to_path_buf()
            } else {
                PathBuf::new()
            }
        }
        None => PathBuf::new(),
    };
    rt.cwd = new_cwd;

    // increment depth, eval, then restore
    rt.set_num("__run_depth", (cur_depth as f64) + 1.0)?;
    let out = rt.eval(&ast)?;
    rt.set_num("__run_depth", (cur_depth as f64))?;

    // Restore cwd
    rt.cwd = prev_cwd;
    Ok(out)
}
