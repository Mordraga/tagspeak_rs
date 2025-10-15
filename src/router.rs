use crate::interpreter::Scanner;
use crate::kernel::ast::{Arg, Node, Packet};
use anyhow::{Result, bail};

pub fn parse(src: &str) -> Result<Node> {
    let mut sc = Scanner::new(src);
    parse_chain(&mut sc)
}

fn parse_chain(sc: &mut Scanner) -> Result<Node> {
    let mut nodes = Vec::new();

    loop {
        sc.skip_comments_and_ws();
        if sc.eof() {
            break;
        }

        match sc.peek().unwrap() {
            '[' => {
                let pkt = parse_packet(sc)?;
                // conditionals: support [if@(cond)] and [if(cond)]
                if pkt.ns.is_none() && (pkt.op == "if" || pkt.op.starts_with("if(")) {
                    let src = if pkt.op == "if" {
                        match pkt.arg {
                            Some(Arg::CondSrc(s)) => s,
                            _ => bail!("if needs (cond) or @(cond)"),
                        }
                    } else {
                        extract_paren(&pkt.op)
                            .ok_or_else(|| anyhow::anyhow!("if needs (cond)"))?
                            .to_string()
                    };
                    nodes.push(parse_if(sc, src)?);
                } else {
                    let mut pkt = pkt;
                    // attach immediate { ... } as packet body if present
                    sc.skip_comments_and_ws();
                    if sc.peek() == Some('{') {
                        if let Node::Block(body) = parse_block(sc)? {
                            pkt.body = Some(body);
                        }
                    }
                    nodes.push(Node::Packet(pkt));
                }
            }
            '{' => {
                nodes.push(parse_block(sc)?);
            }
            '>' => {
                // tolerate stray or repeated separators
                sc.next();
            }
            _ => bail!(unexpected(sc, "top-level")),
        }

        // optional separators; allow multiple and trailing
        sc.skip_comments_and_ws();
        while sc.peek() == Some('>') {
            sc.next();
        }
    }

    Ok(Node::Chain(nodes))
}

fn parse_block(sc: &mut Scanner) -> Result<Node> {
    let inner = sc.read_until_balanced('{', '}')?;
    let mut sub = Scanner::new(&inner);
    match parse_chain(&mut sub)? {
        Node::Chain(v) => Ok(Node::Block(v)),
        other => Ok(Node::Block(vec![other])),
    }
}

fn parse_packet(sc: &mut Scanner) -> Result<Packet> {
    let inner = sc.read_until_balanced('[', ']')?;
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
                if depth_paren > 0 {
                    depth_paren -= 1;
                }
                continue;
            }
            '[' => {
                depth_brack += 1;
                continue;
            }
            ']' => {
                if depth_brack > 0 {
                    depth_brack -= 1;
                }
                continue;
            }
            '{' => {
                depth_brace += 1;
                continue;
            }
            '}' => {
                if depth_brace > 0 {
                    depth_brace -= 1;
                }
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

fn parse_if(sc: &mut Scanner, cond_src: String) -> Result<Node> {
    use crate::packets::conditionals::parse_cond;
    let cond = parse_cond(&cond_src);

    sc.skip_comments_and_ws();
    if sc.peek() == Some('>') {
        sc.next();
    }
    sc.skip_comments_and_ws();

    let then_pkt = parse_packet(sc)?;
    if then_pkt.ns.is_some() || then_pkt.op != "then" {
        bail!("expected [then]");
    }
    sc.skip_comments_and_ws();
    if sc.peek() != Some('{') {
        bail!("[then] needs block");
    }
    let Node::Block(then_b) = parse_block(sc)? else {
        unreachable!()
    };

    sc.skip_comments_and_ws();
    while sc.peek() == Some('>') {
        sc.next();
        sc.skip_comments_and_ws();
    }

    let else_b = parse_or_else(sc)?;
    Ok(Node::If {
        cond,
        then_b,
        else_b,
    })
}

fn parse_or_else(sc: &mut Scanner) -> Result<Vec<Node>> {
    use crate::packets::conditionals::parse_cond;
    sc.skip_comments_and_ws();
    if starts_with(sc, "[or@") || starts_with(sc, "[or(") {
        let pkt = parse_packet(sc)?;
        let src = if pkt.op == "or" {
            match pkt.arg {
                Some(Arg::CondSrc(s)) => s,
                _ => bail!("or needs (cond) or @(cond)"),
            }
        } else if pkt.op.starts_with("or(") {
            extract_paren(&pkt.op)
                .ok_or_else(|| anyhow::anyhow!("or needs (cond)"))?
                .to_string()
        } else {
            bail!("expected or clause")
        };
        sc.skip_comments_and_ws();
        if sc.peek() == Some('>') {
            sc.next();
            sc.skip_comments_and_ws();
        }
        let then_pkt = parse_packet(sc)?;
        if then_pkt.ns.is_some() || then_pkt.op != "then" {
            bail!("expected [then]");
        }
        sc.skip_comments_and_ws();
        if sc.peek() != Some('{') {
            bail!("[then] needs block");
        }
        let Node::Block(then_b) = parse_block(sc)? else {
            unreachable!()
        };
        sc.skip_comments_and_ws();
        while sc.peek() == Some('>') {
            sc.next();
            sc.skip_comments_and_ws();
        }
        let else_b = parse_or_else(sc)?;
        Ok(vec![Node::If {
            cond: parse_cond(&src),
            then_b,
            else_b,
        }])
    } else if starts_with(sc, "[else]") {
        let _pkt = parse_packet(sc)?; // consume [else]
        sc.skip_comments_and_ws();
        if sc.peek() == Some('>') {
            sc.next();
            sc.skip_comments_and_ws();
        }
        let then_pkt = parse_packet(sc)?;
        if then_pkt.ns.is_some() || then_pkt.op != "then" {
            bail!("expected [then]");
        }
        sc.skip_comments_and_ws();
        if sc.peek() != Some('{') {
            bail!("[then] needs block");
        }
        let Node::Block(block) = parse_block(sc)? else {
            unreachable!()
        };
        Ok(block)
    } else {
        Ok(Vec::new())
    }
}

fn starts_with(sc: &Scanner, pat: &str) -> bool {
    let start = sc.pos();
    let end = start + pat.len();
    end <= sc.len() && sc.slice(start, end) == pat.as_bytes()
}

fn unexpected(sc: &Scanner, where_: &str) -> String {
    let pos = sc.pos();
    // compute line/col
    let before = sc.slice(0, pos);
    let mut line: usize = 1;
    let mut last_nl: usize = 0;
    for (idx, b) in before.iter().enumerate() {
        if *b == b'\n' {
            line += 1;
            last_nl = idx + 1;
        }
    }
    let col = pos.saturating_sub(last_nl) + 1;

    // extract current line for caret display
    let mut end = sc.len();
    let after = sc.slice(pos, sc.len());
    for (off, b) in after.iter().enumerate() {
        if *b == b'\n' {
            end = pos + off;
            break;
        }
    }
    let line_str = String::from_utf8_lossy(sc.slice(last_nl, end)).to_string();
    let caret_pad: String = std::iter::repeat(' ').take(col.saturating_sub(1)).collect();

    format!(
        "unexpected character at {where_}: '{}' at {}:{}\n{}\n{}^",
        sc.peek().unwrap_or('?'),
        line,
        col,
        line_str,
        caret_pad,
    )
}

// --- helpers for packet extraction ---

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
pub fn parse_single_packet(src: &str) -> Result<Packet> {
    match parse(src)? {
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
