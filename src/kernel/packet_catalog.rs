pub const KNOWN_PACKET_OPS: &[&str] = &[
    "note", "math", "store", "print", "dump", "call", "funct", "msg", "int", "bool", "env", "help", "lint",
    "cd", "len", "rand", "array", "obj", "reflect", "load", "search", "log", "save", "mod", "exec",
    "run", "tagspeak", "yellow", "confirm", "red", "http", "repl", "parse", "get", "exists",
    "iter", "input", "eq", "ne", "lt", "le", "gt", "ge", "if", "then", "else", "or", "comp",
    "comp!", "merge", "del", "ins", "push", "set", "remove", "append", "delete",
    // UI leaf packets
    "label", "button", "textedit", "textbox", "popup", "separator", "spacer", "checkbox", "app", "scope",
];

pub fn is_known_packet(ns: Option<&str>, op: &str) -> bool {
    if let Some(namespace) = ns {
        let ns_lower = namespace.to_ascii_lowercase();
        if matches!(
            ns_lower.as_str(),
            "store" | "loop" | "funct" | "tagspeak" | "yellow" | "cmp" | "input"
        ) {
            return true;
        }
        // treat custom namespaces (e.g., chem:sodium) as user-defined packets
        return true;
    }

    let op_lower = op.to_ascii_lowercase();

    if KNOWN_PACKET_OPS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&op_lower))
    {
        return true;
    }

    op_lower.starts_with("rand(")
        || op_lower.starts_with("reflect(")
        || op_lower.starts_with("search(")
        || op_lower.starts_with("log")
        || op_lower.starts_with("exec(")
        || op_lower.starts_with("http(")
        || op_lower.starts_with("parse(")
        || op_lower.starts_with("layout(")
        || op_lower.starts_with("tagspeak ")
        || op_lower.starts_with("if(")
        || op_lower.starts_with("or(")
        || op_lower.starts_with("mod(")
        || op_lower.starts_with("loop")
        || op_lower.starts_with("comp")
        || op_lower.starts_with("merge")
        || op_lower.starts_with("del")
        || op_lower.starts_with("ins")
        || op_lower.starts_with("push")
        || op_lower.starts_with("set")
        || op_lower.starts_with("remove")
        || op_lower.starts_with("append")
        || op_lower.starts_with("delete")
        || op_lower.starts_with("get(")
        || op_lower.starts_with("exists(")
        || op_lower.starts_with("key(")
        || op_lower.starts_with("sect(")
}

pub fn suggest_packet(ns: Option<&str>, op: &str) -> Option<&'static str> {
    if let Some(namespace) = ns {
        if namespace.eq_ignore_ascii_case("yellow") {
            return Some("yellow");
        }
    }

    let op_norm = op.to_ascii_lowercase();
    let mut best: Option<&'static str> = None;
    let mut best_score = usize::MAX;

    for candidate in KNOWN_PACKET_OPS {
        let score = edit_distance(&op_norm, candidate);
        if score < best_score {
            best_score = score;
            best = Some(*candidate);
        }
    }

    if best_score <= 2 { best } else { None }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b_chars.len() + 1]; a_chars.len() + 1];

    for i in 0..=a_chars.len() {
        dp[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        dp[0][j] = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[a_chars.len()][b_chars.len()]
}
