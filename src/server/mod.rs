//! Local HTTP file server the console downloads from (DESIGN 9.4, decided by spike S-4).

pub mod http;
pub mod range;
pub mod registry;

use crate::logging::Logger;
use range::Range;
use registry::Registry;
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::Sender,
    },
    thread,
    time::{Duration, Instant},
};

pub const MAX_CONN: usize = 32;
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(30);
pub const SLOW_READ: Duration = Duration::from_secs(10);
const BUF: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum ServerEvent {
    /// The console asked for a file of this item (FW-01 detection, Sesija B diagnostics).
    Request {
        item_id: String,
        range: Option<String>,
        status: u16,
    },
    /// Bytes delivered for an item by a finished response.
    Sent { item_id: String, bytes: u64 },
    /// File size/mtime differs from registration (SRV-03).
    FileChanged { item_id: String },
    /// A single disk read took longer than 10 s (SRV-04).
    SlowDisk { item_id: String, millis: u64 },
}

pub struct Server {
    pub addr: SocketAddrV4,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
struct Shared {
    registry: Registry,
    allowed: Arc<Mutex<Vec<Ipv4Addr>>>,
    allow_loopback: bool,
    live: Arc<AtomicUsize>,
    events: Sender<ServerEvent>,
    log: Logger,
}

impl Server {
    /// Binds `bind` (PC LAN IP; port 0 = any free port). `AddrInUse` maps to SRV-01.
    /// `allowed` is the console IP list (shared so it can change); loopback is allowed when `allow_loopback`.
    pub fn start(
        bind: SocketAddrV4,
        registry: Registry,
        allowed: Arc<Mutex<Vec<Ipv4Addr>>>,
        allow_loopback: bool,
        events: Sender<ServerEvent>,
        log: Logger,
    ) -> io::Result<Server> {
        let listener = TcpListener::bind(bind)?;
        let addr = match listener.local_addr()? {
            SocketAddr::V4(a) => a,
            SocketAddr::V6(_) => return Err(io::Error::other("ipv6 bind")),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Shared {
            registry,
            allowed,
            allow_loopback,
            live: Arc::new(AtomicUsize::new(0)),
            events,
            log,
        };
        let st = stop.clone();
        let handle = thread::Builder::new()
            .name("server-accept".into())
            .spawn(move || accept_loop(listener, st, shared))?;
        Ok(Server {
            addr,
            stop,
            handle: Some(handle),
        })
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&SocketAddr::V4(self.addr), Duration::from_millis(200)); // wake accept()
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Next free port in `start+1 ..= start+20` on `ip` (SRV-01 suggestion).
pub fn suggest_port(ip: Ipv4Addr, start: u16) -> Option<u16> {
    (1..=20u16)
        .filter_map(|d| start.checked_add(d))
        .find(|&p| TcpListener::bind((ip, p)).is_ok())
}

fn accept_loop(listener: TcpListener, stop: Arc<AtomicBool>, sh: Shared) {
    for conn in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(stream) = conn else { continue };
        let peer = match stream.peer_addr() {
            Ok(SocketAddr::V4(a)) => *a.ip(),
            _ => continue,
        };
        let ok = (sh.allow_loopback && peer.is_loopback())
            || sh
                .allowed
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains(&peer);
        if !ok {
            sh.log
                .debug("server", "dropped connection from non-whitelisted IP")
                .field("peer", peer);
            let _ = stream.shutdown(Shutdown::Both); // no response at all (DESIGN 9.4)
            continue;
        }
        if sh.live.fetch_add(1, Ordering::SeqCst) >= MAX_CONN {
            sh.live.fetch_sub(1, Ordering::SeqCst);
            thread::spawn(move || reject_busy(stream));
            continue;
        }
        let sh2 = sh.clone();
        let spawned = thread::Builder::new()
            .name("server-conn".into())
            .spawn(move || {
                let port = stream.peer_addr().map(|a| a.port()).unwrap_or(0);
                if let Err(e) = serve(stream, port, peer, &sh2) {
                    sh2.log.debug("server", "connection ended").field("err", e);
                }
                sh2.live.fetch_sub(1, Ordering::SeqCst);
            });
        if spawned.is_err() {
            sh.live.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Read the request first, then 503; otherwise the client sees a reset (spike S-4).
fn reject_busy(stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut r = BufReader::new(&stream);
    let _ = http::read_request(&mut r);
    let _ = http::respond(&mut &stream, 503, &[("Connection", "close".into())], b"");
}

fn serve(stream: TcpStream, port: u16, peer: Ipv4Addr, sh: &Shared) -> io::Result<()> {
    stream.set_read_timeout(Some(SOCKET_TIMEOUT))?;
    stream.set_write_timeout(Some(SOCKET_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut w = &stream;
    while let Some(req) = http::read_request(&mut reader)? {
        let close = req.wants_close();
        let range_h = req.header("range").map(str::to_string);
        let (status, sent, item) = handle(&req, &mut w, peer, sh)?;
        sh.log
            .debug("server", "request")
            .field("method", &req.method)
            .field("path", &req.target)
            .field("range", range_h.as_deref().unwrap_or("-"))
            .field("port", port)
            .field("status", status)
            .field("bytes", sent);
        if let Some(id) = item {
            let _ = sh.events.send(ServerEvent::Request {
                item_id: id.clone(),
                range: range_h,
                status,
            });
            if sent > 0 {
                let _ = sh.events.send(ServerEvent::Sent {
                    item_id: id,
                    bytes: sent,
                });
            }
        }
        if close {
            break;
        }
    }
    Ok(())
}

/// Returns (status, body bytes sent, matched item id).
fn handle(
    req: &http::Request,
    w: &mut &TcpStream,
    peer: Ipv4Addr,
    sh: &Shared,
) -> io::Result<(u16, u64, Option<String>)> {
    let head = req.method == "HEAD";
    if req.method != "GET" && !head {
        http::respond(w, 405, &[], b"")?;
        return Ok((405, 0, None));
    }
    if req.target == "/healthz" && head && peer.is_loopback() {
        http::respond(w, 200, &[], b"")?;
        return Ok((200, 0, None));
    }
    let Some((id, part)) = sh.registry.lookup(&req.target) else {
        http::respond(w, 404, &[], b"")?;
        return Ok((404, 0, None));
    };
    let changed = || -> io::Result<bool> {
        let m = std::fs::metadata(&part.path)?;
        Ok(m.len() != part.size || (part.mtime.is_some() && m.modified().ok() != part.mtime))
    };
    if changed().unwrap_or(true) {
        let _ = sh.events.send(ServerEvent::FileChanged {
            item_id: id.clone(),
        });
        let _ = w.shutdown(Shutdown::Both);
        return Err(io::Error::other("file changed (SRV-03)"));
    }
    let len = part.size;
    let (status, start, count) = match range::parse(req.header("range"), len) {
        Range::Full => (200, 0, len),
        Range::Partial(s, e) => (206, s, e - s + 1),
        Range::Unsatisfiable => {
            http::respond(w, 416, &[("Content-Range", format!("bytes */{len}"))], b"")?;
            return Ok((416, 0, Some(id)));
        }
    };
    let mut head_s = format!(
        "HTTP/1.1 {status} {}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nContent-Length: {count}\r\n",
        http::reason(status)
    );
    if status == 206 {
        head_s += &format!(
            "Content-Range: bytes {start}-{}/{len}\r\n",
            start + count - 1
        );
    }
    head_s += "\r\n";
    w.write_all(head_s.as_bytes())?;
    if head || count == 0 {
        return Ok((status, 0, Some(id)));
    }
    let mut f = File::open(&part.path)?;
    f.seek(SeekFrom::Start(start))?;
    let mut buf = vec![0u8; BUF];
    let mut left = count;
    let mut sent = 0u64;
    while left > 0 {
        let n = (left.min(BUF as u64)) as usize;
        let t = Instant::now();
        f.read_exact(&mut buf[..n])?;
        let took = t.elapsed();
        if took > SLOW_READ {
            let _ = sh.events.send(ServerEvent::SlowDisk {
                item_id: id.clone(),
                millis: took.as_millis() as u64,
            });
        }
        if let Err(e) = w.write_all(&buf[..n]) {
            if sent > 0 {
                let _ = sh.events.send(ServerEvent::Sent {
                    item_id: id.clone(),
                    bytes: sent,
                });
            }
            return Err(e);
        }
        sent += n as u64;
        left -= n as u64;
    }
    Ok((status, sent, Some(id)))
}

/// `http://<ip>:<port><path>` for the install request.
pub fn url(addr: SocketAddrV4, path: &str) -> String {
    format!("http://{}:{}{path}", addr.ip(), addr.port())
}
