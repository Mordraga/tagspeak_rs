// loop.rs
use std::collections::HashMap;

use crate::packets::store::{Var, VarKind, store_variable};
use crate::router::route_with_vars;

/// Run a loop by tag name, N times
pub fn run(
    tag_name: &str,
    count: usize,
    tag_table: &HashMap<String, String>,
    vars: &mut HashMap<String, Var>,
) {
    if let Some(tag_content) = tag_table.get(tag_name) {
        for _ in 0..count {
            route_with_vars(tag_content, vars, tag_table);
        }
    } else {
        eprintln!("loop error: tag '{}' not found", tag_name);
    }
}

/// Parse loop packet like [loop3@tagname]
pub fn parse_loop(packet: &str) -> Option<(usize, String)> {
    let content = packet.trim_matches(&['[', ']'][..]);
    if let Some((prefix, tagname)) = content.split_once("@") {
        if prefix.starts_with("loop") {
            let count_str = prefix.trim_start_matches("loop");
            if let Ok(count) = count_str.parse::<usize>() {
                return Some((count, tagname.to_string()));
            }
        }
    }
    None
}
