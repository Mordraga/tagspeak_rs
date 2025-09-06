use anyhow::{bail, Result};
use serde_json::Value as JsonValue;

use crate::kernel::ast::Arg;
use crate::kernel::values::{Document, Value};
use crate::kernel::{Packet, Runtime};

// Query packets operating on in-memory documents:
// [get(path)@handle]    -> extracts value at path from document variable and returns it
// [exists(path)@handle] -> returns true if path exists in the document
// Path syntax mirrors [mod] (dot keys and [idx] for arrays): e.g., user.name, items[0]
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let (mode, path) = parse_mode_and_path(&p.op)?;
    let handle = match p.arg.as_ref() {
        Some(Arg::Ident(id)) => id,
        _ => bail!("query needs @<handle>"),
    };

    let doc = match rt.get_var(handle) {
        Some(Value::Doc(d)) => d,
        _ => bail!("handle_unknown"),
    };

    let segs = parse_path(&path)?;
    match mode.as_str() {
        "get" => {
            if let Some(v) = navigate_read(&doc.json, &segs) {
                Ok(json_to_value(v, &doc))
            } else {
                Ok(Value::Unit)
            }
        }
        "exists" => Ok(Value::Bool(navigate_read(&doc.json, &segs).is_some())),
        _ => bail!("unknown_query_mode"),
    }
}

fn parse_mode_and_path(op: &str) -> Result<(String, String)> {
    let start = op.find('(').ok_or_else(|| anyhow::anyhow!("query missing ("))?;
    let end = op.rfind(')').ok_or_else(|| anyhow::anyhow!("query missing )"))?;
    let name = op[..start].to_string();
    let path = op[start + 1..end].to_string();
    Ok((name, path))
}

#[derive(Clone)]
enum Segment { Key(String), Index(usize) }

fn parse_path(path: &str) -> Result<Vec<Segment>> {
    // Special-case: a bare numeric path like "0" means index 0
    if !path.contains('.') && !path.contains('[') && path.chars().all(|c| c.is_ascii_digit()) {
        let idx: usize = path.parse()?;
        return Ok(vec![Segment::Index(idx)]);
    }

    let mut segs = Vec::new();
    let mut buf = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !buf.is_empty() { segs.push(Segment::Key(buf.clone())); buf.clear(); }
            }
            '[' => {
                if !buf.is_empty() { segs.push(Segment::Key(buf.clone())); buf.clear(); }
                let mut num = String::new();
                while let Some(ch) = chars.next() { if ch == ']' { break; } num.push(ch); }
                segs.push(Segment::Index(num.parse()?));
            }
            _ => buf.push(c),
        }
    }
    if !buf.is_empty() { segs.push(Segment::Key(buf)); }
    Ok(segs)
}

fn navigate_read<'a>(root: &'a JsonValue, segs: &[Segment]) -> Option<&'a JsonValue> {
    let mut cur = root;
    for seg in segs {
        match seg {
            Segment::Key(k) => {
                let obj = cur.as_object()?;
                cur = obj.get(k)?;
            }
            Segment::Index(i) => {
                let arr = cur.as_array()?;
                cur = arr.get(*i)?;
            }
        }
    }
    Some(cur)
}

fn json_to_value(v: &JsonValue, meta: &Document) -> Value {
    match v {
        JsonValue::Null => Value::Unit,
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Number(n) => n.as_f64().map(Value::Num).unwrap_or(Value::Unit),
        JsonValue::String(s) => Value::Str(s.clone()),
        JsonValue::Array(_) | JsonValue::Object(_) => {
            let mut d = meta.clone();
            d.json = v.clone();
            Value::Doc(d)
        }
    }
}
