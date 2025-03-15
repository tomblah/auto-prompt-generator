// crates/enclosing_util/src/lib.rs

pub mod traits;
pub mod default;
mod factory;  // internal
pub mod api;

pub use api::extract_context;
pub use factory::ProgrammingLanguage;
