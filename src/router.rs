use std::collections::HashMap;
use crate::packets::{
    r#loop as tagloop,
    math,
    print,
    store::{self, Var, VarKind},
    conditionals,
};

/// Run a packet chain with a fresh state
pub fn route(packet_chain: &str, tag_table: &HashMap<String, String>) {
    let mut vars = HashMap::new();
    route_with_vars(packet_chain, &mut vars, tag_table);
}

/// Run a packet chain with shared vars
pub fn route_with_vars(
    packet_chain: &str,
    vars: &mut HashMap<String, Var>,
    tag_table: &HashMap<String, String>,
) {
    let packets: Vec<&str> = packet_chain
        .split(|c: char| c == '>' || c.is_whitespace())
        .filter(|p| !p.trim().is_empty())
        .collect();

    let mut result = String::new();
    for (i, pkt) in packets.iter().enumerate() {
        let input = if i == 0 { None } else { Some(&result) };
        if let Some(res) = run_packet(pkt, input.map(|s| s.as_str()), vars, tag_table) {
            result = res;
        } else {
            break;
        }
    }
}

/// Parse + execute a single packet
pub fn run_packet(
    packet: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
    tag_table: &HashMap<String, String>,
) -> Option<String> {
    // Ignore standalone braces
    if packet == "{" || packet == "}" {
        return Some(String::new());
    }

    if !packet.starts_with('[') || !packet.ends_with(']') {
        println!("(error) invalid packet format: {}", packet);
        return None;
    }

    let inner = &packet[1..packet.len() - 1];

    // Loops
    if let Some((count, tagname)) = tagloop::parse_loop(packet) {
        tagloop::run(&tagname, count, tag_table, vars);
        return Some(String::new());
    }

    // Action packets
    if let Some((action, rest)) = inner.split_once(':') {
        return handle_action_packet(action, rest, input, vars).ok();
    }

    // Simple operations
    handle_simple_packet(inner, input, vars)
}

/// Handle action packets like store:fluid@x
fn handle_action_packet(
    action: &str,
    rest: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
) -> Result<String, String> {
    match action {
        "store" => {
            let (kind_str, name) = rest
                .split_once('@')
                .ok_or_else(|| format!("malformed store packet: {}", rest))?;

            let kind = match kind_str {
                "fluid" => VarKind::Fluid,
                "rigid" => VarKind::Rigid,
                _ => return Err(format!("unknown store kind: {}", kind_str)),
            };

            let val = input.unwrap_or("").to_string();
            store::store_variable(vars, kind, name, &val)
        }
        _ => Err(format!("unknown action packet: [{}:{}]", action, rest)),
    }
}

/// Handle basic packets
fn handle_simple_packet(
    inner: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
) -> Option<String> {
    if let Some((op, arg)) = inner.split_once('@') {
        match op {
            "math" => math::run(arg, vars),
            "print" => {
                let value = input.unwrap_or_else(|| {
                    vars.get(arg).map(|v| v.value.as_str()).unwrap_or(arg)
                });
                print::run(value);
                Some(value.to_string())
            }
            "get" => vars.get(arg).map(|var| var.value.clone()).or_else(|| {
                println!("(warn) variable '{}' not found", arg);
                None
            }),
            "if" | "elif" | "else" => conditionals::run(op, arg, vars),
            _ => {
                println!("(warn) unknown operation: [{}]", op);
                None
            }
        }
    } else {
        match inner {
            "print" => input.map(|value| {
                print::run(value);
                value.to_string()
            }),
            _ => {
                println!("(error) malformed packet: [{}]", inner);
                None
            }
        }
    }
}
