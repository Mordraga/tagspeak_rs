use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};

use crate::kernel::ast::Arg;
use crate::kernel::runtime::FlowSignal;
use crate::kernel::{Packet, Runtime, Value};

const SKULL_ART: &str = r#"  _____
 /     \
| () () |
 \  ^  /
  |||||
  |||||"#;

const GECKO_BANNER: &str = r#"+===================================================+
| __ _____           ____                   _    __ |
|| _|_   _|_ _  __ _/ ___| _ __   ___  __ _| | _|_ ||
|| |  | |/ _` |/ _` \___ \| '_ \ / _ \/ _` | |/ /| ||
|| |  | | (_| | (_| |___) | |_) |  __/ (_| |   < | ||
|| |  |_|\__,_|\__, |____/| .__/ \___|\__,_|_|\_\| ||
||__|          |___/      |_|                   |__||
+===================================================+"#;

const PLEASE_SNARK: &str = "This isn't going to work just because you begged.";

pub fn handle_power_word_kill(rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    for n in (1..=5).rev() {
        println!("{n}");
        io::stdout().flush().ok();
        if n > 1 {
            thread::sleep(Duration::from_millis(500));
        }
    }

    println!();
    println!("{SKULL_ART}");
    println!();
    println!("Execution terminated by Power Word Kill.");

    rt.set_signal(FlowSignal::Interrupt(Some(Value::Str(
        "Execution terminated by Power Word Kill.".to_string(),
    ))));
    Ok(Value::Unit)
}

pub fn handle_summon(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let thing = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        Some(Arg::Number(n)) => format!("{n}"),
        None => "something indescribable".to_string(),
        _ => "something indescribable".to_string(),
    };
    let msg = format!("You attempt to summon {thing}... nothing happens.");
    println!("{msg}");
    Ok(Value::Str(msg))
}

pub fn handle_gecko(_rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    println!("{GECKO_BANNER}");
    Ok(Value::Str(GECKO_BANNER.to_string()))
}

pub fn handle_please(_rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    println!("{PLEASE_SNARK}");
    Ok(Value::Str(PLEASE_SNARK.to_string()))
}

pub fn handle_please_selene(rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    let echoed = describe_value(&rt.last);
    let msg = if echoed.is_empty() || matches!(rt.last, Value::Unit) {
        "Since you asked nicely~ <3".to_string()
    } else {
        format!("Since you asked nicely~ {echoed} <3")
    };
    println!("{msg}");
    Ok(Value::Str(msg))
}

pub fn handle_deity(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let name_raw = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        Some(Arg::Number(n)) => format!("{n}"),
        None => bail!("Deity packet needs @<name>"),
        _ => bail!("Deity packet needs a simple @<name>"),
    };
    let name = name_raw.trim();
    let msg = match name {
        "" => bail!("Deity packet needs @<name>"),
        n if n.eq_ignore_ascii_case("Astarte") => {
            "Astarte blesses this program. Prepare for war or love.".to_string()
        }
        n if n.eq_ignore_ascii_case("Set") => {
            "Set blesses this program. Prepare for chaos.".to_string()
        }
        other => format!("{other} blesses this program."),
    };
    println!("{msg}");
    Ok(Value::Str(msg))
}

pub fn handle_deadman(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let message = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        Some(Arg::Number(n)) => format!("{n}"),
        None => "Deadman switch armed.".to_string(),
        _ => "Deadman switch armed.".to_string(),
    };
    rt.deadman.arm(message.clone());
    println!("Deadman switch armed.");
    Ok(Value::Str("Deadman switch armed.".to_string()))
}

pub fn handle_disarm(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let target = match p.arg.as_ref() {
        Some(Arg::Str(s)) => Some(s.clone()),
        Some(Arg::Ident(id)) => Some(id.clone()),
        Some(Arg::Number(n)) => Some(format!("{n}")),
        None => None,
        _ => None,
    };

    if let Some(msg) = rt.deadman.disarm(target.as_deref()) {
        let output = format!("Disarmed: {msg}");
        println!("{output}");
        Ok(Value::Str(output))
    } else {
        let output = "No deadman switch to disarm.".to_string();
        println!("{output}");
        Ok(Value::Str(output))
    }
}

pub fn handle_alli(_rt: &mut Runtime, _p: &Packet) -> Result<Value> {
    let msg = "Sarym is a smarty pants. ;p".to_string();
    println!("{msg}");
    Ok(Value::Str(msg))
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Str(s) => s.clone(),
        Value::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::Doc(_) => "<doc>".to_string(),
        Value::Unit => String::new(),
    }
}
