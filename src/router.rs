use crate::error_style::{friendly_hint, render_error_box, unexpected_hint};
use crate::interpreter::Scanner;
use crate::kernel::ast::{Arg, Node, Packet};
use crate::kernel::packet_catalog::{is_known_packet, suggest_packet};
use anyhow::{Result as AnyResult, bail};
use std::fmt;

pub fn parse(src: &str) -> Result<Node, ParseError> {
    let mut sc = Scanner::new(src);
    let mut diagnostics = Vec::new();
    let node = parse_chain(&mut sc, &mut diagnostics);
    if diagnostics.is_empty() {
        Ok(node)
    } else {
        Err(ParseError::from_diagnostics(diagnostics))
    }
}

#[derive(Debug, Clone)]
pub struct ParseDiagnostic {
    pub line: usize,
    pub col: usize,
    pub summary: String,
    pub panel: String,
}

#[derive(Debug)]
pub struct ParseError {
    diagnostics: Vec<ParseDiagnostic>,
}

#[allow(dead_code)]
impl ParseError {
    fn sort_key(diag: &ParseDiagnostic) -> (usize, usize) {
        (diag.line, diag.col)
    }

    pub fn from_diagnostics(mut diagnostics: Vec<ParseDiagnostic>) -> Self {
        diagnostics.sort_by(|a, b| {
            Self::sort_key(a)
                .cmp(&Self::sort_key(b))
                .then_with(|| a.summary.cmp(&b.summary))
                .then_with(|| a.panel.cmp(&b.panel))
        });
        diagnostics.dedup_by(|a, b| {
            a.line == b.line && a.col == b.col && a.summary == b.summary && a.panel == b.panel
        });
        Self { diagnostics }
    }

    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<ParseDiagnostic> {
        self.diagnostics
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for diag in &self.diagnostics {
            if !first {
                writeln!(f)?;
                writeln!(f)?;
            }
            first = false;
            writeln!(f, "{}", diag.summary)?;
            writeln!(f, "{}", diag.panel)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

fn parse_chain(sc: &mut Scanner, diagnostics: &mut Vec<ParseDiagnostic>) -> Node {
    let mut nodes = Vec::new();

    loop {
        sc.skip_comments_and_ws();
        if sc.eof() {
            break;
        }

        let loop_start = sc.pos();
        match sc.peek().unwrap() {
            '[' => {
                let packet_start = sc.pos();
                let pkt = match parse_packet(sc) {
                    Ok(pkt) => pkt,
                    Err(err) => {
                        diagnostics.push(compose_packet_error(sc, packet_start, &err.to_string()));
                        resync_after_error(sc, packet_start);
                        continue;
                    }
                };

                if !is_known_packet(pkt.ns.as_deref(), &pkt.op) {
                    diagnostics.push(unknown_packet(sc, packet_start, &pkt));
                    resync_after_error(sc, packet_start);
                    continue;
                }

                if pkt.ns.is_none() && (pkt.op == "if" || pkt.op.starts_with("if(")) {
                    let cond_src =
                        match extract_conditional_arg(sc, &pkt, packet_start, diagnostics, "if") {
                            Some(src) => src,
                            None => continue,
                        };
                    if let Some(node) = parse_if(sc, cond_src, diagnostics) {
                        nodes.push(node);
                    }
                } else {
                    let mut pkt = pkt;
                    sc.skip_comments_and_ws();
                    if sc.peek() == Some(']') {
                        let err_pos = sc.pos();
                        diagnostics.push(plain_error_box(
                            sc,
                            err_pos,
                            "extra closing ']' detected",
                            "Looks like you have a typo here. It's ok. Happens to me also. <3",
                            "Extra closing bracket",
                        ));
                        sc.next();
                        resync_after_error(sc, err_pos);
                        continue;
                    }
                    if sc.peek() == Some('{') {
                        match parse_block(sc, diagnostics) {
                            Some(Node::Block(body)) => pkt.body = Some(body),
                            Some(_) => unreachable!(),
                            None => continue,
                        }
                    }
                    nodes.push(Node::Packet(pkt));
                }
            }
            '{' => {
                if let Some(block) = parse_block(sc, diagnostics) {
                    nodes.push(block);
                }
            }
            '>' => {
                sc.next();
            }
            '#' => {
                sc.skip_comments_and_ws();
                continue;
            }
            '/' if starts_with(sc, "//") || starts_with(sc, "/*") => {
                sc.skip_comments_and_ws();
                continue;
            }
            _ => {
                let err_pos = sc.pos();
                if let Some(comment_start) = comment_line_start(sc, err_pos) {
                    sc.i = comment_start;
                    sc.skip_comments_and_ws();
                    continue;
                }
                diagnostics.push(unexpected(sc, "top-level"));
                resync_after_error(sc, err_pos);
                continue;
            }
        }

        sc.skip_comments_and_ws();
        while sc.peek() == Some('>') {
            sc.next();
        }

        if sc.pos() <= loop_start {
            sc.next();
        }
    }

    Node::Chain(nodes)
}

fn parse_block(sc: &mut Scanner, diagnostics: &mut Vec<ParseDiagnostic>) -> Option<Node> {
    let start = sc.pos();
    let (_inner, span) = match sc.read_until_balanced('{', '}') {
        Ok(res) => res,
        Err(err) => {
            diagnostics.push(compose_block_error(sc, start, &err.to_string()));
            resync_after_error(sc, start);
            return None;
        }
    };
    let mut sub = sc.subscanner(span.start, span.end);
    let sub_node = parse_chain(&mut sub, diagnostics);
    let body = match sub_node {
        Node::Chain(v) => v,
        other => vec![other],
    };
    Some(Node::Block(body))
}

fn parse_packet(sc: &mut Scanner) -> AnyResult<Packet> {
    let (inner, _) = sc.read_until_balanced('[', ']')?;
    let (ns_part, op_part, arg_part) = split_packet_parts(&inner);

    let ns = ns_part.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let op_trimmed = op_part.trim();
    if op_trimmed.is_empty() {
        bail!("empty packet op in [{}]", inner);
    }
    let op = op_trimmed.to_string();

    let arg = if let Some(raw) = arg_part {
        let raw_trimmed = raw.trim();
        if raw_trimmed.is_empty() {
            None
        } else if raw_trimmed.starts_with('"') && raw_trimmed.contains('+') {
            Some(Arg::Str(raw_trimmed.to_string()))
        } else if raw_trimmed.starts_with('"') {
            let mut sc = Scanner::new(raw_trimmed);
            let s = sc.read_quoted()?;
            Some(Arg::Str(s))
        } else if raw_trimmed.starts_with('(') {
            Some(Arg::CondSrc(raw_trimmed.to_string()))
        } else if let Ok(n) = raw_trimmed.parse::<f64>() {
            Some(Arg::Number(n))
        } else if is_ident_like(raw_trimmed) {
            Some(Arg::Ident(raw_trimmed.to_string()))
        } else {
            Some(Arg::Str(raw_trimmed.to_string()))
        }
    } else {
        None
    };

    Ok(Packet {
        ns,
        op,
        arg,
        body: None,
    })
}

fn split_packet_parts(inner: &str) -> (Option<&str>, &str, Option<&str>) {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return (None, "", None);
    }

    let mut ns = None;
    let mut op_section = trimmed;
    let mut arg = None;

    if let Some(idx) = find_top_level_delim(trimmed, '@') {
        op_section = trimmed[..idx].trim();
        let arg_slice = trimmed[idx + 1..].trim();
        if !arg_slice.is_empty() {
            arg = Some(arg_slice);
        }
    }

    if let Some(idx) = find_top_level_delim(op_section, ':') {
        let ns_slice = op_section[..idx].trim();
        if !ns_slice.is_empty() {
            ns = Some(ns_slice);
        }
        op_section = op_section[idx + 1..].trim();
    }

    (ns, op_section, arg)
}

fn find_top_level_delim(src: &str, needle: char) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_brack = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in src.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => {
                depth_paren += 1;
                continue;
            }
            ')' => {
                depth_paren = depth_paren.saturating_sub(1);
                continue;
            }
            '[' => {
                depth_brack += 1;
                continue;
            }
            ']' => {
                depth_brack = depth_brack.saturating_sub(1);
                continue;
            }
            '{' => {
                depth_brace += 1;
                continue;
            }
            '}' => {
                depth_brace = depth_brace.saturating_sub(1);
                continue;
            }
            _ => {}
        }

        if depth_paren == 0 && depth_brack == 0 && depth_brace == 0 && ch == needle {
            return Some(idx);
        }
    }
    None
}

fn is_ident_like(s: &str) -> bool {
    let mut it = s.chars();
    match it.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    it.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_if(
    sc: &mut Scanner,
    cond_src: String,
    diagnostics: &mut Vec<ParseDiagnostic>,
) -> Option<Node> {
    use crate::packets::conditionals::parse_cond;
    let cond = parse_cond(&cond_src);

    sc.skip_comments_and_ws();
    if sc.peek() == Some('>') {
        sc.next();
    }
    sc.skip_comments_and_ws();

    let then_start = sc.pos();
    let then_pkt = match parse_packet(sc) {
        Ok(pkt) => pkt,
        Err(err) => {
            diagnostics.push(compose_packet_error(sc, then_start, &err.to_string()));
            resync_after_error(sc, then_start);
            return None;
        }
    };
    if then_pkt.ns.is_some() || then_pkt.op != "then" {
        diagnostics.push(plain_error_box(
            sc,
            then_start,
            "expected [then]",
            "Use [then]{...} after conditional branches.",
            "Conditional needs [then]",
        ));
        resync_after_error(sc, then_start);
        return None;
    }

    sc.skip_comments_and_ws();
    if sc.peek() != Some('{') {
        diagnostics.push(plain_error_box(
            sc,
            sc.pos(),
            "[then] needs block",
            "Add a block `{ ... }` immediately after [then].",
            "Missing [then] block",
        ));
        resync_after_error(sc, sc.pos());
        return None;
    }

    let then_b = match parse_block(sc, diagnostics) {
        Some(Node::Block(body)) => body,
        Some(_) => unreachable!(),
        None => {
            resync_after_error(sc, then_start);
            return None;
        }
    };

    sc.skip_comments_and_ws();
    while sc.peek() == Some('>') {
        sc.next();
        sc.skip_comments_and_ws();
    }

    let else_b = match parse_or_else(sc, diagnostics) {
        Some(nodes) => nodes,
        None => return None,
    };

    Some(Node::If {
        cond,
        then_b,
        else_b,
    })
}

fn parse_or_else(sc: &mut Scanner, diagnostics: &mut Vec<ParseDiagnostic>) -> Option<Vec<Node>> {
    use crate::packets::conditionals::parse_cond;
    sc.skip_comments_and_ws();
    if starts_with(sc, "[or@") || starts_with(sc, "[or(") {
        let or_start = sc.pos();
        let pkt = match parse_packet(sc) {
            Ok(pkt) => pkt,
            Err(err) => {
                diagnostics.push(compose_packet_error(sc, or_start, &err.to_string()));
                resync_after_error(sc, or_start);
                return None;
            }
        };
        let src = match extract_conditional_arg(sc, &pkt, or_start, diagnostics, "or") {
            Some(s) => s,
            None => {
                resync_after_error(sc, or_start);
                return None;
            }
        };
        sc.skip_comments_and_ws();
        if sc.peek() == Some('>') {
            sc.next();
            sc.skip_comments_and_ws();
        }
        let then_start = sc.pos();
        let then_pkt = match parse_packet(sc) {
            Ok(pkt) => pkt,
            Err(err) => {
                diagnostics.push(compose_packet_error(sc, then_start, &err.to_string()));
                resync_after_error(sc, then_start);
                return None;
            }
        };
        if then_pkt.ns.is_some() || then_pkt.op != "then" {
            diagnostics.push(plain_error_box(
                sc,
                then_start,
                "expected [then]",
                "Use [then]{...} after conditional branches.",
                "Conditional needs [then]",
            ));
            resync_after_error(sc, then_start);
            return None;
        }
        sc.skip_comments_and_ws();
        if sc.peek() != Some('{') {
            diagnostics.push(plain_error_box(
                sc,
                sc.pos(),
                "[then] needs block",
                "Add a block `{ ... }` immediately after [then].",
                "Missing [then] block",
            ));
            resync_after_error(sc, sc.pos());
            return None;
        }
        let then_b = match parse_block(sc, diagnostics) {
            Some(Node::Block(body)) => body,
            Some(_) => unreachable!(),
            None => {
                resync_after_error(sc, then_start);
                return None;
            }
        };
        sc.skip_comments_and_ws();
        while sc.peek() == Some('>') {
            sc.next();
            sc.skip_comments_and_ws();
        }
        let else_b = match parse_or_else(sc, diagnostics) {
            Some(nodes) => nodes,
            None => return None,
        };
        let mut nodes = Vec::new();
        nodes.push(Node::If {
            cond: parse_cond(&src),
            then_b,
            else_b,
        });
        Some(nodes)
    } else if starts_with(sc, "[else]") {
        let else_start = sc.pos();
        let _pkt = match parse_packet(sc) {
            Ok(pkt) => pkt,
            Err(err) => {
                diagnostics.push(compose_packet_error(sc, else_start, &err.to_string()));
                resync_after_error(sc, else_start);
                return None;
            }
        };
        sc.skip_comments_and_ws();
        if sc.peek() == Some('>') {
            sc.next();
            sc.skip_comments_and_ws();
        }
        let then_start = sc.pos();
        let then_pkt = match parse_packet(sc) {
            Ok(pkt) => pkt,
            Err(err) => {
                diagnostics.push(compose_packet_error(sc, then_start, &err.to_string()));
                resync_after_error(sc, then_start);
                return None;
            }
        };
        if then_pkt.ns.is_some() || then_pkt.op != "then" {
            diagnostics.push(plain_error_box(
                sc,
                then_start,
                "expected [then]",
                "Use [then]{...} inside else branches.",
                "Conditional needs [then]",
            ));
            resync_after_error(sc, then_start);
            return None;
        }
        sc.skip_comments_and_ws();
        if sc.peek() != Some('{') {
            diagnostics.push(plain_error_box(
                sc,
                sc.pos(),
                "[then] needs block",
                "Add a block `{ ... }` immediately after [then].",
                "Missing [then] block",
            ));
            resync_after_error(sc, sc.pos());
            return None;
        }
        let block = match parse_block(sc, diagnostics) {
            Some(Node::Block(body)) => body,
            Some(_) => unreachable!(),
            None => {
                resync_after_error(sc, then_start);
                return None;
            }
        };
        Some(block)
    } else {
        Some(Vec::new())
    }
}

fn plain_error_box(
    sc: &Scanner,
    pos: usize,
    detail: &str,
    hint: &str,
    summary: &str,
) -> ParseDiagnostic {
    let (line_text, col, line_no) = line_context_with_number(sc, pos);
    let snippet = line_text.trim_end_matches('\r').replace('\t', "    ");
    let panel = render_error_box(line_no, col, &snippet, hint, detail);
    ParseDiagnostic {
        line: line_no,
        col,
        summary: format!("{summary} on line {line_no}"),
        panel,
    }
}

fn unknown_packet(sc: &Scanner, start: usize, pkt: &Packet) -> ParseDiagnostic {
    let suggestion = suggest_packet(pkt.ns.as_deref(), &pkt.op);
    let detail = match (pkt.ns.as_deref(), suggestion) {
        (Some(ns), Some(s)) => {
            format!(
                "unknown packet op '{ns}:{op}' (did you mean '{s}'?)",
                op = pkt.op
            )
        }
        (Some(ns), None) => format!("unknown packet op '{ns}:{op}'", op = pkt.op),
        (None, Some(s)) => {
            format!(
                "unknown packet op '{op}' (did you mean '{s}'?)",
                op = pkt.op
            )
        }
        (None, None) => format!("unknown packet op '{op}'", op = pkt.op),
    };

    let hint_string = if let Some(s) = suggestion {
        format!("Packet - Try replacing this with '[{s}@...]' or correct the spelling.")
    } else {
        "Packet - Packet labels are case-sensitive. Define it with [funct@name]{...} or double-check the name."
            .to_string()
    };

    let (line_no, col, panel) =
        render_pretty_error(sc, start, &detail, None, Some(hint_string.as_str()));
    ParseDiagnostic {
        line: line_no,
        col,
        summary: format!("Unknown packet on line {}", line_no),
        panel,
    }
}

fn compose_packet_error(sc: &Scanner, start: usize, detail: &str) -> ParseDiagnostic {
    let detail_pos = extract_line_col(detail);
    let (line_no, col, panel) = render_pretty_error(sc, start, detail, detail_pos, None);
    let prefix = if detail.contains("empty packet op") {
        format!("Empty packet op on line {}", line_no)
    } else if let Some(label) = packet_label_hint(sc, start) {
        format!("Malformed {} packet on line {}", label, line_no)
    } else {
        format!("Malformed packet starting on line {}", line_no)
    };
    ParseDiagnostic {
        line: line_no,
        col,
        summary: prefix,
        panel,
    }
}

fn compose_block_error(sc: &Scanner, start: usize, detail: &str) -> ParseDiagnostic {
    let detail_pos = extract_line_col(detail);
    let (line_no, col, panel) = render_pretty_error(sc, start, detail, detail_pos, None);
    let summary = format!("Block parse error on line {}", line_no);
    ParseDiagnostic {
        line: line_no,
        col,
        summary,
        panel,
    }
}

fn extract_conditional_arg(
    sc: &Scanner,
    pkt: &Packet,
    packet_start: usize,
    diagnostics: &mut Vec<ParseDiagnostic>,
    keyword: &str,
) -> Option<String> {
    if pkt.op == keyword {
        match &pkt.arg {
            Some(Arg::CondSrc(s)) => Some(s.clone()),
            _ => {
                diagnostics.push(compose_packet_error(
                    sc,
                    packet_start,
                    &format!("{keyword} needs (cond) or @(cond)"),
                ));
                None
            }
        }
    } else if pkt.op.starts_with(&format!("{keyword}(")) {
        match extract_paren(&pkt.op) {
            Some(src) => Some(src.to_string()),
            None => {
                diagnostics.push(compose_packet_error(
                    sc,
                    packet_start,
                    &format!("{keyword} needs (cond)"),
                ));
                None
            }
        }
    } else {
        diagnostics.push(compose_packet_error(
            sc,
            packet_start,
            &format!("expected {keyword} clause"),
        ));
        None
    }
}

fn starts_with(sc: &Scanner, pat: &str) -> bool {
    let start = sc.pos();
    let end = start + pat.len();
    end <= sc.len() && sc.slice(start, end) == pat.as_bytes()
}

fn comment_line_start(sc: &Scanner, pos: usize) -> Option<usize> {
    let len = sc.len();
    if len == 0 {
        return None;
    }

    let bytes = sc.slice(0, len);
    let mut pos = pos.min(len);
    while pos > 0 {
        let prev_idx = pos - 1;
        let prev = bytes[prev_idx];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        pos -= 1;
    }

    let mut first = pos;
    while first < len {
        match bytes[first] {
            b' ' | b'\t' | b'\r' | b'\n' => first += 1,
            _ => break,
        }
    }

    if first >= len {
        return None;
    }

    let remaining = &bytes[first..len];
    if remaining.starts_with(b"//")
        || remaining.starts_with(b"/*")
        || remaining.first().copied() == Some(b'#')
    {
        Some(first)
    } else {
        None
    }
}

fn unexpected(sc: &Scanner, where_: &str) -> ParseDiagnostic {
    let (line_text, col, line_no) = line_context_with_number(sc, sc.pos());
    let snippet = line_text.trim_end_matches('\r').replace('\t', "    ");
    let offending = sc.peek().unwrap_or('?');
    let detail = format!(
        "unexpected character at {where_}: '{}' at {line_no}:{col}",
        offending
    );
    let hint = unexpected_hint(offending, where_);
    let panel = render_error_box(line_no, col, &snippet, &hint, &detail);
    ParseDiagnostic {
        line: line_no,
        col,
        summary: format!("Unexpected character near {where_} on line {line_no}"),
        panel,
    }
}

// --- helpers for packet extraction ---

fn packet_label_hint(sc: &Scanner, start: usize) -> Option<String> {
    let bytes = sc.slice(start, sc.len());
    let preview = String::from_utf8_lossy(bytes);
    let first_line = preview.lines().next()?.trim_start();
    if !first_line.starts_with('[') {
        return None;
    }
    let after = &first_line[1..];
    if after.is_empty() {
        return None;
    }
    let mut end = after.len();
    for (idx, ch) in after.char_indices() {
        if matches!(ch, '@' | '(' | ']' | '>' | '{' | ' ' | '\t' | '\r') {
            end = idx;
            break;
        }
    }
    let raw = after[..end].trim();
    if raw.is_empty() {
        return None;
    }
    let tail = raw.rsplit(':').next().unwrap_or(raw);
    Some(capitalize_first(tail))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => {
            let mut out = String::new();
            for c in first.to_uppercase() {
                out.push(c);
            }
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

fn render_pretty_error(
    sc: &Scanner,
    start: usize,
    detail: &str,
    detail_pos: Option<(usize, usize)>,
    hint_override: Option<&str>,
) -> (usize, usize, String) {
    let (packet_line, packet_col, packet_line_no) = line_context_with_number(sc, start);
    let (line_text, col, line_no) = if let Some((detail_line, detail_col)) = detail_pos {
        if detail_line >= packet_line_no {
            if let Some((line, _)) = line_by_number(sc, detail_line) {
                if detail_line > packet_line_no && line.trim().is_empty() {
                    (packet_line, packet_col, packet_line_no)
                } else {
                    (line, detail_col, detail_line)
                }
            } else {
                (packet_line, packet_col, packet_line_no)
            }
        } else {
            (packet_line, packet_col, packet_line_no)
        }
    } else {
        (packet_line, packet_col, packet_line_no)
    };

    let snippet = line_text.trim_end_matches('\r').replace('\t', "    ");
    let hint_owned = hint_override
        .map(|h| h.to_string())
        .unwrap_or_else(|| friendly_hint(detail));
    let detail_line = format!("engine says: {detail}");
    let box_msg = render_error_box(line_no, col, &snippet, &hint_owned, &detail_line);
    (line_no, col, box_msg)
}

fn line_by_number(sc: &Scanner, target: usize) -> Option<(String, usize)> {
    let len = sc.len();
    let bytes = sc.slice(0, len);
    let mut line_no = 1usize;
    let mut current_start = 0usize;
    let mut idx = 0usize;

    while idx < len {
        let nl_len = newline_span(bytes, idx, len);
        if nl_len > 0 {
            if line_no == target {
                let line = String::from_utf8_lossy(&bytes[current_start..idx]).to_string();
                return Some((line, current_start));
            }
            line_no += 1;
            let break_end = idx + nl_len;
            current_start = break_end;
            idx = break_end;
        } else {
            idx += 1;
        }
    }

    if line_no == target {
        let line = String::from_utf8_lossy(&bytes[current_start..len]).to_string();
        return Some((line, current_start));
    }

    None
}

fn newline_span(bytes: &[u8], idx: usize, limit: usize) -> usize {
    if idx >= limit {
        return 0;
    }
    match bytes[idx] {
        b'\n' => 1,
        b'\r' => {
            if idx + 1 < limit && bytes[idx + 1] == b'\n' {
                2
            } else {
                1
            }
        }
        _ => 0,
    }
}

fn extract_line_col(detail: &str) -> Option<(usize, usize)> {
    for marker in ["before ", "at "] {
        if let Some(idx) = detail.rfind(marker) {
            let slice = &detail[idx + marker.len()..];
            let coords: String = slice
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ':')
                .collect();
            let mut parts = coords.split(':');
            let line: usize = parts.next()?.parse().ok()?;
            let col: usize = parts.next()?.parse().ok()?;
            return Some((line, col));
        }
    }
    None
}

fn line_context_with_number(sc: &Scanner, pos: usize) -> (String, usize, usize) {
    let len = sc.len();
    let bytes = sc.slice(0, len);
    let capped_pos = pos.min(len);

    let mut line_start = 0usize;
    let mut line_no = 1usize;
    let mut idx = 0usize;
    while idx < capped_pos {
        let nl_len = newline_span(bytes, idx, len);
        if nl_len > 0 {
            let break_end = idx + nl_len;
            if break_end > capped_pos {
                break;
            }
            line_no += 1;
            line_start = break_end;
            idx = break_end;
        } else {
            idx += 1;
        }
    }

    let mut line_end = len;
    idx = capped_pos;
    while idx < len {
        let nl_len = newline_span(bytes, idx, len);
        if nl_len > 0 {
            line_end = idx;
            break;
        }
        idx += 1;
    }

    let line = String::from_utf8_lossy(&bytes[line_start..line_end]).to_string();
    let col = capped_pos.saturating_sub(line_start) + 1;
    (line, col, line_no)
}

/// Return the substring within the first pair of parentheses in an op like
/// `search(path)`.
pub fn extract_paren(op: &str) -> Option<&str> {
    let start = op.find('(')?;
    let end = op.rfind(')')?;
    if end <= start + 1 {
        return None;
    }
    Some(&op[start + 1..end])
}

/// Parse a source snippet containing exactly one packet and return that packet.
pub fn parse_single_packet(src: &str) -> AnyResult<Packet> {
    match parse(src).map_err(anyhow::Error::new)? {
        Node::Packet(p) => Ok(p),
        Node::Chain(mut v) if v.len() == 1 => {
            if let Node::Packet(p) = v.remove(0) {
                Ok(p)
            } else {
                bail!("expected packet")
            }
        }
        _ => bail!("expected packet"),
    }
}

fn resync_after_error(sc: &mut Scanner, origin: usize) {
    let limit = sc.limit();
    let mut pos = if sc.pos() >= limit {
        origin.saturating_add(1).min(limit)
    } else {
        origin.saturating_add(1).max(sc.pos())
    };
    while pos < limit {
        match sc.char_at(pos) {
            Some('\n') => {
                sc.i = pos + 1;
                return;
            }
            Some('\r') => {
                sc.i = pos + 1;
                return;
            }
            Some('[') | Some('{') | Some('>') => {
                sc.i = pos;
                return;
            }
            Some('#') => {
                sc.i = pos;
                return;
            }
            Some('/') => {
                if pos + 1 < limit {
                    if let Some(next) = sc.char_at(pos + 1) {
                        if next == '/' || next == '*' {
                            sc.i = pos;
                            return;
                        }
                    }
                }
            }
            _ => {}
        }
        pos += 1;
    }
    sc.i = limit;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collects_multiple_errors() {
        let src = "[math@1+1]]\n[if]{[then]{[print]}}\n";
        let err = parse(src).expect_err("expected parse failure with diagnostics");
        let msg = err.to_string();
        assert!(
            msg.contains("extra closing ']' detected"),
            "missing bracket diag:\n{msg}"
        );
        assert!(
            msg.contains("if needs (cond) or @(cond)"),
            "missing conditional diag:\n{msg}"
        );
    }

    #[test]
    fn parse_suppresses_repeated_garbage() {
        let src = "[math@1+1]]]\n";
        let err = parse(src).expect_err("expected parse failure with diagnostics");
        let diag_vec = err.diagnostics().to_vec();
        assert_eq!(
            diag_vec.len(),
            1,
            "expected a single diagnostic, got {}: {:?}",
            diag_vec.len(),
            diag_vec
        );
        let rendered = err.to_string();
        assert!(
            rendered.contains("extra closing ']' detected"),
            "missing extra closing panel:\n{rendered}"
        );
    }

    #[test]
    fn parse_suggests_packet_typo() {
        let src = "[stor@value]";
        let err = parse(src).expect_err("expected parse failure with diagnostics");
        let rendered = err.to_string();
        assert!(
            rendered.contains("did you mean 'store'"),
            "missing suggestion in error output:\n{rendered}"
        );
    }

    #[test]
    fn parse_handles_cr_only_newlines() {
        let src = concat!(
            "[note@Missing opening bracket]\r",
            "print@\"Missing opening bracket\"]\r",
            "\r",
            "// extra context\r"
        );
        let err = parse(src).expect_err("expected parse failure with diagnostics");
        let diagnostics = err.diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "expected a single diagnostic for missing opener, got {:?}",
            diagnostics
        );
        assert!(
            diagnostics[0].summary.contains("Unexpected character"),
            "diagnostic summary missing unexpected character context: {}",
            diagnostics[0].summary
        );
    }

    #[test]
    fn parse_skips_crlf_line_comments() {
        let src = "// heading comment\r\n[print@\"hi\"]\r\n";
        let node = parse(src).expect("expected parse success with CRLF comment");
        match node {
            Node::Packet(_) => {}
            Node::Chain(nodes) => {
                assert_eq!(nodes.len(), 1, "expected single packet node, got {nodes:?}");
            }
            other => panic!("unexpected node shape: {other:?}"),
        }
    }
}
