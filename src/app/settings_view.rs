//! Settings tab (DESIGN 4.5) and the connection test (6.5).

use super::{
    fonts::{icon, size},
    theme::Palette,
    widgets::{self, Kind},
};
use crate::{
    config::Config,
    i18n::I18n,
    net::{self, IpError},
    rpi::{Client, RpiDialect, RpiError, Unreachable},
};
use eframe::egui::{self, Color32, RichText, Ui};
use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mark {
    Ok,
    Warn,
    Err,
    Info,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub mark: Mark,
    pub text: String,
}

/// Result of the console part of the test, for the header indicator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reach {
    Ok(u32),
    Unreachable,
}

pub struct TestResult {
    pub lines: Vec<Line>,
    pub reach: Option<Reach>,
}

pub struct SettingsView {
    pub draft: Draft,
    test_rx: Option<mpsc::Receiver<TestResult>>,
    pub last_test: Option<TestResult>,
    saved_flash: Option<f64>,
}

/// Text fields as typed; validated on every frame (DESIGN 4.5).
#[derive(Clone, PartialEq)]
pub struct Draft {
    pub console_ip: String,
    pub console_port: String,
    pub pc_manual: bool,
    pub pc_ip: String,
    pub server_port: String,
    pub language: String,
    pub splash: bool,
    pub notifications: bool,
}

impl Draft {
    pub fn from(c: &Config) -> Draft {
        Draft {
            console_ip: c.console_ip.clone().unwrap_or_default(),
            console_port: c.console_port.to_string(),
            pc_manual: c.pc_ip_override.is_some(),
            pc_ip: c.pc_ip_override.clone().unwrap_or_default(),
            server_port: c.server_port.to_string(),
            language: c.language.clone(),
            splash: c.splash_enabled,
            notifications: c.notifications_enabled,
        }
    }

    fn port(s: &str) -> Option<u16> {
        s.trim().parse::<u16>().ok().filter(|p| *p > 0)
    }

    /// `None` when something is invalid.
    pub fn to_config(&self, base: &Config) -> Option<Config> {
        let ip = self.console_ip.trim();
        let console_ip = if ip.is_empty() {
            None
        } else {
            Some(net::parse_console_ip(ip).ok()?.to_string())
        };
        let pc = if self.pc_manual {
            Some(self.pc_ip.trim().parse::<Ipv4Addr>().ok()?.to_string())
        } else {
            None
        };
        Some(Config {
            console_ip,
            console_port: Self::port(&self.console_port)?,
            server_port: Self::port(&self.server_port)?,
            pc_ip_override: pc,
            language: self.language.clone(),
            splash_enabled: self.splash,
            notifications_enabled: self.notifications,
            ..base.clone()
        })
    }
}

impl SettingsView {
    pub fn new(c: &Config) -> Self {
        Self {
            draft: Draft::from(c),
            test_rx: None,
            last_test: None,
            saved_flash: None,
        }
    }

    pub fn testing(&self) -> bool {
        self.test_rx.is_some()
    }

    /// Polls a running test; returns the finished result once.
    pub fn poll(&mut self) -> Option<Reach> {
        let rx = self.test_rx.as_ref()?;
        match rx.try_recv() {
            Ok(r) => {
                let reach = r.reach;
                self.last_test = Some(r);
                self.test_rx = None;
                reach
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.test_rx = None;
                None
            }
        }
    }

    pub fn start_test(
        &mut self,
        ctx: &egui::Context,
        cfg: Config,
        server: Option<SocketAddrV4>,
        t: &I18n,
    ) {
        if self.testing() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.test_rx = Some(rx);
        let ctx = ctx.clone();
        let msgs = TestTexts::new(t);
        std::thread::spawn(move || {
            let r = run_test(&cfg, server, &msgs);
            let _ = tx.send(r);
            ctx.request_repaint();
        });
    }

    /// Draws the tab. Returns `Some(config)` when the user saved.
    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        t: &I18n,
        cfg: &Config,
        server: Option<SocketAddrV4>,
        busy: bool,
        data_dir: &std::path::Path,
        fw: &mut super::dialogs::Firewall,
        show_guide: &mut bool,
    ) -> Option<Config> {
        let mut saved = None;
        let now = ui.input(|i| i.time);
        let err = |ui: &mut Ui, msg: &str| {
            ui.label(super::fonts::job(
                icon::WARNING_OCTAGON,
                msg,
                super::fonts::regular(size::CAPTION),
                p.danger,
            ));
        };
        let field = |ui: &mut Ui,
                     label: &str,
                     text: &mut String,
                     width: f32,
                     enabled: bool|
         -> egui::Response {
            ui.add_sized(
                [180.0, 24.0],
                egui::Label::new(RichText::new(label).color(p.text_secondary)),
            );
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(text)
                    .desired_width(width)
                    .margin(egui::vec2(8.0, 6.0)),
            )
        };
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new(t.t("settings.title")).font(super::fonts::semibold(size::TITLE)),
                );
                widgets::space(ui, 3);

                // --- console ---
                widgets::card(ui, p, |ui| {
                    ui.label(
                        RichText::new(t.t("settings.section.console"))
                            .font(super::fonts::semibold(size::HEADING)),
                    );
                    widgets::space(ui, 2);
                    ui.horizontal(|ui| {
                        field(
                            ui,
                            &t.t("settings.console_ip"),
                            &mut self.draft.console_ip,
                            180.0,
                            true,
                        );
                        ui.label(
                            RichText::new(t.t("settings.console_ip.hint"))
                                .color(p.text_secondary)
                                .size(size::CAPTION),
                        );
                    });
                    match net::parse_console_ip(&self.draft.console_ip) {
                        Err(IpError::Hostname) => err(ui, &t.t("settings.err.hostname")),
                        Err(IpError::Invalid) => err(ui, &t.t("settings.err.ip")),
                        Err(IpError::NotUnicast) => err(ui, &t.t("settings.err.ip_unicast")),
                        _ => {}
                    }
                    ui.horizontal(|ui| {
                        field(
                            ui,
                            &t.t("settings.console_port"),
                            &mut self.draft.console_port,
                            80.0,
                            true,
                        );
                        ui.label(
                            RichText::new(t.t("settings.console_port.hint"))
                                .color(p.text_secondary)
                                .size(size::CAPTION),
                        );
                    });
                    if Draft::port(&self.draft.console_port).is_none() {
                        err(ui, &t.t("settings.err.port"));
                    }
                    widgets::space(ui, 2);
                    let can_test = !self.testing() && self.draft.to_config(cfg).is_some();
                    let label = if self.testing() {
                        t.t("test.running")
                    } else {
                        format!("{}  (F5)", t.t("settings.test"))
                    };
                    let f5 = ui.input(|i| i.key_pressed(egui::Key::F5));
                    if (widgets::button_enabled(
                        ui,
                        p,
                        Kind::Secondary,
                        Some(icon::PLUG),
                        &label,
                        can_test,
                    )
                    .clicked()
                        || (f5 && can_test))
                        && let Some(c) = self.draft.to_config(cfg)
                    {
                        self.start_test(ui.ctx(), c, server, t);
                    }
                    if self.testing() {
                        widgets::progress(ui, p, None, 240.0, ui.scope_id().with("test-ind"));
                    }
                    if let Some(r) = &self.last_test {
                        widgets::space(ui, 2);
                        egui::Frame::new()
                            .fill(p.bg_surface2)
                            .corner_radius(8)
                            .inner_margin(12)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                for l in &r.lines {
                                    let (ic, c) = match l.mark {
                                        Mark::Ok => (icon::CHECK, p.success),
                                        Mark::Warn => (icon::WARNING, p.warning),
                                        Mark::Err => (icon::WARNING_OCTAGON, p.danger),
                                        Mark::Info => (icon::INFO, p.info),
                                    };
                                    ui.label(super::fonts::job(
                                        ic,
                                        &l.text,
                                        super::fonts::regular(size::BODY),
                                        if l.mark == Mark::Info {
                                            p.text_secondary
                                        } else {
                                            c
                                        },
                                    ));
                                }
                            });
                    }
                });
                widgets::space(ui, 3);

                // --- this computer ---
                widgets::card(ui, p, |ui| {
                    ui.label(
                        RichText::new(t.t("settings.section.pc"))
                            .font(super::fonts::semibold(size::HEADING)),
                    );
                    widgets::space(ui, 2);
                    let auto = net::parse_console_ip(&self.draft.console_ip)
                        .ok()
                        .and_then(|ip| {
                            net::pc_ip_for(
                                ip,
                                Draft::port(&self.draft.console_port).unwrap_or(12801),
                            )
                            .ok()
                        });
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [180.0, 24.0],
                            egui::Label::new(
                                RichText::new(t.t("settings.pc_ip")).color(p.text_secondary),
                            ),
                        );
                        let auto_text = match auto {
                            Some(ip) => t.t_args("settings.pc_ip.auto", &[("ip", &ip.to_string())]),
                            None => t.t("settings.pc_ip.auto_unknown"),
                        };
                        ui.radio_value(&mut self.draft.pc_manual, false, auto_text);
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(184.0);
                        ui.radio_value(
                            &mut self.draft.pc_manual,
                            true,
                            t.t("settings.pc_ip.manual"),
                        );
                        ui.add_enabled(
                            self.draft.pc_manual,
                            egui::TextEdit::singleline(&mut self.draft.pc_ip).desired_width(160.0),
                        );
                    });
                    if self.draft.pc_manual && self.draft.pc_ip.trim().parse::<Ipv4Addr>().is_err()
                    {
                        err(ui, &t.t("settings.err.ip"));
                    }
                    ui.horizontal(|ui| {
                        field(
                            ui,
                            &t.t("settings.server_port"),
                            &mut self.draft.server_port,
                            80.0,
                            !busy,
                        );
                        if busy {
                            ui.label(
                                RichText::new(t.t("settings.busy"))
                                    .color(p.text_secondary)
                                    .size(size::CAPTION),
                            );
                        }
                    });
                    match Draft::port(&self.draft.server_port) {
                        None => err(ui, &t.t("settings.err.port")),
                        Some(p2) if p2 < 1024 => {
                            ui.label(super::fonts::job(
                                icon::WARNING,
                                &t.t("settings.warn.low_port"),
                                super::fonts::regular(size::CAPTION),
                                p.warning,
                            ));
                        }
                        _ => {}
                    }
                    // firewall rule for the server port (DESIGN 4.5, 9.8)
                    let port = Draft::port(&self.draft.server_port).unwrap_or(cfg.server_port);
                    if fw.row(ui, p, t, port, false) {
                        *show_guide = true;
                    }
                });
                widgets::space(ui, 3);

                // --- application ---
                widgets::card(ui, p, |ui| {
                    ui.label(
                        RichText::new(t.t("settings.section.app"))
                            .font(super::fonts::semibold(size::HEADING)),
                    );
                    widgets::space(ui, 2);
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [180.0, 24.0],
                            egui::Label::new(
                                RichText::new(t.t("settings.language")).color(p.text_secondary),
                            ),
                        );
                        egui::ComboBox::from_id_salt("lang")
                            .selected_text(t.t(&format!("lang.{}", self.draft.language)))
                            .show_ui(ui, |ui| {
                                for l in ["en", "bs"] {
                                    ui.selectable_value(
                                        &mut self.draft.language,
                                        l.to_string(),
                                        t.t(&format!("lang.{l}")),
                                    );
                                }
                            });
                    });
                    ui.checkbox(&mut self.draft.splash, t.t("settings.splash"));
                    ui.checkbox(&mut self.draft.notifications, t.t("settings.notifications"));
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [180.0, 24.0],
                            egui::Label::new(
                                RichText::new(t.t("settings.data_folder")).color(p.text_secondary),
                            ),
                        );
                        ui.label(
                            RichText::new(r"%APPDATA%\PS Package Pusher")
                                .font(super::fonts::mono(size::CAPTION)),
                        );
                        if widgets::button(
                            ui,
                            p,
                            Kind::Ghost,
                            Some(icon::FOLDER_OPEN),
                            &t.t("settings.open_folder"),
                        )
                        .clicked()
                        {
                            let _ = std::fs::create_dir_all(data_dir);
                            let _ = std::process::Command::new("explorer").arg(data_dir).spawn();
                        }
                    });
                });
                widgets::space(ui, 3);

                let candidate = self.draft.to_config(cfg);
                let changed = candidate.as_ref().is_some_and(|c| c != cfg);
                ui.horizontal(|ui| {
                    if widgets::button(
                        ui,
                        p,
                        Kind::Secondary,
                        Some(icon::UNDO),
                        &t.t("settings.defaults"),
                    )
                    .clicked()
                    {
                        let mut d = Draft::from(&Config::default());
                        d.language = self.draft.language.clone();
                        self.draft = d;
                    }
                    if widgets::button_enabled(
                        ui,
                        p,
                        Kind::Primary,
                        Some(icon::SAVE),
                        &t.t("settings.save"),
                        changed,
                    )
                    .clicked()
                    {
                        saved = candidate.clone();
                        self.saved_flash = Some(now);
                    }
                    if self.saved_flash.is_some_and(|s| now - s < 3.0) {
                        ui.label(super::fonts::job(
                            icon::CHECK,
                            &t.t("settings.saved"),
                            super::fonts::regular(size::BODY),
                            p.success,
                        ));
                        ui.ctx().request_repaint_after(Duration::from_millis(500));
                    }
                });
                widgets::space(ui, 4);
                // ui.hyperlink does nothing here: egui-winit's `webbrowser` feature is off
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(
                        RichText::new(format!(
                            "PS Package Pusher {} · ",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .size(size::CAPTION)
                        .color(p.text_secondary),
                    );
                    if ui
                        .link(RichText::new("GitHub").size(size::CAPTION))
                        .clicked()
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg("https://github.com/pinkyBOZZ/PS-Package-Pusher/releases")
                            .spawn();
                    }
                });
                widgets::space(ui, 4);
            });
        saved
    }
}

/// Pre-translated strings so the test thread does not need the i18n tables.
struct TestTexts {
    lang: crate::i18n::Lang,
}

impl TestTexts {
    fn new(t: &I18n) -> Self {
        Self { lang: t.lang }
    }
    fn t(&self, k: &str, a: &[(&str, &str)]) -> String {
        I18n::new(self.lang).t_args(k, a)
    }
    fn code(&self, code: &str, a: &[(&str, &str)]) -> String {
        format!("{code}: {}", self.t(&format!("error.{code}.what"), a))
    }
}

fn run_test(cfg: &Config, server: Option<SocketAddrV4>, m: &TestTexts) -> TestResult {
    let mut lines = Vec::new();
    let mut push = |mark, text: String| lines.push(Line { mark, text });
    let Some(ip) = cfg.console_ip() else {
        push(Mark::Err, m.code("CFG-01", &[]));
        return TestResult { lines, reach: None };
    };
    push(Mark::Ok, m.t("test.ip_ok", &[]));
    let port = cfg.console_port;
    let (ips, ports) = (ip.to_string(), port.to_string());
    // 2: TCP connect
    let t0 = Instant::now();
    match TcpStream::connect_timeout(&SocketAddrV4::new(ip, port).into(), Duration::from_secs(3)) {
        Ok(_) => push(
            Mark::Ok,
            m.t(
                "test.reachable",
                &[("ms", &t0.elapsed().as_millis().to_string())],
            ),
        ),
        Err(e) => {
            let code = if e.kind() == std::io::ErrorKind::ConnectionRefused {
                "NET-02"
            } else {
                "NET-01"
            };
            push(Mark::Err, m.code(code, &[("ip", &ips), ("port", &ports)]));
            return TestResult {
                lines,
                reach: Some(Reach::Unreachable),
            };
        }
    }
    let ms = t0.elapsed().as_millis() as u32;
    // 3: RPI API
    let c = Client::new(ip, port, RpiDialect::default());
    let reach = match c.is_exists("CUSA00000").0 {
        Ok(_) | Err(RpiError::Fail { .. }) => {
            push(
                Mark::Ok,
                m.t("test.rpi_ok", &[("ip", &ips), ("port", &ports)]),
            );
            Some(Reach::Ok(ms))
        }
        Err(e) => {
            let code = match e {
                RpiError::Forbidden => "NET-03",
                RpiError::EndpointMissing => "NET-05",
                RpiError::Unreachable(Unreachable::Refused) => "NET-02",
                RpiError::Unreachable(_) => "NET-01",
                RpiError::BadResponse { .. } | RpiError::Fail { .. } => "NET-04",
            };
            push(Mark::Err, m.code(code, &[("ip", &ips), ("port", &ports)]));
            return TestResult {
                lines,
                reach: Some(Reach::Unreachable),
            };
        }
    };
    // 4: /api/log
    match c.console_log() {
        Ok(_) => push(Mark::Ok, m.t("test.log_ok", &[])),
        Err(_) => push(Mark::Warn, m.t("test.log_old", &[])),
    }
    // 5: PC IP
    let pc = cfg
        .pc_ip_override()
        .map_or_else(|| net::pc_ip_for(ip, port), Ok);
    match pc {
        Ok(pc) => {
            push(Mark::Ok, m.t("test.pc_ip", &[("ip", &pc.to_string())]));
            if !pc.is_loopback() && !net::same_24(pc, ip) {
                push(
                    Mark::Warn,
                    m.t(
                        "test.subnet_warn",
                        &[("pc", &pc.to_string()), ("console", &ips)],
                    ),
                );
            }
            // 6: local server
            match server {
                Some(a) => push(
                    Mark::Ok,
                    m.t("test.server_running", &[("addr", &a.to_string())]),
                ),
                None => match TcpListener::bind((pc, cfg.server_port)) {
                    Ok(_) => push(
                        Mark::Ok,
                        m.t(
                            "test.server_port_free",
                            &[("port", &cfg.server_port.to_string())],
                        ),
                    ),
                    Err(_) => push(
                        Mark::Err,
                        m.code("SRV-01", &[("port", &cfg.server_port.to_string())]),
                    ),
                },
            }
        }
        Err(_) => push(Mark::Err, m.code("NET-06", &[])),
    }
    push(Mark::Info, m.t("test.firewall", &[]));
    TestResult { lines, reach }
}

/// Colour used by the header indicator.
pub fn reach_color(p: &Palette, r: Option<Reach>, configured: bool) -> Color32 {
    match (configured, r) {
        (false, _) => p.text_secondary,
        (_, Some(Reach::Ok(_))) => p.success,
        (_, Some(Reach::Unreachable)) => p.danger,
        _ => p.text_secondary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{Mock, MockConfig};

    fn cfg_for(m: &Mock) -> Config {
        Config {
            console_ip: Some("127.0.0.1".into()),
            console_port: m.addr.port(),
            server_port: 0,
            ..Default::default()
        }
    }

    fn marks(r: &TestResult) -> Vec<Mark> {
        r.lines.iter().map(|l| l.mark).collect()
    }

    /// PLAN 2 acceptance: test lines for normal, 403, legacy and stopped mock.
    #[test]
    fn connection_test_against_mock() {
        let tt = TestTexts {
            lang: crate::i18n::Lang::En,
        };
        let any = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
        let m = Mock::start(any, MockConfig::default()).unwrap();
        let r = run_test(&cfg_for(&m), None, &tt);
        assert_eq!(r.reach.map(|x| matches!(x, Reach::Ok(_))), Some(true));
        assert!(
            !marks(&r).contains(&Mark::Err),
            "{:?}",
            r.lines.iter().map(|l| &l.text).collect::<Vec<_>>()
        );
        m.stop();

        let m = Mock::start(
            any,
            MockConfig {
                reject_all_403: true,
                ..Default::default()
            },
        )
        .unwrap();
        let r = run_test(&cfg_for(&m), None, &tt);
        assert!(r.lines.iter().any(|l| l.text.starts_with("NET-03")));
        m.stop();

        let m = Mock::start(
            any,
            MockConfig {
                legacy: true,
                ..Default::default()
            },
        )
        .unwrap();
        let r = run_test(&cfg_for(&m), None, &tt);
        assert!(marks(&r).contains(&Mark::Warn) && !marks(&r).contains(&Mark::Err));
        m.stop();

        let free = TcpListener::bind(any).unwrap().local_addr().unwrap().port();
        let c = Config {
            console_ip: Some("127.0.0.1".into()),
            console_port: free,
            ..Default::default()
        };
        let r = run_test(&c, None, &tt);
        assert_eq!(r.reach, Some(Reach::Unreachable));
        assert!(r.lines.iter().any(|l| l.text.starts_with("NET-0")));
    }

    #[test]
    fn draft_validation() {
        let base = Config::default();
        let mut d = Draft::from(&base);
        assert_eq!(d.to_config(&base), Some(base.clone()));
        d.console_ip = "ps4.local".into();
        assert!(d.to_config(&base).is_none());
        d.console_ip = "192.168.1.50".into();
        d.server_port = "0".into();
        assert!(d.to_config(&base).is_none());
        d.server_port = "9000".into();
        assert_eq!(d.to_config(&base).unwrap().server_port, 9000);
    }
}
