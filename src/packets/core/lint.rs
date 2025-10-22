use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::kernel::Runtime;
use crate::kernel::ast::{Arg, Node, Packet};
use crate::kernel::fs_guard::resolve;
use crate::kernel::values::Value;

struct LintContext {
    inside_yellow: bool,
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let script = match &p.arg {
        Some(Arg::Str(s)) => resolve_source(rt, s)?,
        Some(Arg::Ident(name)) => match rt.get_var(name) {
            Some(Value::Str(s)) => s,
            Some(_) => {
                return Err(anyhow!(
                    "[lint@{name}] expects the variable to hold script text."
                ));
            }
            None => return Err(anyhow!("[lint@{name}] variable not found.")),
        },
        Some(Arg::Number(_)) => {
            return Err(anyhow!(
                "[lint] expects script text, a variable name, or a path inside the red box."
            ));
        }
        Some(Arg::CondSrc(_)) => {
            return Err(anyhow!(
                "[lint] expects script text, a variable name, or a path inside the red box."
            ));
        }
        None => match &rt.last {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(anyhow!(
                    "[lint] needs script text. Pass it via [lint@\"...\"] or store it in a variable."
                ));
            }
        },
    };

    let mut warnings = collect_todo_warnings(&script);

    match crate::router::parse(&script) {
        Ok(ast) => gather_ast_warnings(&ast, &mut warnings),
        Err(err) => {
            let mut out = String::from("Lint blocked by parse errors:\n");
            let rendered = err.to_string();
            out.push_str(&rendered);
            return Ok(Value::Str(out));
        }
    }

    warnings.sort();
    warnings.dedup();

    if warnings.is_empty() {
        return Ok(Value::Str("Lint clean — no warnings detected.".to_string()));
    }

    let mut out = String::from("Lint findings:\n");
    for warning in warnings {
        out.push_str("- ");
        out.push_str(&warning);
        out.push('\n');
    }
    Ok(Value::Str(out))
}

fn resolve_source(rt: &Runtime, raw: &str) -> Result<String> {
    if raw.contains('\n') || raw.trim().starts_with('[') {
        return Ok(raw.to_string());
    }

    let root = rt
        .effective_root
        .as_ref()
        .ok_or_else(|| anyhow!("No red.tgsk root detected; cannot lint files."))?;

    let trimmed = raw.trim();
    let rel = if trimmed.starts_with('/') {
        &trimmed[1..]
    } else {
        trimmed
    };

    let candidate: PathBuf = if trimmed.starts_with('/') {
        Path::new(rel).to_path_buf()
    } else {
        rt.cwd.join(rel)
    };

    let resolved = resolve(root, &candidate)?;
    let content =
        fs::read_to_string(&resolved).with_context(|| format!("Failed to read {}", rel))?;
    Ok(content)
}

fn collect_todo_warnings(script: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    for (idx, line) in script.lines().enumerate() {
        if line.contains("TODO") || line.contains("FIXME") {
            warnings.push(format!(
                "Line {} contains a TODO/FIXME marker. Convert it into a packet or remove it.",
                idx + 1
            ));
        }
    }
    warnings
}

fn gather_ast_warnings(ast: &Node, warnings: &mut Vec<String>) {
    let mut seen = HashSet::new();
    lint_node(
        ast,
        &LintContext {
            inside_yellow: false,
        },
        warnings,
        &mut seen,
    );
}

fn lint_node(
    node: &Node,
    ctx: &LintContext,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    match node {
        Node::Chain(nodes) | Node::Block(nodes) => {
            for child in nodes {
                lint_node(child, ctx, warnings, seen);
            }
        }
        Node::Packet(pkt) => lint_packet(pkt, ctx, warnings, seen),
        Node::If { then_b, else_b, .. } => {
            for child in then_b {
                lint_node(child, ctx, warnings, seen);
            }
            for child in else_b {
                lint_node(child, ctx, warnings, seen);
            }
        }
    }
}

fn lint_packet(
    pkt: &Packet,
    ctx: &LintContext,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let op_lc = pkt.op.to_ascii_lowercase();
    let ns_lc = pkt.ns.as_deref().map(|ns| ns.to_ascii_lowercase());
    let inside_yellow = ctx.inside_yellow
        || op_lc == "yellow"
        || op_lc == "confirm"
        || ns_lc.as_deref() == Some("yellow");

    match op_lc.as_str() {
        "note" => push_warning(
            seen,
            warnings,
            "Found [note] packet(s). Remove or convert them before shipping production scripts.",
        ),
        "exec" => {
            if !inside_yellow {
                push_warning(
                    seen,
                    warnings,
                    "Found [exec] outside a consent block. Wrap it in [yellow]{ ... } to stay safe.",
                );
            }
        }
        "print" => {
            if let Some(Arg::Str(s)) = &pkt.arg {
                if s.len() > 80 {
                    push_warning(
                        seen,
                        warnings,
                        "Detected a long literal in [print]. Consider using [msg] + [print] or logging to a file.",
                    );
                }
                if s.contains("TODO") || s.contains("FIXME") {
                    push_warning(
                        seen,
                        warnings,
                        "Literal TODO/FIXME inside [print]. Replace with actionable output or remove.",
                    );
                }
            }
        }
        _ => {}
    }

    if let Some(body) = &pkt.body {
        let body_ctx = LintContext { inside_yellow };
        for child in body {
            lint_node(child, &body_ctx, warnings, seen);
        }
    }
}

fn push_warning(seen: &mut HashSet<String>, warnings: &mut Vec<String>, msg: impl Into<String>) {
    let owned = msg.into();
    if seen.insert(owned.clone()) {
        warnings.push(owned);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;

    #[test]
    fn lint_detects_note_packet() {
        let mut rt = Runtime::new().unwrap();
        let packet = router::parse_single_packet("[lint@\"[note@\\\"todo\\\"]>[print]\"]").unwrap();
        let result = handle(&mut rt, &packet).unwrap();
        match result {
            Value::Str(s) => assert!(s.contains("[note]"), "expected note warning, got: {s}"),
            other => panic!("unexpected value: {:?}", other),
        }
    }

    #[test]
    fn lint_reports_clean_script() {
        let mut rt = Runtime::new().unwrap();
        let packet = router::parse_single_packet("[lint@\"[msg@\\\"ok\\\"]>[print]\"]").unwrap();
        let result = handle(&mut rt, &packet).unwrap();
        match result {
            Value::Str(s) => assert!(s.contains("Lint clean")),
            other => panic!("unexpected value: {:?}", other),
        }
    }
}
