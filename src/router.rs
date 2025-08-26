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
                let mut pkt = parse_packet(sc)?;
                // attach immediate { ... } as packet body if present
                sc.skip_comments_and_ws();
                if sc.peek() == Some('{') {
                    if let Node::Block(body) = parse_block(sc)? {
                        pkt.body = Some(body);
                    }
                }
                nodes.push(Node::Packet(pkt));
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
        arg = if peek(i) == Some('"') {
            // quoted string @"..."
            let mut j = i + 1;
            let mut out = String::new();
            while j < len {
                let c = b[j] as char;
                j += 1;
                match c {
                    '\\' => {
                        if j >= len {
                            bail!("unterminated escape in string")
                        }
                        let nc = b[j] as char;
                        j += 1;
                        out.push(match nc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            other => other,
                        });
                    }
                    '"' => {
                        i = j;
                        break;
                    }
                    other => out.push(other),
                }
            }
            Some(Arg::Str(out))
        } else if peek(i) == Some('(') {
            // [myth] goal: grab condition source for later re-parse
            let mut sub = Scanner::new(&inner[i..]);
            let cond_src = sub.read_until_balanced('(', ')')?;
            i += sub.pos();
            Some(Arg::CondSrc(cond_src))
        } else {
            let raw = inner[i..].trim().to_string();
            if raw.is_empty() {
                None
            } else if let Ok(n) = raw.parse::<f64>() {
                Some(Arg::Number(n))
            } else if is_ident_like(&raw) {
                Some(Arg::Ident(raw))
            } else {
                Some(Arg::Str(raw)) // treat as expression string (e.g., "counter+1")
            }
        };
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
