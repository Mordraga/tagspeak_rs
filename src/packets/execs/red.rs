use anyhow::Result;
use std::io::{self, Write};

use crate::kernel::{Packet, Runtime, Value};

fn noninteractive() -> bool {
    std::env::var("TAGSPEAK_NONINTERACTIVE")
        .map(|v| matches!(v.as_str(), "1" | "true"))
        .unwrap_or(false)
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Session-wide red enablement. No body required; optional @message to display.
    let message = p
        .arg
        .as_ref()
        .and_then(|a| match a {
            crate::kernel::Arg::Str(s) => Some(s.clone()),
            crate::kernel::Arg::Ident(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "STOP SIGN: entering red mode".to_string());

    if noninteractive() {
        // Noninteractive sessions cannot enable red
        return Ok(Value::Unit);
    }

    let phrase = "I acknowledge red mode. I accept the risk.";
    let mut stdout = io::stdout();
    writeln!(stdout, "[red] {message}")?;
    writeln!(stdout, "Type the following phrase exactly to enable red:")?;
    writeln!(stdout, "  {}", phrase)?;
    write!(stdout, "> ")?;
    stdout.flush()?;

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return Ok(Value::Unit);
    }
    let entered = line.trim_end_matches(['\r', '\n']);
    if entered == phrase {
        rt.set_var("__red_enabled", Value::Bool(true))?;
        writeln!(stdout, "[red] enabled for this session.")?;
        Ok(Value::Bool(true))
    } else {
        writeln!(stdout, "[red] ritual phrase mismatch. Red remains disabled.")?;
        Ok(Value::Bool(false))
    }
}
