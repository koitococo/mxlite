#![allow(clippy::type_complexity)]
pub mod discovery;
pub mod hash;
pub mod logger;
pub mod protocol;
pub mod system_info;
pub mod utils;
pub mod script;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " - ", env!("GIT_HASH"));

pub use url::Url;
pub use mlua;