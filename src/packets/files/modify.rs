use crate::kernel::Runtime;
use crate::kernel::ast::{Arg, Node, Packet};
use crate::kernel::values::{Document, Value};
use anyhow::{Result, anyhow, bail};
use serde_json::Value as JsonValue;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let options = parse_mod_options(&p.op)?;
    let handle = match &p.arg {
        Some(Arg::Ident(id)) => id.as_str(),
        _ => bail!("mod needs @<ident>"),
    };
    let body = p.body.as_ref().ok_or_else(|| anyhow!("mod needs body"))?;
    let mut doc = match rt.get_var(handle) {
        Some(Value::Doc(d)) => d,
        _ => bail!("handle_unknown"),
    };
    let before = options.debug.then(|| doc.clone());

    for node in body {
        if let Node::Packet(pkt) = node {
            apply_edit(rt, &mut doc, pkt, &options)?;
        }
    }

    if let Some(prev) = before
        && prev.json != doc.json
            && let (Ok(before_s), Ok(after_s)) = (
                serde_json::to_string_pretty(&prev.json),
                serde_json::to_string_pretty(&doc.json),
            ) {
                println!("[mod(debug)] before:\n{before_s}");
                println!("[mod(debug)] after:\n{after_s}");
            }

    rt.set_var(handle, Value::Doc(doc.clone()))?;
    Ok(Value::Doc(doc))
}

fn apply_edit(rt: &Runtime, doc: &mut Document, pkt: &Packet, options: &ModOptions) -> Result<()> {
    let cmd = parse_op(&pkt.op)?;
    let segments = parse_path(&cmd.path)?;
    match cmd.name.as_str() {
        "comp" => {
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("comp needs value"))?,
            )?;
            let create = options.force_overwrite;
            set_value(&mut doc.json, &segments, val, create, true)?;
        }
        "comp!" => {
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("comp! needs value"))?,
            )?;
            set_value(&mut doc.json, &segments, val, true, true)?;
        }
        "merge" => {
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("merge needs value"))?,
            )?;
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
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("ins needs value"))?,
            )?;
            set_value(&mut doc.json, &segments, val, false, false)?;
        }
        "push" => {
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("push needs value"))?,
            )?;
            let target = navigate(&mut doc.json, &segments, true)?;
            if !target.is_array() {
                bail!("not_array");
            }
            target.as_array_mut().unwrap().push(val);
        }
        "set" => {
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("set needs value"))?,
            )?;
            let overwrite = match cmd.modifier.as_deref() {
                Some("missing") => false,
                Some("overwrite") => true,
                None => true,
                Some(other) => bail!("unknown set modifier '{other}'"),
            };
            let overwrite = overwrite || options.force_overwrite;
            if !overwrite && path_exists_read(&doc.json, &segments) {
                return Ok(());
            }
            set_value(&mut doc.json, &segments, val, true, overwrite)?;
        }
        "remove" | "delete" => {
            delete(&mut doc.json, &segments)?;
        }
        "append" => {
            let val = arg_to_json(
                rt,
                pkt.arg
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("append needs value"))?,
            )?;
            let target = navigate(&mut doc.json, &segments, true)?;
            if !target.is_array() {
                bail!("not_array");
            }
            target.as_array_mut().unwrap().push(val);
        }
        other => bail!("unknown edit op: {other}"),
    }
    Ok(())
}

fn parse_op(op: &str) -> Result<EditCommand> {
    let start = op
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("edit missing ("))?;
    let end = op
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("edit missing )"))?;
    let name = op[..start].to_string();
    let inner = op[start + 1..end].trim();
    if inner.is_empty() {
        bail!("edit missing path");
    }
    let mut parts = inner.splitn(2, ',');
    let path = parts
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("edit missing path"))?;
    let modifier = parts
        .next()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());
    Ok(EditCommand {
        name,
        path,
        modifier,
    })
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
                for ch in chars.by_ref() {
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

fn navigate<'a>(
    root: &'a mut JsonValue,
    segs: &[Segment],
    create: bool,
) -> Result<&'a mut JsonValue> {
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
                cur = cur
                    .as_object_mut()
                    .unwrap()
                    .entry(k.clone())
                    .or_insert(JsonValue::Null);
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

fn set_value(
    root: &mut JsonValue,
    segs: &[Segment],
    val: JsonValue,
    create: bool,
    overwrite: bool,
) -> Result<()> {
    if segs.is_empty() {
        bail!("empty path");
    }
    let (head, last) = segs.split_at(segs.len() - 1);
    let parent = navigate(root, head, create)?;
    match last[0].clone() {
        Segment::Key(k) => {
            if !parent.is_object() {
                if create {
                    *parent = JsonValue::Object(Default::default());
                } else {
                    bail!("path_missing");
                }
            }
            let obj = parent.as_object_mut().unwrap();
            if !overwrite && obj.contains_key(&k) {
                bail!("exists");
            }
            obj.insert(k, val);
        }
        Segment::Index(i) => {
            if !parent.is_array() {
                if create {
                    *parent = JsonValue::Array(Vec::new());
                } else {
                    bail!("path_missing");
                }
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
    if segs.is_empty() {
        bail!("empty path");
    }
    let (head, last) = segs.split_at(segs.len() - 1);
    let parent = navigate(root, head, false)?;
    match last[0].clone() {
        Segment::Key(k) => {
            let obj = parent
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("path_missing"))?;
            if obj.remove(&k).is_none() {
                bail!("path_missing");
            }
        }
        Segment::Index(i) => {
            let arr = parent
                .as_array_mut()
                .ok_or_else(|| anyhow::anyhow!("path_missing"))?;
            if i >= arr.len() {
                bail!("path_missing");
            }
            arr.remove(i);
        }
    }
    Ok(())
}

fn path_exists_read(root: &JsonValue, segs: &[Segment]) -> bool {
    let mut cur = root;
    for seg in segs {
        match seg {
            Segment::Key(k) => {
                if let Some(obj) = cur.as_object() {
                    if let Some(next) = obj.get(k) {
                        cur = next;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Segment::Index(i) => {
                if let Some(arr) = cur.as_array() {
                    if let Some(next) = arr.get(*i) {
                        cur = next;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }
    true
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
        Value::Num(n) => JsonValue::Number(
            serde_json::Number::from_f64(n).ok_or_else(|| anyhow::anyhow!("invalid number"))?,
        ),
        Value::Str(s) => serde_json::from_str(&s).unwrap_or(JsonValue::String(s)),
        Value::Doc(d) => d.json,
    })
}

fn arg_to_json(rt: &Runtime, arg: &Arg) -> Result<JsonValue> {
    Ok(match arg {
        Arg::Number(n) => {
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                JsonValue::Number((*n as i64).into())
            } else {
                JsonValue::Number(
                    serde_json::Number::from_f64(*n)
                        .ok_or_else(|| anyhow::anyhow!("invalid number"))?,
                )
            }
        }
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

#[derive(Default)]
struct ModOptions {
    force_overwrite: bool,
    debug: bool,
}

fn parse_mod_options(op: &str) -> Result<ModOptions> {
    let trimmed = op.trim();
    if !trimmed.eq_ignore_ascii_case("mod") && !trimmed.to_ascii_lowercase().starts_with("mod(") {
        bail!("unsupported mod form '{op}'");
    }
    let mut options = ModOptions::default();
    if let Some(inner) = trimmed.strip_prefix("mod(") {
        if !inner.ends_with(')') {
            bail!("malformed mod options");
        }
        let inner = &inner[..inner.len() - 1];
        if inner.trim().is_empty() {
            return Ok(options);
        }
        for token in inner.split(',') {
            let flag = token.trim().to_ascii_lowercase();
            match flag.as_str() {
                "overwrite" => options.force_overwrite = true,
                "debug" => options.debug = true,
                other => bail!("unknown mod option '{other}'"),
            }
        }
    }
    Ok(options)
}

struct EditCommand {
    name: String,
    path: String,
    modifier: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn doc_from_json(value: serde_json::Value) -> Document {
        Document::new(
            value,
            PathBuf::from("doc.json"),
            "json".to_string(),
            SystemTime::now(),
            PathBuf::from("."),
        )
    }

    fn run_mod(script: &str, initial: serde_json::Value) -> serde_json::Value {
        let mut rt = Runtime::new().unwrap();
        let doc = doc_from_json(initial);
        rt.set_var("doc", Value::Doc(doc)).unwrap();
        let ast = router::parse(script).unwrap();
        rt.eval(&ast).unwrap();
        match rt.get_var("doc").unwrap() {
            Value::Doc(doc) => doc.json,
            other => panic!("unexpected value {other:?}"),
        }
    }

    #[test]
    fn set_overwrites_by_default() {
        let after = run_mod(
            "[mod@doc]{[set(user.name)@\"Jen\"]}",
            json!({"user": {"name": "Hal"}}),
        );
        assert_eq!(after["user"]["name"], "Jen");
    }

    #[test]
    fn set_respects_missing_modifier() {
        let after = run_mod(
            "[mod@doc]{[set(user.name, missing)@\"Jen\"]}",
            json!({"user": {"name": "Hal"}}),
        );
        assert_eq!(after["user"]["name"], "Hal");
    }

    #[test]
    fn remove_aliases_del() {
        let after = run_mod(
            "[mod@doc]{[remove(user.name)]}",
            json!({"user": {"name": "Hal","age":19}}),
        );
        assert!(!after["user"].as_object().unwrap().contains_key("name"));
    }

    #[test]
    fn append_aliases_push() {
        let after = run_mod("[mod@doc]{[append(items)@4]}", json!({"items": [1,2,3]}));
        assert_eq!(after["items"], json!([1, 2, 3, 4]));
    }

    #[test]
    fn mod_overwrite_upgrades_comp() {
        let after = run_mod("[mod(overwrite)@doc]{[comp(user.score)@42]}", json!({}));
        assert_eq!(after["user"]["score"], 42);
    }
}
