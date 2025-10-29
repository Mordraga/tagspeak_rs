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
    pub mod call;
    pub mod conditionals;
    pub mod funct;
    pub mod iter;
    pub mod r#loop;
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
    pub mod select;
    pub mod window;
    pub mod app;
    pub mod scope;
}

// Re-export for backward compatibility with existing paths
#[allow(unused_imports)]
pub use core::{
    array, bool, cd, compare, dump, env, help, input, int, len, lint, math, msg, note, obj, parse,
    print, rand, reflect, store, var,
};
pub use execs::{confirm, exec, http, red, repl, run, tagspeak};
pub use files::{load, log, modify, query, save, search};
pub use flow::{call, conditionals, funct, iter, r#loop};
pub use ui::{alert as ui_alert, select as ui_select, window as ui_window, app as ui_app, scope as ui_scope};
