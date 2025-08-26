use crate::kernel::{Packet, Runtime, Value};
use anyhow::Result;

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
        Value::Unit => String::from("()"),
    }
}
