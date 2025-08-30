use anyhow::{bail, Result};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::kernel::{Arg, Packet, Runtime, Value};
use crate::kernel::values::Document;

fn detect_mode(op: &str) -> Option<&str> {
    if let Some(rest) = op.strip_prefix("parse(") {
        if let Some(end) = rest.find(')') { return Some(&rest[..end]); }
    }
    None
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let mode = detect_mode(&p.op).ok_or_else(|| anyhow::anyhow!("parse needs mode: parse(json|yaml|toml)"))?;
    let s = match &p.arg {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        Some(Arg::Number(n)) => n.to_string(),
        None => match &rt.last {
            Value::Str(s) => s.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    };

    let json_val: serde_json::Value = match mode.to_lowercase().as_str() {
        "json" => serde_json::from_str(&s)?,
        "yaml" => {
            let yv: serde_yaml::Value = serde_yaml::from_str(&s)?;
            serde_json::to_value(yv)?
        }
        "toml" => {
            let tv: toml::Value = toml::from_str(&s)?;
            serde_json::to_value(tv)?
        }
        other => bail!(format!("parse_mode_unsupported:{other}")),
    };

    // Build a memory-backed Document so users can [mod] and [dump]; [save] is optional if they set a file later
    let root = rt.effective_root.as_ref().ok_or_else(|| anyhow::anyhow!("E_BOX_REQUIRED: no red.tgsk"))?;
    let cwd = rt.cwd.clone();
    let path = root.join(&cwd).join("_parsed.json");
    let doc = Document::new(
        json_val,
        PathBuf::from(path),
        String::from("json"),
        SystemTime::now(),
        root.clone(),
    );
    Ok(Value::Doc(doc))
}
