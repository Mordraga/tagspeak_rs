use anyhow::{bail, Result};
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

use crate::kernel::{Packet, Runtime, Value};

fn parse_model(op: &str) -> Option<String> {
    if let Some(rest) = op.strip_prefix("repl(") {
        if let Some(end) = rest.find(')') {
            let raw = &rest[..end];
            let trimmed = raw.trim();
            if !trimmed.is_empty() { return Some(trimmed.to_string()); }
        }
    }
    None
}

fn noninteractive() -> bool {
    std::env::var("TAGSPEAK_NONINTERACTIVE")
        .map(|v| matches!(v.as_str(), "1" | "true"))
        .unwrap_or(false)
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    // Require red mode enabled
    let red = rt
        .get_var("__red_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !red {
        bail!("E_RED_REQUIRED: enable red with [red@\"...\"] first");
    }

    static ACTIVE: OnceLock<Mutex<bool>> = OnceLock::new();
    let slot = ACTIVE.get_or_init(|| Mutex::new(false));
    // Prevent nested or concurrent REPL
    if let Ok(mut flag) = slot.lock() {
        if *flag { bail!("E_REPL_ACTIVE: a REPL is already running"); }
        *flag = true;
    }

    if noninteractive() {
        // REPL requires interaction; default-deny
        // release flag before return
        if let Ok(mut flag) = ACTIVE.get().unwrap().lock() { *flag = false; }
        return Ok(Value::Unit);
    }

    let model = parse_model(&p.op).unwrap_or_else(|| "repl".to_string());
    let prompt_symbol = format!("{}> ", model);

    let mut stdout = io::stdout();
    writeln!(stdout, "[repl] starting (model: {}) — type 'exit' to quit", model)?;
    stdout.flush()?;

    loop {
        // Read one line of input
        write!(stdout, "{}", prompt_symbol)?;
        stdout.flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() { break; }
        let q = line.trim_end_matches(['\r', '\n']).to_string();
        if q.eq_ignore_ascii_case("exit") || q.eq_ignore_ascii_case("quit") { break; }
        if q.is_empty() { continue; }

        // Expose input as variable 'q'
        rt.set_var("q", Value::Str(q.clone()))?;

        // Evaluate body if present, else just echo
        let out = if let Some(b) = &p.body {
            rt.eval(&crate::kernel::Node::Block(b.clone()))?
        } else {
            Value::Str(q)
        };

        // Print the output neatly; also set as last
        match &out {
            Value::Unit => { writeln!(stdout, "(ok)")?; }
            Value::Str(s) => { writeln!(stdout, "{}", s)?; }
            Value::Num(n) => { writeln!(stdout, "{}", n)?; }
            Value::Bool(b) => { writeln!(stdout, "{}", b)?; }
            Value::Doc(_) => { writeln!(stdout, "<doc>")?; }
        }
        stdout.flush()?;
        rt.last = out;
    }
    // release flag when exiting
    if let Ok(mut flag) = ACTIVE.get().unwrap().lock() { *flag = false; }
    Ok(Value::Unit)
}
