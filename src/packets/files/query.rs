use anyhow::{Result, bail};
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
            if let Some(v) = navigate_read(rt, &doc.json, &segs) {
                Ok(json_to_value(v, &doc))
            } else {
                Ok(Value::Unit)
            }
        }
        "exists" => Ok(Value::Bool(navigate_read(rt, &doc.json, &segs).is_some())),
        _ => bail!("unknown_query_mode"),
    }
}

fn parse_mode_and_path(op: &str) -> Result<(String, String)> {
    let start = op
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("query missing ("))?;
    let end = op
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("query missing )"))?;
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
                    if ch == ']' {
                        break;
                    }
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

fn navigate_read<'a>(rt: &Runtime, root: &'a JsonValue, segs: &[Segment]) -> Option<&'a JsonValue> {
    let mut cur = root;
    for seg in segs {
        match seg {
            Segment::Key(k) => {
                if let Some(obj) = cur.as_object() {
                    if let Some(next) = obj.get(k) {
                        cur = next;
                        continue;
                    }
                    if let Some(var) = rt.get_var(k) {
                        if let Some(key) = value_to_key(&var) {
                            if let Some(next) = obj.get(&key) {
                                cur = next;
                                continue;
                            }
                        }
                        if let Some(idx) = value_to_index(&var) {
                            let key_name = idx.to_string();
                            if let Some(next) = obj.get(&key_name) {
                                cur = next;
                                continue;
                            }
                        }
                    }
                    return None;
                } else if let Some(arr) = cur.as_array() {
                    if let Ok(idx) = k.parse::<usize>() {
                        cur = arr.get(idx)?;
                        continue;
                    }
                    if let Some(var) = rt.get_var(k) {
                        if let Some(idx) = value_to_index(&var) {
                            cur = arr.get(idx)?;
                            continue;
                        }
                    }
                    return None;
                } else {
                    return None;
                }
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
fn value_to_index(v: &Value) -> Option<usize> {
    match v {
        Value::Num(n) if n.is_finite() && n.fract() == 0.0 && *n >= 0.0 => Some(*n as usize),
        Value::Str(s) => s.trim().parse::<usize>().ok(),
        Value::Bool(b) => Some(if *b { 1 } else { 0 }),
        _ => None,
    }
}

fn value_to_key(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.clone()),
        _ => value_to_index(v).map(|n| n.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, bail};
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn mk_doc(json: serde_json::Value) -> Document {
        Document::new(
            json,
            PathBuf::from("doc.json"),
            "json".into(),
            SystemTime::now(),
            PathBuf::new(),
        )
    }

    #[test]
    fn get_with_variable_index() -> Result<()> {
        let mut rt = Runtime::new()?;
        let doc = mk_doc(serde_json::json!([5, 10, 15]));
        rt.set_var("arr", Value::Doc(doc))?;
        rt.set_var("randIndex", Value::Num(1.0))?;

        let node = crate::router::parse("[get(randIndex)@arr]")?;
        let out = rt.eval(&node)?;
        match out {
            Value::Num(n) => assert_eq!(n, 10.0),
            _ => bail!("expected number"),
        }
        Ok(())
    }

    #[test]
    fn get_with_variable_key() -> Result<()> {
        let mut rt = Runtime::new()?;
        let doc = mk_doc(serde_json::json!({"alpha": 42, "beta": 99}));
        rt.set_var("obj", Value::Doc(doc))?;
        rt.set_var("field", Value::Str("beta".into()))?;

        let node = crate::router::parse("[get(field)@obj]")?;
        let out = rt.eval(&node)?;
        match out {
            Value::Num(n) => assert_eq!(n, 99.0),
            _ => bail!("expected number"),
        }
        Ok(())
    }
}
