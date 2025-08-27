mod interpreter;
mod kernel;
mod packets;
mod router;

use kernel::Runtime;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", format_error(&e));
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    // path arg or default
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/smoke.tgsk".to_string());
    println!("Running file: {path}");
    let src = fs::read_to_string(&path)?;

    let ast = router::parse(&src)?;
    let mut rt = Runtime::from_entry(Path::new(&path))?;
    rt.eval(&ast)?;
    Ok(())
}

fn format_error(e: &anyhow::Error) -> String {
    let msg = e.to_string();
    if let Some(name) = msg.strip_prefix("undefined function: ") {
        format!("function '{name}' not defined")
    } else if let Some(tuple) = msg.strip_prefix("unknown operation: ") {
        parse_unknown_op(tuple)
    } else {
        msg
    }
}

fn parse_unknown_op(tuple: &str) -> String {
    // tuple formats: (None, "op") or (Some("ns"), "op")
    if let Some(rest) = tuple.strip_prefix("(Some(\"") {
        if let Some((ns, op_rest)) = rest.split_once("\"), \"") {
            if let Some(op) = op_rest.strip_suffix("\")") {
                return format!("unknown operation: [{ns}:{op}]");
            }
        }
    } else if let Some(rest) = tuple.strip_prefix("(None, \"") {
        if let Some(op) = rest.strip_suffix("\")") {
            return format!("unknown operation: [{op}]");
        }
    }
    tuple.to_string()
}
