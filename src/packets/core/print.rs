use anyhow::{Result, bail};
use std::iter::Peekable;
use std::str::Chars;

use crate::kernel::ast::Arg;
use crate::kernel::{Packet, Runtime, Value};

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    if let Some(Arg::Str(raw)) = p.arg.as_ref() {
        if let Some(rendered) = render_template(rt, raw)? {
            println!("{}", rendered);
            return Ok(Value::Str(rendered));
        }
    }

    let v = match p.arg.as_ref() {
        Some(arg) => rt.resolve_arg(arg)?,
        None => rt.last.clone(),
    };
    println!("{}", pretty(&v));
    Ok(v)
}

fn pretty(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Num(n) => format!("{}", n),
        Value::Bool(b) => format!("{}", b),
        Value::Doc(_) => String::from("<doc>"),
        Value::Unit => String::from("()"),
    }
}

fn render_template(rt: &Runtime, raw: &str) -> Result<Option<String>> {
    let needs_interp = raw.starts_with('"') || raw.contains("${");
    if !needs_interp {
        return Ok(None);
    }

    let mut chars = raw.chars().peekable();
    let mut out = String::new();
    let mut consumed = false;

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        match c {
            '"' => {
                chars.next();
                let mut literal = String::new();
                loop {
                    let Some(ch) = chars.next() else {
                        bail!("unterminated string literal in print template");
                    };
                    match ch {
                        '\\' => {
                            let Some(esc) = chars.next() else {
                                bail!("unterminated escape in string literal");
                            };
                            literal.push(match esc {
                                'n' => '\n',
                                'r' => '\r',
                                't' => '\t',
                                '\\' => '\\',
                                '"' => '"',
                                other => other,
                            });
                        }
                        '"' => break,
                        '$' => {
                            if matches!(chars.peek(), Some('{')) {
                                chars.next(); // consume '{'
                                out.push_str(&literal);
                                literal.clear();
                                let placeholder = read_placeholder(&mut chars)?;
                                out.push_str(&resolve_variable(rt, &placeholder)?);
                                continue;
                            } else {
                                literal.push('$');
                            }
                        }
                        other => literal.push(other),
                    }
                }
                out.push_str(&literal);
                consumed = true;
            }
            '$' => {
                chars.next();
                if chars.next() != Some('{') {
                    bail!("expected '{{' after '$' in print template");
                }
                let placeholder = read_placeholder(&mut chars)?;
                out.push_str(&resolve_variable(rt, &placeholder)?);
                consumed = true;
            }
            other if is_ident_start(other) => {
                let mut ident = String::new();
                ident.push(other);
                chars.next();
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphanumeric() || next == '_' {
                        ident.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push_str(&resolve_variable(rt, &ident)?);
                consumed = true;
            }
            _ => bail!("unexpected token in print template"),
        }
    }

    Ok(consumed.then_some(out))
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn read_placeholder(chars: &mut Peekable<Chars<'_>>) -> Result<String> {
    let mut placeholder = String::new();
    let mut closed = false;
    while let Some(ch) = chars.next() {
        if ch == '}' {
            closed = true;
            break;
        }
        placeholder.push(ch);
    }
    if !closed {
        bail!("unterminated placeholder in print template");
    }
    Ok(placeholder.trim().to_string())
}

fn resolve_variable(rt: &Runtime, name: &str) -> Result<String> {
    if !is_valid_ident(name) {
        bail!("invalid placeholder name '{name}'");
    }
    let val = rt.get_var(name).unwrap_or(Value::Unit);
    Ok(pretty(&val))
}

fn is_valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if is_ident_start(first) => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use crate::{kernel::Runtime, router};

    #[test]
    fn concat_variables_and_literals() -> Result<()> {
        let script = "\
[int@5]>[store@min]\
[int@42]>[store@sec]\
[print@\"Time: \" min \":\" sec]";
        let node = router::parse(script).map_err(Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        match &rt.last {
            Value::Str(s) => assert_eq!(s, "Time: 5:42"),
            other => bail!("expected string result, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn placeholder_expansion_inside_literal() -> Result<()> {
        let script = "\
[int@7]>[store@min]\
[int@9]>[store@sec]\
[print@\"Time: ${min}:${sec}\"]";
        let node = router::parse(script).map_err(Error::new)?;
        let mut rt = Runtime::new()?;
        rt.eval(&node)?;
        match &rt.last {
            Value::Str(s) => assert_eq!(s, "Time: 7:9"),
            other => bail!("expected string result, got {other:?}"),
        }
        Ok(())
    }
}
