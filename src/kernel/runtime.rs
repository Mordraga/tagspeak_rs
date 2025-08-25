use std::collections::HashMap;
use anyhow::{Result, bail};

use crate::kernel::ast::{Node, Packet, Arg, BExpr};
use crate::kernel::values::Value;

pub struct Runtime {
    pub vars: HashMap<String, Value>,
    pub last: Value,
    pub tags: HashMap<String, Vec<Node>>, // named blocks from [funct:tag]{...}
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            last: Value::Unit,
            tags: HashMap::new(),
        }
    }

    // ---- variables ----
    pub fn set_var(&mut self, name: &str, val: Value) { self.vars.insert(name.to_string(), val); }
    pub fn get_var(&self, name: &str) -> Option<Value> { self.vars.get(name).cloned() }

    // ---- tags ----
    pub fn register_tag(&mut self, name: &str, body: Vec<Node>) { self.tags.insert(name.to_string(), body); }

    // ---- args ----
    pub fn resolve_arg(&self, arg: &Arg) -> Result<Value> {
        Ok(match arg {
            Arg::Number(n) => Value::Num(*n),
            Arg::Str(s)    => Value::Str(s.clone()),
            Arg::Ident(id) => self.get_var(id).unwrap_or(Value::Unit),
            _               => Value::Unit, // reserve for CondSrc/etc
        })
    }

    // ---- eval ----
    pub fn eval(&mut self, n: &Node) -> Result<Value> {
        let out = match n {
            Node::Chain(v) | Node::Block(v) => self.eval_list(v)?,
            Node::Packet(p) => self.eval_packet(p)?,
            Node::If { cond, then_b, else_b } => {
                // [myth] goal: runtime branching
                if self.eval_if(cond)? {
                    self.eval_list(then_b)?
                } else if else_b.is_empty() {
                    Value::Unit
                } else {
                    self.eval_list(else_b)?
                }
            }
        };
        self.last = out.clone();
        Ok(out)
    }

    fn eval_list(&mut self, list: &[Node]) -> Result<Value> {
        let mut last = Value::Unit;
        for node in list {
            last = self.eval(node)?;
        }
        Ok(last)
    }

    fn eval_packet(&mut self, p: &Packet) -> Result<Value> {
        match (p.ns.as_deref(), p.op.as_str()) {
            // namespaced
            (Some("funct"), _)  => crate::packets::funct::handle(self, p),

            // core
            (None, "note")      => crate::packets::note::handle(self, p),
            (None, "math")      => crate::packets::math::handle(self, p),
            (None, "store")     => crate::packets::store::handle(self, p),
            (None, "print")     => crate::packets::print::handle(self, p),
            (None, "eq")        => crate::packets::eq::handle(self, p),
            (None, "ne")        => crate::packets::ne::handle(self, p),
            (None, "gt")        => crate::packets::gt::handle(self, p),
            (None, "lt")        => crate::packets::lt::handle(self, p),
            (None, "ge")        => crate::packets::ge::handle(self, p),
            (None, "le")        => crate::packets::le::handle(self, p),
            (None, "and")       => crate::packets::and::handle(self, p),
            (None, "or")        => crate::packets::or::handle(self, p),
            (None, "not")       => crate::packets::not::handle(self, p),

            // loop forms: [loop3@tag] or [loop@N]{...}
            (None, op) if op.starts_with("loop") => crate::packets::r#loop::handle(self, p),

            other => bail!("unknown operation: {:?}", other),
        }
    }

    // small helpers for numeric vars used by packets
    pub fn get_num(&self, name: &str) -> Option<f64> {
        self.get_var(name).and_then(|v| v.try_num())
    }
    pub fn set_num(&mut self, name: &str, n: f64) {
        self.set_var(name, Value::Num(n));
    }

    fn eval_if(&mut self, cond: &BExpr) -> Result<bool> {
        crate::packets::conditionals::eval_cond(self, cond)
    }
}
