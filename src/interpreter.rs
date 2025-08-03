use std::collections::HashMap;
use crate::router;
use crate::packets::store::Var;

pub fn run_file(path: &str) {
    let content = std::fs::read_to_string(path).expect("Could not read file");
    let lines: Vec<&str> = content.lines().collect();
    run_lines(lines);
}

pub fn run_lines(lines: Vec<&str>) {
    let mut vars: HashMap<String, Var> = HashMap::new();

    for line in lines {
        if line.trim().is_empty() { continue; }
        router::route_with_vars(line, &mut vars);
    }
}
