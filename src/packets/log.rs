use anyhow::{bail, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde_json::{Map, Value as JsonValue};

use crate::kernel::ast::{Arg, Node, Packet};
use crate::kernel::fs_guard::resolve;
use crate::kernel::{Runtime, Value};

fn to_json(v: &Value) -> JsonValue {
    match v {
        Value::Unit => JsonValue::Null,
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Num(n) => serde_json::Number::from_f64(*n)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Value::Str(s) => JsonValue::String(s.clone()),
        Value::Doc(d) => d.json.clone(),
    }
}

fn extract_fmt(op: &str) -> Option<&str> {
    op.strip_prefix("log(").and_then(|s| s.strip_suffix(')'))
}

fn build_struct(rt: &mut Runtime, body: &[Node]) -> Result<JsonValue> {
    let mut map = Map::new();
    for node in body {
        if let Node::Packet(pkt) = node {
            if let Some(name) = pkt.op.strip_prefix("key(").and_then(|s| s.strip_suffix(')')) {
                let arg = pkt.arg.as_ref().ok_or_else(|| anyhow::anyhow!("key needs @value"))?;
                let val = rt.resolve_arg(arg)?;
                map.insert(name.to_string(), to_json(&val));
            } else if pkt.op == "sect" {
                let section = match pkt.arg.as_ref() {
                    Some(Arg::Ident(s)) | Some(Arg::Str(s)) => s.clone(),
                    _ => bail!("sect needs @name"),
                };
                let inner = pkt.body.as_ref().ok_or_else(|| anyhow::anyhow!("sect requires body"))?;
                map.insert(section, build_struct(rt, inner)?);
            }
        }
    }
    Ok(JsonValue::Object(map))
}

fn resolve_path(rt: &Runtime, raw: &str) -> Result<std::path::PathBuf> {
    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;
    let rel = if raw.starts_with('/') { &raw[1..] } else { raw };
    let candidate = if raw.starts_with('/') {
        Path::new(rel).to_path_buf()
    } else {
        rt.cwd.join(rel)
    };
    resolve(root, &candidate)
}

fn quick_dump(rt: &mut Runtime, raw: &str) -> Result<Value> {
    let path = resolve_path(rt, raw)?;
    let json = serde_json::to_string_pretty(&to_json(&rt.last))?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", json)?;
    Ok(rt.last.clone())
}

fn structured_dump(rt: &mut Runtime, p: &Packet, fmt: &str, raw: &str) -> Result<Value> {
    let path = resolve_path(rt, raw)?;
    let body = p.body.as_ref().ok_or_else(|| anyhow::anyhow!("log({fmt}) needs {{..}}"))?;
    let data = build_struct(rt, body)?;
    let content = match fmt {
        "json" => serde_json::to_string_pretty(&data)?,
        "yaml" => serde_yaml::to_string(&data)?,
        "toml" => toml::to_string_pretty(&data)?,
        other => bail!("unsupported format: {other}"),
    };
    let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
    write!(file, "{}", content)?;
    Ok(rt.last.clone())
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let raw = match &p.arg {
        Some(Arg::Str(s)) | Some(Arg::Ident(s)) => s,
        _ => bail!("log needs @<path>"),
    };
    if let Some(fmt) = extract_fmt(&p.op) {
        structured_dump(rt, p, fmt, raw)
    } else {
        quick_dump(rt, raw)
    }
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
        let node = router::parse(&fs::read_to_string(&script)? )?;
        let mut rt = Runtime::from_entry(&script)?;
        rt.eval(&node)?;
        let content = fs::read_to_string(base.join("out.json"))?;
        assert!(content.contains("\"hi\""));
        fs::remove_dir_all(base)?;
        Ok(())
    }

    #[test]
    fn writes_structured_yaml() -> Result<()> {
        let base = std::env::temp_dir().join(format!("tgsk_log_struct_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base)?;
        fs::write(base.join("red.tgsk"), "")?;
        let script = base.join("main.tgsk");
        let src = "[log(yaml)@out.yaml]{[key(name)@\"Saryn\"]}";
        fs::write(&script, src)?;
        let node = router::parse(&fs::read_to_string(&script)? )?;
        let mut rt = Runtime::from_entry(&script)?;
        rt.eval(&node)?;
        let content = fs::read_to_string(base.join("out.yaml"))?;
        assert!(content.contains("name: Saryn"));
        fs::remove_dir_all(base)?;
        Ok(())
    }
}
