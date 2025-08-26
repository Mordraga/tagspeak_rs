use anyhow::{bail, Result};
use crate::kernel::ast::{Arg, Node, Packet};
use crate::kernel::values::{Document, Value};
use crate::kernel::Runtime;
use serde_json::Value as JsonValue;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let handle = match &p.arg {
        Some(Arg::Ident(id)) => id.as_str(),
        _ => bail!("mod needs @<ident>"),
    };
    let body = p.body.as_ref().ok_or_else(|| anyhow::anyhow!("mod needs body"))?;
    let mut doc = match rt.get_var(handle) {
        Some(Value::Doc(d)) => d,
        _ => bail!("handle_unknown"),
    };

    for node in body {
        if let Node::Packet(pkt) = node {
            apply_edit(rt, &mut doc, pkt)?;
        }
    }

    rt.set_var(handle, Value::Doc(doc.clone()));
    Ok(Value::Doc(doc))
}

fn apply_edit(rt: &Runtime, doc: &mut Document, pkt: &Packet) -> Result<()> {
    let (op, path) = parse_op(&pkt.op)?;
    let segments = parse_path(&path)?;
    match op.as_str() {
        "comp" => {
            let val = arg_to_json(rt, pkt.arg.as_ref().ok_or_else(|| anyhow::anyhow!("comp needs value"))?)?;
            set_value(&mut doc.json, &segments, val, false, true)?;
        }
        "comp!" => {
            let val = arg_to_json(rt, pkt.arg.as_ref().ok_or_else(|| anyhow::anyhow!("comp! needs value"))?)?;
            set_value(&mut doc.json, &segments, val, true, true)?;
        }
        "merge" => {
            let val = arg_to_json(rt, pkt.arg.as_ref().ok_or_else(|| anyhow::anyhow!("merge needs value"))?)?;
            if !val.is_object() {
                bail!("merge requires object value");
            }
            let target = navigate(&mut doc.json, &segments, true)?;
            deep_merge(target, &val);
        }
        "del" => {
            delete(&mut doc.json, &segments)?;
        }
        "ins" => {
            let val = arg_to_json(rt, pkt.arg.as_ref().ok_or_else(|| anyhow::anyhow!("ins needs value"))?)?;
            set_value(&mut doc.json, &segments, val, false, false)?;
        }
        other => bail!("unknown edit op: {other}"),
    }
    Ok(())
}

fn parse_op(op: &str) -> Result<(String, String)> {
    let start = op.find('(').ok_or_else(|| anyhow::anyhow!("edit missing ("))?;
    let end = op.rfind(')').ok_or_else(|| anyhow::anyhow!("edit missing )"))?;
    let name = op[..start].to_string();
    let path = op[start + 1..end].to_string();
    Ok((name, path))
}

#[derive(Clone)]
enum Segment {
    Key(String),
    Index(usize),
}

fn parse_path(path: &str) -> Result<Vec<Segment>> {
    let mut segs = Vec::new();
    let mut buf = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !buf.is_empty() {
                    segs.push(Segment::Key(buf.clone()));
                    buf.clear();
                }
            }
            '[' => {
                if !buf.is_empty() {
                    segs.push(Segment::Key(buf.clone()));
                    buf.clear();
                }
                let mut num = String::new();
                while let Some(ch) = chars.next() {
                    if ch == ']' { break; }
                    num.push(ch);
                }
                segs.push(Segment::Index(num.parse()?));
            }
            _ => buf.push(c),
        }
    }
    if !buf.is_empty() {
        segs.push(Segment::Key(buf));
    }
    Ok(segs)
}

fn navigate<'a>(root: &'a mut JsonValue, segs: &[Segment], create: bool) -> Result<&'a mut JsonValue> {
    let mut cur = root;
    for seg in segs {
        match seg {
            Segment::Key(k) => {
                if !cur.is_object() {
                    if create {
                        *cur = JsonValue::Object(Default::default());
                    } else {
                        bail!("path_missing");
                    }
                }
                cur = cur.as_object_mut().unwrap().entry(k.clone()).or_insert(JsonValue::Null);
            }
            Segment::Index(i) => {
                if !cur.is_array() {
                    if create {
                        *cur = JsonValue::Array(Vec::new());
                    } else {
                        bail!("path_missing");
                    }
                }
                let arr = cur.as_array_mut().unwrap();
                if *i >= arr.len() {
                    if create {
                        arr.resize(i + 1, JsonValue::Null);
                    } else {
                        bail!("path_missing");
                    }
                }
                cur = &mut arr[*i];
            }
        }
    }
    Ok(cur)
}

fn set_value(root: &mut JsonValue, segs: &[Segment], val: JsonValue, create: bool, overwrite: bool) -> Result<()> {
    if segs.is_empty() { bail!("empty path"); }
    let (head, last) = segs.split_at(segs.len() - 1);
    let parent = navigate(root, head, create)?;
    match last[0].clone() {
        Segment::Key(k) => {
            if !parent.is_object() {
                bail!("path_missing");
            }
            let obj = parent.as_object_mut().unwrap();
            if !overwrite && obj.contains_key(&k) {
                bail!("exists");
            }
            obj.insert(k, val);
        }
        Segment::Index(i) => {
            if !parent.is_array() {
                bail!("path_missing");
            }
            let arr = parent.as_array_mut().unwrap();
            if i >= arr.len() {
                if create {
                    arr.resize(i + 1, JsonValue::Null);
                } else {
                    bail!("path_missing");
                }
            }
            if !overwrite && arr[i] != JsonValue::Null {
                bail!("exists");
            }
            arr[i] = val;
        }
    }
    Ok(())
}

fn delete(root: &mut JsonValue, segs: &[Segment]) -> Result<()> {
    if segs.is_empty() { bail!("empty path"); }
    let (head, last) = segs.split_at(segs.len() - 1);
    let parent = navigate(root, head, false)?;
    match last[0].clone() {
        Segment::Key(k) => {
            let obj = parent.as_object_mut().ok_or_else(|| anyhow::anyhow!("path_missing"))?;
            if obj.remove(&k).is_none() {
                bail!("path_missing");
            }
        }
        Segment::Index(i) => {
            let arr = parent.as_array_mut().ok_or_else(|| anyhow::anyhow!("path_missing"))?;
            if i >= arr.len() {
                bail!("path_missing");
            }
            arr.remove(i);
        }
    }
    Ok(())
}

fn deep_merge(dest: &mut JsonValue, src: &JsonValue) {
    match (dest, src) {
        (JsonValue::Object(a), JsonValue::Object(b)) => {
            for (k, v) in b {
                deep_merge(a.entry(k.clone()).or_insert(JsonValue::Null), v);
            }
        }
        (dest, src) => {
            *dest = src.clone();
        }
    }
}

fn value_to_json(v: Value) -> Result<JsonValue> {
    Ok(match v {
        Value::Unit => JsonValue::Null,
        Value::Bool(b) => JsonValue::Bool(b),
        Value::Num(n) => JsonValue::Number(serde_json::Number::from_f64(n).ok_or_else(|| anyhow::anyhow!("invalid number"))?),
        Value::Str(s) => serde_json::from_str(&s).unwrap_or(JsonValue::String(s)),
        Value::Doc(d) => d.json,
    })
}

fn arg_to_json(rt: &Runtime, arg: &Arg) -> Result<JsonValue> {
    Ok(match arg {
        Arg::Number(n) => JsonValue::Number(serde_json::Number::from_f64(*n).ok_or_else(|| anyhow::anyhow!("invalid number"))?),
        Arg::Str(s) => serde_json::from_str(s).unwrap_or(JsonValue::String(s.clone())),
        Arg::Ident(id) => match id.as_str() {
            "true" => JsonValue::Bool(true),
            "false" => JsonValue::Bool(false),
            "null" => JsonValue::Null,
            other => {
                if let Some(v) = rt.get_var(other) {
                    value_to_json(v)?
                } else {
                    JsonValue::String(other.to_string())
                }
            }
        },
        _ => JsonValue::Null,
    })
}
