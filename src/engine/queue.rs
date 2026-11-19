//! Queue items, states and transitions (DESIGN 5, 7.6). Pure logic: no I/O, no clock.

use crate::{
    errors::ErrorReport,
    pkg::{Icon, Kind, PkgInfo},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Parsing,
    Invalid,
    Ready,
    Queued,
    Checking,
    AwaitingDecision,
    Uninstalling,
    Starting,
    Transferring,
    Installing,
    Paused,
    Done,
    Failed,
    Cancelled,
    Skipped,
}

impl State {
    /// Active = owns the single "running" slot (DESIGN 5.2 rules).
    pub fn is_active(self) -> bool {
        use State::*;
        matches!(
            self,
            Checking
                | AwaitingDecision
                | Uninstalling
                | Starting
                | Transferring
                | Installing
                | Paused
        )
    }
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            State::Done | State::Failed | State::Cancelled | State::Skipped
        )
    }
    /// Persisted form (DESIGN 7.6).
    pub fn persisted(self) -> PState {
        use State::*;
        match self {
            Invalid => PState::Invalid,
            Transferring | Installing | Paused => PState::Interrupted,
            Done => PState::Done,
            Failed => PState::Failed,
            Cancelled => PState::Cancelled,
            Skipped => PState::Skipped,
            Parsing | Ready | Queued | Checking | AwaitingDecision | Uninstalling | Starting => {
                PState::Ready
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PState {
    Ready,
    Invalid,
    Done,
    Failed,
    Cancelled,
    Skipped,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Skip,
    Reinstall,
}

/// What happened to an item; `next` says where it goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ev {
    ParseOk,
    ParseErr,
    StartQueue,
    StopQueue,
    TakeTurn,
    /// Check result: `exists` and the stored decision.
    Checked {
        exists: bool,
        decision: Option<Decision>,
    },
    /// Update/DLC base game missing or the check failed.
    CheckFailed,
    Decide(Decision),
    UninstallOk,
    InstallStarted,
    TransferDone,
    Confirmed,
    Error,
    Pause,
    Resume,
    Cancel,
    Retry,
    /// Resume of a saved task after restart (DESIGN 9.3).
    Resumed,
    /// Standalone uninstall (DESIGN 6.7) start / end.
    UninstallBegin,
}

/// DESIGN 5.2. `None` = transition not allowed (caller ignores the event).
pub fn next(s: State, ev: Ev) -> Option<State> {
    use Ev::*;
    use State::*;
    Some(match (s, ev) {
        (Parsing, ParseOk) => Ready,
        (Parsing, ParseErr) => Invalid,
        (Ready, StartQueue) => Queued,
        (Queued, StopQueue) => Ready,
        (Queued, TakeTurn) => Checking,
        (Checking, Checked { exists: false, .. }) => Starting,
        (
            Checking,
            Checked {
                exists: true,
                decision: None,
            },
        ) => AwaitingDecision,
        (
            Checking,
            Checked {
                exists: true,
                decision: Some(Decision::Skip),
            },
        ) => Skipped,
        (
            Checking,
            Checked {
                exists: true,
                decision: Some(Decision::Reinstall),
            },
        ) => Uninstalling,
        (Checking, CheckFailed) => Failed,
        (Checking, Resumed) => Transferring,
        (AwaitingDecision, Decide(Decision::Skip)) => Skipped,
        (AwaitingDecision, Decide(Decision::Reinstall)) => Uninstalling,
        (Uninstalling, UninstallOk) => Starting,
        (Starting, InstallStarted) => Transferring,
        (Transferring, TransferDone) => Installing,
        (Transferring | Installing, Confirmed) => Done,
        (Transferring, Pause) => Paused,
        (Paused, Resume) => Transferring,
        (Starting | Transferring | Installing | Paused, Cancel) => Cancelled,
        (
            Checking | AwaitingDecision | Uninstalling | Starting | Transferring | Installing
            | Paused,
            Error,
        ) => Failed,
        (Failed | Cancelled | Skipped, Retry) => Ready,
        (Ready | Done | Skipped | Failed | Cancelled, UninstallBegin) => Uninstalling,
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartRef {
    pub path: PathBuf,
    pub size: u64,
    /// Unix seconds; detects changed files (PKG-07).
    pub mtime: u64,
}

/// Live transfer numbers (DESIGN 9.2).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Live {
    pub fraction: Option<f64>,
    pub transferred: u64,
    pub total: u64,
    /// Smoothed bytes/s, shown after 3 samples.
    pub speed: Option<f64>,
    /// Smoothed seconds remaining.
    pub eta: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub parts: Vec<PartRef>,
    pub content_id: String,
    pub title_id: String,
    pub title: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
    pub state: State,
    pub decision: Option<Decision>,
    pub task_id: Option<u64>,
    pub error: Option<ErrorReport>,
    pub finished_at: Option<u64>,
    pub icon: Option<Arc<Icon>>,
    pub live: Live,
    /// Short context line (e.g. "split: 3 parts", missing parts).
    pub note: Option<String>,
    /// Info from the check right after adding (DESIGN 6.1/6); never replaces the check before install.
    pub pre: Option<Pre>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pre {
    NotInstalled,
    /// Game already on the console (Faza 4 asks Skip/Reinstall).
    Installed,
    /// Update/DLC whose base game is earlier in the queue.
    WaitsFor(String),
    /// Update/DLC whose base game is neither installed nor queued.
    BaseMissing,
}

impl Item {
    pub fn kind(&self) -> Kind {
        self.category
            .as_deref()
            .map_or(Kind::Other("?".into()), Kind::from_category)
    }
    pub fn total_size(&self) -> u64 {
        self.parts.iter().map(|p| p.size).sum()
    }
    pub fn from_info(id: String, parts: Vec<PartRef>, info: &PkgInfo) -> Item {
        Item {
            id,
            parts,
            content_id: info.content_id.clone(),
            title_id: info.title_id.clone(),
            title: info.title.clone(),
            version: info.display_version().map(str::to_string),
            category: info.category.clone(),
            state: State::Ready,
            decision: None,
            task_id: None,
            error: None,
            finished_at: None,
            icon: info.icon.clone().map(Arc::new),
            live: Live::default(),
            note: None,
            pre: None,
        }
    }
}

/// `queue.json` item (DESIGN 7.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    pub id: String,
    pub parts: Vec<PartRef>,
    pub content_id: String,
    pub title_id: String,
    pub title: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
    pub state: PState,
    pub decision: Option<Decision>,
    pub task_id: Option<u64>,
    pub error: Option<ErrorReport>,
    pub finished_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedQueue {
    pub schema: u32,
    pub items: Vec<Saved>,
}

impl From<&Item> for Saved {
    fn from(i: &Item) -> Saved {
        Saved {
            id: i.id.clone(),
            parts: i.parts.clone(),
            content_id: i.content_id.clone(),
            title_id: i.title_id.clone(),
            title: i.title.clone(),
            version: i.version.clone(),
            category: i.category.clone(),
            state: i.state.persisted(),
            decision: i.decision,
            task_id: i.task_id,
            error: i.error.clone(),
            finished_at: i.finished_at,
        }
    }
}

impl From<Saved> for Item {
    /// `interrupted` becomes Failed RPI-07 on load (DESIGN 5.2 rules); task_id is kept for resume.
    fn from(s: Saved) -> Item {
        let (state, error) = match s.state {
            PState::Ready => (State::Ready, s.error),
            PState::Invalid => (State::Invalid, s.error),
            PState::Done => (State::Done, s.error),
            PState::Failed => (State::Failed, s.error),
            PState::Cancelled => (State::Cancelled, s.error),
            PState::Skipped => (State::Skipped, s.error),
            PState::Interrupted => (
                State::Failed,
                Some(ErrorReport::new(
                    crate::errors::Code::Rpi07,
                    "interrupted by closing the app",
                )),
            ),
        };
        Item {
            id: s.id,
            parts: s.parts,
            content_id: s.content_id,
            title_id: s.title_id,
            title: s.title,
            version: s.version,
            category: s.category,
            state,
            decision: s.decision,
            task_id: s.task_id,
            error,
            finished_at: s.finished_at,
            icon: None,
            live: Live::default(),
            note: None,
            pre: None,
        }
    }
}

/// Speed/ETA smoothing (DESIGN 9.2): EMA α=0.2 for speed, α=0.3 for RPI's rest_sec.
#[derive(Debug, Clone, Default)]
pub struct Smoother {
    last: Option<(f64, u64)>,
    samples: u32,
    speed: Option<f64>,
    eta: Option<f64>,
}

impl Smoother {
    /// `t` = seconds (monotonic), `transferred`/`total` bytes, `rest_sec` from RPI if any.
    pub fn sample(
        &mut self,
        t: f64,
        transferred: u64,
        total: u64,
        rest_sec: Option<u64>,
    ) -> (Option<f64>, Option<f64>) {
        if let Some((t0, b0)) = self.last
            && t > t0
        {
            let inst = transferred.saturating_sub(b0) as f64 / (t - t0);
            self.speed = Some(self.speed.map_or(inst, |s| s + 0.2 * (inst - s)));
            self.samples += 1;
        }
        self.last = Some((t, transferred));
        let shown_speed = self.speed.filter(|_| self.samples >= 3);
        self.eta = match rest_sec.filter(|&r| r > 0) {
            Some(r) => Some(self.eta.map_or(r as f64, |e| e + 0.3 * (r as f64 - e))),
            None => shown_speed
                .filter(|&s| s > 0.0)
                .map(|s| total.saturating_sub(transferred) as f64 / s),
        };
        (shown_speed, self.eta.filter(|_| self.samples >= 3))
    }
}

/// 64-bit random hex id (DESIGN 7.6; not cryptographic, the server also filters by IP).
pub fn new_id() -> String {
    use std::{
        collections::hash_map::RandomState,
        hash::{BuildHasher, Hasher},
        sync::atomic::{AtomicU64, Ordering},
    };
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut h = RandomState::new().build_hasher();
    h.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use State::*;

    #[test]
    fn happy_path_and_decisions() {
        let mut s = Parsing;
        for ev in [
            Ev::ParseOk,
            Ev::StartQueue,
            Ev::TakeTurn,
            Ev::Checked {
                exists: false,
                decision: None,
            },
            Ev::InstallStarted,
            Ev::TransferDone,
            Ev::Confirmed,
        ] {
            s = next(s, ev).unwrap_or_else(|| panic!("{s:?} {ev:?}"));
        }
        assert_eq!(s, Done);
        assert_eq!(
            next(
                Checking,
                Ev::Checked {
                    exists: true,
                    decision: None
                }
            ),
            Some(AwaitingDecision)
        );
        assert_eq!(
            next(
                Checking,
                Ev::Checked {
                    exists: true,
                    decision: Some(Decision::Skip)
                }
            ),
            Some(Skipped)
        );
        assert_eq!(
            next(AwaitingDecision, Ev::Decide(Decision::Reinstall)),
            Some(Uninstalling)
        );
        assert_eq!(next(Uninstalling, Ev::UninstallOk), Some(Starting));
        assert_eq!(next(Transferring, Ev::Pause), Some(Paused));
        assert_eq!(next(Paused, Ev::Cancel), Some(Cancelled));
        assert_eq!(next(Failed, Ev::Retry), Some(Ready));
        assert_eq!(next(Done, Ev::Retry), None);
        assert_eq!(next(Queued, Ev::Pause), None);
        assert_eq!(next(Done, Ev::UninstallBegin), Some(Uninstalling));
        assert_eq!(next(Queued, Ev::UninstallBegin), None);
    }

    #[test]
    fn persistence_mapping() {
        assert_eq!(Transferring.persisted(), PState::Interrupted);
        assert_eq!(AwaitingDecision.persisted(), PState::Ready);
        let saved = Saved {
            id: "a".into(),
            parts: vec![],
            content_id: "c".into(),
            title_id: "CUSA00001".into(),
            title: None,
            version: None,
            category: Some("gd".into()),
            state: PState::Interrupted,
            decision: None,
            task_id: Some(8),
            error: None,
            finished_at: None,
        };
        let j = serde_json::to_string(&saved).unwrap();
        assert!(j.contains("\"state\":\"interrupted\""));
        let item: Item = serde_json::from_str::<Saved>(&j).unwrap().into();
        assert_eq!((item.state, item.task_id), (Failed, Some(8)));
        assert_eq!(item.error.unwrap().code, crate::errors::Code::Rpi07);
    }

    #[test]
    fn smoothing() {
        let mut sm = Smoother::default();
        let mut out = (None, None);
        for i in 0..5u64 {
            out = sm.sample(i as f64, i * 10_000_000, 100_000_000, None);
            if i < 3 {
                assert_eq!(out, (None, None));
            }
        }
        let (speed, eta) = out;
        assert!((speed.unwrap() - 10_000_000.0).abs() < 1.0);
        assert!((eta.unwrap() - 6.0).abs() < 0.01);
        assert_ne!(new_id(), new_id());
        assert_eq!(new_id().len(), 16);
    }
}
