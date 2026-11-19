//! Config, load/save, schema migration (DESIGN 7.5, defaults 6.6).

use crate::{net, storage};
use serde::{Deserialize, Serialize};
use std::{net::Ipv4Addr, path::Path};

pub const SCHEMA: u32 = 1;
pub const FILE: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema: u32,
    /// Stored as a string so an invalid value can be dropped with a warning instead of failing the load.
    pub console_ip: Option<String>,
    pub console_port: u16,
    pub server_port: u16,
    pub pc_ip_override: Option<String>,
    pub language: String,
    pub splash_enabled: bool,
    pub firewall_notice_shown: bool,
    pub window: Option<WindowState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub maximized: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            console_ip: None,
            console_port: 12801,
            server_port: 8095,
            pc_ip_override: None,
            language: "en".into(),
            splash_enabled: true,
            firewall_notice_shown: false,
            window: None,
        }
    }
}

impl Config {
    pub fn console_ip(&self) -> Option<Ipv4Addr> {
        self.console_ip
            .as_deref()
            .and_then(|s| net::parse_console_ip(s).ok())
    }

    pub fn pc_ip_override(&self) -> Option<Ipv4Addr> {
        self.pc_ip_override
            .as_deref()
            .and_then(|s| s.trim().parse().ok())
    }
}

#[derive(Debug, PartialEq)]
pub enum LoadIssue {
    /// File was corrupt; renamed and defaults used (CFG-02).
    Broken(std::path::PathBuf),
    /// Stored console IP was not a valid IPv4 literal; cleared.
    InvalidConsoleIp(String),
    Io(String),
}

/// Loads `config.json` from `dir`. Never fails: problems are returned as issues to log.
pub fn load(dir: &Path) -> (Config, Vec<LoadIssue>) {
    let mut issues = Vec::new();
    let mut cfg = match storage::read_json::<Config>(&dir.join(FILE)) {
        Ok(storage::Loaded::Ok(c)) => c,
        Ok(storage::Loaded::Missing) => Config::default(),
        Ok(storage::Loaded::Broken(p)) => {
            issues.push(LoadIssue::Broken(p));
            Config::default()
        }
        Err(e) => {
            issues.push(LoadIssue::Io(e.to_string()));
            Config::default()
        }
    };
    if let Some(ip) = cfg.console_ip.clone()
        && net::parse_console_ip(&ip).is_err()
    {
        issues.push(LoadIssue::InvalidConsoleIp(ip));
        cfg.console_ip = None;
    }
    if !matches!(cfg.language.as_str(), "en" | "bs") {
        cfg.language = "en".into();
    }
    cfg.schema = SCHEMA;
    (cfg, issues)
}

pub fn save(dir: &Path, cfg: &Config) -> std::io::Result<()> {
    storage::write_json(&dir.join(FILE), cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_partial_and_invalid() {
        let d = std::env::temp_dir().join(format!("pspp-test-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (c, i) = load(&d);
        assert_eq!(c, Config::default());
        assert!(i.is_empty());

        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join(FILE),
            r#"{"console_ip":"ps4.local","server_port":9000,"unknown":1}"#,
        )
        .unwrap();
        let (c, i) = load(&d);
        assert_eq!(c.server_port, 9000);
        assert_eq!(c.console_port, 12801);
        assert_eq!(c.console_ip, None);
        assert_eq!(i, vec![LoadIssue::InvalidConsoleIp("ps4.local".into())]);

        let c = Config {
            console_ip: Some("192.168.1.50".into()),
            ..Default::default()
        };
        save(&d, &c).unwrap();
        let (c2, i) = load(&d);
        assert_eq!(c2.console_ip(), Some(Ipv4Addr::new(192, 168, 1, 50)));
        assert!(i.is_empty());
        std::fs::remove_dir_all(d).unwrap();
    }
}
