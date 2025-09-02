// Grouped packet modules by role
pub mod core {
    pub mod bool;
    pub mod int;
    pub mod math;
    pub mod msg;
    pub mod note;
    pub mod print;
    pub mod dump;
    pub mod store;
    pub mod parse;
    pub mod env;
    pub mod cd;
    pub mod len;
    pub mod compare;
    pub mod array;
    pub mod obj;
    pub mod reflect;
    pub mod input;
}

pub mod files {
    pub mod load;
    pub mod log;
    pub mod save;
    pub mod modify;
    pub mod query;
}

pub mod flow {
    pub mod funct;
    pub mod call;
    pub mod r#loop;
    pub mod conditionals;
    pub mod iter;
}

pub mod execs {
    pub mod exec;
    pub mod run;
    pub mod http;
    pub mod confirm;
    pub mod red;
    pub mod repl;
}

// Re-export for backward compatibility with existing paths
pub use core::{bool, int, math, msg, note, print, dump, store, parse};
pub use core::{env, cd, len, compare, array, obj, reflect};
pub use core::input;
pub use files::{load, log, save, modify, query};
pub use flow::{funct, call, r#loop, conditionals, iter};
pub use execs::{exec, run, http, confirm, red, repl};
