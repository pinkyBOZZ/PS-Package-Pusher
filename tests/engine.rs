//! Engine + server + mock in-process scenarios (DESIGN 8.3, PLAN 1.13).

use pspp::{
    config::Config,
    engine::{
        self, EngineOptions, EngineTimings,
        commands::{Command, Event, UninstallTarget},
        queue::{Decision, Item, State},
    },
    errors::Code,
    logging::Logger,
    mock::{Mock, MockConfig},
    pkg::fixture::Fixture,
    rpi::{RpiDialect, UninstallKind},
};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

struct T {
    mock: Mock,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Event>,
    items: HashMap<String, Item>,
    order: Vec<String>,
    asked: Vec<String>,
    installed: Vec<(String, Result<bool, String>)>,
    uninstalled: Vec<(String, Result<bool, Code>)>,
    dir: PathBuf,
    join: Option<std::thread::JoinHandle<()>>,
}

fn timings() -> EngineTimings {
    EngineTimings {
        tick: Duration::from_millis(20),
        fw01: Duration::from_millis(800),
        stall: Duration::from_millis(800),
        verify_tries: 3,
        verify_gap: Duration::from_millis(20),
        confirm_gap: Duration::from_millis(20),
        confirm_max: Duration::from_secs(5),
        dereg_delay: Duration::from_millis(100),
        save_debounce: Duration::from_millis(20),
        unreachable_limit: 3,
    }
}

fn start(name: &str, mcfg: MockConfig, persist: bool) -> T {
    let dir = std::env::temp_dir().join(format!("pspp-it-engine-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let mock = Mock::start(
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0),
        MockConfig {
            chunk: 256 * 1024,
            ..mcfg
        },
    )
    .unwrap();
    start_with(dir, mock, persist)
}

fn free_port() -> u16 {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn start_with(dir: PathBuf, mock: Mock, persist: bool) -> T {
    start_port(dir, mock, persist, 0)
}

/// `server_port` fixed when a test restarts the engine: saved console tasks keep their URLs (DESIGN 9.3).
fn start_port(dir: PathBuf, mock: Mock, persist: bool, server_port: u16) -> T {
    let config = Config {
        console_ip: Some("127.0.0.1".into()),
        console_port: mock.addr.port(),
        server_port,
        ..Default::default()
    };
    let (etx, erx) = mpsc::channel();
    let h = engine::spawn(
        EngineOptions {
            data_dir: persist.then(|| dir.join("data")),
            config,
            timings: timings(),
            dialect: RpiDialect::default(),
            log: Logger::memory(),
        },
        etx,
        Arc::new(|| {}),
    );
    T {
        mock,
        tx: h.tx,
        rx: erx,
        items: HashMap::new(),
        order: vec![],
        asked: vec![],
        installed: vec![],
        uninstalled: vec![],
        dir,
        join: Some(h.join),
    }
}

impl T {
    fn pump(&mut self) {
        while let Ok(e) = self.rx.recv_timeout(Duration::from_millis(5)) {
            match e {
                Event::ItemUpsert(i) => {
                    if !self.order.contains(&i.id) {
                        self.order.push(i.id.clone());
                    }
                    self.items.insert(i.id.clone(), *i);
                }
                Event::ItemRemoved(id) => {
                    self.items.remove(&id);
                    self.order.retain(|x| *x != id);
                }
                Event::Order(o) => self.order = o,
                Event::AskDecision(id) => self.asked.push(id),
                Event::InstalledResult { target, result } => {
                    self.installed.push((target.id, result))
                }
                Event::UninstallResult { target, result } => self
                    .uninstalled
                    .push((target.id, result.map_err(|e| e.code))),
                _ => {}
            }
        }
    }

    fn wait(&mut self, what: &str, f: impl Fn(&T) -> bool) {
        let t0 = Instant::now();
        loop {
            self.pump();
            if f(self) {
                return;
            }
            assert!(
                t0.elapsed() < Duration::from_secs(20),
                "timeout waiting for {what}: {:#?}",
                self.items
                    .values()
                    .map(|i| (&i.title, i.state, &i.error))
                    .collect::<Vec<_>>()
            );
        }
    }

    fn by_title(&self, t: &str) -> &Item {
        self.items
            .values()
            .find(|i| i.title.as_deref() == Some(t))
            .unwrap_or_else(|| panic!("no item {t}"))
    }

    fn state(&self, t: &str) -> Option<State> {
        self.items
            .values()
            .find(|i| i.title.as_deref() == Some(t))
            .map(|i| i.state)
    }

    fn add(&mut self, paths: Vec<PathBuf>, n: usize) {
        self.tx.send(Command::AddPaths(paths)).unwrap();
        self.wait("parsed", |t| {
            t.items.len() >= n && t.items.values().all(|i| i.state != State::Parsing)
        });
    }

    fn stop(mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(j) = self.join.take() {
            j.join().unwrap();
        }
        self.mock.stop();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn pkg(dir: &std::path::Path, name: &str, f: Fixture) -> PathBuf {
    let p = dir.join(name);
    let mut f = f;
    f.total_size = f.total_size.max(1024 * 1024);
    f.write(&p).unwrap();
    p
}

#[test]
fn success_split_order_and_persistence() {
    let mut t = start("ok", MockConfig::default(), true);
    let d = t.dir.clone();
    let dlc = pkg(&d, "c.pkg", Fixture::dlc("CUSA00001"));
    let upd = pkg(&d, "b.pkg", Fixture::update("CUSA00001", "01.05"));
    let mut split = Fixture::game("CUSA00001");
    split.total_size = 3 * 1024 * 1024;
    split.write_split(&d, "Game", 3).unwrap();
    t.add(
        vec![
            dlc,
            upd,
            d.join("Game_0.pkg"),
            d.join("Game_1.pkg"),
            d.join("Game_2.pkg"),
        ],
        3,
    );
    // added together: game, update, DLC
    let kinds: Vec<_> = t
        .order
        .iter()
        .map(|id| t.items[id].category.clone().unwrap())
        .collect();
    assert_eq!(kinds, ["gd", "gp", "ac"]);
    assert_eq!(t.by_title("Test Game CUSA00001").parts.len(), 3);
    t.tx.send(Command::StartQueue).unwrap();
    t.wait("all done", |t| {
        t.items.values().all(|i| i.state == State::Done)
    });
    assert_eq!(t.mock.installed(), ["CUSA00001"]);
    assert!(t.by_title("Test Game CUSA00001").live.fraction == Some(1.0));
    let saved = std::fs::read_to_string(d.join("data").join(engine::QUEUE_FILE)).unwrap();
    assert_eq!(saved.matches("\"state\": \"done\"").count(), 3, "{saved}");
    t.stop();
}

#[test]
fn already_installed_skip_and_reinstall() {
    let mut t = start(
        "dec",
        MockConfig {
            installed: vec!["CUSA00010".into(), "CUSA00011".into()],
            ..Default::default()
        },
        false,
    );
    let d = t.dir.clone();
    let mut a = Fixture::game("CUSA00010");
    a.title = "A".into();
    let mut b = Fixture::game("CUSA00011");
    b.title = "B".into();
    let pa = pkg(&d, "a.pkg", a);
    let pb = pkg(&d, "b.pkg", b);
    t.add(vec![pa, pb], 2);
    t.tx.send(Command::StartQueue).unwrap();
    t.wait("decision asked", |t| {
        t.state("A") == Some(State::AwaitingDecision)
    });
    let a_id = t.by_title("A").id.clone();
    assert_eq!(t.asked, std::slice::from_ref(&a_id));
    t.tx.send(Command::Decide {
        id: a_id,
        decision: Decision::Skip,
        apply_all: false,
    })
    .unwrap();
    t.wait("A skipped, B asks", |t| {
        t.state("A") == Some(State::Skipped) && t.state("B") == Some(State::AwaitingDecision)
    });
    let b_id = t.by_title("B").id.clone();
    t.tx.send(Command::Decide {
        id: b_id,
        decision: Decision::Reinstall,
        apply_all: false,
    })
    .unwrap();
    t.wait("B reinstalled", |t| t.state("B") == Some(State::Done));
    assert!(t.mock.tasks().len() == 1, "only B was installed");
    t.stop();
}

#[test]
fn update_without_base_fails_rpi06() {
    let mut t = start("nobase", MockConfig::default(), false);
    let p = pkg(
        &t.dir.clone(),
        "u.pkg",
        Fixture::update("CUSA00777", "01.02"),
    );
    t.add(vec![p], 1);
    t.tx.send(Command::StartQueue).unwrap();
    t.wait("failed", |t| {
        t.items.values().all(|i| i.state == State::Failed)
    });
    assert_eq!(
        t.items
            .values()
            .next()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .code,
        Code::Rpi06
    );
    t.stop();
}

#[test]
fn stall_fail_code_and_fw01() {
    for (name, mcfg, code) in [
        (
            "stall",
            MockConfig {
                stall_after: Some(256 * 1024),
                ..Default::default()
            },
            Code::Rpi05,
        ),
        (
            "failcode",
            MockConfig {
                fail_task_at: Some(0.3),
                error_code: 0x8099_0085,
                ..Default::default()
            },
            Code::Rpi04,
        ),
        (
            "nofetch",
            MockConfig {
                no_fetch: true,
                ..Default::default()
            },
            Code::Fw01,
        ),
    ] {
        let mut t = start(name, mcfg, false);
        let p = pkg(&t.dir.clone(), "g.pkg", Fixture::game("CUSA00020"));
        t.add(vec![p], 1);
        t.tx.send(Command::StartQueue).unwrap();
        t.wait(name, |t| t.items.values().all(|i| i.state == State::Failed));
        let err = t.items.values().next().unwrap().error.clone().unwrap();
        assert_eq!(err.code, code, "{name}: {err:?}");
        if code == Code::Rpi04 {
            assert_eq!(err.detail, "0x80990085");
        }
        assert!(
            t.mock.tasks().is_empty(),
            "{name}: stale task left on console"
        );
        assert!(
            t.items.values().all(|i| i.task_id.is_none()),
            "{name}: task_id kept"
        );
        t.stop();
    }
}

#[test]
fn pause_resume_cancel() {
    let mut t = start(
        "pause",
        MockConfig {
            speed_bps: Some(1024 * 1024),
            ..Default::default()
        },
        false,
    );
    let d = t.dir.clone();
    let mut f = Fixture::game("CUSA00030");
    f.total_size = 4 * 1024 * 1024;
    let p = pkg(&d, "g.pkg", f);
    t.add(vec![p], 1);
    let id = t.items.keys().next().unwrap().clone();
    t.tx.send(Command::StartQueue).unwrap();
    t.wait("transferring", |t| {
        t.items[&id].state == State::Transferring && t.items[&id].live.transferred > 0
    });
    t.tx.send(Command::Pause(id.clone())).unwrap();
    t.wait("paused", |t| t.items[&id].state == State::Paused);
    std::thread::sleep(Duration::from_millis(1000)); // longer than the stall limit: pause is not a stall
    t.pump();
    assert_eq!(t.items[&id].state, State::Paused);
    t.tx.send(Command::Resume(id.clone())).unwrap();
    t.wait("resumed", |t| t.items[&id].state == State::Transferring);
    t.tx.send(Command::Cancel(id.clone())).unwrap();
    t.wait("cancelled", |t| t.items[&id].state == State::Cancelled);
    assert!(
        t.mock.tasks().is_empty(),
        "task unregistered on the console"
    );
    t.stop();
}

#[test]
fn restart_resumes_interrupted_task() {
    let port = free_port();
    let dir = std::env::temp_dir().join(format!("pspp-it-engine-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let mock = Mock::start(
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0),
        MockConfig {
            speed_bps: Some(1024 * 1024),
            chunk: 256 * 1024,
            ..Default::default()
        },
    )
    .unwrap();
    let mut t = start_port(dir, mock, true, port);
    let d = t.dir.clone();
    let mut f = Fixture::game("CUSA00040");
    f.total_size = 3 * 1024 * 1024;
    let p = pkg(&d, "g.pkg", f);
    t.add(vec![p], 1);
    let id = t.items.keys().next().unwrap().clone();
    t.tx.send(Command::StartQueue).unwrap();
    t.wait("transferring", |t| {
        t.items[&id].state == State::Transferring && t.items[&id].live.transferred > 0
    });
    // close the app mid-transfer; the mock keeps its task (like the console would)
    t.tx.send(Command::Shutdown).unwrap();
    t.join.take().unwrap().join().unwrap();
    let mock = t.mock.clone();
    let mut t2 = start_port(d.clone(), mock, true, port);
    t2.wait("loaded as failed RPI-07", |t| {
        t.items.get(&id).is_some_and(|i| i.state == State::Failed)
    });
    assert_eq!(t2.items[&id].error.as_ref().unwrap().code, Code::Rpi07);
    assert!(t2.items[&id].task_id.is_some());
    t2.tx.send(Command::Retry(id.clone())).unwrap();
    t2.tx.send(Command::StartQueue).unwrap();
    t2.wait("done after resume", |t| t.items[&id].state == State::Done);
    assert_eq!(t2.mock.tasks().len(), 1, "no second install task");
    t2.stop();
}

#[test]
fn invalid_inputs() {
    let mut t = start("invalid", MockConfig::default(), false);
    let d = t.dir.clone();
    let parts = Fixture::game("CUSA00050")
        .write_split(&d, "Broken", 3)
        .unwrap();
    std::fs::remove_file(&parts[1]).unwrap();
    std::fs::write(d.join("zip.pkg"), b"PK\x03\x04").unwrap();
    std::fs::write(d.join("x.txt"), b"x").unwrap();
    let ok = pkg(&d, "ok.pkg", Fixture::game("CUSA00051"));
    t.add(vec![d.clone()], 3);
    let codes: Vec<_> = t
        .items
        .values()
        .filter_map(|i| i.error.as_ref().map(|e| e.code))
        .collect();
    assert!(
        codes.contains(&Code::Pkg04) && codes.contains(&Code::Pkg01),
        "{codes:?}"
    );
    // the same file again is ignored
    t.tx.send(Command::AddPaths(vec![ok])).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    t.pump();
    assert_eq!(t.items.len(), 3);
    t.stop();
}

#[test]
fn standalone_uninstall_item_and_by_id() {
    let mut t = start(
        "uninst",
        MockConfig {
            installed: vec![
                "CUSA00060".into(),
                "CUSA00061".into(),
                "UP0000-CUSA00061_00-DLCXXXXXXXXXXXXX".into(),
            ],
            ..Default::default()
        },
        false,
    );
    let p = pkg(&t.dir.clone(), "g.pkg", Fixture::game("CUSA00060"));
    t.add(vec![p], 1);
    let id = t.items.keys().next().unwrap().clone();
    let target = UninstallTarget {
        kind: UninstallKind::Game,
        id: "CUSA00060".into(),
        item: Some(id.clone()),
    };
    t.tx.send(Command::CheckInstalled(target.clone())).unwrap();
    t.wait("check answered", |t| !t.installed.is_empty());
    assert_eq!(t.installed[0], ("CUSA00060".into(), Ok(true)));
    t.tx.send(Command::Uninstall(target)).unwrap();
    t.wait("uninstalled", |t| !t.uninstalled.is_empty());
    assert_eq!(t.uninstalled[0], ("CUSA00060".into(), Ok(true)));
    assert_eq!(t.items[&id].state, State::Ready);
    assert!(!t.mock.installed().contains(&"CUSA00060".to_string()));
    // by ID: DLC by Content ID, then a title that is not installed (404 → Ok(false))
    t.tx.send(Command::Uninstall(UninstallTarget {
        kind: UninstallKind::Dlc,
        id: "UP0000-CUSA00061_00-DLCXXXXXXXXXXXXX".into(),
        item: None,
    }))
    .unwrap();
    t.tx.send(Command::Uninstall(UninstallTarget {
        kind: UninstallKind::Game,
        id: "CUSA09999".into(),
        item: None,
    }))
    .unwrap();
    t.wait("two more results", |t| t.uninstalled.len() == 3);
    assert!(
        t.uninstalled
            .contains(&("UP0000-CUSA00061_00-DLCXXXXXXXXXXXXX".into(), Ok(true)))
    );
    assert!(t.uninstalled.contains(&("CUSA09999".into(), Ok(false))));
    t.stop();
}

#[test]
fn decision_apply_to_all() {
    let mut t = start(
        "applyall",
        MockConfig {
            installed: vec!["CUSA00070".into(), "CUSA00071".into()],
            ..Default::default()
        },
        false,
    );
    let d = t.dir.clone();
    let mut a = Fixture::game("CUSA00070");
    a.title = "A".into();
    let mut b = Fixture::game("CUSA00071");
    b.title = "B".into();
    t.add(vec![pkg(&d, "a.pkg", a), pkg(&d, "b.pkg", b)], 2);
    t.tx.send(Command::StartQueue).unwrap();
    t.wait("first asks", |t| {
        t.state("A") == Some(State::AwaitingDecision)
    });
    let a_id = t.by_title("A").id.clone();
    t.tx.send(Command::Decide {
        id: a_id,
        decision: Decision::Skip,
        apply_all: true,
    })
    .unwrap();
    t.wait("both skipped without a second question", |t| {
        t.state("A") == Some(State::Skipped) && t.state("B") == Some(State::Skipped)
    });
    assert_eq!(t.asked.len(), 1);
    t.stop();
}
