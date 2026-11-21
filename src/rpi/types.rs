//! Tolerant RPI JSON reading (DESIGN 9.1/9.2) and the single place for unconfirmed details (PLAN 4.1).

use serde_json::Value;

/// Everything about RPI 3.58 that is not confirmed on a console yet ([PROVJERA]).
/// Defaults follow DESIGN; changed in code after the console sessions, not a user setting.
#[derive(Debug, Clone, PartialEq)]
pub struct RpiDialect {
    /// Body field for `/api/uninstall_patch` (DESIGN 6.7 table).
    pub uninstall_patch_field: &'static str,
    /// Body field for `/api/uninstall_ac`.
    pub uninstall_ac_field: &'static str,
    /// First method tried for `/api/log`; the other one is the fallback on 404/405.
    pub log_post_first: bool,
}

impl Default for RpiDialect {
    fn default() -> Self {
        Self {
            uninstall_patch_field: "title_id",
            uninstall_ac_field: "content_id",
            log_post_first: true,
        }
    }
}

impl RpiDialect {
    /// The alternative the mock can also speak (`--uninstall-fields alt`, `--log-method get`).
    pub fn alt() -> Self {
        Self {
            uninstall_patch_field: "id",
            uninstall_ac_field: "id",
            log_post_first: false,
        }
    }
}

/// Number as JSON number, decimal string or `"0x…"` hex string.
pub fn num(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
        Value::String(s) => {
            let s = s.trim();
            match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                Some(h) => u64::from_str_radix(h, 16).ok(),
                None => s.parse().ok(),
            }
        }
        _ => None,
    }
}

/// First present key among aliases, parsed tolerantly.
pub fn num_field(obj: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|k| obj.get(k).and_then(num))
}

/// Bool as `true`, `"true"`, `1`, `"1"`, `"yes"`.
pub fn boolish(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_u64().map(|n| n != 0),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// `error` as number (incl. hex string) or text. `None` / 0 / "0" means no error.
pub fn error_text(v: Option<&Value>) -> Option<String> {
    let v = v?;
    match num(v) {
        Some(0) => None,
        Some(n) => Some(format!("0x{n:08X}")),
        None => match v {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Value::Null | Value::String(_) => None,
            other => Some(other.to_string()),
        },
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Exists {
    pub exists: bool,
    pub size: Option<u64>,
    pub version: Option<String>,
}

impl Exists {
    pub fn from_json(v: &Value) -> Option<Exists> {
        Some(Exists {
            exists: v.get("exists").and_then(boolish)?,
            size: num_field(v, &["size"]),
            version: v
                .get("version")
                .and_then(|x| x.as_str())
                .map(str::to_string),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Progress {
    pub transferred: Option<u64>,
    pub total: Option<u64>,
    /// Fork field; preferred for the percentage when present.
    pub percent: Option<f64>,
    pub rest_sec: Option<u64>,
    /// `Some` when the task failed (hex code or text).
    pub error: Option<String>,
}

impl Progress {
    pub fn from_json(v: &Value) -> Progress {
        Progress {
            transferred: num_field(
                v,
                &["transferred_total", "transferred_bytes", "transferred"],
            ),
            total: num_field(v, &["length_total", "total_size", "total_bytes", "length"])
                .filter(|&t| t > 0),
            percent: v
                .get("progress_percent")
                .and_then(|p| p.as_f64().or_else(|| num(p).map(|n| n as f64))),
            rest_sec: num_field(v, &["rest_sec_total", "rest_sec"]),
            error: error_text(v.get("error")),
        }
    }

    /// 0.0–1.0, fork percent first; `fallback_total` = sum of part sizes.
    pub fn fraction(&self, fallback_total: u64) -> Option<f64> {
        if let Some(p) = self.percent {
            return Some((p / 100.0).clamp(0.0, 1.0));
        }
        let total = self.total.unwrap_or(fallback_total);
        let done = self.transferred?;
        (total > 0).then(|| (done as f64 / total as f64).clamp(0.0, 1.0))
    }

    pub fn complete(&self, fallback_total: u64) -> bool {
        self.error.is_none() && self.fraction(fallback_total).is_some_and(|f| f >= 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpi::client::{RpiError, classify};
    use serde_json::json;

    #[test]
    fn tolerant_numbers() {
        assert_eq!(num(&json!(123)), Some(123));
        assert_eq!(num(&json!("123")), Some(123));
        assert_eq!(num(&json!("0x7B")), Some(123));
        assert_eq!(num(&json!(1.0e10)), Some(10_000_000_000));
        assert_eq!(num(&json!(-1)), None);
        assert_eq!(num(&json!("x")), None);
        assert_eq!(boolish(&json!("true")), Some(true));
        assert_eq!(boolish(&json!(0)), Some(false));
        assert_eq!(error_text(Some(&json!(0))), None);
        assert_eq!(
            error_text(Some(&json!("0x80990085"))),
            Some("0x80990085".into())
        );
        assert_eq!(
            error_text(Some(&json!(2157510789u64))),
            Some("0x80990085".into())
        );
        assert_eq!(
            error_text(Some(&json!("no space"))),
            Some("no space".into())
        );
    }

    #[test]
    fn progress_flatz_and_fork() {
        let flatz = json!({"status":"success","length_total":"0x64","transferred_total":"0x32","rest_sec_total":"10","error":0});
        let p = Progress::from_json(&flatz);
        assert_eq!(
            (p.total, p.transferred, p.rest_sec, p.error.clone()),
            (Some(100), Some(50), Some(10), None)
        );
        assert_eq!(p.fraction(0), Some(0.5));
        let fork = json!({"progress_percent": 100, "transferred_bytes": 7});
        let p = Progress::from_json(&fork);
        assert!(p.complete(7));
        let legacy = json!({"transferred": 7});
        assert_eq!(Progress::from_json(&legacy).fraction(14), Some(0.5));
        assert!(
            !Exists::from_json(&json!({"exists":"false"}))
                .unwrap()
                .exists
        );
        assert!(Exists::from_json(&json!({"status":"success"})).is_none());
    }

    /// Raw console answers from Sesija B (2026-11-21), see docs/faza-6-izvjestaj.md.
    #[test]
    fn sesija_b_console_fixtures() {
        let j = |s: &str| serde_json::from_str::<Value>(s).unwrap();

        let no = j(include_str!(
            "../../tests/fixtures/rpi-3.58/sesija-B-2026-11-21/is_exists-not-installed.json"
        ));
        assert_eq!(
            Exists::from_json(&no),
            Some(Exists {
                exists: false,
                size: None,
                version: None
            })
        );
        let yes = j(include_str!(
            "../../tests/fixtures/rpi-3.58/sesija-B-2026-11-21/is_exists-installed.json"
        ));
        assert_eq!(
            Exists::from_json(&yes),
            Some(Exists {
                exists: true,
                size: Some(9_328_275_456),
                version: None
            })
        );
        let upd = j(include_str!(
            "../../tests/fixtures/rpi-3.58/sesija-B-2026-11-21/is_exists-with-update.json"
        ));
        assert_eq!(
            Exists::from_json(&upd),
            Some(Exists {
                exists: true,
                size: Some(11_889_328_128),
                version: None
            })
        );

        let fin = Progress::from_json(&j(include_str!(
            "../../tests/fixtures/rpi-3.58/sesija-B-2026-11-21/progress-finished.json"
        )));
        assert_eq!(
            (fin.transferred, fin.total, fin.percent, fin.error.clone()),
            (Some(9_316_925_440), Some(9_316_925_440), Some(100.0), None)
        );
        assert!(fin.complete(0));
        let mid = Progress::from_json(&j(include_str!(
            "../../tests/fixtures/rpi-3.58/sesija-B-2026-11-21/progress-40.json"
        )));
        assert_eq!(
            (mid.transferred, mid.total, mid.rest_sec),
            (Some(3_772_186_624), Some(9_316_925_440), Some(83))
        );
        assert!((mid.fraction(0).unwrap() - 0.4).abs() < 1e-9);
        assert!(!mid.complete(0));

        // Known gap: the failure carries `error_code` (0x80990019), which classify does not read
        // yet, so only the classification is asserted here. See docs/faza-6-izvjestaj.md.
        let gone = include_str!(
            "../../tests/fixtures/rpi-3.58/sesija-B-2026-11-21/progress-unknown-task.json"
        );
        assert!(matches!(
            classify(200, gone),
            Err(RpiError::Fail { status: 200, .. })
        ));
    }
}
