//! RPI REST client: endpoints, timeouts, response classification (DESIGN 9.1).

use super::types::{Exists, Progress, RpiDialect, num_field};
use serde_json::{Value, json};
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
pub const CALL_TIMEOUT: Duration = Duration::from_secs(5);
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(10);
const BODY_LIMIT: u64 = 1024 * 1024;
const LOG_LIMIT: u64 = 2 * 1024 * 1024;

/// Outcome of the last exchange with a console, shared by the clients of one engine:
/// `None` = no exchange yet, `Some(None)` = unreachable, `Some(Some(ms))` = answered in `ms`.
pub type LastReach = Arc<Mutex<Option<Option<u32>>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Unreachable {
    Timeout,
    Refused,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RpiError {
    Unreachable(Unreachable),
    /// HTTP 403 (DNS rebinding / Origin checks).
    Forbidden,
    /// RPI JSON with `status: fail` (incl. 404 "not installed").
    Fail {
        status: u16,
        error: String,
    },
    /// 404 without RPI JSON: endpoint does not exist in this RPI version.
    EndpointMissing,
    BadResponse {
        status: u16,
        body: String,
    },
}

impl std::fmt::Display for RpiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpiError::Unreachable(u) => write!(f, "unreachable: {u:?}"),
            RpiError::Forbidden => f.write_str("403 forbidden"),
            RpiError::Fail { status, error } => write!(f, "fail ({status}): {error}"),
            RpiError::EndpointMissing => f.write_str("endpoint missing (404)"),
            RpiError::BadResponse { status, body } => write!(f, "bad response {status}: {body}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallKind {
    Game,
    Patch,
    Dlc,
}

/// Classification per DESIGN 9.1 (pure, unit-tested).
pub fn classify(status: u16, body: &str) -> Result<Value, RpiError> {
    let json: Option<Value> = serde_json::from_str(body)
        .ok()
        .filter(|v: &Value| v.get("status").is_some());
    let fail_text = |v: &Value| {
        let text = v.get("error").map_or_else(String::new, |e| {
            e.as_str().map_or_else(|| e.to_string(), str::to_string)
        });
        if !text.is_empty() {
            return text;
        }
        // RPI 3.58 sends error_code / error_code_hex instead of error.
        if let Some(hex) = v.get("error_code_hex").and_then(Value::as_str) {
            return hex.to_string();
        }
        v.get("error_code")
            .and_then(Value::as_i64)
            .map_or_else(String::new, |c| format!("0x{:08X}", c as u32))
    };
    match (status, json) {
        (403, _) => Err(RpiError::Forbidden),
        (404, Some(v)) => Err(RpiError::Fail {
            status,
            error: fail_text(&v),
        }),
        (404, None) => Err(RpiError::EndpointMissing),
        (200..=299, Some(v)) if v["status"] == "success" => Ok(v),
        (_, Some(v)) if v["status"] == "fail" => Err(RpiError::Fail {
            status,
            error: fail_text(&v),
        }),
        (_, _) => Err(RpiError::BadResponse {
            status,
            body: crate::logging::truncate(body, 300),
        }),
    }
}

pub struct Client {
    agent: ureq::Agent,
    pub base: String,
    pub dialect: RpiDialect,
    /// Session cache: which `/api/log` method worked.
    log_post: Mutex<Option<bool>>,
    reach: LastReach,
}

/// Raw exchange, for the log (DESIGN 7.8: bodies truncated to 2 KB by the logger).
#[derive(Debug, Clone)]
pub struct Exchange {
    pub path: String,
    pub request: String,
    pub status: Option<u16>,
    pub body: String,
}

impl Client {
    pub fn new(ip: Ipv4Addr, port: u16, dialect: RpiDialect) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(CALL_TIMEOUT))
            .http_status_as_error(false)
            .max_redirects(0)
            .proxy(None)
            .build()
            .into();
        Self {
            agent,
            base: format!("http://{ip}:{port}"),
            dialect,
            log_post: Mutex::new(None),
            reach: LastReach::default(),
        }
    }

    /// Records this client's exchanges in a shared cell instead of a private one.
    pub fn with_reach(mut self, reach: LastReach) -> Self {
        self.reach = reach;
        self
    }

    fn send(
        &self,
        post: bool,
        path: &str,
        body: &Value,
        timeout: Duration,
        limit: u64,
    ) -> (Result<(u16, String), RpiError>, Exchange) {
        let url = format!("{}{path}", self.base);
        let req_s = body.to_string();
        let res = if post {
            self.agent
                .post(&url)
                .config()
                .timeout_global(Some(timeout))
                .build()
                .header("Content-Type", "application/json")
                .send(req_s.as_bytes())
        } else {
            self.agent
                .get(&url)
                .config()
                .timeout_global(Some(timeout))
                .build()
                .call()
        };
        let started = Instant::now();
        let out = match res {
            Ok(mut r) => {
                let status = r.status().as_u16();
                match r.body_mut().with_config().limit(limit).read_to_string() {
                    Ok(b) => Ok((status, b)),
                    Err(e) => Err(map_err(e)),
                }
            }
            Err(e) => Err(map_err(e)),
        };
        // any HTTP answer (even a 403 or RPI fail) proves the console is there
        let reach = match &out {
            Err(RpiError::Unreachable(_)) => None,
            _ => Some(started.elapsed().as_millis() as u32),
        };
        *self.reach.lock().unwrap_or_else(|e| e.into_inner()) = Some(reach);
        let ex = Exchange {
            path: path.to_string(),
            request: req_s,
            status: out.as_ref().ok().map(|(s, _)| *s),
            body: out
                .as_ref()
                .map_or_else(|e| e.to_string(), |(_, b)| b.clone()),
        };
        (out, ex)
    }

    /// POST JSON and classify. Returns the exchange too, for logging.
    pub fn call(&self, path: &str, body: Value) -> (Result<Value, RpiError>, Exchange) {
        self.call_t(path, body, CALL_TIMEOUT)
    }

    fn call_t(&self, path: &str, body: Value, t: Duration) -> (Result<Value, RpiError>, Exchange) {
        let (r, ex) = self.send(true, path, &body, t, BODY_LIMIT);
        (r.and_then(|(s, b)| classify(s, &b)), ex)
    }

    pub fn install(&self, urls: &[String]) -> (Result<u64, RpiError>, Exchange) {
        let (r, ex) = self.call_t(
            "/api/install",
            json!({"type": "direct", "packages": urls}),
            INSTALL_TIMEOUT,
        );
        let r = r.and_then(|v| {
            num_field(&v, &["task_id"]).ok_or(RpiError::BadResponse {
                status: 200,
                body: "no task_id".into(),
            })
        });
        (r, ex)
    }

    pub fn is_exists(&self, title_id: &str) -> (Result<Exists, RpiError>, Exchange) {
        let (r, ex) = self.call("/api/is_exists", json!({"title_id": title_id}));
        let r = r.and_then(|v| {
            Exists::from_json(&v).ok_or(RpiError::BadResponse {
                status: 200,
                body: "no exists field".into(),
            })
        });
        (r, ex)
    }

    pub fn progress(&self, task_id: u64) -> (Result<Progress, RpiError>, Exchange) {
        let (r, ex) = self.call("/api/get_task_progress", json!({"task_id": task_id}));
        (r.map(|v| Progress::from_json(&v)), ex)
    }

    /// `pause_task`, `resume_task`, `stop_task`, `unregister_task`, `start_task`.
    pub fn task(&self, op: &str, task_id: u64) -> (Result<Value, RpiError>, Exchange) {
        self.call(&format!("/api/{op}"), json!({"task_id": task_id}))
    }

    pub fn uninstall(&self, kind: UninstallKind, id: &str) -> (Result<Value, RpiError>, Exchange) {
        let (path, field) = match kind {
            UninstallKind::Game => ("/api/uninstall_game", "title_id"),
            UninstallKind::Patch => ("/api/uninstall_patch", self.dialect.uninstall_patch_field),
            UninstallKind::Dlc => ("/api/uninstall_ac", self.dialect.uninstall_ac_field),
        };
        self.call(path, json!({ field: id }))
    }

    /// Console log text. Tries the dialect's method first, the other on 404/405; caches the winner.
    pub fn console_log(&self) -> Result<String, RpiError> {
        let cached = *self.log_post.lock().unwrap_or_else(|e| e.into_inner());
        let order = match cached {
            Some(p) => vec![p],
            None => vec![self.dialect.log_post_first, !self.dialect.log_post_first],
        };
        let mut last = RpiError::EndpointMissing;
        for post in order {
            match self
                .send(post, "/api/log", &json!({}), CALL_TIMEOUT, LOG_LIMIT)
                .0
            {
                Ok((s, b)) if (200..300).contains(&s) => {
                    *self.log_post.lock().unwrap_or_else(|e| e.into_inner()) = Some(post);
                    return Ok(b);
                }
                Ok((403, _)) => return Err(RpiError::Forbidden),
                Ok((s @ (404 | 405), _)) => {
                    last = if s == 404 {
                        RpiError::EndpointMissing
                    } else {
                        RpiError::BadResponse {
                            status: 405,
                            body: String::new(),
                        }
                    }
                }
                Ok((s, b)) => return Err(RpiError::BadResponse { status: s, body: b }),
                Err(e) => return Err(e),
            }
        }
        Err(last)
    }
}

fn map_err(e: ureq::Error) -> RpiError {
    match e {
        ureq::Error::Timeout(_) => RpiError::Unreachable(Unreachable::Timeout),
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::ConnectionRefused => {
            RpiError::Unreachable(Unreachable::Refused)
        }
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => {
            RpiError::Unreachable(Unreachable::Timeout)
        }
        ureq::Error::ConnectionFailed => RpiError::Unreachable(Unreachable::Refused),
        ureq::Error::BodyExceedsLimit(n) => RpiError::BadResponse {
            status: 200,
            body: format!("body over {n} B"),
        },
        other => RpiError::Unreachable(Unreachable::Other(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_order() {
        assert!(classify(200, r#"{"status":"success","task_id":3}"#).is_ok());
        assert_eq!(classify(403, ""), Err(RpiError::Forbidden));
        assert_eq!(
            classify(404, r#"{"status":"fail","error":"not installed"}"#),
            Err(RpiError::Fail {
                status: 404,
                error: "not installed".into()
            })
        );
        assert_eq!(classify(404, ""), Err(RpiError::EndpointMissing));
        assert_eq!(
            classify(404, "Unknown endpoint"),
            Err(RpiError::EndpointMissing)
        );
        assert_eq!(
            classify(200, r#"{"status":"fail","error":"bad url"}"#),
            Err(RpiError::Fail {
                status: 200,
                error: "bad url".into()
            })
        );
        assert_eq!(
            classify(
                200,
                r#"{"status":"fail","error_code":-2137456615,"error_code_hex":"0x80990019"}"#
            ),
            Err(RpiError::Fail {
                status: 200,
                error: "0x80990019".into()
            })
        );
        assert_eq!(
            classify(200, r#"{"status":"fail","error_code":-2137456619}"#),
            Err(RpiError::Fail {
                status: 200,
                error: "0x80990015".into()
            })
        );
        assert!(matches!(
            classify(500, "oops"),
            Err(RpiError::BadResponse { status: 500, .. })
        ));
        assert!(matches!(
            classify(200, "<html>"),
            Err(RpiError::BadResponse { .. })
        ));
    }

    #[test]
    fn refused_and_timeout() {
        // nothing listens on port 1 of loopback
        let c = Client::new(Ipv4Addr::LOCALHOST, 1, RpiDialect::default());
        assert!(matches!(
            c.is_exists("CUSA00000").0,
            Err(RpiError::Unreachable(_))
        ));
    }
}
