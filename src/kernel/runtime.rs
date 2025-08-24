use std::collections::HashMap;
use crate::kernel::ast::{Node, Packet, BExpr, /* Comparator if you already use it */};
use crate::kernel::values::Value;
use crate::kernel::boolops::cmp_eval; // keep if you use conditionals later

pub struct Runtime {
    pub vars: HashMap<String, Value>,
    pub last: Value, // last value in the current chain for pipeline-ish packets
}

impl Runtime {
    pub fn new() -> Self {
        Self { vars: HashMap::new(), last: Value::Unit }
    }

    // store/get
    pub fn set_var(&mut self, name: &str, val: Value) {
        self.vars.insert(name.to_string(), val);
    }
    pub fn get_var(&self, name: &str) -> Option<Value> {
        self.vars.get(name).cloned()
    }

    // resolve a packet arg into a Value (Ident/Str/Number)
    pub fn resolve_arg(&self, arg: &crate::kernel::ast::Arg) -> anyhow::Result<Value> {
        use crate::kernel::ast::Arg::*;
        Ok(match arg {
            Number(n) => Value::Num(*n),
            Str(s)    => Value::Str(s.clone()),
            Ident(id) => self.get_var(id).unwrap_or(Value::Unit),
            _ => Value::Unit, // CondSrc etc. are for conditionals; not used here
        })
    }

    // evaluate an AST node (update self.last after each step)
    pub fn eval(&mut self, n: &Node) -> anyhow::Result<Value> {
        let out = match n {
            Node::Chain(list) => {
                let mut last = Value::Unit;
                for node in list {
                    last = self.eval(node)?;
                }
                last
            }
            Node::Block(list) => {
                let mut last = Value::Unit;
                for node in list {
                    last = self.eval(node)?;
                }
                last
            }
            Node::Packet(p) => self.eval_packet(p)?,
            // If you’ve implemented Node::If already, you can branch here:
            Node::If { cond: _cond, then_b: _tb, else_b: _eb } => {
                // Placeholder: wire later when your cond compiler is ready.
                Value::Unit
            }
        };
        self.last = out.clone();
        Ok(out)
    }

    fn eval_packet(&mut self, p: &Packet) -> anyhow::Result<Value> {
        // Dispatch to packet handlers in src/packets/*
        match (p.ns.as_deref(), p.op.as_str()) {
            (None, "math")  => crate::packets::math::handle(self, p),
            (None, "store") => crate::packets::store::handle(self, p),
            (None, "print") => crate::packets::print::handle(self, p),
            (None, "note")  => crate::packets::note::handle(self, p),

            // (None, "loop")  => crate::packets::r#loop::handle(self, p),
            // (None, "if")    => crate::packets::conditionals::handle(self, p),

            other => Err(anyhow::anyhow!("unknown operation: [{:?}]", other)),
        }
    }
}
