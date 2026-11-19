//! CP-1 demo (PLAN 1.16): add packages → run the queue against the mock → print events.
//! Run: cargo run --example headless -- [ok|split|stall|nofetch|failcode]   (default: all)
//! Not part of the release.

use pspp::{
    config::Config,
    engine::{
        self, EngineOptions, EngineTimings,
        commands::{Command, Event},
        queue::State,
    },
    logging::Logger,
    mock::{Mock, MockConfig},
    pkg::fixture::Fixture,
    rpi::RpiDialect,
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

fn scenario(name: &str) {
    println!("\n=== scenario: {name} ===");
    let mcfg = match name {
        "stall" => MockConfig { stall_after: Some(1 << 20), ..Default::default() },
        "nofetch" => MockConfig { no_fetch: true, ..Default::default() },
        "failcode" => MockConfig { fail_task_at: Some(0.4), error_code: 0x8099_0085, ..Default::default() },
        _ => MockConfig::default(),
    };
    let mock = Mock::start(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0), MockConfig { speed_bps: Some(16 << 20), chunk: 1 << 20, ..mcfg }).expect("mock");
    let dir = std::env::temp_dir().join(format!("pspp-headless-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let mut paths = Vec::new();
    if name == "split" {
        let mut f = Fixture::game("CUSA10002");
        f.total_size = 24 << 20;
        paths.extend(f.write_split(&dir, "Split", 3).expect("split"));
    } else {
        let mut f = Fixture::game("CUSA10001");
        f.total_size = 16 << 20;
        let p = dir.join("game.pkg");
        f.write(&p).expect("pkg");
        paths.push(p);
    }
    let log = Logger::memory();
    let config = Config { console_ip: Some("127.0.0.1".into()), console_port: mock.addr.port(), server_port: 0, ..Default::default() };
    let timings = EngineTimings { tick: Duration::from_millis(250), fw01: Duration::from_secs(3), stall: Duration::from_secs(3), ..Default::default() };
    let (etx, erx) = mpsc::channel();
    let h = engine::spawn(EngineOptions { data_dir: None, config, timings, dialect: RpiDialect::default(), log: log.clone() }, etx, Arc::new(|| {}));
    h.tx.send(Command::AddPaths(paths)).expect("send");
    let t0 = Instant::now();
    let mut started = false;
    let mut last = String::new();
    while t0.elapsed() < Duration::from_secs(30) {
        let Ok(ev) = erx.recv_timeout(Duration::from_millis(100)) else { continue };
        let ts = format!("{:6.2}s", t0.elapsed().as_secs_f64());
        match ev {
            Event::ItemUpsert(i) => {
                let pct = i.live.fraction.map_or(String::new(), |f| format!(" {:5.1}%", f * 100.0));
                let speed = i.live.speed.map_or(String::new(), |s| format!(" {:.1} MB/s", s / 1048576.0));
                let eta = i.live.eta.map_or(String::new(), |e| format!(" ETA {e:.0}s"));
                let err = i.error.as_ref().map_or(String::new(), |e| format!("  {} {}{}", e.code, e.detail, e.console_log.as_ref().map_or(String::new(), |_| " (+console log)".into())));
                let line = format!("{:?}{pct}{speed}{eta}{err}", i.state);
                if line != last {
                    println!("{ts} {:<24} {line}", i.title.unwrap_or_default());
                    last = line;
                }
                if i.state == State::Ready && !started {
                    started = true;
                    h.tx.send(Command::StartQueue).expect("send");
                }
            }
            Event::QueueRunning(false) if started => {
                println!("{ts} queue stopped");
                break;
            }
            other => println!("{ts} {other:?}"),
        }
    }
    let _ = h.tx.send(Command::Shutdown);
    let _ = h.join.join();
    mock.stop();
    let errors: Vec<String> = log.entries().iter().filter(|e| e.level <= pspp::logging::Level::Warn).map(|e| e.line()).collect();
    for e in errors {
        println!("  log: {e}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

fn main() {
    let which: Vec<String> = std::env::args().skip(1).collect();
    let all = ["ok", "split", "stall", "nofetch", "failcode"];
    for s in all.iter().filter(|s| which.is_empty() || which.iter().any(|w| w == *s)) {
        scenario(s);
    }
}
