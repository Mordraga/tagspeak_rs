use std::collections::HashMap;
use crate::packets::store::Var;

/// Runs a math expression, replacing variable names with their stored values.
/// Missing variables default to 0.
pub fn run(expr: &str, vars: &HashMap<String, Var>) -> Option<String> {
    // Build the expression by replacing variables directly
    let mut replaced = String::new();
    let mut ident = String::new();

    for ch in expr.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else {
            if !ident.is_empty() {
                if let Some(var) = vars.get(&ident) {
                    replaced.push_str(&var.value);
                } else if ident.chars().all(|c| c.is_alphabetic()) {
                    replaced.push('0'); // default for unknown vars
                } else {
                    replaced.push_str(&ident);
                }
                ident.clear();
            }
            replaced.push(ch);
        }
    }

    // Flush any last identifier
    if !ident.is_empty() {
        if let Some(var) = vars.get(&ident) {
            replaced.push_str(&var.value);
        } else if ident.chars().all(|c| c.is_alphabetic()) {
            replaced.push('0');
        } else {
            replaced.push_str(&ident);
        }
    }

    // Now we should have a clean math string like "5+5+0"
    match meval::eval_str(&replaced) {
        Ok(val) => Some(val.to_string()),
        Err(e) => {
            println!("(error) math failed: {}", e);
            None
        }
    }
}
