use anyhow::{Result, bail};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::kernel::values::Document;
use crate::kernel::{Arg, Node, Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("obj needs body"))?;
    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("E_BOX_REQUIRED: no red.tgsk"))?;

    let obj = build_object_from_body(rt, body)?;
    let path = root.join(&rt.cwd).join("_object.json");
    let doc = Document::new(
        obj,
        PathBuf::from(path),
        String::from("json"),
        SystemTime::now(),
        root.clone(),
    );
    Ok(Value::Doc(doc))
}

fn build_object_from_body(rt: &Runtime, body: &Vec<Node>) -> Result<serde_json::Value> {
    use serde_json::Value as J;
    let mut root = serde_json::Map::new();
    for node in body {
        if let Node::Packet(pkt) = node {
            if let Some(k) = parse_key_name(&pkt.op) {
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
                bail!(format!("unsupported packet in [obj] body: [{}]", pkt.op));
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
                serde_json::Value::Number(serde_json::Number::from((*n as i64)))
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
