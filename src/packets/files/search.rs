use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value as YamlValue;
use toml::Value as TomlValue;

use crate::kernel::Runtime;
use crate::kernel::ast::{Arg, Node, Packet};
use crate::kernel::fs_guard::resolve;
use crate::kernel::values::{Document, Value};
use crate::router; // for parsing helpers

/// Opens a file, detects its format, and returns the value at the requested path.
///
/// * `.tgsk` files expect the arg to contain a single packet snippet
///   like `"[chem:sodium]"`.
/// * JSON/YAML/TOML files expect a dot-separated key path such as
///   `"chem.sodium"`. Array indices are supported.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Parse path from op like search(path)
    let raw_path =
        router::extract_paren(&p.op).ok_or_else(|| anyhow::anyhow!("search needs (path)"))?;

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no red.tgsk root"))?;

    let rel = if raw_path.starts_with('/') {
        &raw_path[1..]
    } else {
        raw_path
    };
    let candidate = if raw_path.starts_with('/') {
        Path::new(rel).to_path_buf()
    } else {
        rt.cwd.join(rel)
    };
    let path = resolve(root, &candidate)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let content = fs::read_to_string(&path)?;

    match ext.as_str() {
        // TagSpeak file: search for a packet snippet
        "tgsk" | "" => {
            let ast = router::parse(&content)?;
            let label_src = match &p.arg {
                Some(Arg::Str(s)) => s,
                _ => bail!("search needs @\"[packet]\""),
            };
            let target = router::parse_single_packet(label_src)?;
            let pkt = find_packet(&ast, &target.ns, &target.op)
                .ok_or_else(|| anyhow::anyhow!("packet_not_found"))?;
            if let Some(arg) = &pkt.arg {
                rt.resolve_arg(arg)
            } else if let Some(body) = &pkt.body {
                rt.eval(&Node::Block(body.clone()))
            } else {
                Ok(Value::Unit)
            }
        }
        // Structured data: traverse by key path
        "json" | "yaml" | "yml" | "toml" => {
            let json_val: serde_json::Value = match ext.as_str() {
                "json" => serde_json::from_str(&content)?,
                "yaml" | "yml" => {
                    let yv: YamlValue = serde_yaml::from_str(&content)?;
                    serde_json::to_value(yv)?
                }
                "toml" => {
                    let tv: TomlValue = toml::from_str(&content)?;
                    serde_json::to_value(tv)?
                }
                _ => unreachable!(),
            };

            let key_path = match &p.arg {
                Some(Arg::Str(s)) => s,
                _ => bail!("search needs @\"key.path\""),
            };
            let val = traverse_json(&json_val, key_path)
                .ok_or_else(|| anyhow::anyhow!("path_not_found"))?;
            json_to_value(val, &path, &ext, root)
        }
        other => bail!(format!("unsupported_ext:{other}")),
    }
}

fn find_packet<'a>(node: &'a Node, ns: &Option<String>, op: &str) -> Option<&'a Packet> {
    match node {
        Node::Packet(pkt) => {
            if &pkt.ns == ns && pkt.op == op {
                Some(pkt)
            } else {
                None
            }
        }
        Node::Chain(list) | Node::Block(list) => {
            for n in list {
                if let Some(p) = find_packet(n, ns, op) {
                    return Some(p);
                }
            }
            None
        }
        Node::If { then_b, else_b, .. } => {
            for n in then_b {
                if let Some(p) = find_packet(n, ns, op) {
                    return Some(p);
                }
            }
            for n in else_b {
                if let Some(p) = find_packet(n, ns, op) {
                    return Some(p);
                }
            }
            None
        }
    }
}

fn traverse_json<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = value;
    if path.is_empty() {
        return Some(cur);
    }
    for seg in path.split('.') {
        cur = match cur {
            serde_json::Value::Object(map) => map.get(seg)?,
            serde_json::Value::Array(list) => {
                let idx: usize = seg.parse().ok()?;
                list.get(idx)?
            }
            _ => return None,
        };
    }
    Some(cur)
}

fn json_to_value(value: &serde_json::Value, path: &Path, ext: &str, root: &Path) -> Result<Value> {
    Ok(match value {
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => Value::Num(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let meta = fs::metadata(path)?;
            let mtime = meta.modified()?;
            let doc = Document::new(
                value.clone(),
                PathBuf::from(path),
                ext.to_string(),
                mtime,
                PathBuf::from(root),
            );
            Value::Doc(doc)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn search_tagspeak_packet() {
        let base = std::env::temp_dir().join(format!("tgsk_search_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(base.join("chem.tgsk"), "[chem:sodium]{[int@11]}").unwrap();
        let script = base.join("sub/main.tgsk");
        fs::write(&script, "[search(/chem.tgsk)@\"[chem:sodium]\"]>[store@na]").unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        rt.eval(&ast).unwrap();

        match rt.get_var("na") {
            Some(Value::Num(n)) => assert_eq!(n as i32, 11),
            other => panic!("unexpected value: {:?}", other),
        }

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn search_json_value() {
        let base =
            std::env::temp_dir().join(format!("tgsk_search_json_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        fs::write(
            base.join("chem.json"),
            "{\"chem\":{\"sodium\":{\"atomic_number\":11}}}",
        )
        .unwrap();
        let script = base.join("sub/main.tgsk");
        fs::write(
            &script,
            "[search(/chem.json)@\"chem.sodium.atomic_number\"]>[store@na]",
        )
        .unwrap();

        let ast = crate::router::parse(&fs::read_to_string(&script).unwrap()).unwrap();
        let mut rt = Runtime::from_entry(&script).unwrap();
        rt.eval(&ast).unwrap();

        match rt.get_var("na") {
            Some(Value::Num(n)) => assert_eq!(n as i32, 11),
            other => panic!("unexpected value: {:?}", other),
        }

        fs::remove_dir_all(base).unwrap();
    }
}
