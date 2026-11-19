//! File server integration tests on loopback (DESIGN 9.4, PLAN 1.11).

use pspp::{
    logging::Logger,
    pkg::fixture::write_sparse,
    server::{
        MAX_CONN, Server, ServerEvent,
        registry::{Part, Registry},
    },
};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

const BIG: u64 = 5 << 30;

struct Env {
    server: Server,
    urls: Vec<String>,
    events: mpsc::Receiver<ServerEvent>,
    dir: PathBuf,
}

fn setup(name: &str, allow_loopback: bool) -> Env {
    let dir = std::env::temp_dir().join(format!("pspp-it-server-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.pkg");
    write_sparse(&path, b"\x7fCNT", BIG).unwrap();
    {
        let mut f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        use std::io::{Seek, SeekFrom};
        f.seek(SeekFrom::Start(BIG - 16)).unwrap();
        f.write_all(b"END-OF-5GB-FILE!").unwrap();
    }
    let meta = std::fs::metadata(&path).unwrap();
    let reg = Registry::default();
    let urls = reg.register(
        "item1",
        "UP0000-CUSA00001_00-GAME",
        &[Part {
            path,
            size: BIG,
            mtime: meta.modified().ok(),
        }],
    );
    let (tx, rx) = mpsc::channel();
    let server = Server::start(
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0),
        reg,
        Arc::new(Mutex::new(Vec::new())),
        allow_loopback,
        tx,
        Logger::memory(),
    )
    .unwrap();
    Env {
        server,
        urls,
        events: rx,
        dir,
    }
}

fn raw(addr: SocketAddrV4, req: &str) -> (String, Vec<u8>) {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut out = Vec::new();
    let _ = s.read_to_end(&mut out);
    let split = out
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or(out.len(), |p| p + 4);
    (
        String::from_utf8_lossy(&out[..split]).into_owned(),
        out[split..].to_vec(),
    )
}

fn get(addr: SocketAddrV4, path: &str, range: Option<&str>) -> (String, Vec<u8>) {
    let r = range.map_or(String::new(), |r| format!("Range: {r}\r\n"));
    raw(
        addr,
        &format!("GET {path} HTTP/1.1\r\nHost: x\r\n{r}Connection: close\r\n\r\n"),
    )
}

#[test]
fn range_over_4gb_and_errors() {
    let e = setup("range", true);
    let a = e.server.addr;
    let (h, b) = get(a, &e.urls[0], Some("bytes=0-3"));
    assert!(
        h.starts_with("HTTP/1.1 206") && h.contains("Content-Range: bytes 0-3/5368709120"),
        "{h}"
    );
    assert_eq!(b, b"\x7fCNT");
    let (h, b) = get(a, &e.urls[0], Some("bytes=-16"));
    assert!(h.starts_with("HTTP/1.1 206"), "{h}");
    assert_eq!(b, b"END-OF-5GB-FILE!");
    let s = u64::from(u32::MAX) + 1;
    let (h, b) = get(a, &e.urls[0], Some(&format!("bytes={s}-{}", s + 1023)));
    assert!(
        h.contains(&format!("Content-Range: bytes {s}-{}/", s + 1023)),
        "{h}"
    );
    assert_eq!(b.len(), 1024);
    let (h, _) = raw(
        a,
        &format!("HEAD {} HTTP/1.1\r\nConnection: close\r\n\r\n", e.urls[0]),
    );
    assert!(
        h.starts_with("HTTP/1.1 200") && h.contains("Content-Length: 5368709120"),
        "{h}"
    );
    assert!(!h.to_lowercase().contains("server:"));
    assert!(
        get(a, &e.urls[0], Some("bytes=0-1,5-6"))
            .0
            .starts_with("HTTP/1.1 416")
    );
    assert!(
        get(a, &format!("{}?x=1", e.urls[0]), None)
            .0
            .starts_with("HTTP/1.1 404")
    );
    assert!(
        get(a, "/pkg/item1/../big.pkg", None)
            .0
            .starts_with("HTTP/1.1 404")
    );
    assert!(
        raw(a, "POST /x HTTP/1.1\r\nConnection: close\r\n\r\n")
            .0
            .starts_with("HTTP/1.1 405")
    );
    assert!(
        raw(a, "HEAD /healthz HTTP/1.1\r\nConnection: close\r\n\r\n")
            .0
            .starts_with("HTTP/1.1 200")
    );
    let ev: Vec<_> = e.events.try_iter().collect();
    assert!(ev.iter().any(
        |x| matches!(x, ServerEvent::Request { item_id, status: 206, .. } if item_id == "item1")
    ));
    assert!(
        ev.iter()
            .any(|x| matches!(x, ServerEvent::Sent { bytes: 1024, .. }))
    );
    drop(e.server);
    std::fs::remove_dir_all(e.dir).unwrap();
}

#[test]
fn keep_alive_and_limit() {
    let e = setup("limit", true);
    let a = e.server.addr;
    // two requests on one connection
    let mut s = TcpStream::connect(a).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let req = format!(
        "GET {} HTTP/1.1\r\nRange: bytes=0-3\r\n\r\nGET {} HTTP/1.1\r\nRange: bytes=0-3\r\nConnection: close\r\n\r\n",
        e.urls[0], e.urls[0]
    );
    s.write_all(req.as_bytes()).unwrap();
    let mut out = Vec::new();
    let _ = s.read_to_end(&mut out);
    assert_eq!(
        String::from_utf8_lossy(&out)
            .matches("HTTP/1.1 206")
            .count(),
        2
    );

    // MAX_CONN idle connections occupy all slots → the next one gets 503 (not a reset)
    let hold: Vec<TcpStream> = (0..MAX_CONN)
        .map(|_| TcpStream::connect(a).unwrap())
        .collect();
    // slow CI runners accept the held connections late: poll instead of a fixed sleep
    let wait_for = |want: &str| {
        let t0 = std::time::Instant::now();
        loop {
            let (h, _) = get(a, &e.urls[0], Some("bytes=0-3"));
            if h.starts_with(want) || t0.elapsed() > Duration::from_secs(5) {
                return h;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    let h = wait_for("HTTP/1.1 503");
    assert!(h.starts_with("HTTP/1.1 503"), "{h}");
    drop(hold);
    let h = wait_for("HTTP/1.1 206");
    assert!(h.starts_with("HTTP/1.1 206"), "{h}");
    drop(e.server);
    std::fs::remove_dir_all(e.dir).unwrap();
}

#[test]
fn non_whitelisted_peer_gets_no_response() {
    let e = setup("filter", false);
    let (h, b) = get(e.server.addr, &e.urls[0], Some("bytes=0-3"));
    assert!(h.is_empty() && b.is_empty(), "{h}");
    drop(e.server);
    std::fs::remove_dir_all(e.dir).unwrap();
}

#[test]
fn changed_file_is_reported() {
    let e = setup("changed", true);
    let path = e.dir.join("big.pkg");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(10)
        .unwrap();
    let (h, _) = get(e.server.addr, &e.urls[0], Some("bytes=0-3"));
    assert!(h.is_empty(), "{h}");
    let ev: Vec<_> = e
        .events
        .recv_timeout(Duration::from_secs(2))
        .into_iter()
        .collect();
    assert!(matches!(&ev[..], [ServerEvent::FileChanged { item_id }] if item_id == "item1"));
    drop(e.server);
    std::fs::remove_dir_all(e.dir).unwrap();
}
