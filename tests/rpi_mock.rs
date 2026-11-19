//! RPI client ⇄ mock, both dialect variants (PLAN 1.9/1.10, 4.1).

use pspp::{
    logging::Logger,
    mock::{ExistsFormat, Mock, MockConfig, NumFormat, TaskState},
    pkg::fixture::Fixture,
    rpi::{Client, RpiDialect, RpiError, UninstallKind},
    server::{
        Server,
        registry::{Part, Registry},
        url,
    },
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

fn any() -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)
}

fn client(m: &Mock, d: RpiDialect) -> Client {
    Client::new(*m.addr.ip(), m.addr.port(), d)
}

#[test]
fn exists_formats_and_numbers() {
    for (ef, nf) in [
        (ExistsFormat::Bool, NumFormat::Number),
        (ExistsFormat::Str, NumFormat::Dec),
        (ExistsFormat::Int, NumFormat::HexStr),
    ] {
        let m = Mock::start(
            any(),
            MockConfig {
                installed: vec!["CUSA00001".into()],
                exists_format: ef,
                num_format: nf,
                ..Default::default()
            },
        )
        .unwrap();
        let c = client(&m, RpiDialect::default());
        assert!(c.is_exists("CUSA00001").0.unwrap().exists, "{ef:?}");
        assert!(!c.is_exists("CUSA99999").0.unwrap().exists);
        assert!(matches!(
            c.progress(42).0,
            Err(RpiError::Fail { status: 200, .. })
        ));
        m.stop();
    }
}

#[test]
fn uninstall_both_dialects_and_404s() {
    for alt in [false, true] {
        let m = Mock::start(
            any(),
            MockConfig {
                installed: vec!["CUSA00001".into(), "CUSA00002".into()],
                uninstall_alt: alt,
                ..Default::default()
            },
        )
        .unwrap();
        let c = client(
            &m,
            if alt {
                RpiDialect::alt()
            } else {
                RpiDialect::default()
            },
        );
        assert!(c.uninstall(UninstallKind::Game, "CUSA00001").0.is_ok());
        assert_eq!(
            c.uninstall(UninstallKind::Game, "CUSA00001").0,
            Err(RpiError::Fail {
                status: 404,
                error: "not installed".into()
            })
        );
        assert!(
            c.uninstall(UninstallKind::Patch, "CUSA00002").0.is_ok(),
            "alt={alt}"
        );
        assert_eq!(
            c.call("/api/no_such", serde_json::json!({})).0,
            Err(RpiError::EndpointMissing)
        );
        m.stop();
    }
}

#[test]
fn console_log_method_fallback_and_legacy() {
    for post in [true, false] {
        let m = Mock::start(
            any(),
            MockConfig {
                log_post: post,
                ..Default::default()
            },
        )
        .unwrap();
        let c = client(&m, RpiDialect::default());
        let _ = c.is_exists("CUSA00001");
        assert!(
            c.console_log().unwrap().contains("/api/is_exists"),
            "post={post}"
        );
        m.stop();
    }
    let m = Mock::start(
        any(),
        MockConfig {
            legacy: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        client(&m, RpiDialect::default()).console_log(),
        Err(RpiError::EndpointMissing)
    );
    m.stop();
}

#[test]
fn install_downloads_from_pspp_server() {
    let dir = std::env::temp_dir().join(format!("pspp-it-rpi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut f = Fixture::game("CUSA00042");
    f.total_size = 3 * 1024 * 1024;
    let parts = f.write_split(&dir, "G", 3).unwrap();
    let reg = Registry::default();
    let parts_meta: Vec<Part> = parts
        .iter()
        .map(|p| Part {
            path: p.clone(),
            size: std::fs::metadata(p).unwrap().len(),
            mtime: None,
        })
        .collect();
    let paths = reg.register("it1", &f.content_id, &parts_meta);
    let (tx, rx) = mpsc::channel();
    let server = Server::start(
        any(),
        reg,
        Arc::new(Mutex::new(vec![])),
        true,
        tx,
        Logger::memory(),
    )
    .unwrap();
    let urls: Vec<String> = paths.iter().map(|p| url(server.addr, p)).collect();

    let m = Mock::start(
        any(),
        MockConfig {
            chunk: 512 * 1024,
            ..Default::default()
        },
    )
    .unwrap();
    let c = client(&m, RpiDialect::default());
    assert!(!c.is_exists("CUSA00042").0.unwrap().exists);
    let id = c.install(&urls).0.unwrap();
    let t0 = Instant::now();
    loop {
        let p = c.progress(id).0.unwrap();
        if p.complete(0) {
            break;
        }
        assert!(p.error.is_none(), "{p:?}");
        assert!(t0.elapsed() < Duration::from_secs(20), "timeout {p:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(c.is_exists("CUSA00042").0.unwrap().exists);
    assert_eq!(m.tasks()[&id].state, TaskState::Done);
    assert!(
        rx.try_iter()
            .any(|e| matches!(e, pspp::server::ServerEvent::Request { .. }))
    );
    assert!(matches!(
        c.install(&["http://127.0.0.1:1/a.pkg?x=1".into()]).0,
        Err(RpiError::Fail { .. })
    ));
    m.stop();
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}
