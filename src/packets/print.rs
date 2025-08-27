use anyhow::Result;
use crate::kernel::{Packet, Runtime, Value};

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
        Value::Doc(_) => String::from("<doc>"),
        Value::Unit    => String::from("()"),
    }
}
