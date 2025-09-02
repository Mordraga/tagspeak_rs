use anyhow::Result;
use std::io::{self, Write};

use crate::kernel::{Packet, Runtime, Value};

fn is_noninteractive() -> bool {
    std::env::var("TAGSPEAK_NONINTERACTIVE")
        .map(|v| matches!(v.as_str(), "1" | "true"))
        .unwrap_or(false)
}

pub fn handle(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Prompt text from @arg (string or ident); default to "> "
    let prompt = p
        .arg
        .as_ref()
        .and_then(|a| match a {
            crate::kernel::Arg::Str(s) => Some(s.clone()),
            crate::kernel::Arg::Ident(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "> ".to_string());

    if is_noninteractive() {
        // default-deny in noninteractive mode; return Unit
        return Ok(Value::Unit);
    }

    let mut stdout = io::stdout();
    write!(stdout, "{}", prompt)?;
    stdout.flush()?;

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return Ok(Value::Unit);
    }
    Ok(Value::Str(line.trim_end_matches(['\r', '\n']).to_string()))
}

