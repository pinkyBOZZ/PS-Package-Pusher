//! Windows Firewall rule for the local server (DESIGN 9.8, spike S-5).
//!
//! The GUI process never runs elevated: "Allow in firewall" starts this same exe through UAC with
//! `--firewall add <port>` / `--firewall remove`, which runs `netsh` (no shell) and exits.

use serde_json::Value;
use std::{path::PathBuf, process::Command};

pub const RULE: &str = "PS Package Pusher";
const ERROR_CANCELLED: u32 = 1223;

fn exe() -> Option<String> {
    // netsh rejects 8.3 short paths (S-5): use the long canonical path without the \\?\ prefix
    let p = std::fs::canonicalize(std::env::current_exe().ok()?).ok()?;
    Some(
        p.display()
            .to_string()
            .trim_start_matches(r"\\?\")
            .to_string(),
    )
}

fn result_file() -> PathBuf {
    std::env::temp_dir().join("pspp-firewall.txt")
}

fn netsh(args: &[String]) -> (i32, String) {
    match Command::new("netsh")
        .args(["advfirewall", "firewall"])
        .args(args)
        .output()
    {
        Ok(o) => (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
        ),
        Err(e) => (-1, e.to_string()),
    }
}

/// Port is the only parameter of the elevated step and must be a plain number (DESIGN 11).
pub fn parse_port(s: &str) -> Option<u16> {
    s.parse::<u16>().ok().filter(|p| *p >= 1024)
}

/// Elevated side. Returns the process exit code; `None` if the args are not ours.
pub fn run_cli(args: &[String]) -> Option<i32> {
    if args.get(1).map(String::as_str) != Some("--firewall") {
        return None;
    }
    let exe = exe()?;
    let name = format!("name={RULE}");
    let (code, out) = match (
        args.get(2).map(String::as_str),
        args.get(3).and_then(|p| parse_port(p)),
    ) {
        (Some("add"), Some(port)) => {
            netsh(&["delete".into(), "rule".into(), name.clone()]); // exit 1 = nothing to delete, fine (S-5)
            netsh(&[
                "add".into(),
                "rule".into(),
                name,
                "dir=in".into(),
                "action=allow".into(),
                "protocol=TCP".into(),
                format!("localport={port}"),
                "profile=private".into(),
                "remoteip=localsubnet".into(),
                format!("program={exe}"),
            ])
        }
        (Some("remove"), _) => {
            let (c, o) = netsh(&["delete".into(), "rule".into(), name]);
            (if c == 1 { 0 } else { c }, o)
        }
        _ => (
            2,
            "usage: --firewall add <port 1024-65535> | --firewall remove".into(),
        ),
    };
    let _ = std::fs::write(result_file(), &out);
    Some(code)
}

#[derive(Debug, Clone, PartialEq)]
pub enum FwError {
    /// UAC declined (FW-02).
    Cancelled,
    /// netsh failed (FW-03) with its output.
    Failed(String),
}

/// GUI side: run `--firewall <op>` elevated and wait. Blocking — call from a worker thread.
pub fn request(add_port: Option<u16>) -> Result<(), FwError> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError},
        System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject},
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_HIDE,
        },
    };
    let exe = exe().ok_or_else(|| FwError::Failed("cannot resolve exe path".into()))?;
    let params = match add_port {
        Some(p) => format!("--firewall add {p}"),
        None => "--firewall remove".into(),
    };
    let w = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (verb, file, params) = (w("runas"), w(&exe), w(&params));
    // SAFETY: zeroed SHELLEXECUTEINFOW is a valid "empty" struct; all pointers outlive the call.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = SW_HIDE;
    let _ = std::fs::remove_file(result_file());
    // SAFETY: see above.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let e = unsafe { GetLastError() };
        return Err(if e == ERROR_CANCELLED {
            FwError::Cancelled
        } else {
            FwError::Failed(format!("ShellExecuteExW error {e}"))
        });
    }
    let mut code = 0u32;
    // SAFETY: hProcess is valid because SEE_MASK_NOCLOSEPROCESS was set and the call succeeded.
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
    }
    if code == 0 {
        Ok(())
    } else {
        Err(FwError::Failed(
            std::fs::read_to_string(result_file()).unwrap_or_else(|_| format!("exit code {code}")),
        ))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// Our rule exists: (port, enabled).
    pub rule: Option<(u16, bool)>,
    /// `AllowLocalFirewallRules` on all profiles (false → FW-04).
    pub local_rules_allowed: bool,
    /// Windows shows its own "allow access" prompt (false on managed PCs, S-5).
    pub notify_on_listen: bool,
}

/// Reads the rule and policy via PowerShell (locale-independent JSON, ~2–3 s, worker thread only).
pub fn status() -> Result<Status, String> {
    let ps = format!(
        "$r = Get-NetFirewallRule -DisplayName '{RULE}' -ErrorAction SilentlyContinue | Select-Object -First 1; \
         $port = if ($r) {{ ($r | Get-NetFirewallPortFilter).LocalPort }} else {{ $null }}; \
         $p = Get-NetFirewallProfile -PolicyStore ActiveStore; \
         [pscustomobject]@{{ exists = [bool]$r; enabled = if ($r) {{ [string]$r.Enabled }} else {{ '' }}; port = [string]$port; \
           local = -not ($p | Where-Object {{ -not $_.AllowLocalFirewallRules }}); notify = [bool]($p | Where-Object {{ $_.NotifyOnListen }}) }} | ConvertTo-Json -Compress"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .output()
        .map_err(|e| e.to_string())?;
    parse_status(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_status(json: &str) -> Result<Status, String> {
    let v: Value =
        serde_json::from_str(json.trim()).map_err(|e| format!("{e}: {}", json.trim()))?;
    let rule = v["exists"].as_bool().unwrap_or(false).then(|| {
        let port = v["port"].as_str().and_then(|p| p.parse().ok()).unwrap_or(0);
        let enabled = matches!(v["enabled"].as_str(), Some("True" | "1" | "true"));
        (port, enabled)
    });
    Ok(Status {
        rule,
        local_rules_allowed: v["local"].as_bool().unwrap_or(true),
        notify_on_listen: v["notify"].as_bool().unwrap_or(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevated_step_only_takes_a_numeric_port() {
        assert_eq!(parse_port("8095"), Some(8095));
        assert_eq!(parse_port("80"), None);
        assert_eq!(parse_port("8095 & calc"), None);
        assert_eq!(parse_port("-1"), None);
        assert!(run_cli(&["x".into(), "--other".into()]).is_none());
    }

    #[test]
    fn status_json() {
        let s = parse_status(
            r#"{"exists":true,"enabled":"True","port":"8095","local":true,"notify":false}"#,
        )
        .unwrap();
        assert_eq!(
            s,
            Status {
                rule: Some((8095, true)),
                local_rules_allowed: true,
                notify_on_listen: false
            }
        );
        let s =
            parse_status(r#"{"exists":false,"enabled":"","port":"","local":false,"notify":true}"#)
                .unwrap();
        assert_eq!((s.rule, s.local_rules_allowed), (None, false));
        assert!(parse_status("garbage").is_err());
    }
}
