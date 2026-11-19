//! Engine thread: sole owner of the queue and `queue.json` (DESIGN 7.3, flow 6.2).

pub mod commands;
pub mod queue;

use crate::{
    config::Config,
    errors::{Code, ErrorReport, find_console_codes},
    logging::Logger,
    net,
    pkg::{self, Kind, PkgError, scan},
    rpi::{Client, RpiDialect, RpiError, UninstallKind},
    server::{self, Server, ServerEvent, registry},
    storage, winapi,
};
use commands::{Banner, Command, Event};
use queue::{Decision, Ev, Item, PartRef, Smoother, State};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const QUEUE_FILE: &str = "queue.json";

/// All engine time constants, so tests can run in milliseconds (PLAN 1.13).
#[derive(Debug, Clone)]
pub struct EngineTimings {
    pub tick: Duration,
    pub fw01: Duration,
    pub stall: Duration,
    pub verify_tries: u32,
    pub verify_gap: Duration,
    pub confirm_gap: Duration,
    pub confirm_max: Duration,
    pub dereg_delay: Duration,
    pub save_debounce: Duration,
    pub unreachable_limit: u32,
}

impl Default for EngineTimings {
    fn default() -> Self {
        Self {
            tick: Duration::from_secs(1),
            fw01: Duration::from_secs(20),
            stall: Duration::from_secs(60),
            verify_tries: 3,
            verify_gap: Duration::from_secs(2),
            confirm_gap: Duration::from_secs(2),
            confirm_max: Duration::from_secs(600),
            dereg_delay: Duration::from_secs(60),
            save_debounce: Duration::from_millis(500),
            unreachable_limit: 5,
        }
    }
}

pub struct EngineOptions {
    /// Where `queue.json` lives; `None` = no persistence (tests, headless).
    pub data_dir: Option<PathBuf>,
    pub config: Config,
    pub timings: EngineTimings,
    pub dialect: RpiDialect,
    pub log: Logger,
}

pub struct EngineHandle {
    pub tx: Sender<Command>,
    pub join: thread::JoinHandle<()>,
}

pub type Notify = Arc<dyn Fn() + Send + Sync>;

pub fn spawn(opts: EngineOptions, events: Sender<Event>, notify: Notify) -> EngineHandle {
    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();
    let join = thread::Builder::new()
        .name("engine".into())
        .spawn(move || Engine::new(opts, events, notify, tx2).run(rx))
        .expect("spawn engine thread");
    EngineHandle { tx, join }
}

/// Runtime of the item in the single active slot.
struct Active {
    id: String,
    task_started: Option<Instant>,
    saw_request: bool,
    best: u64,
    best_at: Instant,
    smoother: Smoother,
    clock: Instant,
    unreachable: u32,
    confirm_since: Option<Instant>,
    next_confirm: Instant,
}

impl Active {
    fn new(id: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            task_started: None,
            saw_request: false,
            best: 0,
            best_at: now,
            smoother: Smoother::default(),
            clock: now,
            unreachable: 0,
            confirm_since: None,
            next_confirm: now,
        }
    }
}

struct Engine {
    o: EngineOptions,
    items: Vec<Item>,
    events: Sender<Event>,
    notify: Notify,
    self_tx: Sender<Command>,
    running: bool,
    stop_after: bool,
    active: Option<Active>,
    registry: registry::Registry,
    server: Option<Server>,
    server_rx: Option<Receiver<ServerEvent>>,
    allowed: Arc<Mutex<Vec<Ipv4Addr>>>,
    client: Option<Client>,
    dirty_since: Option<Instant>,
    dereg: Vec<(Instant, String)>,
    pending_batches: HashMap<u64, Vec<String>>,
    batch_of: HashMap<String, u64>,
    next_batch: u64,
    awake: bool,
    banner: Option<Banner>,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn mtime(p: &Path) -> u64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

impl Engine {
    fn new(
        o: EngineOptions,
        events: Sender<Event>,
        notify: Notify,
        self_tx: Sender<Command>,
    ) -> Self {
        let mut e = Engine {
            o,
            items: Vec::new(),
            events,
            notify,
            self_tx,
            running: false,
            stop_after: false,
            active: None,
            registry: registry::Registry::default(),
            server: None,
            server_rx: None,
            allowed: Arc::new(Mutex::new(Vec::new())),
            client: None,
            dirty_since: None,
            dereg: Vec::new(),
            pending_batches: HashMap::new(),
            batch_of: HashMap::new(),
            next_batch: 0,
            awake: false,
            banner: None,
        };
        e.load();
        e
    }

    // ---------- plumbing ----------

    fn emit(&self, ev: Event) {
        let _ = self.events.send(ev);
        (self.notify)();
    }

    fn upsert(&self, i: usize) {
        self.emit(Event::ItemUpsert(Box::new(self.items[i].clone())));
    }

    fn set_banner(&mut self, b: Option<Banner>) {
        if self.banner != b {
            self.banner = b.clone();
            self.emit(Event::Banner(b));
        }
    }

    fn idx(&self, id: &str) -> Option<usize> {
        self.items.iter().position(|i| i.id == id)
    }

    fn dirty(&mut self) {
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    fn save_now(&mut self) {
        self.dirty_since = None;
        let Some(dir) = &self.o.data_dir else { return };
        let q = queue::SavedQueue {
            schema: 1,
            items: self
                .items
                .iter()
                .filter(|i| i.state != State::Parsing)
                .map(queue::Saved::from)
                .collect(),
        };
        if let Err(e) = storage::write_json(&dir.join(QUEUE_FILE), &q) {
            self.o
                .log
                .error("engine", "cannot save queue.json")
                .field("err", e);
        }
    }

    fn load(&mut self) {
        let Some(dir) = self.o.data_dir.clone() else {
            return;
        };
        match storage::read_json::<queue::SavedQueue>(&dir.join(QUEUE_FILE)) {
            Ok(storage::Loaded::Ok(q)) => {
                for s in q.items {
                    let mut it: Item = s.into();
                    let changed = it.parts.iter().any(|p| {
                        std::fs::metadata(&p.path).map_or(true, |m| m.len() != p.size)
                            || mtime(&p.path) != p.mtime
                    });
                    if changed && !matches!(it.state, State::Done | State::Invalid) {
                        it.state = State::Failed;
                        it.error = Some(ErrorReport::new(
                            Code::Pkg07,
                            it.parts
                                .first()
                                .map(|p| p.path.display().to_string())
                                .unwrap_or_default(),
                        ));
                    }
                    self.items.push(it);
                }
                // icons are not stored; re-read them in the background
                for it in &self.items {
                    if let Some(p) = it.parts.first() {
                        let (tx, id, path) = (self.self_tx.clone(), it.id.clone(), p.path.clone());
                        let parts = it.parts.clone();
                        thread::spawn(move || {
                            if let Ok(info) = pkg::read_info(&path) {
                                let _ = tx.send(Command::Parsed {
                                    id,
                                    result: Box::new(Ok((info, parts, Some("reload".into())))),
                                });
                            }
                        });
                    }
                }
            }
            Ok(storage::Loaded::Missing) => {}
            Ok(storage::Loaded::Broken(p)) => {
                self.o
                    .log
                    .warn(
                        "engine",
                        "queue.json was damaged; starting with an empty queue",
                    )
                    .code(Code::Cfg02)
                    .field("saved_as", p.display());
            }
            Err(e) => {
                self.o
                    .log
                    .error("engine", "cannot read queue.json")
                    .field("err", e);
            }
        }
    }

    fn set_state(&mut self, i: usize, ev: Ev) -> bool {
        match queue::next(self.items[i].state, ev) {
            Some(s) => {
                self.items[i].state = s;
                if s.is_finished() {
                    self.items[i].finished_at = Some(unix_now());
                }
                self.dirty();
                self.upsert(i);
                true
            }
            None => {
                self.o
                    .log
                    .debug("engine", "ignored transition")
                    .field("state", format!("{:?}", self.items[i].state))
                    .field("ev", format!("{ev:?}"));
                false
            }
        }
    }

    fn fail(&mut self, i: usize, mut report: ErrorReport) {
        let it = &self.items[i];
        if report.code.wants_console_log()
            && let Some(c) = &self.client
        {
            match c.console_log() {
                Ok(text) => {
                    report.console_log = self
                        .o
                        .log
                        .write_console_log(it.task_id.unwrap_or(0), &text)
                        .map(|p| p.display().to_string());
                    if report.technical.is_empty() {
                        let codes = find_console_codes(&text);
                        if !codes.is_empty() {
                            report.technical = format!("console codes: {}", codes.join(", "));
                        }
                    }
                }
                Err(e) => report.technical += &format!(" (console log unavailable: {e})"),
            }
        }
        self.o
            .log
            .error("engine", report.detail.clone())
            .title(it.title_id.clone())
            .code(report.code)
            .field("task_id", it.task_id.map_or("-".into(), |t| t.to_string()));
        self.items[i].error = Some(report);
        self.items[i].live = queue::Live::default();
        self.set_state(i, Ev::Error);
        let id = self.items[i].id.clone();
        self.registry.unregister(&id);
        if self.active.as_ref().is_some_and(|a| a.id == id) {
            self.active = None;
        }
    }

    // ---------- connection setup ----------

    fn console(&self) -> Option<Ipv4Addr> {
        self.o.config.console_ip()
    }

    fn global_stop(&mut self, report: ErrorReport) {
        self.o
            .log
            .error("engine", "queue stopped")
            .code(report.code)
            .field("detail", &report.detail);
        self.stop_queue();
        self.set_banner(Some(Banner::Global(report)));
    }

    fn stop_queue(&mut self) {
        self.running = false;
        self.stop_after = false;
        for i in 0..self.items.len() {
            if self.items[i].state == State::Queued {
                self.set_state(i, Ev::StopQueue);
            }
        }
        self.emit(Event::QueueRunning(false));
    }

    /// Client + server up; returns false (after a banner) when that is impossible.
    fn ensure_connection(&mut self) -> bool {
        let Some(console) = self.console() else {
            self.set_banner(Some(Banner::ConsoleNotConfigured));
            self.stop_queue();
            return false;
        };
        if self.client.is_none() {
            self.client = Some(Client::new(
                console,
                self.o.config.console_port,
                self.o.dialect.clone(),
            ));
        }
        if self.server.is_none() {
            let pc = match self
                .o
                .config
                .pc_ip_override()
                .map_or_else(|| net::pc_ip_for(console, self.o.config.console_port), Ok)
            {
                Ok(ip) => ip,
                Err(e) => {
                    self.global_stop(ErrorReport::new(Code::Net06, e.to_string()));
                    return false;
                }
            };
            *self.allowed.lock().unwrap_or_else(|e| e.into_inner()) = vec![console];
            let (tx, rx) = mpsc::channel();
            match Server::start(
                SocketAddrV4::new(pc, self.o.config.server_port),
                self.registry.clone(),
                self.allowed.clone(),
                true,
                tx,
                self.o.log.clone(),
            ) {
                Ok(s) => {
                    self.o.log.info("server", "listening").field("addr", s.addr);
                    self.emit(Event::ServerStatus(Some(s.addr)));
                    self.server = Some(s);
                    self.server_rx = Some(rx);
                }
                Err(e) => {
                    let code = if e.kind() == std::io::ErrorKind::AddrInUse {
                        Code::Srv01
                    } else {
                        Code::Srv02
                    };
                    let suggest = server::suggest_port(pc, self.o.config.server_port)
                        .map_or(String::new(), |p| format!(" (free: {p})"));
                    self.global_stop(
                        ErrorReport::new(
                            code,
                            format!("{}:{}{suggest}", pc, self.o.config.server_port),
                        )
                        .with_technical(e.to_string()),
                    );
                    return false;
                }
            }
        }
        self.set_banner(None);
        true
    }

    fn drop_connection(&mut self) {
        self.client = None;
        self.server_rx = None;
        if self.server.take().is_some() {
            self.emit(Event::ServerStatus(None));
        }
    }

    // ---------- main loop ----------

    fn run(mut self, rx: Receiver<Command>) {
        for i in 0..self.items.len() {
            self.upsert(i);
        }
        self.emit(Event::Order(
            self.items.iter().map(|i| i.id.clone()).collect(),
        ));
        let wait = self.o.timings.tick.min(self.o.timings.save_debounce);
        loop {
            match rx.recv_timeout(wait) {
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(cmd) => self.command(cmd),
                Err(RecvTimeoutError::Timeout) => {}
            }
            // drain anything else queued before ticking
            while let Ok(cmd) = rx.try_recv() {
                if matches!(cmd, Command::Shutdown) {
                    return self.shutdown();
                }
                self.command(cmd);
            }
            self.tick();
        }
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.save_now();
        self.drop_connection();
        if self.awake {
            winapi::keep_awake(false);
        }
    }

    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::AddPaths(paths) => self.add_paths(paths),
            Command::Parsed { id, result } => self.parsed(id, *result),
            Command::Remove(id) => {
                if let Some(i) = self.idx(&id) {
                    if self.items[i].state.is_active() {
                        self.cancel(i);
                    }
                    self.registry.unregister(&id);
                    self.items.remove(i);
                    self.dirty();
                    self.emit(Event::ItemRemoved(id));
                }
            }
            Command::Move { id, to } => {
                if let Some(i) = self.idx(&id)
                    && !self.items[i].state.is_active()
                {
                    let it = self.items.remove(i);
                    self.items.insert(to.min(self.items.len()), it);
                    self.dirty();
                    self.emit(Event::Order(
                        self.items.iter().map(|i| i.id.clone()).collect(),
                    ));
                }
            }
            Command::StartQueue => {
                if self.ensure_connection() {
                    for i in 0..self.items.len() {
                        if self.items[i].state == State::Ready {
                            self.set_state(i, Ev::StartQueue);
                        }
                    }
                    self.running = true;
                    self.stop_after = false;
                    self.emit(Event::QueueRunning(true));
                }
            }
            Command::StopAfterCurrent => {
                self.stop_after = true;
                if self.active.is_none() {
                    self.stop_queue();
                }
            }
            Command::Pause(id) => self.task_op(&id, "pause_task", Ev::Pause),
            Command::Resume(id) => self.task_op(&id, "resume_task", Ev::Resume),
            Command::Cancel(id) => {
                if let Some(i) = self.idx(&id) {
                    self.cancel(i);
                }
            }
            Command::Decide {
                id,
                decision,
                apply_all,
            } => self.decide(&id, decision, apply_all),
            Command::Retry(id) => {
                if let Some(i) = self.idx(&id) {
                    self.items[i].decision = None;
                    self.items[i].error = None;
                    if self.set_state(i, Ev::Retry) && self.running {
                        self.set_state(i, Ev::StartQueue);
                    }
                }
            }
            Command::ApplySettings(cfg) => {
                let conn_changed = (
                    cfg.console_ip(),
                    cfg.console_port,
                    cfg.server_port,
                    cfg.pc_ip_override.clone(),
                ) != (
                    self.o.config.console_ip(),
                    self.o.config.console_port,
                    self.o.config.server_port,
                    self.o.config.pc_ip_override.clone(),
                );
                self.o.config = *cfg;
                if conn_changed && self.active.is_none() {
                    self.drop_connection();
                }
            }
            Command::ClearFinished => {
                let (keep, gone): (Vec<Item>, Vec<Item>) =
                    std::mem::take(&mut self.items).into_iter().partition(|i| {
                        !matches!(i.state, State::Done | State::Skipped | State::Cancelled)
                    });
                self.items = keep;
                for g in gone {
                    self.emit(Event::ItemRemoved(g.id));
                }
                self.dirty();
            }
            Command::Shutdown => {}
        }
    }

    // ---------- adding ----------

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        let ex = scan::expand(&paths);
        if ex.skipped > 0 {
            let msg = format!("skipped {} files that are not .pkg", ex.skipped);
            self.o.log.info("engine", msg.clone());
            self.emit(Event::Info(msg));
        }
        let batch = self.next_batch;
        self.next_batch += 1;
        let mut jobs = Vec::new();
        for g in scan::group(ex.pkgs) {
            let id = queue::new_id();
            let first = g.first().to_path_buf();
            let canon = std::fs::canonicalize(&first).unwrap_or(first.clone());
            if self.items.iter().any(|i| {
                i.parts.first().is_some_and(|p| {
                    std::fs::canonicalize(&p.path).unwrap_or(p.path.clone()) == canon
                })
            }) {
                self.o
                    .log
                    .info("engine", "duplicate path ignored")
                    .field("path", first.display());
                continue;
            }
            let it = Item {
                id: id.clone(),
                parts: vec![PartRef {
                    path: first.clone(),
                    size: 0,
                    mtime: 0,
                }],
                content_id: String::new(),
                title_id: String::new(),
                title: first.file_name().map(|n| n.to_string_lossy().into_owned()),
                version: None,
                category: None,
                state: State::Parsing,
                decision: None,
                task_id: None,
                error: None,
                finished_at: None,
                icon: None,
                live: queue::Live::default(),
                note: None,
            };
            self.items.push(it);
            self.upsert(self.items.len() - 1);
            self.batch_of.insert(id.clone(), batch);
            self.pending_batches
                .entry(batch)
                .or_default()
                .push(id.clone());
            jobs.push((id, g));
        }
        let tx = self.self_tx.clone();
        // parse pool: at most 4 threads (DESIGN 6.1/3)
        let jobs = Arc::new(Mutex::new(jobs));
        for _ in 0..4 {
            let (jobs, tx) = (jobs.clone(), tx.clone());
            thread::spawn(move || {
                loop {
                    let job = jobs.lock().unwrap_or_else(|e| e.into_inner()).pop();
                    let Some((id, g)) = job else { break };
                    let _ = tx.send(Command::Parsed {
                        id,
                        result: Box::new(parse_group(g)),
                    });
                }
            });
        }
    }

    fn parsed(&mut self, id: String, result: commands::ParseResult) {
        let Some(i) = self.idx(&id) else { return };
        match result {
            Ok((info, _, note)) if note.as_deref() == Some("reload") => {
                self.items[i].icon = info.icon.map(Arc::new);
                self.upsert(i);
                return;
            }
            Ok((info, parts, note)) => {
                let size: u64 = parts.iter().map(|p| p.size).sum();
                let dup = self.items.iter().position(|o| {
                    o.id != id
                        && o.state != State::Parsing
                        && o.content_id == info.content_id
                        && o.version.as_deref() == info.display_version()
                        && o.total_size() == size
                });
                if let Some(pos) = dup {
                    let msg = format!("{} ({})", info.title.clone().unwrap_or_default(), pos + 1);
                    self.o
                        .log
                        .info("engine", "duplicate package ignored")
                        .code(Code::Pkg05)
                        .field("pkg", &msg);
                    self.emit(Event::Info(format!("{}: {msg}", Code::Pkg05)));
                    self.items.remove(i);
                    self.emit(Event::ItemRemoved(id.clone()));
                    self.finish_batch(&id);
                    return;
                }
                if self
                    .items
                    .iter()
                    .any(|o| o.id != id && o.content_id == info.content_id)
                {
                    self.o
                        .log
                        .warn(
                            "engine",
                            "same Content ID with a different version is already queued",
                        )
                        .title(info.title_id.clone());
                }
                if info.sfo_missing {
                    self.o
                        .log
                        .warn(
                            "engine",
                            "param.sfo missing; Title ID taken from Content ID",
                        )
                        .code(Code::Pkg03)
                        .title(info.title_id.clone());
                }
                if let Kind::Other(c) = info.kind()
                    && c != "?"
                {
                    self.o
                        .log
                        .warn("engine", "unknown package category")
                        .code(Code::Pkg06)
                        .field("category", &c);
                }
                let mut it = Item::from_info(id.clone(), parts, &info);
                it.note = note;
                self.items[i] = it;
            }
            Err(report) => {
                self.items[i].state = State::Invalid;
                self.o
                    .log
                    .warn("engine", report.detail.clone())
                    .code(report.code);
                self.items[i].error = Some(report);
            }
        }
        self.dirty();
        self.upsert(i);
        if self.running && self.items[i].state == State::Ready {
            self.set_state(i, Ev::StartQueue);
        }
        self.finish_batch(&id);
    }

    /// When a whole add action is parsed, order it: games, updates by version, DLC (DESIGN 6.1/5).
    fn finish_batch(&mut self, id: &str) {
        let Some(b) = self.batch_of.remove(id) else {
            return;
        };
        let done = self
            .pending_batches
            .get(&b)
            .is_some_and(|ids| ids.iter().all(|x| !self.batch_of.contains_key(x)));
        if !done {
            return;
        }
        let ids = self.pending_batches.remove(&b).unwrap_or_default();
        let rank = |it: &Item| match it.kind() {
            Kind::Game => 0,
            Kind::Update => 1,
            Kind::Dlc => 2,
            Kind::Other(_) => 3,
        };
        let mut batch: Vec<Item> = Vec::new();
        self.items.retain(|it| {
            if ids.contains(&it.id) && !it.state.is_active() {
                batch.push(it.clone());
                false
            } else {
                true
            }
        });
        batch.sort_by(|a, b| {
            rank(a)
                .cmp(&rank(b))
                .then_with(|| {
                    if rank(a) == 1 {
                        a.version.cmp(&b.version)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then_with(|| a.title.cmp(&b.title))
        });
        self.items.extend(batch);
        self.emit(Event::Order(
            self.items.iter().map(|i| i.id.clone()).collect(),
        ));
    }

    // ---------- user decisions / task ops ----------

    fn decide(&mut self, id: &str, decision: Decision, apply_all: bool) {
        if let Some(i) = self.idx(id) {
            self.items[i].decision = Some(decision);
            if self.items[i].state == State::AwaitingDecision {
                self.set_state(i, Ev::Decide(decision));
                if decision == Decision::Skip {
                    self.active = None;
                } else if self.items[i].kind() != Kind::Game {
                    // update/DLC "install again" goes straight to install, no uninstall (DESIGN 6.2/3)
                    self.items[i].state = State::Starting;
                    self.upsert(i);
                }
            }
            self.dirty();
        }
        if apply_all {
            // only items queued now, never ones added later (DESIGN 6.3)
            for it in self
                .items
                .iter_mut()
                .filter(|it| it.state == State::Queued && it.decision.is_none())
            {
                it.decision = Some(decision);
            }
        }
    }

    fn task_op(&mut self, id: &str, op: &str, ev: Ev) {
        let Some(i) = self.idx(id) else { return };
        let (Some(task), Some(c)) = (self.items[i].task_id, &self.client) else {
            return;
        };
        let (r, ex) = c.task(op, task);
        match r {
            Ok(_) => {
                self.set_state(i, ev);
                if let Some(a) = self.active.as_mut() {
                    a.best_at = Instant::now(); // pause time is not a stall
                }
            }
            Err(e) => {
                self.o
                    .log
                    .warn("engine", format!("{op} failed: {e}"))
                    .title(self.items[i].title_id.clone())
                    .body(&ex.body);
            }
        }
    }

    fn cancel(&mut self, i: usize) {
        let st = self.items[i].state;
        if let (Some(task), Some(c)) = (self.items[i].task_id, &self.client) {
            for op in ["stop_task", "unregister_task"] {
                if let Err(e) = c.task(op, task).0 {
                    self.o
                        .log
                        .warn(
                            "engine",
                            format!("{op} failed; the console may still list the task: {e}"),
                        )
                        .title(self.items[i].title_id.clone());
                }
            }
        }
        if !self.set_state(i, Ev::Cancel) && st.is_active() {
            // Checking/AwaitingDecision/Uninstalling: back to Ready
            self.items[i].state = State::Ready;
            self.upsert(i);
        }
        self.items[i].task_id = None;
        self.items[i].live = queue::Live::default();
        let id = self.items[i].id.clone();
        self.registry.unregister(&id);
        if self.active.as_ref().is_some_and(|a| a.id == id) {
            self.active = None;
        }
        self.dirty();
    }

    // ---------- the queue driver (DESIGN 6.2) ----------

    fn tick(&mut self) {
        let now = Instant::now();
        let reg = self.registry.clone();
        self.dereg.retain(|(at, id)| {
            if *at <= now {
                reg.unregister(id);
                false
            } else {
                true
            }
        });
        self.server_events();
        if self.running && self.active.is_none() {
            if let Some(i) = self.items.iter().position(|it| it.state == State::Queued) {
                self.active = Some(Active::new(self.items[i].id.clone()));
                self.set_state(i, Ev::TakeTurn);
            } else if self.stop_after || !self.items.iter().any(|it| it.state == State::Parsing) {
                self.stop_queue();
            }
        }
        if let Some(id) = self.active.as_ref().map(|a| a.id.clone()) {
            match self.idx(&id) {
                Some(i) => self.step(i),
                None => self.active = None,
            }
            if self.active.is_none() && self.stop_after {
                self.stop_queue();
            }
        }
        let want_awake = self
            .items
            .iter()
            .any(|i| matches!(i.state, State::Transferring | State::Installing));
        if want_awake != self.awake {
            winapi::keep_awake(want_awake);
            self.awake = want_awake;
        }
        if self
            .dirty_since
            .is_some_and(|t| t.elapsed() >= self.o.timings.save_debounce)
        {
            self.save_now();
        }
    }

    fn server_events(&mut self) {
        let Some(rx) = &self.server_rx else { return };
        let evs: Vec<ServerEvent> = rx.try_iter().collect();
        for ev in evs {
            match ev {
                ServerEvent::Request { item_id, .. } => {
                    if let Some(a) = self.active.as_mut().filter(|a| a.id == item_id) {
                        a.saw_request = true;
                    }
                }
                ServerEvent::FileChanged { item_id } => {
                    if let Some(i) = self.idx(&item_id) {
                        let path = self.items[i]
                            .parts
                            .first()
                            .map(|p| p.path.display().to_string())
                            .unwrap_or_default();
                        self.cancel_task_quietly(i);
                        self.fail(i, ErrorReport::new(Code::Srv03, path));
                    }
                }
                ServerEvent::SlowDisk { item_id, millis } => {
                    let tid = self
                        .idx(&item_id)
                        .map(|i| self.items[i].title_id.clone())
                        .unwrap_or_default();
                    self.o
                        .log
                        .warn("server", "slow disk read")
                        .code(Code::Srv04)
                        .title(tid)
                        .field("ms", millis);
                }
                ServerEvent::Sent { .. } => {}
            }
        }
    }

    fn cancel_task_quietly(&mut self, i: usize) {
        if let (Some(task), Some(c)) = (self.items[i].task_id, &self.client) {
            let _ = c.task("stop_task", task);
            let _ = c.task("unregister_task", task);
        }
    }

    fn step(&mut self, i: usize) {
        if !self.ensure_connection() {
            return;
        }
        match self.items[i].state {
            State::Checking => self.step_check(i),
            State::Uninstalling => self.step_uninstall(i),
            State::Starting => self.step_start(i),
            State::Transferring => self.step_transfer(i),
            State::Installing => self.step_confirm(i),
            State::AwaitingDecision | State::Paused => {}
            _ => self.active = None,
        }
    }

    fn files_unchanged(&self, i: usize) -> bool {
        self.items[i].parts.iter().all(|p| {
            std::fs::metadata(&p.path).is_ok_and(|m| m.len() == p.size) && mtime(&p.path) == p.mtime
        })
    }

    fn step_check(&mut self, i: usize) {
        if !self.files_unchanged(i) {
            let p = self.items[i]
                .parts
                .first()
                .map(|p| p.path.display().to_string())
                .unwrap_or_default();
            return self.fail(i, ErrorReport::new(Code::Pkg07, p));
        }
        let c = self.client.as_ref().expect("ensure_connection");
        // resume a task saved before a restart (DESIGN 9.3)
        if let Some(task) = self.items[i].task_id {
            let (r, _) = c.progress(task);
            match r {
                Ok(p) if p.error.is_none() && !p.complete(self.items[i].total_size()) => {
                    if c.task("resume_task", task).0.is_ok() {
                        self.register(i);
                        if let Some(a) = self.active.as_mut() {
                            a.task_started = Some(Instant::now());
                            a.saw_request = true;
                        }
                        self.o
                            .log
                            .info("engine", "resumed saved task")
                            .title(self.items[i].title_id.clone())
                            .field("task_id", task);
                        self.set_state(i, Ev::Resumed);
                        return;
                    }
                }
                _ => {
                    let _ = c.task("unregister_task", task);
                }
            }
            self.items[i].task_id = None;
        }
        let kind = self.items[i].kind();
        let title = self.items[i].title_id.clone();
        let (r, ex) = c.is_exists(&title);
        let exists = match r {
            Ok(e) => e,
            Err(e) => {
                let unreachable = matches!(e, RpiError::Unreachable(_));
                self.fail(
                    i,
                    ErrorReport::new(Code::Rpi02, e.to_string()).with_technical(ex.body),
                );
                if unreachable {
                    self.global_stop(ErrorReport::new(
                        Code::Net01,
                        self.o.config.console_ip.clone().unwrap_or_default(),
                    ));
                }
                return;
            }
        };
        match kind {
            Kind::Update | Kind::Dlc => {
                if !exists.exists {
                    return self.fail(i, ErrorReport::new(Code::Rpi06, title));
                }
                // "already installed" for updates only if RPI reports a version ([PROVJERA] 14.1/2)
                let ours = self.items[i].version.clone();
                let already =
                    kind == Kind::Update && exists.version.is_some() && exists.version >= ours;
                let decision = self.items[i].decision;
                match (already, decision) {
                    (false, _) | (true, Some(Decision::Reinstall)) => {
                        self.set_state(
                            i,
                            Ev::Checked {
                                exists: false,
                                decision: None,
                            },
                        );
                    }
                    (true, Some(Decision::Skip)) => {
                        self.set_state(
                            i,
                            Ev::Checked {
                                exists: true,
                                decision: Some(Decision::Skip),
                            },
                        );
                        self.active = None;
                    }
                    (true, None) => {
                        self.set_state(
                            i,
                            Ev::Checked {
                                exists: true,
                                decision: None,
                            },
                        );
                        self.emit(Event::AskDecision(self.items[i].id.clone()));
                    }
                }
            }
            _ => {
                let decision = self.items[i].decision;
                self.set_state(
                    i,
                    Ev::Checked {
                        exists: exists.exists,
                        decision,
                    },
                );
                match self.items[i].state {
                    State::AwaitingDecision => {
                        self.emit(Event::AskDecision(self.items[i].id.clone()))
                    }
                    State::Skipped => self.active = None,
                    _ => {}
                }
            }
        }
    }

    fn step_uninstall(&mut self, i: usize) {
        let c = self.client.as_ref().expect("ensure_connection");
        let title = self.items[i].title_id.clone();
        let (r, ex) = c.uninstall(UninstallKind::Game, &title);
        match r {
            Ok(_) | Err(RpiError::Fail { status: 404, .. }) => {}
            Err(e) => {
                return self.fail(
                    i,
                    ErrorReport::new(Code::Rpi08, e.to_string()).with_technical(ex.body),
                );
            }
        }
        for n in 0..self.o.timings.verify_tries {
            if n > 0 {
                thread::sleep(self.o.timings.verify_gap);
            }
            if let (Ok(e), _) = c.is_exists(&title)
                && !e.exists
            {
                self.o
                    .log
                    .info("engine", "uninstalled before reinstall")
                    .title(title);
                self.set_state(i, Ev::UninstallOk);
                return;
            }
        }
        self.fail(
            i,
            ErrorReport::new(Code::Rpi08, "title still installed after uninstall"),
        );
    }

    fn register(&mut self, i: usize) -> Vec<String> {
        let parts: Vec<registry::Part> = self.items[i]
            .parts
            .iter()
            .map(|p| registry::Part {
                path: p.path.clone(),
                size: p.size,
                mtime: std::fs::metadata(&p.path).and_then(|m| m.modified()).ok(),
            })
            .collect();
        let addr = self.server.as_ref().expect("ensure_connection").addr;
        self.registry
            .register(&self.items[i].id, &self.items[i].content_id, &parts)
            .iter()
            .map(|p| server::url(addr, p))
            .collect()
    }

    fn step_start(&mut self, i: usize) {
        let urls = self.register(i);
        let c = self.client.as_ref().expect("ensure_connection");
        let (r, ex) = c.install(&urls);
        match r {
            Ok(task) => {
                self.items[i].task_id = Some(task);
                if let Some(a) = self.active.as_mut() {
                    a.task_started = Some(Instant::now());
                    a.best_at = Instant::now();
                }
                self.o
                    .log
                    .info("engine", "install started")
                    .title(self.items[i].title_id.clone())
                    .field("task_id", task);
                self.set_state(i, Ev::InstallStarted);
                self.save_now();
            }
            Err(e) => {
                let detail = match &e {
                    RpiError::Fail { error, .. } => error.clone(),
                    other => other.to_string(),
                };
                self.fail(
                    i,
                    ErrorReport::new(Code::Rpi01, detail).with_technical(ex.body),
                );
            }
        }
    }

    fn step_transfer(&mut self, i: usize) {
        let t = &self.o.timings;
        let (fw01, stall, limit) = (t.fw01, t.stall, t.unreachable_limit);
        let task = self.items[i].task_id.expect("Transferring has a task");
        let total_parts = self.items[i].total_size();
        let c = self.client.as_ref().expect("ensure_connection");
        let (r, ex) = c.progress(task);
        let a = self.active.as_mut().expect("active");
        let p = match r {
            Ok(p) => {
                a.unreachable = 0;
                p
            }
            Err(RpiError::Unreachable(u)) => {
                a.unreachable += 1;
                if a.unreachable >= limit {
                    self.fail(i, ErrorReport::new(Code::Rpi07, format!("{u:?}")));
                    self.global_stop(ErrorReport::new(
                        Code::Net01,
                        self.o.config.console_ip.clone().unwrap_or_default(),
                    ));
                }
                return;
            }
            Err(e) => {
                self.cancel_task_quietly(i);
                return self.fail(
                    i,
                    ErrorReport::new(Code::Rpi04, e.to_string()).with_technical(ex.body),
                );
            }
        };
        if let Some(code) = &p.error {
            let report = ErrorReport::new(Code::Rpi04, code.clone()).with_technical(ex.body);
            return self.fail(i, report);
        }
        let transferred = p.transferred.unwrap_or(0);
        let total = p.total.unwrap_or(total_parts);
        let now = Instant::now();
        if transferred > a.best {
            a.best = transferred;
            a.best_at = now;
        }
        let (speed, eta) = a.smoother.sample(
            now.duration_since(a.clock).as_secs_f64(),
            transferred,
            total,
            p.rest_sec,
        );
        let started = a.task_started.unwrap_or(now);
        let (saw, best_at) = (a.saw_request, a.best_at);
        self.items[i].live = queue::Live {
            fraction: p.fraction(total_parts),
            transferred,
            total,
            speed,
            eta,
        };
        if p.complete(total_parts) {
            self.set_state(i, Ev::TransferDone);
            return;
        }
        self.upsert(i);
        if !saw && now.duration_since(started) >= fw01 {
            self.cancel_task_quietly(i);
            return self.fail(
                i,
                ErrorReport::new(Code::Fw01, format!("no request in {} s", fw01.as_secs())),
            );
        }
        if now.duration_since(best_at) >= stall {
            self.cancel_task_quietly(i);
            self.fail(
                i,
                ErrorReport::new(Code::Rpi05, stall.as_secs().to_string()),
            );
        }
    }

    /// Installing → Done. Games are confirmed with is_exists; updates/DLC are done when RPI says so.
    fn step_confirm(&mut self, i: usize) {
        let (gap, max) = (self.o.timings.confirm_gap, self.o.timings.confirm_max);
        let done = if self.items[i].kind() == Kind::Game {
            let a = self.active.as_mut().expect("active");
            let now = Instant::now();
            let since = *a.confirm_since.get_or_insert(now);
            if now < a.next_confirm {
                return;
            }
            a.next_confirm = now + gap;
            let c = self.client.as_ref().expect("ensure_connection");
            match c.is_exists(&self.items[i].title_id).0 {
                Ok(e) if e.exists => true,
                _ if now.duration_since(since) >= max => {
                    return self.fail(
                        i,
                        ErrorReport::new(Code::Rpi04, "installation not confirmed"),
                    );
                }
                _ => false,
            }
        } else {
            true
        };
        if done {
            if let Some(f) = self.items[i].live.fraction.as_mut() {
                *f = 1.0;
            }
            self.o
                .log
                .info("engine", "installed")
                .title(self.items[i].title_id.clone());
            self.set_state(i, Ev::Confirmed);
            let id = self.items[i].id.clone();
            self.dereg
                .push((Instant::now() + self.o.timings.dereg_delay, id));
            self.active = None;
        }
    }
}

/// Parse one scan group into item data (runs on a pool thread).
fn parse_group(g: scan::Group) -> commands::ParseResult {
    let part_ref = |p: &Path| -> Result<PartRef, ErrorReport> {
        let m = std::fs::metadata(p).map_err(|e| {
            ErrorReport::new(Code::Pkg02, p.display().to_string()).with_technical(e.to_string())
        })?;
        Ok(PartRef {
            path: p.to_path_buf(),
            size: m.len(),
            mtime: mtime(p),
        })
    };
    let (paths, note) = match g {
        scan::Group::Single(p) => (vec![p], None),
        scan::Group::Split(v) => {
            let n = v.len();
            (v, Some(format!("split: {n}")))
        }
        scan::Group::SplitMissing { present, missing } => {
            let list = missing
                .iter()
                .map(|m| format!("_{m}"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ErrorReport::new(Code::Pkg04, list)
                .with_technical(present[0].display().to_string()));
        }
    };
    let parts = paths
        .iter()
        .map(|p| part_ref(p))
        .collect::<Result<Vec<_>, _>>()?;
    let first = &paths[0];
    let info = pkg::read_info(first).map_err(|e| {
        let (code, t) = match e {
            PkgError::NotPkg => (Code::Pkg01, String::new()),
            PkgError::Damaged(w) => (Code::Pkg02, w.to_string()),
            PkgError::NoSfo => (Code::Pkg03, String::new()),
            PkgError::Io(e) => (Code::Pkg02, e.to_string()),
        };
        ErrorReport::new(code, first.display().to_string()).with_technical(t)
    })?;
    Ok((info, parts, note))
}
