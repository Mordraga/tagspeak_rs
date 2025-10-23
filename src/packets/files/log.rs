use anyhow::{Result, bail};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::kernel::ast::Arg;
use crate::kernel::fs_guard::resolve;
use crate::kernel::{Packet, Runtime, Value};

fn to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Unit => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Num(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::Doc(d) => d.json.clone(),
    }
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let raw = match &p.arg {
        Some(Arg::Str(s)) => s,
        _ => bail!("log needs @<path>"),
    };

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

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

    // Structured mode: if body present, interpret inner packets as literals and build a structured document
    if let Some(body) = &p.body {
        let mode = detect_mode(
            &p.op,
            path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        );
        let obj = build_object_from_body(rt, body)?;
        match mode {
            Mode::Json => {
                let s = serde_json::to_string_pretty(&obj)?;
                write_all(&path, &s)?;
            }
            Mode::Yaml => {
                let s = serde_yaml::to_string(&obj)?;
                write_all(&path, &s)?;
            }
            Mode::Toml => {
                let s = toml::to_string_pretty(&obj)?;
                write_all(&path, &s)?;
            }
        }
        return Ok(rt.last.clone());
    }

    // Fallback: dump last value as pretty JSON
    let json = serde_json::to_string_pretty(&to_json(&rt.last))?;
    write_all(&path, &json)?;
    Ok(rt.last.clone())
}

 

// ---- helpers ----

enum Mode {
    Json,
    Yaml,
    Toml,
}

fn detect_mode(op: &str, ext: &str) -> Mode {
    if let Some(rest) = op.strip_prefix("log(")
        && let Some(end) = rest.find(')') {
            match &rest[..end].to_lowercase() {
                s if s == "json" => return Mode::Json,
                s if s == "yaml" => return Mode::Yaml,
                s if s == "toml" => return Mode::Toml,
                _ => {}
            }
        }
    match ext.to_lowercase().as_str() {
        "yaml" | "yml" => Mode::Yaml,
        "toml" => Mode::Toml,
        _ => Mode::Json,
    }
}

fn write_all(path: &Path, s: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    writeln!(file, "{}", s)?;
    Ok(())
}

fn build_object_from_body(
    rt: &Runtime,
    body: &Vec<crate::kernel::Node>,
) -> Result<serde_json::Value> {
    use serde_json::Value as J;
    let mut root = serde_json::Map::new();
    for node in body {
        if let crate::kernel::Node::Packet(pkt) = node {
            if let Some(k) = parse_key_name(&pkt.op) {
                // key(name) with @value or body
                let v = if let Some(arg) = pkt.arg.as_ref() {
                    arg_to_json(rt, arg)?
                } else if let Some(inner) = pkt.body.as_ref() {
                    build_object_from_body(rt, inner)?
                } else {
                    J::Null
                };
                root.insert(k.to_string(), v);
            } else if pkt.op == "sect" || pkt.op.starts_with("sect(") {
                let name = if let Some(Arg::Ident(id)) = pkt.arg.as_ref() {
                    id.clone()
                } else if let Some(Arg::Str(s)) = pkt.arg.as_ref() {
                    s.clone()
                } else if let Some(n) = parse_paren_name(&pkt.op) {
                    n.to_string()
                } else {
                    bail!("sect needs @<name> or (name)");
                };
                let inner = pkt
                    .body
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("sect missing body"))?;
                let obj = build_object_from_body(rt, inner)?;
                root.insert(name, obj);
            } else {
                bail!(format!("unsupported packet in [log] body: [{}]", pkt.op));
            }
        }
    }
    Ok(J::Object(root))
}

fn parse_key_name(op: &str) -> Option<&str> {
    if op.starts_with("key(") && op.ends_with(')') {
        return op.get(4..op.len() - 1);
    }
    None
}

fn parse_paren_name(op: &str) -> Option<&str> {
    let start = op.find('(')?;
    let end = op.rfind(')')?;
    if end <= start + 1 {
        return None;
    }
    op.get(start + 1..end)
}

fn value_to_json(v: Value) -> Result<serde_json::Value> {
    Ok(match v {
        Value::Unit => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(b),
        Value::Num(n) => serde_json::Value::Number(
            serde_json::Number::from_f64(n).ok_or_else(|| anyhow::anyhow!("invalid number"))?,
        ),
        Value::Str(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)),
        Value::Doc(d) => d.json,
    })
}

fn arg_to_json(rt: &Runtime, arg: &Arg) -> Result<serde_json::Value> {
    Ok(match arg {
        Arg::Number(n) => {
            if n.fract() == 0.0 && *n >= (i64::MIN as f64) && *n <= (i64::MAX as f64) {
                serde_json::Value::Number(serde_json::Number::from(*n as i64))
            } else {
                serde_json::Value::Number(
                    serde_json::Number::from_f64(*n)
                        .ok_or_else(|| anyhow::anyhow!("invalid number"))?,
                )
            }
        }
        Arg::Str(s) => serde_json::from_str(s).unwrap_or(serde_json::Value::String(s.clone())),
        Arg::Ident(id) => match id.as_str() {
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            "null" => serde_json::Value::Null,
            other => {
                if let Some(v) = rt.get_var(other) {
                    value_to_json(v)?
                } else {
                    serde_json::Value::String(other.to_string())
                }
            }
        },
        _ => serde_json::Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;
    use std::fs;

    #[test]
    fn writes_json() -> Result<()> {
        let base = std::env::temp_dir().join(format!("tgsk_log_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub"))?;
        fs::write(base.join("red.tgsk"), "")?;
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "[msg@\"hi\"]>[log@/out.json]")?;
        let node = router::parse(&fs::read_to_string(&script)?).map_err(anyhow::Error::new)?;
        let mut rt = Runtime::from_entry(&script)?;
        rt.eval(&node)?;
        let content = fs::read_to_string(base.join("out.json"))?;
        assert!(content.contains("\"hi\""));
        fs::remove_dir_all(base)?;
        Ok(())
    }

    #[test]
    fn structured_json_log() -> Result<()> {
        use std::fs;
        let base =
            std::env::temp_dir().join(format!("tgsk_log_struct_json_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub"))?;
        fs::write(base.join("red.tgsk"), "")?;
        let script = base.join("sub").join("main.tgsk");
        fs::write(
            &script,
            "[log(json)@/profile.json]{[key(name)@\"Saryn\"][key(age)@25][key(active)@true]}",
        )?;
        let node =
            crate::router::parse(&fs::read_to_string(&script)?).map_err(anyhow::Error::new)?;
        let mut rt = crate::kernel::Runtime::from_entry(&script)?;
        rt.eval(&node)?;
        let content = fs::read_to_string(base.join("profile.json"))?;
        let val: serde_json::Value = serde_json::from_str(&content)?;
        assert_eq!(val["name"], "Saryn");
        assert_eq!(val["age"], 25);
        assert_eq!(val["active"], true);
        fs::remove_dir_all(base)?;
        Ok(())
    }
}
