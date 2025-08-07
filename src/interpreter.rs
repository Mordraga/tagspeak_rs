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

        // ===== MULTI-LINE TAG DEFINITION =====
        if trimmed.starts_with('[') && trimmed.split_once(':').is_some() && !trimmed.ends_with(']') {
            let mut buffer = String::new();
            buffer.push_str(trimmed);
            i += 1;

            while i < lines.len() {
                let ln = lines[i].trim();
                if ln == "]" {
                    break; // found closing bracket line
                }
                if !ln.is_empty() && !ln.starts_with('#') {
                    buffer.push(' ');
                    buffer.push_str(ln);
                }
                if ln.ends_with(']') {
                    break; // closing bracket inline
                }
                i += 1;
            }

            // skip final closing bracket line
            if i < lines.len() && lines[i].trim() == "]" {
                i += 1;
            }

            store_tag(&buffer, &mut tag_table);
            continue;
        }

        // ===== SINGLE-LINE TAG DEFINITION =====
        if trimmed.starts_with('[') && trimmed.split_once(':').is_some() && trimmed.ends_with(']') {
            store_tag(trimmed, &mut tag_table);
            i += 1;
            continue;
        }

        // ===== NORMAL PACKET OR BLOCK CHAIN =====
        let clean_line = trimmed.replace('>', " "); // ignore > completely

        if clean_line.contains('{') {
            execute_inline_blocks(&clean_line, &mut vars, &tag_table);
        } else {
            router::route_with_vars(clean_line.trim(), &mut vars, &tag_table);
        }

        i += 1;
    }
}

/// Store a cleaned tag definition into the table
fn store_tag(line: &str, table: &mut HashMap<String, String>) {
    let clean = line.replace("\n", " ").replace(">>", " ").replace('>', " ");
    if clean.starts_with('[') && clean.ends_with(']') {
        let inner = &clean[1..clean.len() - 1];
        if let Some((name, body)) = inner.split_once(':') {
            table.insert(name.trim().to_string(), body.trim().to_string());
        }
    }
}

/// Run any `{ ... }` inline block(s) inside a string
fn execute_inline_blocks(line: &str, vars: &mut HashMap<String, Var>, tag_table: &HashMap<String, String>) {
    let mut start = 0;
    let mut result_line = String::new();

    while let Some(open) = line[start..].find('{') {
        let abs_open = start + open;
        result_line.push_str(&line[start..abs_open]);
        if let Some(close) = find_matching_brace(&line[abs_open..]) {
            let abs_close = abs_open + close;
            let block_code = &line[abs_open + 1..abs_close];
            let res = interpret_inline(block_code.trim(), vars, tag_table);
            result_line.push_str(&res);
            start = abs_close + 1;
        } else {
            break;
        }
    }
    result_line.push_str(&line[start..]);
    router::route_with_vars(result_line.trim(), vars, tag_table);
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Run a `{...}` block inline and return its result
pub fn interpret_inline(
    code: &str,
    vars: &mut HashMap<String, Var>,
    tag_table: &HashMap<String, String>,
) -> String {
    let mut expanded = String::new();
    let mut idx = 0;
    let chars: Vec<char> = code.chars().collect();

    while idx < chars.len() {
        if chars[idx] == '{' {
            // collect block contents
            let mut depth = 1;
            let mut j = idx + 1;
            let mut block = String::new();

            while j < chars.len() && depth > 0 {
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                if depth > 0 {
                    block.push(chars[j]);
                }
                j += 1;
            }

            // run the inner block recursively
            let block_result = interpret_inline(block.trim(), vars, tag_table);
            expanded.push_str(&block_result);

            idx = j + 1; // skip past closing '}'
        } else {
            expanded.push(chars[idx]);
            idx += 1;
        }
    }

    // Tokenize expanded string into full packets using router's tokenizer
    let packets = crate::router::tokenize_packets(&expanded);

    let mut result = String::new();
    for (i, pkt) in packets.iter().enumerate() {
        let input = if i == 0 { None } else { Some(&result) };
        if let Some(res) = crate::router::run_packet(pkt.trim(), input.map(|s| s.as_str()), vars, tag_table) {
            result = res;
        }
    }
    result
}
