use std::collections::HashMap;
use crate::packets::{
    r#loop as tagloop,
    math,
    print,
    store::{self, Var, VarKind},
    conditionals,
};

pub fn route(packet_chain: &str, tag_table: &HashMap<String, String>) {
    let mut vars = HashMap::new();
    route_with_vars(packet_chain, &mut vars, tag_table);
}

pub fn route_with_vars(
    packet_chain: &str,
    vars: &mut HashMap<String, Var>,
    tag_table: &HashMap<String, String>,
) {
    let packets = tokenize_packets(packet_chain);

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

/// Tokenize into full [ ... ] or { ... } units, ignoring '>' and whitespace
pub fn tokenize_packets(chain: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut depth_square = 0;
    let mut depth_curly = 0;
    let mut in_string = false;
    let mut chars = chain.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_string = !in_string;
                buf.push(c);
            }
            '[' if !in_string => {
                if depth_square == 0 && depth_curly == 0 && !buf.trim().is_empty() {
                    tokens.push(buf.trim().to_string());
                    buf.clear();
                }
                depth_square += 1;
                buf.push(c);
            }
            ']' if !in_string => {
                buf.push(c);
                depth_square -= 1;
                if depth_square == 0 && depth_curly == 0 {
                    tokens.push(buf.trim().to_string());
                    buf.clear();
                }
            }
            '{' if !in_string => {
                if depth_curly == 0 && depth_square == 0 && !buf.trim().is_empty() {
                    tokens.push(buf.trim().to_string());
                    buf.clear();
                }
                depth_curly += 1;
                buf.push(c);
            }
            '}' if !in_string => {
                buf.push(c);
                depth_curly -= 1;
                if depth_curly == 0 && depth_square == 0 {
                    tokens.push(buf.trim().to_string());
                    buf.clear();
                }
            }
            '>' if !in_string => {
                // ignore cosmetic >
            }
            _ => buf.push(c),
        }
    }

    if !buf.trim().is_empty() {
        tokens.push(buf.trim().to_string());
    }
    tokens
}


pub fn run_packet(
    packet: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
    tag_table: &HashMap<String, String>,
) -> Option<String> {
    if packet == "{" || packet == "}" {
        return Some(String::new());
    }

    if !packet.starts_with('[') || !packet.ends_with(']') {
        println!("(error) invalid packet format: {}", packet);
        return None;
    }

    let inner = &packet[1..packet.len() - 1];

    if let Some((count, tagname)) = tagloop::parse_loop(packet) {
        tagloop::run(&tagname, count, tag_table, vars);
        return Some(String::new());
    }

    if let Some((action, rest)) = inner.split_once(':') {
        return handle_action_packet(action, rest, input, vars, tag_table).ok();
    }

    handle_simple_packet(inner, input, vars, tag_table)
}

fn handle_action_packet(
    action: &str,
    rest: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
    _tag_table: &HashMap<String, String>,
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

fn handle_simple_packet(
    inner: &str,
    input: Option<&str>,
    vars: &mut HashMap<String, Var>,
    tag_table: &HashMap<String, String>,
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
            "if" | "elif" | "else" => conditionals::run(op, arg, vars, tag_table),
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
