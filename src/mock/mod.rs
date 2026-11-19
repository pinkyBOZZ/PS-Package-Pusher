//! Mock RPI 3.58 (DESIGN 8): same REST API, downloads packages from PSPP like BGFT would.
//! Every unconfirmed detail has a flag with both variants (PLAN 1.10).

use crate::{
    pkg::{Kind, header::read_info_from},
    server::http,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    io::{BufReader, Cursor, Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExistsFormat {
    Bool,
    Str,
    Int,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumFormat {
    Number,
    Dec,
    HexStr,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OriginCheck {
    Destructive,
    All,
    None,
}

#[derive(Debug, Clone)]
pub struct MockConfig {
    pub speed_bps: Option<u64>,
    pub latency: Duration,
    pub fail_install: Option<String>,
    /// Fraction 0..1 at which the task fails with `error_code`.
    pub fail_task_at: Option<f64>,
    pub error_code: u32,
    /// Stop pulling after this many bytes (no error) → PSPP must detect the stall.
    pub stall_after: Option<u64>,
    /// Accept install but never fetch → FW-01.
    pub no_fetch: bool,
    /// No `/api/log`, no fork progress fields.
    pub legacy: bool,
    pub reject_all_403: bool,
    pub installed: Vec<String>,
    pub exists_format: ExistsFormat,
    pub log_post: bool,
    /// `false` = DESIGN field names, `true` = alternative (`id`) for uninstall_patch/ac.
    pub uninstall_alt: bool,
    /// Unknown endpoint body: JSON (`true`) or empty (`false`).
    pub notfound_json: bool,
    pub origin_check: OriginCheck,
    pub num_format: NumFormat,
    pub chunk: u64,
    /// RPI aborts a download when the PC sends nothing for this long (30 s on RPI 3.58).
    pub stall_timeout: Duration,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            speed_bps: None,
            latency: Duration::ZERO,
            fail_install: None,
            fail_task_at: None,
            error_code: 0x8099_0085,
            stall_after: None,
            no_fetch: false,
            legacy: false,
            reject_all_403: false,
            installed: Vec::new(),
            exists_format: ExistsFormat::Bool,
            log_post: true,
            uninstall_alt: false,
            notfound_json: false,
            origin_check: OriginCheck::Destructive,
            num_format: NumFormat::Number,
            chunk: 8 * 1024 * 1024,
            stall_timeout: Duration::from_secs(30),
        }
    }
}

/// Code the mock reports when the PC stops sending (stand-in; real code is [PROVJERA]).
pub const STALL_CODE: u32 = 0x8099_0088;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Running,
    Paused,
    Stopped,
    Done,
    Error(u32),
}

#[derive(Debug, Clone)]
pub struct Task {
    pub urls: Vec<String>,
    pub state: TaskState,
    pub transferred: u64,
    pub total: u64,
    pub started: Instant,
}

#[derive(Default)]
struct State {
    installed: HashSet<String>,
    tasks: BTreeMap<u64, Task>,
    next_id: u64,
    log: VecDeque<String>,
}

#[derive(Clone)]
pub struct Mock {
    pub addr: SocketAddrV4,
    cfg: Arc<MockConfig>,
    st: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
}

impl Mock {
    pub fn start(bind: SocketAddrV4, cfg: MockConfig) -> std::io::Result<Mock> {
        let listener = TcpListener::bind(bind)?;
        let SocketAddr::V4(addr) = listener.local_addr()? else {
            return Err(std::io::Error::other("ipv6"));
        };
        let st = State {
            installed: cfg.installed.iter().cloned().collect(),
            next_id: 1,
            ..Default::default()
        };
        let m = Mock {
            addr,
            cfg: Arc::new(cfg),
            st: Arc::new(Mutex::new(st)),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let m2 = m.clone();
        thread::spawn(move || {
            for c in listener.incoming() {
                if m2.stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(s) = c {
                    let m3 = m2.clone();
                    thread::spawn(move || m3.serve(s));
                }
            }
        });
        Ok(m)
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&SocketAddr::V4(self.addr), Duration::from_millis(200));
    }

    pub fn installed(&self) -> Vec<String> {
        let mut v: Vec<String> = self.lock().installed.iter().cloned().collect();
        v.sort();
        v
    }

    pub fn tasks(&self) -> BTreeMap<u64, Task> {
        self.lock().tasks.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.st.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn log(&self, line: String) {
        let mut st = self.lock();
        if st.log.len() > 500 {
            st.log.pop_front();
        }
        st.log.push_back(line);
    }

    fn serve(&self, s: TcpStream) {
        let _ = s.set_read_timeout(Some(Duration::from_secs(30)));
        let Ok(r) = s.try_clone() else { return };
        let mut reader = BufReader::new(r);
        let mut w = &s;
        while let Ok(Some(req)) = http::read_request(&mut reader) {
            thread::sleep(self.cfg.latency);
            let (status, body) = self.route(&req);
            let ct = if body.starts_with('{') {
                "application/json"
            } else {
                "text/plain"
            };
            if http::respond(
                &mut w,
                status,
                &[("Content-Type", ct.into())],
                body.as_bytes(),
            )
            .is_err()
                || req.wants_close()
            {
                break;
            }
        }
    }

    fn n(&self, v: u64) -> Value {
        match self.cfg.num_format {
            NumFormat::Number => json!(v),
            NumFormat::Dec => json!(v.to_string()),
            NumFormat::HexStr => json!(format!("0x{v:X}")),
        }
    }

    fn route(&self, req: &http::Request) -> (u16, String) {
        let fail = |e: &str| json!({"status": "fail", "error": e}).to_string();
        let ok = |mut v: Value| {
            v["status"] = json!("success");
            v.to_string()
        };
        if self.cfg.reject_all_403 {
            return (403, fail("forbidden"));
        }
        let host = req.header("host").unwrap_or("");
        let host_ip = host.rsplit_once(':').map_or(host, |(h, _)| h);
        if host_ip.parse::<Ipv4Addr>().is_err() {
            return (403, fail("forbidden"));
        }
        let path = req.target.as_str();
        let destructive = path.starts_with("/api/uninstall_")
            || matches!(
                path,
                "/api/stop_task" | "/api/unregister_task" | "/api/exit" | "/api/install"
            );
        let cross = req.header("origin").is_some()
            || req
                .header("sec-fetch-site")
                .is_some_and(|v| v.eq_ignore_ascii_case("cross-site"));
        let check = match self.cfg.origin_check {
            OriginCheck::Destructive => destructive,
            OriginCheck::All => true,
            OriginCheck::None => false,
        };
        if check && cross {
            return (403, fail("forbidden"));
        }
        if path.starts_with("/api/uninstall_") && req.method != "POST" {
            return (405, String::new());
        }
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
        self.log(format!(
            "{} {} {}",
            req.method,
            path,
            String::from_utf8_lossy(&req.body)
        ));
        let task_id = body.get("task_id").and_then(crate::rpi::types::num);
        match path {
            "/api/install" => self.install(&body),
            "/api/is_exists" => {
                let Some(t) = body.get("title_id").and_then(Value::as_str) else {
                    return (200, fail("title_id missing"));
                };
                let e = self.lock().installed.contains(t);
                let ev = match self.cfg.exists_format {
                    ExistsFormat::Bool => json!(e),
                    ExistsFormat::Str => json!(if e { "true" } else { "false" }),
                    ExistsFormat::Int => json!(u8::from(e)),
                };
                (
                    200,
                    ok(json!({"exists": ev, "size": self.n(if e { 1024 } else { 0 })})),
                )
            }
            "/api/get_task_progress" => {
                let st = self.lock();
                let Some(t) = task_id.and_then(|id| st.tasks.get(&id)) else {
                    return (200, fail("task not found"));
                };
                let err = match t.state {
                    TaskState::Error(c) => c,
                    _ => 0,
                };
                let speed = t.transferred as f64 / t.started.elapsed().as_secs_f64().max(0.001);
                let rest = if speed > 0.0 {
                    ((t.total - t.transferred) as f64 / speed) as u64
                } else {
                    0
                };
                let mut v = json!({
                    "bits": self.n(0), "error": self.n(u64::from(err)),
                    "length": self.n(t.total), "transferred": self.n(t.transferred),
                    "length_total": self.n(t.total), "transferred_total": self.n(t.transferred),
                    "num_index": self.n(1), "num_total": self.n(t.urls.len() as u64),
                    "rest_sec": self.n(rest), "rest_sec_total": self.n(rest),
                    "preparing_percent": self.n(100), "local_copy_percent": self.n(0),
                });
                if !self.cfg.legacy {
                    let pct = (t.transferred * 100).checked_div(t.total).unwrap_or(0);
                    v["progress_percent"] = json!(pct);
                    v["transferred_bytes"] = self.n(t.transferred);
                    v["total_size"] = self.n(t.total);
                }
                (200, ok(v))
            }
            "/api/pause_task"
            | "/api/resume_task"
            | "/api/stop_task"
            | "/api/unregister_task"
            | "/api/start_task" => {
                let mut st = self.lock();
                let Some(id) = task_id.filter(|id| st.tasks.contains_key(id)) else {
                    return (200, fail("task not found"));
                };
                match path {
                    "/api/unregister_task" => {
                        st.tasks.remove(&id);
                    }
                    p => {
                        let t = st.tasks.get_mut(&id).expect("checked");
                        if matches!(t.state, TaskState::Running | TaskState::Paused) {
                            t.state = match p {
                                "/api/pause_task" => TaskState::Paused,
                                "/api/stop_task" => TaskState::Stopped,
                                _ => TaskState::Running,
                            };
                        }
                    }
                }
                (200, ok(json!({})))
            }
            "/api/uninstall_game" | "/api/uninstall_patch" | "/api/uninstall_ac" => {
                let field = if path == "/api/uninstall_game" || !self.cfg.uninstall_alt {
                    if path == "/api/uninstall_ac" {
                        "content_id"
                    } else {
                        "title_id"
                    }
                } else {
                    "id"
                };
                let Some(id) = body.get(field).and_then(Value::as_str) else {
                    return (200, fail("missing id"));
                };
                let removed = self.lock().installed.remove(id);
                if removed {
                    (200, ok(json!({})))
                } else {
                    (404, fail("not installed"))
                }
            }
            "/api/log" if !self.cfg.legacy && (req.method == "POST") == self.cfg.log_post => (
                200,
                self.lock()
                    .log
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            "/api/log" if !self.cfg.legacy => (405, String::new()),
            "/api/exit" if req.method == "POST" => (200, ok(json!({}))),
            _ => (
                404,
                if self.cfg.notfound_json {
                    fail("Unknown endpoint")
                } else {
                    String::new()
                },
            ),
        }
    }

    fn install(&self, body: &Value) -> (u16, String) {
        let fail = |e: &str| (200, json!({"status": "fail", "error": e}).to_string());
        if let Some(msg) = &self.cfg.fail_install {
            return fail(msg);
        }
        let Some(urls) = body.get("packages").and_then(Value::as_array) else {
            return fail("packages missing");
        };
        let urls: Vec<String> = urls
            .iter()
            .filter_map(|u| u.as_str().map(str::to_string))
            .collect();
        if urls.is_empty() || !urls.iter().all(|u| valid_url(u)) {
            return fail("invalid url");
        }
        let id = {
            let mut st = self.lock();
            let id = st.next_id;
            st.next_id += 1;
            st.tasks.insert(
                id,
                Task {
                    urls: urls.clone(),
                    state: TaskState::Running,
                    transferred: 0,
                    total: 0,
                    started: Instant::now(),
                },
            );
            id
        };
        if !self.cfg.no_fetch {
            let m = self.clone();
            thread::spawn(move || m.download(id, urls));
        }
        (
            200,
            json!({"status": "success", "task_id": id, "title": "Mock task"}).to_string(),
        )
    }

    fn set_state(&self, id: u64, s: TaskState) {
        if let Some(t) = self.lock().tasks.get_mut(&id) {
            t.state = s;
        }
    }

    fn state(&self, id: u64) -> Option<TaskState> {
        self.lock().tasks.get(&id).map(|t| t.state)
    }

    /// BGFT-like: HEAD each part for its size, then sequential Range GETs in chunks.
    fn download(&self, id: u64, urls: Vec<String>) {
        let mut sizes = Vec::new();
        for u in &urls {
            match fetch(u, "HEAD", None, self.cfg.stall_timeout)
                .and_then(|(s, h, _)| if s == 200 { content_length(&h) } else { None })
            {
                Some(n) => sizes.push(n),
                None => return self.set_state(id, TaskState::Error(STALL_CODE)),
            }
        }
        let total: u64 = sizes.iter().sum();
        if let Some(t) = self.lock().tasks.get_mut(&id) {
            t.total = total;
        }
        let mut header: Option<Vec<u8>> = None;
        let mut done = 0u64;
        for (u, size) in urls.iter().zip(sizes) {
            let mut off = 0;
            while off < size {
                loop {
                    match self.state(id) {
                        Some(TaskState::Paused) => thread::sleep(Duration::from_millis(50)),
                        Some(TaskState::Running) => break,
                        _ => return,
                    }
                }
                if self.cfg.stall_after.is_some_and(|s| done >= s) {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                if let Some(f) = self.cfg.fail_task_at
                    && total > 0
                    && done as f64 >= f * total as f64
                {
                    return self.set_state(id, TaskState::Error(self.cfg.error_code));
                }
                let end = (off + self.cfg.chunk).min(size) - 1;
                let t0 = Instant::now();
                let got = fetch(u, "GET", Some((off, end)), self.cfg.stall_timeout);
                let Some((206, _, body)) = got else {
                    return self.set_state(id, TaskState::Error(STALL_CODE));
                };
                if body.len() as u64 != end - off + 1 {
                    return self.set_state(id, TaskState::Error(STALL_CODE));
                }
                if header.is_none() {
                    header = Some(body.clone());
                }
                off = end + 1;
                done += body.len() as u64;
                if let Some(t) = self.lock().tasks.get_mut(&id) {
                    t.transferred = done;
                }
                if let Some(bps) = self.cfg.speed_bps {
                    let want = Duration::from_secs_f64(body.len() as f64 / bps as f64);
                    thread::sleep(want.saturating_sub(t0.elapsed()));
                }
            }
        }
        // a finished game counts as installed (DESIGN 8.1 is_exists)
        if let Some(h) = header
            && let Ok(info) = read_info_from(&mut Cursor::new(&h), total.max(h.len() as u64))
            && info.kind() == Kind::Game
        {
            self.lock().installed.insert(info.title_id);
        }
        self.set_state(id, TaskState::Done);
    }
}

/// RPI 3.58 install validation: `http://`, IPv4 host, no query string, ASCII path.
pub fn valid_url(u: &str) -> bool {
    let Some(rest) = u.strip_prefix("http://") else {
        return false;
    };
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = hostport.rsplit_once(':').map_or(hostport, |(h, _)| h);
    host.parse::<Ipv4Addr>().is_ok() && !u.contains('?') && path.is_ascii()
}

fn content_length(headers: &str) -> Option<u64> {
    headers.lines().find_map(|l| {
        l.split_once(':')
            .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.trim().parse().ok())
    })
}

/// Tiny HTTP client: one request per connection. Returns (status, raw headers, body).
fn fetch(
    url: &str,
    method: &str,
    range: Option<(u64, u64)>,
    timeout: Duration,
) -> Option<(u16, String, Vec<u8>)> {
    let rest = url.strip_prefix("http://")?;
    let (hostport, path) = rest.split_once('/')?;
    let addr: SocketAddr = hostport.parse().ok()?;
    let mut s = TcpStream::connect_timeout(&addr, timeout).ok()?;
    s.set_read_timeout(Some(timeout)).ok()?;
    let r = range.map_or(String::new(), |(a, b)| format!("Range: bytes={a}-{b}\r\n"));
    write!(
        s,
        "{method} /{path} HTTP/1.1\r\nHost: {hostport}\r\n{r}Connection: close\r\n\r\n"
    )
    .ok()?;
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).ok()?;
    let split = buf.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let head = String::from_utf8_lossy(&buf[..split]).into_owned();
    let status = head.split_whitespace().nth(1)?.parse().ok()?;
    Some((status, head, buf[split..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::valid_url;

    #[test]
    fn url_rules() {
        assert!(valid_url(
            "http://192.168.1.10:8095/pkg/ab/UP0000-CUSA00001_00-X.pkg"
        ));
        assert!(!valid_url("https://192.168.1.10/pkg/a.pkg"));
        assert!(!valid_url("http://pc.local:8095/pkg/a.pkg"));
        assert!(!valid_url("http://192.168.1.10:8095/pkg/a.pkg?x=1"));
        assert!(!valid_url("http://192.168.1.10:8095/pkg/č.pkg"));
    }
}
