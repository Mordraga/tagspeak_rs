use anyhow::Result;
use crate::kernel::{Packet, Runtime, Value};

// [dump] -> pretty-print last value
// [dump@var] -> pretty-print value of variable/arg
// Documents are rendered as pretty JSON to stdout.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let v = match p.arg.as_ref() {
        Some(arg) => rt.resolve_arg(arg)?,
        None => rt.last.clone(),
    };

    match &v {
        Value::Doc(d) => {
            let s = serde_json::to_string_pretty(&d.json)?;
            println!("{}", s);
        }
        Value::Str(s) => println!("{}", s),
        Value::Num(n) => println!("{}", n),
        Value::Bool(b) => println!("{}", b),
        Value::Unit => println!("()"),
    }

    Ok(v)
}

