use anyhow::Result;
use std::time::SystemTime;

use crate::kernel::values::Document;
use crate::kernel::{Node, Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let root_path = rt
        .effective_root
        .as_ref()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("E_BOX_REQUIRED: no red.tgsk"))?;

    let json = if let Some(body) = &p.body {
        let mut items: Vec<serde_json::Value> = Vec::new();
        for node in body {
            let v = match node {
                Node::Packet(_) | Node::Block(_) | Node::Chain(_) | Node::If { .. } => {
                    rt.eval(node)?
                }
            };
            items.push(value_to_json(v)?);
        }
        serde_json::Value::Array(items)
    } else if let Some(crate::kernel::ast::Arg::Str(s)) = &p.arg {
        // Sugar: [array@[1,2,3]] or [array@["a","b"]]
        let trimmed = s.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let val: serde_json::Value = serde_json::from_str(trimmed)?;
            if !val.is_array() {
                anyhow::bail!("array_sugar_not_array");
            }
            val
        } else {
            anyhow::bail!("array needs body or @[...] sugar");
        }
    } else {
        anyhow::bail!("array needs body or @[...] sugar");
    };

    let path = root_path.join(&rt.cwd).join("_array.json");
    let doc = Document::new(
        json,
        path,
        String::from("json"),
        SystemTime::now(),
        root_path.clone(),
    );
    Ok(Value::Doc(doc))
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
