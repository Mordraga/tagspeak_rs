use anyhow::Result;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    match p.arg.as_ref() {
        Some(crate::kernel::ast::Arg::Str(s)) => {
            if s.contains('"') {
                if let Some(line) = format_composite(rt, s) {
                    println!("{}", line);
                    return Ok(Value::Unit);
                }
            }
            // Simple string literal or composite parse failed
            println!("{}", s);
            Ok(Value::Unit)
        }
        Some(arg) => {
            let v = rt.resolve_arg(arg)?;
            println!("{}", pretty(&v));
            Ok(Value::Unit)
        }
        None => {
            let v = rt.last.clone();
            println!("{}", pretty(&v));
            Ok(Value::Unit)
        }
    }
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

// Support simple composite printing: tokens of idents and quoted strings
// Example: [print@sq " is the square of " x]
fn format_composite(rt: &Runtime, raw: &str) -> Option<String> {
    let mut i = 0usize;
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut saw = false;

    while i < chars.len() {
        // skip whitespace
        while i < chars.len() && chars[i].is_whitespace() { i += 1; }
        if i >= chars.len() { break; }

        match chars[i] {
            '"' => {
                i += 1; // skip opening quote
                let mut buf = String::new();
                while i < chars.len() {
                    let c = chars[i];
                    i += 1;
                    if c == '\\' {
                        if i < chars.len() { buf.push(chars[i]); i += 1; }
                        continue;
                    }
                    if c == '"' { break; }
                    buf.push(c);
                }
                out.push_str(&buf);
                saw = true;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut ident = String::new();
                ident.push(c);
                i += 1;
                while i < chars.len() {
                    let c2 = chars[i];
                    if c2.is_ascii_alphanumeric() || c2 == '_' { ident.push(c2); i += 1; }
                    else { break; }
                }
                let val = rt.get_var(&ident).unwrap_or(Value::Unit);
                out.push_str(&pretty(&val));
                saw = true;
            }
            _ => {
                // unknown token; bail to default behavior
                return None;
            }
        }
    }

    saw.then_some(out)
}
