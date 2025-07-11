#![allow(clippy::type_complexity)]
pub mod discovery;
pub mod logger;
pub mod protocol;
pub mod script;
pub mod system_info;
pub mod utils;

pub mod agent;
pub mod daemon;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " - ", env!("GIT_HASH"));
