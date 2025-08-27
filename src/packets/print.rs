use crate::kernel::{Packet, Runtime, Value};
use anyhow::Result;
use serde_json;

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let v = match p.arg.as_ref() {
        Some(arg) => rt.resolve_arg(arg)?,
        None => rt.last.clone(),
    };
    println!("{}", pretty(&v));
    Ok(Value::Unit)
}

fn pretty(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Num(n) => format!("{}", n),
        Value::Bool(b) => format!("{}", b),
        Value::Doc(d) => serde_json::to_string_pretty(&d.json).unwrap_or_else(|_| "[doc]".to_string()),
        Value::Unit => String::from("()"),
    }
}
