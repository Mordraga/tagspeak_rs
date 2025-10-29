use anyhow::{Result, bail, Context};
use anyhow::anyhow;
use crate::packets::core::var as pkt_var;
use std::collections::{HashMap, HashSet, VecDeque};
use std::mem;
use std::path::{Path, PathBuf};
use std::thread;

use crate::kernel::ast::{Arg, BExpr, Node, Packet};
use crate::kernel::fs_guard::find_root;
use crate::kernel::packet_catalog::suggest_packet;
use crate::kernel::values::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum FlowSignal {
    None,
    Break,
    Return(Option<Value>),
    Interrupt(Option<Value>),
}

#[derive(Clone, Debug)]
pub struct FunctionDef {
    pub body: Vec<Node>,
    pub is_async: bool,
}

pub struct AsyncTask {
    pub handle: Option<thread::JoinHandle<anyhow::Result<Value>>>,
}

pub struct Runtime {
    pub vars: HashMap<String, Value>,
    pub ctx_vars: HashMap<String, Vec<(BExpr, Value)>>,
    pub rigid: HashSet<String>,
    pub last: Value,
    pub tags: HashMap<String, FunctionDef>, // named blocks from [funct:tag]{...}
    pub effective_root: Option<PathBuf>,
    pub cwd: PathBuf,
    // safety limits
    pub call_depth: usize,
    pub max_call_depth: usize,
    pub flow_signal: FlowSignal,
    pub async_tasks: HashMap<String, VecDeque<AsyncTask>>,
    pub task_counter: usize,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        Self::from_start(&cwd)
    }

    pub fn from_entry(entry: &Path) -> Result<Self> {
        let start = entry.parent().unwrap_or_else(|| Path::new("."));
        Self::from_start(start)
    }

    fn from_start(start: &Path) -> Result<Self> {
        let root = find_root(start);
        let cwd = match &root {
            Some(r) => start.strip_prefix(r).unwrap_or(Path::new("")).to_path_buf(),
            None => PathBuf::new(),
        };
        Ok(Self {
            vars: HashMap::new(),
            ctx_vars: HashMap::new(),
            rigid: HashSet::new(),
            last: Value::Unit,
            tags: HashMap::new(),
            async_tasks: HashMap::new(),
            task_counter: 0,
            flow_signal: FlowSignal::None,
            effective_root: root,
            cwd,
            call_depth: 0,
            max_call_depth: std::env::var("TAGSPEAK_MAX_CALL_DEPTH").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(256),
        })
    }

    // ---- variables ----
    pub fn set_var(&mut self, name: &str, val: Value) -> Result<()> {
        self.vars.insert(name.to_string(), val);
        Ok(())
    }
    pub fn get_var(&self, name: &str) -> Option<Value> {
        // direct binding wins
        if let Some(v) = self.vars.get(name) {
            return Some(v.clone());
        }

        // context-aware binding: pick first matching condition
        if let Some(entries) = self.ctx_vars.get(name) {
            // Evaluate conditions against a temporary runtime seeded with our vars
            for (cond, val) in entries {
                // create a temp runtime to evaluate the condition without side effects
                let mut tmp = match Runtime::new() {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                tmp.vars = self.vars.clone();
                tmp.tags = self.tags.clone();
                if crate::packets::conditionals::eval_cond(&mut tmp, cond).unwrap_or(false) {
                    return Some(val.clone());
                }
            }
        }

        None
    }

    // ---- tags ----
    pub fn register_tag(&mut self, name: &str, body: Vec<Node>, is_async: bool) {
        self.tags
            .insert(name.to_string(), FunctionDef { body, is_async });
    }

    pub fn get_tag(&self, name: &str) -> Option<&FunctionDef> {
        self.tags.get(name)
    }

    // ---- args ----
    pub fn resolve_arg(&self, arg: &Arg) -> Result<Value> {
        Ok(match arg {
            Arg::Number(n) => Value::Num(*n),
            Arg::Str(s) => Value::Str(s.clone()),
            Arg::Ident(id) => self.get_var(id).unwrap_or(Value::Unit),
            _ => Value::Unit, // reserve for CondSrc/etc
        })
    }

    // ---- eval ----
    pub fn eval(&mut self, n: &Node) -> Result<Value> {
        let out = match n {
            Node::Chain(v) | Node::Block(v) => self.eval_list(v)?,
            Node::Packet(p) => self.eval_packet(p)?,
            Node::If {
                cond,
                then_b,
                else_b,
            } => {
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
            if self.signal_active() {
                break;
            }
            last = self.eval(node)?;
            if self.signal_active() {
                break;
            }
        }
        Ok(last)
    }

    fn eval_packet(&mut self, p: &Packet) -> Result<Value> {
        match (p.ns.as_deref(), p.op.as_str()) {
            // UI namespace
            (Some("ui"), "alert") => crate::packets::ui_alert::handle(self, p),
            (Some("ui"), "select") => crate::packets::ui_select::handle(self, p),
            (Some("ui"), "window") => crate::packets::ui_window::handle(self, p),
            (None, "app") => crate::packets::ui_app::handle(self, p),
            (None, "scope") => crate::packets::ui_scope::handle(self, p),
            // namespaced
            (Some(ns), "async") if ns.starts_with("fn(") => crate::packets::funct::handle(self, p),
            (Some("funct"), _) => crate::packets::funct::handle(self, p),
            (None, "funct") => crate::packets::funct::handle(self, p),
            (Some("tagspeak"), _) => crate::packets::tagspeak::handle(self, p),

            // allow namespaced loop syntax: [loop:tag@N]
            (Some("loop"), _) => crate::packets::r#loop::handle(self, p),
            // allow namespaced store modes: [store:rigid@x], [store:context(cond)@x]
            (Some("store"), _) => crate::packets::store::handle(self, p),

            // core
            (None, "note") => crate::packets::note::handle(self, p),
            (None, "math") => crate::packets::math::handle(self, p),
            (None, "store") => crate::packets::store::handle(self, p),
            (None, "print") => crate::packets::print::handle(self, p),
            (None, "var") => pkt_var::handle(self, p),
            (None, "dump") => crate::packets::dump::handle(self, p),
            (None, "call") => crate::packets::call::handle(self, p),
            (None, "msg") => crate::packets::msg::handle(self, p),
            (None, "int") => crate::packets::int::handle(self, p),
            (None, "bool") => crate::packets::bool::handle(self, p),
            (None, "env") => crate::packets::env::handle(self, p),
            (None, "help") => crate::packets::help::handle(self, p),
            (None, "lint") => crate::packets::lint::handle(self, p),
            (None, "cd") => crate::packets::cd::handle(self, p),
            (None, "len") => crate::packets::len::handle(self, p),
            (None, "rand") => crate::packets::rand::handle(self, p),
            (None, op) if op.starts_with("rand(") => crate::packets::rand::handle(self, p),
            (None, "array") => crate::packets::array::handle(self, p),
            (None, "obj") => crate::packets::obj::handle(self, p),
            (None, op) if op.starts_with("reflect(") => crate::packets::reflect::handle(self, p),
            (None, "load") => crate::packets::load::handle(self, p),
            (None, op) if op.starts_with("search(") => crate::packets::search::handle(self, p),
            (None, op) if op.starts_with("log") => crate::packets::log::handle(self, p),
            (None, "save") => crate::packets::save::handle(self, p),
            (None, "mod") => crate::packets::modify::handle(self, p),
            (None, op) if op.starts_with("mod(") => crate::packets::modify::handle(self, p),
            (None, "exec") => crate::packets::exec::handle(self, p),
            (None, op) if op.starts_with("exec(") => crate::packets::exec::handle(self, p),
            (None, "run") => crate::packets::run::handle(self, p),
            (None, op) if op == "tagspeak" || op.starts_with("tagspeak ") => {
                crate::packets::tagspeak::handle(self, p)
            }
            (None, "yellow") => crate::packets::confirm::handle(self, p),
            (None, "confirm") => crate::packets::confirm::handle(self, p),
            (None, "red") => crate::packets::red::handle(self, p),
            (None, op) if op.starts_with("http(") => crate::packets::http::handle(self, p),
            (None, op) if op.starts_with("repl(") => crate::packets::repl::handle(self, p),
            (None, op) if op.starts_with("parse(") => crate::packets::parse::handle(self, p),
            (None, op) if op.starts_with("get(") || op.starts_with("exists(") => {
                crate::packets::query::handle(self, p)
            }
            (None, "iter") => crate::packets::iter::handle(self, p),
            (None, op) if op.eq_ignore_ascii_case("utc") => crate::packets::clock::handle_utc(self, p),
            (None, op) if op.eq_ignore_ascii_case("local") => crate::packets::clock::handle_local(self, p),
            (None, "async") => crate::packets::async_run::handle(self, p),
            (None, "await") => crate::packets::await_pkt::handle(self, p),
            (None, "break") => crate::packets::r#break::handle(self, p),
            (None, "return") => crate::packets::r#return::handle(self, p),
            (None, "interrupt") => crate::packets::interrupt::handle(self, p),
            (Some("interval"), _) => crate::packets::interval::handle(self, p),
            (Some("timeout"), _) => crate::packets::timeout::handle(self, p),
            (Some("input"), "line") => crate::packets::input::handle(self, p),
            (None, "input") => crate::packets::input::handle(self, p),
            (None, op) if matches!(op, "eq" | "ne" | "lt" | "le" | "gt" | "ge") => {
                crate::packets::compare::handle(self, p)
            }

            // loop forms: [loop3@tag] or [loop@N]{...}
            (None, op) if op.starts_with("loop") => crate::packets::r#loop::handle(self, p),

            // namespaced comparators: [cmp:eq@rhs]
            (Some("cmp"), _) => crate::packets::compare::handle(self, p),

            // namespaced yellow sugar
            (Some("yellow"), "exec") => crate::packets::confirm::handle_exec(self, p),
            (Some("yellow"), "run") => crate::packets::confirm::handle_run(self, p),

            other => {
                let suggestion = suggest_packet(other.0, other.1);
                if let Some(s) = suggestion {
                    bail!("unknown operation: {:?} (did you mean '{s}'?)", other);
                } else {
                    bail!("unknown operation: {:?}", other);
                }
            }
        }
    }

    pub fn set_signal(&mut self, signal: FlowSignal) {
        self.flow_signal = signal;
    }

    pub fn signal_active(&self) -> bool {
        !matches!(self.flow_signal, FlowSignal::None)
    }

    pub fn take_signal(&mut self) -> FlowSignal {
        mem::replace(&mut self.flow_signal, FlowSignal::None)
    }

    pub fn fork(&self) -> Result<Self> {
        Ok(Self {
            vars: self.vars.clone(),
            ctx_vars: self.ctx_vars.clone(),
            rigid: self.rigid.clone(),
            last: self.last.clone(),
            tags: self.tags.clone(),
            async_tasks: HashMap::new(),
            task_counter: 0,
            flow_signal: FlowSignal::None,
            effective_root: self.effective_root.clone(),
            cwd: self.cwd.clone(),
            call_depth: 0,
            max_call_depth: self.max_call_depth,
        })
    }

    pub fn spawn_async_block(&mut self, body: Vec<Node>) -> Result<()> {
        let mut child = self.fork()?;
        thread::spawn(move || {
            if let Err(err) = child.eval(&Node::Block(body)) {
                eprintln!("async block error: {err:?}");
            }
        });
        Ok(())
    }

    pub fn enqueue_async_function(&mut self, name: &str) -> Result<()> {
        let func_ref = self
            .get_tag(name)
            .with_context(|| format!("unknown async funct '{name}'"))?;
        if !func_ref.is_async {
            bail!("'{name}' is not marked async");
        }
        let func = func_ref.clone();
        let mut child = self.fork()?;
        let handle = thread::spawn(move || child.eval(&Node::Block(func.body)));
        let entry = self.async_tasks.entry(name.to_string()).or_default();
        entry.push_back(AsyncTask {
            handle: Some(handle),
        });
        self.task_counter = self.task_counter.wrapping_add(1);
        Ok(())
    }

    pub fn await_async_function(&mut self, name: &str) -> Result<Value> {
        if !self.async_tasks.contains_key(name) {
            self.enqueue_async_function(name)?;
        }

        if self
            .async_tasks
            .get(name)
            .map(|queue| queue.is_empty())
            .unwrap_or(true)
        {
            self.enqueue_async_function(name)?;
        }

        let (mut task, remove_entry) = {
            let queue = self
                .async_tasks
                .get_mut(name)
                .with_context(|| format!("no async tasks for '{name}'"))?;
            let popped = queue
                .pop_front()
                .with_context(|| format!("no pending async task for '{name}'"))?;
            let should_remove = queue.is_empty();
            (popped, should_remove)
        };

        let handle = task
            .handle
            .take()
            .with_context(|| "async task missing handle")?;
        let result = handle
            .join()
            .map_err(|_| anyhow!("async task panicked"))??;

        if remove_entry {
            self.async_tasks.remove(name);
        }

        Ok(result)
    }

    // small helpers for numeric vars used by packets
    pub fn get_num(&self, name: &str) -> Option<f64> {
        self.get_var(name).and_then(|v| v.try_num())
    }
    pub fn set_num(&mut self, name: &str, n: f64) -> Result<()> {
        self.set_var(name, Value::Num(n))?;
        Ok(())
    }

    fn eval_if(&mut self, cond: &BExpr) -> Result<bool> {
        crate::packets::conditionals::eval_cond(self, cond)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn from_entry_detects_red_root() {
        let base = std::env::temp_dir().join(format!("tgsk_rt_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("red.tgsk"), "").unwrap();
        let script = base.join("sub").join("main.tgsk");
        fs::write(&script, "").unwrap();

        let rt = Runtime::from_entry(&script).unwrap();
        assert_eq!(rt.effective_root.as_deref(), Some(base.as_path()));

        fs::remove_dir_all(base).unwrap();
    }
}

