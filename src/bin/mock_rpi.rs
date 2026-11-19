//! Mock RPI 3.58 for development without a console (DESIGN 8). Not shipped in the release zip.
//! Example: mock-rpi --bind 127.0.0.1:12801 --installed CUSA00001 --speed 40MB --latency 50ms

use pspp::mock::{ExistsFormat, Mock, MockConfig, NumFormat, OriginCheck};
use std::{net::SocketAddrV4, time::Duration};

const HELP: &str = "mock-rpi [--bind ip:port] [--installed CUSA1,CUSA2] [--speed 40MB] [--latency 50ms]
  [--fail-install msg] [--fail-task-at 30%] [--error-code 0x80990085] [--stall-after 100MB]
  [--no-fetch] [--legacy] [--reject-all-403] [--exists-format bool|string|int] [--log-method post|get]
  [--uninstall-fields design|alt] [--notfound-style json|empty] [--origin-check destructive|all|none]
  [--num-format number|dec|hexstr] [--chunk 8MB] [--stall-timeout 30s]";

fn bytes(s: &str) -> u64 {
    let s = s.trim().to_ascii_uppercase();
    let (n, mul) = [
        ("GB", 1u64 << 30),
        ("MB", 1 << 20),
        ("KB", 1 << 10),
        ("B", 1),
    ]
    .iter()
    .find_map(|(u, m)| s.strip_suffix(u).map(|n| (n.trim().to_string(), *m)))
    .unwrap_or((s.clone(), 1));
    (n.parse::<f64>()
        .unwrap_or_else(|_| fail(&format!("bad size {s}")))
        * mul as f64) as u64
}

fn dur(s: &str) -> Duration {
    let ms = s.strip_suffix("ms").map(|n| n.parse::<u64>().ok());
    let sec = s
        .strip_suffix('s')
        .map(|n| n.parse::<u64>().ok().map(|v| v * 1000));
    Duration::from_millis(
        ms.or(sec)
            .flatten()
            .unwrap_or_else(|| fail(&format!("bad duration {s}"))),
    )
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}\n{HELP}");
    std::process::exit(2)
}

fn main() {
    let mut cfg = MockConfig::default();
    let mut bind: SocketAddrV4 = "127.0.0.1:12801".parse().expect("default bind");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || {
            args.next()
                .unwrap_or_else(|| fail(&format!("{a} needs a value")))
        };
        match a.as_str() {
            "--bind" => bind = val().parse().unwrap_or_else(|_| fail("bad --bind")),
            "--installed" => {
                cfg.installed = val()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
            "--speed" => cfg.speed_bps = Some(bytes(&val())),
            "--latency" => cfg.latency = dur(&val()),
            "--fail-install" => cfg.fail_install = Some(val()),
            "--fail-task-at" => {
                cfg.fail_task_at = Some(
                    val()
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .unwrap_or_else(|_| fail("bad %"))
                        / 100.0,
                )
            }
            "--error-code" => {
                let v = val();
                cfg.error_code = u32::from_str_radix(v.trim_start_matches("0x"), 16)
                    .unwrap_or_else(|_| fail("bad code"));
            }
            "--stall-after" => cfg.stall_after = Some(bytes(&val())),
            "--no-fetch" => cfg.no_fetch = true,
            "--legacy" => cfg.legacy = true,
            "--reject-all-403" => cfg.reject_all_403 = true,
            "--exists-format" => {
                cfg.exists_format = match val().as_str() {
                    "bool" => ExistsFormat::Bool,
                    "string" => ExistsFormat::Str,
                    "int" => ExistsFormat::Int,
                    _ => fail("bad --exists-format"),
                }
            }
            "--log-method" => cfg.log_post = val() == "post",
            "--uninstall-fields" => cfg.uninstall_alt = val() == "alt",
            "--notfound-style" => cfg.notfound_json = val() == "json",
            "--origin-check" => {
                cfg.origin_check = match val().as_str() {
                    "destructive" => OriginCheck::Destructive,
                    "all" => OriginCheck::All,
                    "none" => OriginCheck::None,
                    _ => fail("bad --origin-check"),
                }
            }
            "--num-format" => {
                cfg.num_format = match val().as_str() {
                    "number" => NumFormat::Number,
                    "dec" => NumFormat::Dec,
                    "hexstr" => NumFormat::HexStr,
                    _ => fail("bad --num-format"),
                }
            }
            "--chunk" => cfg.chunk = bytes(&val()).max(1),
            "--stall-timeout" => cfg.stall_timeout = dur(&val()),
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            other => fail(&format!("unknown flag {other}")),
        }
    }
    let m = Mock::start(bind, cfg).unwrap_or_else(|e| fail(&format!("bind {bind}: {e}")));
    println!("mock RPI listening on http://{}", m.addr);
    loop {
        std::thread::park();
    }
}
