use std::fmt::Write;

use anyhow::{Result, anyhow};

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

struct HelpEntry {
    names: &'static [&'static str],
    summary: &'static str,
    when: &'static str,
    example: &'static str,
    notes: &'static str,
}

const HELP_ENTRIES: &[HelpEntry] = &[
    HelpEntry {
        names: &["print"],
        summary: "Send the incoming value to stdout.",
        when: "Use at the tail of a chain when you want to see the current value without mutating it.",
        example: "[msg@\"hi there\"]>[print]",
        notes: "Pairs well with [msg] or [dump] when debugging a flow.",
    },
    HelpEntry {
        names: &["store"],
        summary: "Capture the current value under a runtime variable.",
        when: "Call it whenever you want to reuse the value later in the script.",
        example: "[math@5+5]>[store@total]",
        notes: "Stored values are fetched implicitly (e.g. [math@total+2]) or via packets like [dump].",
    },
    HelpEntry {
        names: &["math"],
        summary: "Evaluate a simple math expression.",
        when: "Reach for it when you need quick arithmetic on numbers or stored variables.",
        example: "[math@qty*price]>[store@subtotal]",
        notes: "Supports + - * / and parentheses. Variables resolve against prior [store] packets.",
    },
    HelpEntry {
        names: &["log", "log(json)", "log(yaml)", "log(toml)"],
        summary: "Write structured data to disk inside the red box.",
        when: "Prefer [log] over ad-hoc file writes whenever you want reproducible output.",
        example: "[log(json)@reports/summary.json]{ [key(total)@subtotal] }",
        notes: "Respects the red.tgsk sandbox and auto-formats based on the chosen mode.",
    },
    HelpEntry {
        names: &["exec"],
        summary: "Run a shell command and return its stdout.",
        when: "Only use it when scripts must delegate to external tools and you can secure consent.",
        example: "[yellow@\"Run ls?\"{ [exec@\"ls\"] }]",
        notes: "Always gate exec inside a [yellow] or [confirm] packet to respect safety defaults.",
    },
    HelpEntry {
        names: &["help"],
        summary: "Display short-form documentation for common packets.",
        when: "Run [help@packet] when you need a quick reminder without leaving the editor.",
        example: "[help@print]>[print]",
        notes: "Use [help@*] to list all topics, or [help@lint] to learn about the lint packet.",
    },
    HelpEntry {
        names: &["lint"],
        summary: "Run lightweight heuristics over a TagSpeak script.",
        when: "Use it before shipping a file to catch lingering notes, TODO markers, or unsafe exec usage.",
        example: "[lint@\"[note@\\\"todo\\\"]>[print]\"]",
        notes: "Accepts inline script text, a stored string variable, or a path inside the red box.",
    },
];

pub fn handle(_rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let topic_raw = match &p.arg {
        Some(Arg::Str(s)) => s.trim(),
        Some(Arg::Ident(id)) => id.trim(),
        Some(Arg::Number(_)) => {
            return Err(anyhow!(
                "[help] expects a packet name like [help@print] or [help@*]"
            ));
        }
        Some(Arg::CondSrc(_)) => {
            return Err(anyhow!(
                "[help] expects a packet name, not a conditional expression"
            ));
        }
        None => "",
    };

    let topic = topic_raw.to_ascii_lowercase();
    if topic.is_empty() || topic == "*" || topic == "list" {
        return Ok(Value::Str(render_topic_list()));
    }

    if let Some(entry) = lookup(&topic) {
        return Ok(Value::Str(render_entry(entry)));
    }

    let mut out = String::new();
    writeln!(
        &mut out,
        "No help entry found for '{topic_raw}'. Try one of these topics:"
    )?;
    let mut names: Vec<&str> = HELP_ENTRIES.iter().map(|e| e.names[0]).collect();
    names.sort_unstable();
    for name in names {
        writeln!(&mut out, "- {name}")?;
    }
    Ok(Value::Str(out))
}

fn lookup(topic: &str) -> Option<&'static HelpEntry> {
    HELP_ENTRIES.iter().find(|entry| {
        entry
            .names
            .iter()
            .any(|name| name.to_ascii_lowercase() == topic)
    })
}

fn render_topic_list() -> String {
    let mut names: Vec<&str> = HELP_ENTRIES.iter().map(|entry| entry.names[0]).collect();
    names.sort_unstable();
    let mut out = String::new();
    out.push_str("Available help topics:\n");
    for name in names {
        let _ = writeln!(&mut out, "- {name}");
    }
    out
}

fn render_entry(entry: &HelpEntry) -> String {
    let mut out = String::new();
    let primary = entry.names[0];
    let aliases: Vec<&str> = entry.names.iter().skip(1).copied().collect();

    let _ = writeln!(&mut out, "Packet: [{primary}]");
    if !aliases.is_empty() {
        let joined = aliases.join(", ");
        let _ = writeln!(&mut out, "Aliases: {joined}");
    }
    let _ = writeln!(&mut out, "\nSummary: {}", entry.summary);
    let _ = writeln!(&mut out, "When to use: {}", entry.when);
    let _ = writeln!(&mut out, "Example:\n  {}", entry.example);
    if !entry.notes.is_empty() {
        let _ = writeln!(&mut out, "Notes: {}", entry.notes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;

    #[test]
    fn lists_topics_when_missing_arg() {
        let packet = router::parse_single_packet("[help]").unwrap();
        let mut rt = Runtime::new().unwrap();
        let value = handle(&mut rt, &packet).unwrap();
        match value {
            Value::Str(s) => assert!(s.contains("Available help topics")),
            other => panic!("unexpected value: {:?}", other),
        }
    }

    #[test]
    fn renders_specific_entry() {
        let packet = router::parse_single_packet("[help@print]").unwrap();
        let mut rt = Runtime::new().unwrap();
        let value = handle(&mut rt, &packet).unwrap();
        match value {
            Value::Str(s) => {
                assert!(s.contains("Packet: [print]"));
                assert!(s.contains("Summary"));
            }
            other => panic!("unexpected value: {:?}", other),
        }
    }
}
