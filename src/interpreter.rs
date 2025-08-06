use std::{collections::HashMap, fs, path::Path};
use crate::router;
use crate::packets::store::Var;

/// Read a file and run it
pub fn run_file(path: impl AsRef<Path>) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Could not read file: {}", e))?;
    let lines: Vec<&str> = content.lines().collect();
    run_lines(lines);
    Ok(())
}

/// Execute script lines with shared state + tag table
pub fn run_lines(lines: Vec<&str>) {
    let mut vars: HashMap<String, Var> = HashMap::new();
    let mut tag_table: HashMap<String, String> = HashMap::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        // Detect start of a tag definition (single or multi-line)
        if trimmed.starts_with('[') && trimmed.split_once(':').is_some() {
            if trimmed.ends_with(']') {
                // Single-line tag
                store_tag(trimmed, &mut tag_table);
                i += 1;
            } else {
                // Multi-line tag: read until closing ']'
                let mut buffer = String::new();
                buffer.push_str(trimmed);
                i += 1;

                while i < lines.len() && !lines[i].trim_end().ends_with(']') {
                    if !lines[i].trim().is_empty() && !lines[i].trim().starts_with('#') {
                        buffer.push('>');
                        buffer.push_str(lines[i].trim());
                    }
                    i += 1;
                }

                // Append final closing line
                if i < lines.len() {
                    let close_line = lines[i].trim();
                    if close_line != "]" {
                        buffer.push('>');
                        buffer.push_str(close_line);
                    }
                    i += 1;
                }

                store_tag(&buffer, &mut tag_table);
            }
            continue;
        }

        // Everything else → straight to router
        router::route_with_vars(trimmed, &mut vars, &tag_table);
        i += 1;
    }
}

/// Store a cleaned tag definition into the table
fn store_tag(line: &str, table: &mut HashMap<String, String>) {
    let clean = line.replace("\n", "").replace(">>", ">");
    if clean.starts_with('[') && clean.ends_with(']') {
        let inner = &clean[1..clean.len() - 1];
        if let Some((name, body)) = inner.split_once(':') {
            table.insert(name.trim().to_string(), body.trim().to_string());
        }
    }
}

/// Run a `{...}` block inline and return its result
pub fn interpret_inline(code: &str, vars: &mut HashMap<String, Var>) -> String {
    let tag_table = HashMap::new();
    let packets: Vec<&str> = code
        .split(|c: char| c == '>' || c.is_whitespace())
        .filter(|p| !p.trim().is_empty())
        .collect();

    let mut result = String::new();
    for (i, pkt) in packets.iter().enumerate() {
        let input = if i == 0 { None } else { Some(&result) };
        if let Some(res) =
            crate::router::run_packet(pkt, input.map(|s| s.as_str()), vars, &tag_table)
        {
            result = res;
        }
    }
    result
}
