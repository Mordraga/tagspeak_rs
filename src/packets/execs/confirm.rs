use anyhow::Result;
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

use crate::kernel::{Packet, Runtime, Value};

static ALLOW: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn is_allowed(key: &str) -> bool {
    if std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "y"))
        .unwrap_or(false)
    {
        return true;
    }
    let set = ALLOW.get_or_init(|| Mutex::new(HashSet::new()));
    set.lock().ok().map(|s| s.contains(key)).unwrap_or(false)
}

fn mark_allowed(key: &str) {
    let set = ALLOW.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut s) = set.lock() {
        s.insert(key.to_string());
    }
}

pub fn prompt_yes_no(msg: &str, env_allow_key: &str) -> Result<bool> {
    if is_allowed(env_allow_key) {
        return Ok(true);
    }

    // Non-interactive opt-out
    if std::env::var("TAGSPEAK_NONINTERACTIVE")
        .map(|v| matches!(v.as_str(), "1" | "true"))
        .unwrap_or(false)
    {
        return Ok(false);
    }

    let mut stdout = io::stdout();
    writeln!(stdout, "[confirm] {msg}")?;
    write!(stdout, "Proceed? [y/N/a] ")?;
    stdout.flush()?;

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return Ok(false);
    }
    let ans = line.trim().to_lowercase();
    if ans == "a" || ans == "always" {
        mark_allowed(env_allow_key);
        return Ok(true);
    }
    Ok(ans == "y" || ans == "yes")
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let message = p
        .arg
        .as_ref()
        .and_then(|a| match a {
            crate::kernel::Arg::Str(s) => Some(s.clone()),
            crate::kernel::Arg::Ident(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "Are you sure you want to continue?".to_string());

    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("[yellow]/[confirm] needs a body"))?;

    if prompt_yes_no(&message, "TAGSPEAK_ALLOW_YELLOW")? {
        // Depth-based yellow gate
        let cur = rt.get_num("__yellow_depth").unwrap_or(0.0);
        rt.set_num("__yellow_depth", cur + 1.0)?;
        let out = rt.eval(&crate::kernel::Node::Block(body.clone()))?;
        rt.set_num("__yellow_depth", cur)?;
        Ok(out)
    } else {
        Ok(Value::Unit)
    }
}

// Sugar: [yellow:exec@"cmd"]
pub fn handle_exec(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let cmd = match p.arg.as_ref() {
        Some(crate::kernel::Arg::Str(s)) => s.clone(),
        Some(crate::kernel::Arg::Ident(s)) => s.clone(),
        _ => "".to_string(),
    };
    let msg = if cmd.is_empty() {
        "Execute external command?".to_string()
    } else {
        format!("Execute external command?\n  cmd: {}", cmd)
    };
    if !prompt_yes_no(&msg, "TAGSPEAK_ALLOW_EXEC")? {
        return Ok(Value::Unit);
    }
    let cur = rt.get_num("__yellow_depth").unwrap_or(0.0);
    rt.set_num("__yellow_depth", cur + 1.0)?;
    let out = crate::packets::exec::handle(rt, p)?;
    rt.set_num("__yellow_depth", cur)?;
    Ok(out)
}

// Sugar: [yellow:run@"/path/file.tgsk"]
pub fn handle_run(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let path_s = match p.arg.as_ref() {
        Some(crate::kernel::Arg::Str(s)) => s.clone(),
        Some(crate::kernel::Arg::Ident(s)) => s.clone(),
        _ => "(unknown)".to_string(),
    };
    let msg = format!("Run TagSpeak script?\n  file: {}", path_s);
    if !prompt_yes_no(&msg, "TAGSPEAK_ALLOW_RUN")? {
        return Ok(Value::Unit);
    }
    let cur = rt.get_num("__yellow_depth").unwrap_or(0.0);
    rt.set_num("__yellow_depth", cur + 1.0)?;
    let out = crate::packets::run::handle(rt, p)?;
    rt.set_num("__yellow_depth", cur)?;
    Ok(out)
}
