mod api;
pub mod config;
pub mod database;
pub mod error;
pub mod provider;
pub mod tools;
mod turn;

pub use api::application;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
