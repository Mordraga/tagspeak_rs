use std::collections::HashMap;
use crate::packets::{math, print, store::{self, Var, VarKind}};

/// Run a full packet chain with a fresh state (used by main.rs)
pub fn route(packet_chain: &str) {
    let mut vars = HashMap::new();
    route_with_vars(packet_chain, &mut vars);
}

/// Run a packet chain with shared state (used by interpreter.rs)
pub fn route_with_vars(packet_chain: &str, vars: &mut HashMap<String, Var>) {
    let packets: Vec<&str> = packet_chain.split('>').collect();

    let mut result = match run_packet(packets[0].trim(), None, vars) {
        Some(res) => res,
        None => return,
    };

    for pkt in packets.iter().skip(1) {
        result = match run_packet(pkt.trim(), Some(&result), vars) {
            Some(res) => res,
            None => return,
        };
    }
}

/// Core packet runner: parses + executes a single packet
fn run_packet(packet: &str, input: Option<&str>, vars: &mut HashMap<String, Var>) -> Option<String> {
    if !packet.starts_with('[') || !packet.ends_with(']') {
        println!("(error) invalid packet format: {}", packet);
        return None;
    }

    let inner = &packet[1..packet.len() - 1];

    // [action:modifier@target] (e.g., store:fluid@x)
    if let Some((action, rest)) = inner.split_once(':') {
        return match handle_action_packet(action, rest, input, vars) {
            Ok(val) => Some(val),
            Err(e) => {
                println!("(error) {}", e);
                None
            }
        };
    }

    // [op@arg] or [op]
    handle_simple_packet(inner, input, vars)
}

/// Executes action packets with modifiers (e.g. store:fluid@x)
fn handle_action_packet(
    action: &str,
    rest: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
) -> Result<String, String> {
    match action {
        "store" => {
            let (kind_str, name) = rest.split_once('@')
                .ok_or(format!("malformed store packet: {}", rest))?;

            let kind = match kind_str {
                "fluid" => VarKind::Fluid,
                "rigid" => VarKind::Rigid,
                _ => return Err(format!("unknown store kind: {}", kind_str)),
            };

            let val = input.ok_or(format!("store:{}@{} has no input", kind_str, name))?;
            store::store_variable(vars, kind, name, val)
        }
        _ => Err(format!("unknown action packet: [{}:{}]", action, rest))
    }
}

/// Handles [op@arg] and [op] style packets
fn handle_simple_packet(
    inner: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
) -> Option<String> {
    if let Some((op, arg)) = inner.split_once('@') {
        match op {
            "math" => math::run(arg),
            "print" => {
                let value = input.unwrap_or(arg);
                print::run(value);
                Some(value.to_string())
            }
            "get" => {
                match vars.get(arg) {
                    Some(var) => Some(var.value.clone()),
                    None => {
                        println!("(warn) variable '{}' not found", arg);
                        None
                    }
                }
            }
            _ => {
                println!("(warn) unknown operation: [{}]", op);
                None
            }
        }
    } else {
        match inner {
            "print" => {
                if let Some(value) = input {
                    print::run(value);
                    Some(value.to_string())
                } else {
                    println!("(warn) [print] has no input");
                    None
                }
            }
            _ => {
                println!("(error) malformed packet: [{}]", inner);
                None
            }
        }
    }
}
