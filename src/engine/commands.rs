//! Command (UI → engine) and Event (engine → UI), DESIGN 7.3.

use super::queue::{Decision, Item, PartRef};
use crate::{config::Config, errors::ErrorReport, pkg::PkgInfo, rpi::UninstallKind};

/// What to uninstall (DESIGN 6.7): a queue item or a typed ID (dialog 4.12).
#[derive(Debug, Clone, PartialEq)]
pub struct UninstallTarget {
    pub kind: UninstallKind,
    /// Title ID for games/updates, Content ID for DLC.
    pub id: String,
    /// Queue item this came from, if any.
    pub item: Option<String>,
}
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
    /// Is the target installed? Answer: `Event::InstalledResult` (no state change).
    CheckInstalled(UninstallTarget),
    /// Uninstall after the user confirmed dialog 4.11.
    Uninstall(UninstallTarget),
    /// Internal: background uninstall finished. Ok(true) = removed, Ok(false) = was not installed.
    UninstallDone {
        target: UninstallTarget,
        result: Result<bool, ErrorReport>,
    },
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
    /// Ok(installed) or the error text (RPI-02).
    InstalledResult {
        target: UninstallTarget,
        result: Result<bool, String>,
    },
    UninstallResult {
        target: UninstallTarget,
        result: Result<bool, ErrorReport>,
    },
}
