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
}

pub mod files {
    pub mod load;
    pub mod log;
    pub mod save;
    pub mod modify;
}

pub mod flow {
    pub mod funct;
    pub mod call;
    pub mod r#loop;
    pub mod conditionals;
}

pub mod execs {
    pub mod exec;
    pub mod run;
    pub mod http;
    pub mod confirm;
}

// Re-export for backward compatibility with existing paths
pub use core::{bool, int, math, msg, note, print, dump, store, parse};
pub use files::{load, log, save, modify};
pub use flow::{funct, call, r#loop, conditionals};
pub use execs::{exec, run, http, confirm};
