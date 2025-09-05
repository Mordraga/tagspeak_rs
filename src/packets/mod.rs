// Grouped packet modules by role
pub mod core {
    pub mod bool;
    pub mod dump;
    pub mod int;
    pub mod math;
    pub mod msg;
    pub mod note;
    pub mod parse;
    pub mod print;
    pub mod store;
}

pub mod files {
    pub mod load;
    pub mod log;
    pub mod modify;
    pub mod save;
    pub mod search;
}

pub mod flow {
    pub mod call;
    pub mod conditionals;
    pub mod funct;
    pub mod r#loop;
}

pub mod execs {
    pub mod confirm;
    pub mod exec;
    pub mod http;
    pub mod run;
}

// Re-export for backward compatibility with existing paths
pub use core::{bool, dump, int, math, msg, note, parse, print, store};
pub use execs::{confirm, exec, http, run};
pub use files::{load, log, modify, save, search};
pub use flow::{call, conditionals, funct, r#loop};
