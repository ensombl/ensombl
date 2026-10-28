//! Application layer: use cases split into commands (writes) and queries
//! (reads), plus the ports infrastructure must implement.

pub mod commands;
pub mod cqrs;
pub mod error;
pub mod ids;
pub mod index_registry;
pub mod ports;
pub mod queries;
pub mod worktrees;

pub use error::{Error, Result};
