use std::collections::HashMap;
use crate::packets::store::Var;

/// Run a conditional packet.
/// - `cmd` is "if", "elif", or "else"
/// - `arg` is `condition{block}` for if/elif, or `{block}` for else
/// - `vars` is the shared variable store
pub fn run(cmd: &str, arg: &str, vars: &mut HashMap<String, Var>) -> Option<String> {
    // Find the start of the block
    if let Some(block_start) = arg.find('{') {
        let condition_str = if cmd == "else" {
            None
        } else {
            Some(arg[..block_start].trim())
        };

        let block = arg[block_start+1..].trim_end_matches('}').trim();

        // Evaluate condition (if/elif) or always true (else)
        let condition_met = match condition_str {
            Some(cond) => evaluate_condition(cond, vars),
            None => true,
        };

        if condition_met {
            // Run block inline so it can return a value
            return Some(crate::interpreter::interpret_inline(block, vars));
        }
    } else {
        println!("(error) malformed conditional: [{}@{}]", cmd, arg);
    }
    None
}

/// Evaluates a condition string with boolean logic
fn evaluate_condition(cond: &str, vars: &HashMap<String, Var>) -> bool {
    // Handle OR
    if cond.contains("||") {
        return cond.split("||").any(|p| evaluate_condition(p.trim(), vars));
    }
    // Handle AND
    if cond.contains("&&") {
        return cond.split("&&").all(|p| evaluate_condition(p.trim(), vars));
    }
    // Handle NOT
    if cond.starts_with('!') {
        return !evaluate_condition(&cond[1..], vars);
    }

    // Supported comparison operators
    let ops = ["==", "!=", ">=", "<=", ">", "<"];
    for op in ops {
        if let Some(idx) = cond.find(op) {
            let left = cond[..idx].trim();
            let right = cond[idx + op.len()..].trim();

            let left_val = vars
                .get(left)
                .map(|v| v.value.clone())
                .unwrap_or_else(|| left.to_string());
            let right_val = vars
                .get(right)
                .map(|v| v.value.clone())
                .unwrap_or_else(|| right.to_string());

            match op {
                "==" => return left_val == right_val,
                "!=" => return left_val != right_val,
                ">"  => return left_val > right_val,
                "<"  => return left_val < right_val,
                ">=" => return left_val >= right_val,
                "<=" => return left_val <= right_val,
                _ => {}
            }
        }
    }
    false
}
