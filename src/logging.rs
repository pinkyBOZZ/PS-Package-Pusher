//! Log entries, daily file with rotation, ring buffer for the UI (DESIGN 7.8).

use crate::{errors::Code, winapi};
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub const RING: usize = 5000;
pub const KEEP_FILES: usize = 14;
pub const MAX_FILE: u64 = 10 * 1024 * 1024;
pub const BODY_LIMIT: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// `YYYY-MM-DD HH:MM:SS.mmm`, local time.
    pub time: String,
    pub level: Level,
    pub source: &'static str,
    pub title_id: Option<String>,
    pub code: Option<Code>,
    pub msg: String,
    /// Extra `key=value` fields (task_id, http, body, ...).
    pub fields: Vec<(String, String)>,
}

impl Entry {
    pub fn line(&self) -> String {
        let mut s = format!("{} {:<5} [{}]", self.time, self.level.as_str(), self.source);
        if let Some(t) = &self.title_id {
            s += &format!(" [{t}]");
        }
        if let Some(c) = self.code {
            s += &format!(" {c}");
        }
        // one line per entry; only quotes/newlines escaped so Windows paths stay readable
        let msg = self.msg.replace('"', "\\\"").replace(['\r', '\n'], " ");
        s += &format!(" msg=\"{msg}\"");
        for (k, v) in &self.fields {
            s += &format!(" {k}={v}");
        }
        s
    }
}

struct State {
    dir: Option<PathBuf>,
    file: Option<(String, u32, File)>,
    ring: VecDeque<Entry>,
}

/// Cheap to clone; shared by all threads.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<Mutex<State>>,
    debug: bool,
    profile: Option<String>,
}

impl Logger {
    /// Logs to `<dir>/pspp-YYYY-MM-DD.log`; DEBUG goes to the file only if `debug`.
    pub fn new(dir: &Path, debug: bool) -> Self {
        let _ = fs::create_dir_all(dir);
        Self::build(Some(dir.to_path_buf()), debug)
    }

    /// Ring buffer only (tests, headless demo).
    pub fn memory() -> Self {
        Self::build(None, true)
    }

    fn build(dir: Option<PathBuf>, debug: bool) -> Self {
        let profile = std::env::var("USERPROFILE").ok().filter(|p| p.len() > 3);
        Self {
            inner: Arc::new(Mutex::new(State {
                dir,
                file: None,
                ring: VecDeque::new(),
            })),
            debug,
            profile,
        }
    }

    pub fn log(
        &self,
        level: Level,
        source: &'static str,
        msg: impl Into<String>,
    ) -> EntryBuilder<'_> {
        EntryBuilder {
            logger: self,
            entry: Entry {
                time: now_string(),
                level,
                source,
                title_id: None,
                code: None,
                msg: msg.into(),
                fields: Vec::new(),
            },
        }
    }

    pub fn error(&self, source: &'static str, msg: impl Into<String>) -> EntryBuilder<'_> {
        self.log(Level::Error, source, msg)
    }
    pub fn warn(&self, source: &'static str, msg: impl Into<String>) -> EntryBuilder<'_> {
        self.log(Level::Warn, source, msg)
    }
    pub fn info(&self, source: &'static str, msg: impl Into<String>) -> EntryBuilder<'_> {
        self.log(Level::Info, source, msg)
    }
    pub fn debug(&self, source: &'static str, msg: impl Into<String>) -> EntryBuilder<'_> {
        self.log(Level::Debug, source, msg)
    }

    /// Snapshot of the ring buffer (oldest first).
    pub fn entries(&self) -> Vec<Entry> {
        self.lock().ring.iter().cloned().collect()
    }

    /// Writes the console's `/api/log` to `console-<task>-<time>.txt`; returns the path.
    pub fn write_console_log(&self, task_id: u64, text: &str) -> Option<PathBuf> {
        let dir = self.lock().dir.clone()?;
        let t = now_string().replace([':', ' ', '.'], "-");
        let p = dir.join(format!("console-{task_id}-{t}.txt"));
        fs::write(&p, self.anonymize(text)).ok()?;
        Some(p)
    }

    /// Replaces the user profile path (case-insensitive) with `%USERPROFILE%`.
    pub fn anonymize(&self, s: &str) -> String {
        match &self.profile {
            Some(p) => replace_ci(s, p, "%USERPROFILE%"),
            None => s.to_string(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        // A panic while logging must not take logging down for the rest of the app.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn push(&self, mut e: Entry) {
        e.msg = self.anonymize(&e.msg);
        for (_, v) in &mut e.fields {
            *v = self.anonymize(v);
        }
        let mut st = self.lock();
        if e.level != Level::Debug || self.debug {
            write_file(&mut st, &e.line());
        }
        if st.ring.len() == RING {
            st.ring.pop_front();
        }
        st.ring.push_back(e);
    }
}

pub struct EntryBuilder<'a> {
    logger: &'a Logger,
    entry: Entry,
}

impl EntryBuilder<'_> {
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.entry.title_id = Some(t.into());
        self
    }
    pub fn code(mut self, c: Code) -> Self {
        self.entry.code = Some(c);
        self
    }
    pub fn field(mut self, k: impl Into<String>, v: impl ToString) -> Self {
        self.entry.fields.push((k.into(), v.to_string()));
        self
    }
    /// RPI bodies are truncated to 2 KB (DESIGN 7.8).
    pub fn body(self, b: &str) -> Self {
        let t = truncate(b, BODY_LIMIT);
        self.field("body", t)
    }
}

impl Drop for EntryBuilder<'_> {
    fn drop(&mut self) {
        let e = std::mem::replace(
            &mut self.entry,
            Entry {
                time: String::new(),
                level: Level::Debug,
                source: "",
                title_id: None,
                code: None,
                msg: String::new(),
                fields: Vec::new(),
            },
        );
        self.logger.push(e);
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…(+{} B)", &s[..end], s.len() - end)
}

fn replace_ci(s: &str, needle: &str, with: &str) -> String {
    let (ls, ln) = (s.to_lowercase(), needle.to_lowercase());
    // to_lowercase can change byte lengths for non-ASCII; fall back to exact replace then.
    if ls.len() != s.len() || ln.len() != needle.len() {
        return s.replace(needle, with);
    }
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(p) = ls[i..].find(&ln) {
        out += &s[i..i + p];
        out += with;
        i += p + needle.len();
    }
    out + &s[i..]
}

fn now_string() -> String {
    let t = winapi::local_time();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.year, t.month, t.day, t.hour, t.minute, t.second, t.millis
    )
}

fn write_file(st: &mut State, line: &str) {
    let Some(dir) = st.dir.clone() else { return };
    let date = line[..10].to_string();
    let need_new = match &st.file {
        Some((d, _, f)) => *d != date || f.metadata().map_or(true, |m| m.len() > MAX_FILE),
        None => true,
    };
    if need_new {
        let mut n = match &st.file {
            Some((d, n, _)) if *d == date => n + 1,
            _ => 0,
        };
        let file = loop {
            let name = if n == 0 {
                format!("pspp-{date}.log")
            } else {
                format!("pspp-{date}.{n}.log")
            };
            let p = dir.join(name);
            if fs::metadata(&p).map_or(true, |m| m.len() < MAX_FILE) {
                break OpenOptions::new().create(true).append(true).open(p).ok();
            }
            n += 1;
        };
        st.file = file.map(|f| (date, n, f));
        prune(&dir);
    }
    if let Some((_, _, f)) = &mut st.file {
        let _ = writeln!(f, "{line}");
    }
}

/// Keeps the newest KEEP_FILES `pspp-*.log` files (names sort by date).
fn prune(dir: &Path) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut logs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("pspp-") && n.ends_with(".log"))
        })
        .collect();
    logs.sort();
    let excess = logs.len().saturating_sub(KEEP_FILES);
    for p in &logs[..excess] {
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_ring_anonymize() {
        let d = std::env::temp_dir().join(format!("pspp-test-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        for i in 0..20 {
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(format!("pspp-2020-01-{:02}.log", i + 1)), "old").unwrap();
        }
        let mut l = Logger::new(&d, false);
        l.profile = Some(r"C:\Users\Someone".into());
        l.error("engine", r"cannot read c:\users\someone\Downloads\x.pkg")
            .title("CUSA00001")
            .code(Code::Srv03)
            .field("task_id", 8);
        l.debug("server", "hidden from file");
        let e = l.entries();
        assert_eq!(e.len(), 2);
        let line = e[0].line();
        assert!(line.contains(r"%USERPROFILE%\Downloads\x.pkg"), "{line}");
        assert!(
            line.contains("ERROR [engine] [CUSA00001] SRV-03") && line.ends_with("task_id=8"),
            "{line}"
        );
        let files: Vec<_> = fs::read_dir(&d).unwrap().filter_map(|e| e.ok()).collect();
        assert_eq!(files.len(), KEEP_FILES);
        let today = files.iter().map(|f| f.path()).max().unwrap();
        let text = fs::read_to_string(today).unwrap();
        assert!(text.contains("SRV-03") && !text.contains("hidden from file"));
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn truncation_and_ci_replace() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdef", 3), "abc…(+3 B)");
        assert_eq!(truncate("čćž", 3), "č…(+4 B)");
        assert_eq!(
            replace_ci("A/USER/b and a/user/c", "a/User", "X"),
            "X/b and X/c"
        );
    }
}
