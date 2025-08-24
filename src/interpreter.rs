use anyhow::{Result, bail};

#[derive(Clone, Debug)]
pub struct Span { pub start: usize, pub end: usize }

pub struct Scanner<'a> {
    src: &'a [u8],
    pub i: usize,
    len: usize,
}

impl<'a> Scanner<'a> {
    // ---- debug/introspection helpers ----
    pub fn pos(&self) -> usize { self.i }
    pub fn len(&self) -> usize { self.len }
    pub fn slice(&self, start: usize, end: usize) -> &[u8] { &self.src[start..end] }

    // ---- core ----
    pub fn new(s: &'a str) -> Self { Self { src: s.as_bytes(), i: 0, len: s.len() } }
    pub fn peek(&self) -> Option<char> { (self.i < self.len).then(|| self.src[self.i] as char) }
    pub fn next(&mut self) -> Option<char> { let c = self.peek()?; self.i += 1; Some(c) }

    fn starts_with(&self, s: &str) -> bool {
        let n = s.len();
        self.i + n <= self.len && &self.src[self.i..self.i+n] == s.as_bytes()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() { self.i += 1; } else { break; }
        }
    }

    pub fn skip_comments_and_ws(&mut self) {
        loop {
            self.skip_ws();

            // # line comment
            if self.peek() == Some('#') {
                while let Some(c) = self.next() { if c == '\n' { break; } }
                continue;
            }

            // // line comment
            if self.starts_with("//") {
                self.i += 2;
                while let Some(c) = self.next() { if c == '\n' { break; } }
                continue;
            }

            // /* block comment */
            if self.starts_with("/*") {
                self.i += 2;
                while self.i + 1 < self.len {
                    if self.starts_with("*/") { self.i += 2; break; }
                    self.i += 1;
                }
                continue;
            }

            break;
        }
    }

    pub fn eof(&self) -> bool { self.i >= self.len }

    pub fn read_quoted(&mut self) -> Result<String> {
        if self.next() != Some('"') { bail!("expected '\"'"); }
        let mut out = String::new();
        while let Some(c) = self.next() {
            match c {
                '\\' => {
                    let Some(nc) = self.next() else { bail!("unterminated escape in string"); };
                    out.push(match nc {
                        'n' => '\n', 'r' => '\r', 't' => '\t', '\\' => '\\', '"' => '"', other => other
                    });
                }
                '"' => return Ok(out),
                other => out.push(other),
            }
        }
        bail!("unterminated string")
    }

    pub fn read_until_balanced(&mut self, open: char, close: char) -> Result<String> {
        // assumes current char == open
        if self.next() != Some(open) { bail!("expected opener {}", open); }
        let mut out = String::new();
        let mut depth = 1usize;
        while let Some(c) = self.next() {
            if c == '\\' {
                if let Some(nc) = self.next() { out.push(c); out.push(nc); }
                continue;
            }
            if c == open { depth += 1; }
            if c == close {
                depth -= 1;
                if depth == 0 { return Ok(out); }
            }
            out.push(c);
        }
        bail!("unbalanced {} ... {}", open, close)
    }

    pub fn read_ident_or_number(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' { s.push(c); self.i += 1; }
            else { break; }
        }
        s
    }

    pub fn read_raw_until(&mut self, terminator: char) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == terminator { break; }
            s.push(c); self.i += 1;
        }
        s.trim().to_string()
    }
}
