// Grouped packet modules by role
pub mod core {
    pub mod array;
    pub mod bool;
    pub mod cd;
    pub mod compare;
    pub mod dump;
    pub mod env;
    pub mod help;
    pub mod input;
    pub mod int;
    pub mod len;
    pub mod lint;
    pub mod math;
    pub mod msg;
    pub mod note;
    pub mod obj;
    pub mod parse;
    pub mod print;
    pub mod rand;
    pub mod reflect;
    pub mod store;
    pub mod var;
}

pub mod files {
    pub mod load;
    pub mod log;
    pub mod modify;
    pub mod query;
    pub mod save;
    pub mod search;
}

pub mod flow {
    pub mod async_run;
    pub mod await_pkt;
    pub mod r#break;
    pub mod call;
    pub mod conditionals;
    pub mod funct;
    pub mod interrupt;
    pub mod interval;
    pub mod iter;
    pub mod r#loop;
    pub mod r#return;
}

pub mod execs {
    pub mod confirm;
    pub mod exec;
    pub mod http;
    pub mod red;
    pub mod repl;
    pub mod run;
    pub mod tagspeak;
}

pub mod ui {
    pub mod alert;
    pub mod app;
    pub mod scope;
    pub mod select;
    pub mod window;
}

pub mod time {
    pub mod clock;
    pub mod timeout;
}

pub mod fun;

// Re-export for backward compatibility with existing paths
pub use self::time::{clock, timeout};
#[allow(unused_imports)]
pub use core::{
    array, bool, cd, compare, dump, env, help, input, int, len, lint, math, msg, note, obj, parse,
    print, rand, reflect, store, var,
};
pub use execs::{confirm, exec, http, red, repl, run, tagspeak};
pub use files::{load, log, modify, query, save, search};
pub use flow::{
    async_run, await_pkt, r#break, call, conditionals, funct, interrupt, interval, iter, r#loop,
    r#return,
};
pub use ui::{
    alert as ui_alert, app as ui_app, scope as ui_scope, select as ui_select, window as ui_window,
};
