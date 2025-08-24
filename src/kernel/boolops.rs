use crate::kernel::ast::{Comparator, CmpBase};
use crate::kernel::values::Value;
use crate::kernel::ast::{BExpr, Node, Arg, Packet};
use anyhow::{Result, bail};

pub fn reduce_op_chain_is_valid() -> bool { true } // placeholder if needed

pub fn cmp_eval(cmp: &Comparator, a: &Value, b: &Value) -> anyhow::Result<bool> {
    use CmpBase::*;
    let mut out = match cmp.base {
        Eq => eq_values(a, b),
        Lt => order(a, b, |x, y| x < y)?,
        Gt => order(a, b, |x, y| x > y)?,
    };
    if matches!(cmp.base, Lt | Gt) && cmp.include_eq {
        out = out || eq_values(a, b);
    }
    if cmp.negate { out = !out; }
    Ok(out)
}

fn eq_values(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x),  Value::Num(y))  => x == y,
        (Value::Str(x),  Value::Str(y))  => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

fn order<F: Fn(f64, f64) -> bool>(a: &Value, b: &Value, f: F) -> anyhow::Result<bool> {
    let xa = to_num(a)?;
    let xb = to_num(b)?;
    Ok(f(xa, xb))
}

fn to_num(v: &Value) -> anyhow::Result<f64> {
    match v {
        Value::Num(n) => Ok(*n),
        Value::Str(s) => s.parse::<f64>().map_err(|_| anyhow::anyhow!("non-numeric string")),
        _ => Err(anyhow::anyhow!("non-numeric value")),
    }
}

// === Boolean expression parsing ===

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Ident(String),
    Cmp(Comparator),
    And,
    Or,
    Not,
    LParen,
    RParen,
}

struct Lexer<'a> {
    src: &'a [u8],
    i: usize,
    len: usize,
}

impl<'a> Lexer<'a> {
    fn new(s: &'a str) -> Self { Self { src: s.as_bytes(), i: 0, len: s.len() } }
    fn peek(&self) -> Option<char> { (self.i < self.len).then(|| self.src[self.i] as char) }
    fn next(&mut self) -> Option<char> { let c = self.peek()?; self.i += 1; Some(c) }
    fn skip_ws(&mut self) { while let Some(c) = self.peek() { if c.is_whitespace() { self.i += 1; } else { break; } } }
    fn starts_with(&self, s: &str) -> bool {
        let n = s.len();
        self.i + n <= self.len && &self.src[self.i..self.i+n] == s.as_bytes()
    }
    fn read_ident_or_number(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' { s.push(c); self.i += 1; }
            else { break; }
        }
        s
    }
}

fn lex(src: &str) -> Result<Vec<Token>> {
    let mut lx = Lexer::new(src);
    let mut out = Vec::new();
    while lx.i < lx.len {
        lx.skip_ws();
        if lx.i >= lx.len { break; }
        // punctuation and operators
        if lx.starts_with("&&") { lx.i += 2; out.push(Token::And); continue; }
        if lx.starts_with("||") { lx.i += 2; out.push(Token::Or); continue; }
        if lx.starts_with("!") { lx.i += 1; out.push(Token::Not); continue; }
        if lx.starts_with("==") { lx.i += 2; out.push(Token::Cmp(Comparator { base: CmpBase::Eq, include_eq: false, negate: false })); continue; }
        if lx.starts_with("!=") { lx.i += 2; out.push(Token::Cmp(Comparator { base: CmpBase::Eq, include_eq: false, negate: true })); continue; }
        if lx.starts_with("<=") { lx.i += 2; out.push(Token::Cmp(Comparator { base: CmpBase::Lt, include_eq: true, negate: false })); continue; }
        if lx.starts_with(">=") { lx.i += 2; out.push(Token::Cmp(Comparator { base: CmpBase::Gt, include_eq: true, negate: false })); continue; }
        if lx.starts_with("<") { lx.i += 1; out.push(Token::Cmp(Comparator { base: CmpBase::Lt, include_eq: false, negate: false })); continue; }
        if lx.starts_with(">") { lx.i += 1; out.push(Token::Cmp(Comparator { base: CmpBase::Gt, include_eq: false, negate: false })); continue; }
        if lx.starts_with("(") { lx.i += 1; out.push(Token::LParen); continue; }
        if lx.starts_with(")") { lx.i += 1; out.push(Token::RParen); continue; }

        // bracketed keywords
        if lx.starts_with("[eq]") { lx.i += 4; out.push(Token::Cmp(Comparator { base: CmpBase::Eq, include_eq: false, negate: false })); continue; }
        if lx.starts_with("[neq]") { lx.i += 5; out.push(Token::Cmp(Comparator { base: CmpBase::Eq, include_eq: false, negate: true })); continue; }
        if lx.starts_with("[lt]") { lx.i += 4; out.push(Token::Cmp(Comparator { base: CmpBase::Lt, include_eq: false, negate: false })); continue; }
        if lx.starts_with("[gt]") { lx.i += 4; out.push(Token::Cmp(Comparator { base: CmpBase::Gt, include_eq: false, negate: false })); continue; }
        if lx.starts_with("[and]") { lx.i += 5; out.push(Token::And); continue; }
        if lx.starts_with("[or]") { lx.i += 4; out.push(Token::Or); continue; }
        if lx.starts_with("[not]") { lx.i += 5; out.push(Token::Not); continue; }

        // identifiers or numbers
        let word = lx.read_ident_or_number();
        if word.is_empty() { bail!("unexpected token in expression"); }
        if let Ok(n) = word.parse::<f64>() {
            out.push(Token::Number(n));
        } else {
            out.push(Token::Ident(word));
        }
    }
    Ok(out)
}

struct Parser { toks: Vec<Token>, i: usize }

impl Parser {
    fn new(t: Vec<Token>) -> Self { Self { toks: t, i: 0 } }
    fn peek(&self) -> Option<&Token> { self.toks.get(self.i) }
    fn next(&mut self) -> Option<Token> { if self.i < self.toks.len() { let t = self.toks[self.i].clone(); self.i += 1; Some(t) } else { None } }

    fn parse_or(&mut self) -> Result<BExpr> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.next();
            let right = self.parse_and()?;
            left = BExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    fn parse_and(&mut self) -> Result<BExpr> {
        let mut left = self.parse_not()?;
        while matches!(self.peek(), Some(Token::And)) {
            self.next();
            let right = self.parse_not()?;
            left = BExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    fn parse_not(&mut self) -> Result<BExpr> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.next();
            Ok(BExpr::Not(Box::new(self.parse_not()?)))
        } else {
            self.parse_cmp()
        }
    }
    fn parse_cmp(&mut self) -> Result<BExpr> {
        // handle parenthesized expression
        if matches!(self.peek(), Some(Token::LParen)) {
            self.next();
            let expr = self.parse_or()?;
            match self.next() {
                Some(Token::RParen) => Ok(expr),
                _ => bail!("expected )"),
            }
        } else {
            let left = self.parse_operand()?;
            if let Some(Token::Cmp(cmp)) = self.peek().cloned() {
                self.next();
                let right = self.parse_operand()?;
                Ok(BExpr::Cmp { lhs: Box::new(left), cmp, rhs: Box::new(right) })
            } else {
                Ok(BExpr::Lit(Box::new(left)))
            }
        }
    }
    fn parse_operand(&mut self) -> Result<Node> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Node::Packet(Packet { ns: None, op: "math".into(), arg: Some(Arg::Number(n)) })),
            Some(Token::Ident(id)) => Ok(Node::Packet(Packet { ns: None, op: "math".into(), arg: Some(Arg::Ident(id)) })),
            Some(Token::LParen) => bail!("unexpected '(' in operand"),
            other => bail!("unexpected token {:?}", other),
        }
    }
}

pub fn parse_bexpr(src: &str) -> Result<BExpr> {
    let toks = lex(src)?;
    let mut p = Parser::new(toks);
    let expr = p.parse_or()?;
    if p.peek().is_some() { bail!("unexpected token at end"); }
    Ok(expr)
}

