//! RPI REST client (DESIGN 9.1).

pub mod client;
pub mod types;

pub use client::{Client, LastReach, RpiError, UninstallKind, Unreachable};
pub use types::{Exists, Progress, RpiDialect};
