use anyhow::{Result, bail};
use crate::kernel::ast::{Node, Packet, Arg};
use crate::interpreter::Scanner;

pub fn parse(src: &str) -> Result<Node> {
    let mut sc = Scanner::new(src);
    parse_chain(&mut sc)
}

fn parse_chain(sc: &mut Scanner) -> Result<Node> {
    let mut nodes = Vec::new();
    loop {
        sc.skip_comments_and_ws();
        if sc.eof() { break; }
        let ch = sc.peek().unwrap();

        match ch {
            '[' => {
                let pkt = parse_packet(sc)?;
                if pkt.op == "if" {
                    nodes.push(parse_if_chain(sc, pkt)?);
                } else if pkt.op == "or" || pkt.op == "else" {
                    bail!("unexpected [{}] without preceding [if]", pkt.op);
                } else {
                    nodes.push(Node::Packet(pkt));
                }
            }
            '{' => nodes.push(parse_block(sc)?),
            '>' => { sc.next(); continue; } // tolerate stray > (optional)
            _   => {
                // unknown token at top level → ignore line or bail. We’ll ignore until newline.
                // This keeps comments/blank lines safe if lexer misses them.
                // But better: bail with context:
                bail!("unexpected character at top-level: '{}'", ch);
            }
        }

        // optional '>' separators
        sc.skip_comments_and_ws();
        if sc.peek() == Some('>') { sc.next(); }
    }
    Ok(Node::Chain(nodes))
}

fn parse_block(sc: &mut Scanner) -> Result<Node> {
    let inner = sc.read_until_balanced('{', '}')?;
    let mut sub = Scanner::new(&inner);
    let node = parse_chain(&mut sub)?;
    // unwrap one level if we got Chain
    if let Node::Chain(v) = node {
        Ok(Node::Block(v))
    } else {
        Ok(Node::Block(vec![node]))
    }
}

fn parse_packet(sc: &mut Scanner) -> Result<Packet> {
    let inner = sc.read_until_balanced('[', ']')?;
    // inner like:   ns:op@arg    |  op@arg   |  op    | ns:op
    let mut ns: Option<String> = None;
    let mut op = String::new();
    let mut arg: Option<Arg> = None;

    let mut i = 0usize;
    let b = inner.as_bytes();
    let len = b.len();

    // helper to peek/next inside 'inner'
    let peek = |i: usize| -> Option<char> { (i < len).then(|| b[i] as char) };

    // parse ns:op
    let mut acc = String::new();
    while let Some(c) = peek(i) {
        if c == ':' || c == '@' { break; }
        acc.push(c); i += 1;
    }
    // acc holds 'ns_or_op'
    // decide: if next char is ':', split into ns + op
    if peek(i) == Some(':') {
        ns = Some(acc.trim().to_string());
        i += 1; acc.clear();
        while let Some(c) = peek(i) {
            if c == '@' { break; }
            acc.push(c); i += 1;
        }
        op = acc.trim().to_string();
    } else {
        op = acc.trim().to_string();
    }

    // parse @arg if present
    if peek(i) == Some('@') {
        i += 1;
        // allow `"quoted"`, number, ident, or raw expr until end
        // skip one optional leading space
        while peek(i) == Some(' ') { i += 1; }
        arg = if peek(i) == Some('"') {
            // quoted string
            // reuse a tiny local scanner on the remainder to capture the quoted
            let mut j = i;
            // read quoted
            if b[j] as char != '"' { bail!("internal string parse error"); }
            j += 1;
            let mut out = String::new();
            while j < len {
                let c = b[j] as char; j += 1;
                match c {
                    '\\' => {
                        if j >= len { bail!("unterminated escape"); }
                        let nc = b[j] as char; j += 1;
                        out.push(match nc {
                            'n' => '\n', 'r' => '\r', 't' => '\t', '\\' => '\\', '"' => '"', other => other
                        });
                    }
                    '"' => { i = j; break; }
                    other => out.push(other),
                }
            }
            Some(Arg::Str(out))
        } else {
            // read raw until end
            let raw = inner[i..].trim().to_string();
            if matches!(op.as_str(), "if" | "or") {
                Some(Arg::CondSrc(raw))
            } else if let Ok(n) = raw.parse::<f64>() {
                Some(Arg::Number(n))
            } else if is_ident_like(&raw) {
                Some(Arg::Ident(raw))
            } else {
                Some(Arg::Str(raw))
            }
        };
    }

    if op.is_empty() {
        bail!("empty packet op in [{inner}]");
    }

    Ok(Packet { ns, op, arg })
}

fn is_ident_like(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {},
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' )
}

fn parse_if_chain(sc: &mut Scanner, first: Packet) -> Result<Node> {
    use crate::kernel::boolops::parse_bexpr;
    use crate::kernel::ast::{BExpr};
    // first.op is "if" or "or"
    let cond_src = match first.arg {
        Some(Arg::CondSrc(ref s)) => s.clone(),
        _ => bail!("if/or requires a condition"),
    };
    let cond: BExpr = parse_bexpr(&cond_src)?;

    // optional separator
    sc.skip_comments_and_ws();
    if sc.peek() == Some('>') { sc.next(); }
    sc.skip_comments_and_ws();

    // expect [then]
    let then_pkt = parse_packet(sc)?;
    if then_pkt.op != "then" { bail!("expected [then] after [{}]", first.op); }

    // optional separator
    sc.skip_comments_and_ws();
    if sc.peek() == Some('>') { sc.next(); }
    sc.skip_comments_and_ws();

    if sc.peek() != Some('{') { bail!("expected block after [then]"); }
    let then_node = parse_block(sc)?;
    let then_b = match then_node { Node::Block(v) => v, other => vec![other] };

    // after block, consume optional separator
    sc.skip_comments_and_ws();
    if sc.peek() == Some('>') { sc.next(); }
    sc.skip_comments_and_ws();

    // check for [or] or [else]
    let else_b = if sc.peek() == Some('[') {
        let pkt = parse_packet(sc)?;
        match pkt.op.as_str() {
            "or" => {
                let nested = parse_if_chain(sc, pkt)?;
                vec![nested]
            }
            "else" => {
                // optional separator then block
                sc.skip_comments_and_ws();
                if sc.peek() == Some('>') { sc.next(); }
                sc.skip_comments_and_ws();
                if sc.peek() != Some('{') { bail!("expected block after [else]"); }
                let eb = parse_block(sc)?;
                match eb { Node::Block(v) => v, other => vec![other] }
            }
            other => bail!("unexpected packet [{}] after conditional", other),
        }
    } else {
        Vec::new()
    };

    Ok(Node::If { cond, then_b, else_b })
}
