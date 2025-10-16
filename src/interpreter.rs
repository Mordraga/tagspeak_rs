use anyhow::{Result, bail};

#[derive(Clone, Debug)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

pub struct Scanner<'a> {
    src: &'a [u8],
    pub i: usize,
    len: usize,
    limit: usize,
}

impl<'a> Scanner<'a> {
    // ---- debug/introspection helpers ----
    pub fn pos(&self) -> usize {
        self.i
    }
    pub fn len(&self) -> usize {
        self.limit
    }
    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        &self.src[start..end]
    }

    pub fn total_len(&self) -> usize {
        self.len
    }

    // ---- core ----
    pub fn new(s: &'a str) -> Self {
        let bytes = s.as_bytes();
        let len = bytes.len();
        Self {
            src: bytes,
            i: 0,
            len,
            limit: len,
        }
    }

    pub fn subscanner(&self, start: usize, end: usize) -> Self {
        assert!(start <= end, "invalid scanner range");
        assert!(end <= self.len, "subscanner exceeds source length");
        Self {
            src: self.src,
            i: start,
            len: self.len,
            limit: end,
        }
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
    pub fn peek(&self) -> Option<char> {
        (self.i < self.limit).then(|| self.src[self.i] as char)
    }
    pub fn next(&mut self) -> Option<char> {
        if self.i >= self.limit {
            return None;
        }
        let c = self.src[self.i] as char;
        self.i += 1;
        Some(c)
    }

    fn newline_len_at(&self, idx: usize) -> usize {
        if idx >= self.limit {
            return 0;
        }
        match self.src[idx] {
            b'\n' => 1,
            b'\r' => {
                if idx + 1 < self.limit && self.src[idx + 1] == b'\n' {
                    2
                } else {
                    1
                }
            }
            _ => 0,
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        let n = s.len();
        self.i + n <= self.limit && &self.src[self.i..self.i + n] == s.as_bytes()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    pub fn skip_comments_and_ws(&mut self) {
        loop {
            self.skip_ws();

            // # line comment
            if self.peek() == Some('#') {
                while let Some(c) = self.next() {
                    if c == '\n' {
                        break;
                    }
                    if c == '\r' {
                        if self.peek() == Some('\n') {
                            self.i += 1;
                        }
                        break;
                    }
                }
                continue;
            }

            // // line comment
            if self.starts_with("//") {
                self.i += 2;
                while let Some(c) = self.next() {
                    if c == '\n' {
                        break;
                    }
                    if c == '\r' {
                        if self.peek() == Some('\n') {
                            self.i += 1;
                        }
                        break;
                    }
                }
                continue;
            }

            // /* block comment */
            if self.starts_with("/*") {
                self.i += 2;
                while self.i + 1 < self.limit {
                    if self.starts_with("*/") {
                        self.i += 2;
                        break;
                    }
                    self.i += 1;
                }
                continue;
            }

            break;
        }
    }

    pub fn eof(&self) -> bool {
        self.i >= self.limit
    }

    // --- location helpers ---
    pub fn line_col_at(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.limit);
        let mut line: usize = 1;
        let mut last_break_end: usize = 0;
        let mut idx = 0usize;
        while idx < pos {
            let newline_len = self.newline_len_at(idx);
            if newline_len > 0 {
                let break_end = idx + newline_len;
                if break_end > pos {
                    break;
                }
                line += 1;
                last_break_end = break_end;
                idx = break_end;
            } else {
                idx += 1;
            }
        }
        let col = pos.saturating_sub(last_break_end) + 1;
        (line, col)
    }
    pub fn cur_line_col(&self) -> (usize, usize) {
        self.line_col_at(self.i)
    }

    pub fn read_quoted(&mut self) -> Result<String> {
        if self.next() != Some('"') {
            let (ln, col) = self.cur_line_col();
            bail!("expected '\"' at {}:{}", ln, col);
        }
        let mut out = String::new();
        while let Some(c) = self.next() {
            match c {
                '\\' => {
                    let Some(nc) = self.next() else {
                        bail!("unterminated escape in string");
                    };
                    out.push(match nc {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        other => other,
                    });
                }
                '"' => return Ok(out),
                other => out.push(other),
            }
        }
        let (ln, col) = self.cur_line_col();
        bail!("unterminated string starting before {}:{}", ln, col)
    }

    pub fn read_until_balanced(&mut self, open: char, close: char) -> Result<(String, Span)> {
        // assumes current char == open
        if self.next() != Some(open) {
            let (ln, col) = self.cur_line_col();
            bail!("expected opener {} at {}:{}", open, ln, col);
        }
        let mut out = String::new();
        let mut depth = 1usize;
        let inner_start = self.i;
        while let Some(c) = self.next() {
            if c == '\\' {
                if let Some(nc) = self.next() {
                    out.push(c);
                    out.push(nc);
                }
                continue;
            }
            if c == open {
                depth += 1;
            }
            if c == close {
                depth -= 1;
                if depth == 0 {
                    let span = Span {
                        start: inner_start,
                        end: self.i - 1,
                    };
                    return Ok((out, span));
                }
            }
            out.push(c);
        }
        let (ln, col) = self.cur_line_col();
        bail!("unbalanced {} ... {} before {}:{}", open, close, ln, col)
    }

    pub fn char_at(&self, idx: usize) -> Option<char> {
        (idx < self.limit).then(|| self.src[idx] as char)
    }

    pub fn read_ident_or_number(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                s.push(c);
                self.i += 1;
            } else {
                break;
            }
        }
        s
    }

    pub fn read_raw_until(&mut self, terminator: char) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == terminator {
                break;
            }
            s.push(c);
            self.i += 1;
        }
        s.trim().to_string()
    }
}
