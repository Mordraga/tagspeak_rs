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
                // [if@(...)] introduces conditional branches
                if pkt.ns.is_none() && pkt.op == "if" {
                    let src = match pkt.arg {
                        Some(Arg::CondSrc(s)) => s,
                        _ => bail!("if needs @(cond)"),
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
    // inner like: ns:op@arg | op@arg | op | ns:op
    let mut ns: Option<String> = None;
    let mut op = String::new();
    let mut arg: Option<Arg> = None;

    let mut i = 0usize;
    let b = inner.as_bytes();
    let len = b.len();
    let peek = |i: usize| -> Option<char> { (i < len).then(|| b[i] as char) };

    // ns/op (split on first ':', stop at '@')
    let mut acc = String::new();
    while let Some(c) = peek(i) {
        if c == ':' || c == '@' {
            break;
        }
        acc.push(c);
        i += 1;
    }
    if peek(i) == Some(':') {
        ns = Some(acc.trim().to_string());
        i += 1;
        acc.clear();
        while let Some(c) = peek(i) {
            if c == '@' {
                break;
            }
            acc.push(c);
            i += 1;
        }
        op = acc.trim().to_string();
    } else {
        op = acc.trim().to_string();
    }

    // @arg (optional)
    if peek(i) == Some('@') {
        i += 1;
        while peek(i) == Some(' ') {
            i += 1;
        }
        let raw = inner[i..].trim().to_string();
        if raw.is_empty() {
            arg = None;
        } else if raw.starts_with('"') && raw.contains('+') {
            arg = Some(Arg::Str(raw));
        } else if raw.starts_with('"') {
            let mut sc = Scanner::new(&raw);
            let s = sc.read_quoted()?;
            arg = Some(Arg::Str(s));
        } else if raw.starts_with('(') {
            arg = Some(Arg::CondSrc(raw));
        } else if let Ok(n) = raw.parse::<f64>() {
            arg = Some(Arg::Number(n));
        } else if is_ident_like(&raw) {
            arg = Some(Arg::Ident(raw));
        } else {
            arg = Some(Arg::Str(raw));
        }
        i = len;
    }

    if op.is_empty() {
        bail!("empty packet op in [{inner}]");
    }

    Ok(Packet {
        ns,
        op,
        arg,
        body: None,
    })
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
    if starts_with(sc, "[or@") {
        let pkt = parse_packet(sc)?;
        let src = match pkt.arg {
            Some(Arg::CondSrc(s)) => s,
            _ => bail!("or needs @(cond)"),
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
    let start = sc.pos().saturating_sub(8);
    let end = (sc.pos() + 16).min(sc.len());
    let mut snippet = String::from_utf8_lossy(sc.slice(start, end)).to_string();
    snippet = snippet.replace('\n', "\\n");
    format!(
        "unexpected character at {where_}: '{}' near \"{}\"",
        sc.peek().unwrap_or('?'),
        snippet
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
