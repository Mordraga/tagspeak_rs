use anyhow::{bail, Result};

use crate::kernel::{Node, Packet, Runtime, Value};
use crate::kernel::values::Document;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let handle = match &p.arg {
        Some(crate::kernel::ast::Arg::Ident(id)) => id,
        Some(crate::kernel::ast::Arg::Str(s)) => s,
        _ => bail!("iter needs @<handle>"),
    };
    let body = p.body.as_ref().ok_or_else(|| anyhow::anyhow!("iter needs body"))?;

    let doc = match rt.get_var(handle) {
        Some(Value::Doc(d)) => d,
        _ => bail!("handle_unknown"),
    };
    if !doc.json.is_array() { bail!("not_array"); }
    let arr = doc.json.as_array().unwrap();

    let mut last = Value::Unit;
    for (idx, item) in arr.iter().enumerate() {
        let it_val = json_to_value(item, &doc);
        rt.set_var("it", it_val)?;
        rt.set_var("idx", Value::Num(idx as f64))?;
        last = rt.eval(&Node::Block(body.clone()))?;
    }
    Ok(last)
}

fn json_to_value(v: &serde_json::Value, meta: &Document) -> Value {
    match v {
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => n.as_f64().map(Value::Num).unwrap_or(Value::Unit),
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let mut d = meta.clone();
            d.json = v.clone();
            Value::Doc(d)
        }
    }
}

