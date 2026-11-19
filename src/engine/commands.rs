//! Command (UI → engine) and Event (engine → UI), DESIGN 7.3.

use super::queue::{Decision, Item, PartRef};
use crate::{config::Config, errors::ErrorReport, pkg::PkgInfo};
use std::{net::SocketAddrV4, path::PathBuf};

/// Parsed package data, its parts and an optional note (`split: n`, internal `reload`).
pub type ParseResult = Result<(PkgInfo, Vec<PartRef>, Option<String>), ErrorReport>;

#[derive(Debug)]
pub enum Command {
    AddPaths(Vec<PathBuf>),
    Remove(String),
    Move {
        id: String,
        to: usize,
    },
    StartQueue,
    StopAfterCurrent,
    Pause(String),
    Resume(String),
    Cancel(String),
    Decide {
        id: String,
        decision: Decision,
        apply_all: bool,
    },
    Retry(String),
    ApplySettings(Box<Config>),
    ClearFinished,
    Shutdown,
    /// Internal: result of the check right after adding.
    Precheck {
        id: String,
        exists: bool,
    },
    /// Internal: result from the parse pool.
    Parsed {
        id: String,
        result: Box<ParseResult>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Banner {
    ConsoleNotConfigured,
    /// Global stop: console unreachable / server failure (DESIGN 5.2, 6.4/4).
    Global(ErrorReport),
}

#[derive(Debug, Clone)]
pub enum Event {
    ItemUpsert(Box<Item>),
    ItemRemoved(String),
    Order(Vec<String>),
    AskDecision(String),
    Banner(Option<Banner>),
    ServerStatus(Option<SocketAddrV4>),
    QueueRunning(bool),
    /// One-off info line (duplicates, skipped non-pkg files), already in the log too.
    Info(String),
}
