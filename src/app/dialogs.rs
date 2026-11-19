//! Dialogs 4.7 (already installed), 4.8 (error), 4.9 (firewall), 4.11/4.12 (uninstall).

use super::{
    fonts::{self, icon, size},
    theme::Palette,
    widgets::{self, Kind as BKind},
};
use crate::{
    engine::{
        commands::{Command, UninstallTarget},
        queue::{Decision, Item},
    },
    errors::{Code, describe},
    firewall::{self, FwError, Status},
    i18n::I18n,
    pkg::Kind,
    rpi::UninstallKind,
};
use eframe::egui::{self, Align, Layout, RichText, Ui};
use std::sync::mpsc::{self, Receiver, Sender};

/// Modal with `motion.dialog.in`: content and backdrop fade in over 180 ms (DESIGN 2.6).
/// A dialog counts as newly opened when it was not shown in the previous frame.
pub fn modal(ctx: &egui::Context, p: &Palette, id: &str, width: f32, add: impl FnOnce(&mut Ui)) {
    let key = egui::Id::unique(("modal-open", id));
    let (now, frame) = (ctx.input(|i| i.time), ctx.cumulative_pass_nr());
    let start = ctx
        .data(|d| d.get_temp::<(f64, u64)>(key))
        .filter(|(_, last)| last + 1 >= frame)
        .map_or(now, |(s, _)| s);
    ctx.data_mut(|d| d.insert_temp(key, (start, frame)));
    let k = super::anim::progress(
        ctx,
        start,
        super::anim::DIALOG_IN,
        egui::emath::easing::cubic_out,
    )
    .unwrap_or(1.0);
    egui::Modal::new(egui::Id::unique(id))
        .frame(
            egui::Frame::new()
                .fill(p.bg_surface)
                .corner_radius(12)
                .inner_margin(24),
        )
        .backdrop_color(p.overlay.gamma_multiply(k))
        .show(ctx, |ui| {
            ui.multiply_opacity(k);
            ui.set_width(width);
            add(ui);
        });
}

fn title(ui: &mut Ui, ic: &str, text: &str, color: egui::Color32) {
    ui.label(fonts::job(ic, text, fonts::semibold(size::TITLE), color));
    widgets::space(ui, 2);
}

fn item_header(ui: &mut Ui, p: &Palette, it: &Item, tex: Option<&egui::TextureHandle>) {
    ui.horizontal(|ui| {
        match tex {
            Some(t) => {
                ui.add(
                    egui::Image::new(t)
                        .fit_to_exact_size(egui::vec2(48.0, 48.0))
                        .corner_radius(6),
                );
            }
            None => {
                ui.label(fonts::icon_rt(icon::PACKAGE, 40.0).color(p.text_secondary));
            }
        }
        ui.vertical(|ui| {
            ui.label(
                RichText::new(it.title.clone().unwrap_or_default())
                    .font(fonts::semibold(size::HEADING)),
            );
            let mut sub = it.title_id.clone();
            if let Some(v) = &it.version {
                sub += &format!(" · v{v}");
            }
            ui.label(
                RichText::new(sub)
                    .color(p.text_secondary)
                    .size(size::CAPTION),
            );
        });
    });
    widgets::space(ui, 2);
}

pub fn target_for(it: &Item) -> UninstallTarget {
    let (kind, id) = match it.kind() {
        Kind::Update => (UninstallKind::Patch, it.title_id.clone()),
        Kind::Dlc => (UninstallKind::Dlc, it.content_id.clone()),
        _ => (UninstallKind::Game, it.title_id.clone()),
    };
    UninstallTarget {
        kind,
        id,
        item: Some(it.id.clone()),
    }
}

/// DESIGN 4.12 validation: Title ID `AAAA00000`, Content ID `XX0000-AAAA00000_00-XXXXXXXXXXXXXXXX`.
pub fn valid_id(kind: UninstallKind, s: &str) -> bool {
    let b = s.as_bytes();
    let title = |b: &[u8]| {
        b.len() == 9
            && b[..4].iter().all(u8::is_ascii_uppercase)
            && b[4..].iter().all(u8::is_ascii_digit)
    };
    match kind {
        UninstallKind::Game | UninstallKind::Patch => title(b),
        UninstallKind::Dlc => {
            b.len() == 36
                && b[..2].iter().all(u8::is_ascii_uppercase)
                && b[2..6].iter().all(u8::is_ascii_digit)
                && b[6] == b'-'
                && title(&b[7..16])
                && b[16] == b'_'
                && b[17..19].iter().all(u8::is_ascii_digit)
                && b[19] == b'-'
                && b[20..]
                    .iter()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        }
    }
}

pub struct ById {
    pub kind: UninstallKind,
    pub text: String,
    pub result: Option<Result<bool, String>>,
    pub checking: bool,
}

#[derive(Default)]
pub struct Dialogs {
    pub decision: Option<String>,
    apply_all: bool,
    pub error: Option<String>,
    show_tech: bool,
    show_log: bool,
    pub uninstall: Option<UninstallTarget>,
    pub by_id: Option<ById>,
    pub firewall: bool,
    dont_show: bool,
}

enum FwMsg {
    Status(Result<Status, String>),
    Done(Result<bool, FwError>),
}

/// Firewall rule state for Settings and dialog 4.9 (DESIGN 9.8). All work happens on threads.
#[derive(Default)]
pub struct Firewall {
    pub status: Option<Result<Status, String>>,
    rx: Option<Receiver<FwMsg>>,
    pub busy: bool,
    /// Last action outcome for a short message: Ok(added) or the error.
    pub last: Option<Result<bool, FwError>>,
}

impl Firewall {
    pub fn refresh(&mut self, ctx: &egui::Context) {
        if self.busy {
            return;
        }
        self.busy = true;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(FwMsg::Status(firewall::status()));
            ctx.request_repaint();
        });
    }

    /// `Some(port)` = add/update the rule, `None` = remove. UAC is shown by Windows.
    pub fn apply(&mut self, ctx: &egui::Context, port: Option<u16>) {
        if self.busy {
            return;
        }
        self.busy = true;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(FwMsg::Done(firewall::request(port).map(|_| port.is_some())));
            let _ = tx.send(FwMsg::Status(firewall::status()));
            ctx.request_repaint();
        });
    }

    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        while let Ok(m) = rx.try_recv() {
            match m {
                FwMsg::Done(r) => self.last = Some(r),
                FwMsg::Status(s) => {
                    self.status = Some(s);
                    self.busy = false;
                    self.rx = None;
                    return;
                }
            }
        }
    }

    /// Settings row (DESIGN 4.5): state + action buttons. Returns true when "Guide" was clicked.
    pub fn row(&mut self, ui: &mut Ui, p: &Palette, t: &I18n, port: u16, in_guide: bool) -> bool {
        self.poll();
        let mut guide = false;
        ui.horizontal(|ui| {
            ui.add_sized(
                [180.0, 24.0],
                egui::Label::new(RichText::new(t.t("firewall.label")).color(p.text_secondary)),
            );
            let port_s = port.to_string();
            match (&self.status, self.busy) {
                (_, true) | (None, _) => {
                    ui.label(fonts::job(
                        icon::SPINNER,
                        &t.t("firewall.checking"),
                        fonts::regular(size::BODY),
                        p.text_secondary,
                    ));
                }
                (Some(Err(e)), _) => {
                    ui.label(fonts::job(
                        icon::WARNING,
                        e,
                        fonts::regular(size::CAPTION),
                        p.warning,
                    ));
                }
                (Some(Ok(s)), _) if !s.local_rules_allowed => {
                    ui.label(fonts::job(
                        icon::SHIELD_CHECK,
                        &format!("{} (FW-04)", t.t("firewall.policy")),
                        fonts::regular(size::BODY),
                        p.warning,
                    ));
                }
                (Some(Ok(s)), _) => {
                    let s = s.clone();
                    match s.rule {
                        Some((rp, _)) if rp == port => {
                            ui.label(fonts::job(
                                icon::SHIELD_CHECK,
                                &t.t_args("firewall.active", &[("port", &port_s)]),
                                fonts::regular(size::BODY),
                                p.success,
                            ));
                            if widgets::button(
                                ui,
                                p,
                                BKind::Ghost,
                                Some(icon::TRASH),
                                &t.t("firewall.remove"),
                            )
                            .clicked()
                            {
                                self.apply(ui.ctx(), None);
                            }
                        }
                        Some((rp, _)) => {
                            ui.label(fonts::job(
                                icon::WARNING,
                                &t.t_args("firewall.other_port", &[("port", &rp.to_string())]),
                                fonts::regular(size::BODY),
                                p.warning,
                            ));
                            if widgets::button(
                                ui,
                                p,
                                BKind::Secondary,
                                Some(icon::SHIELD_CHECK),
                                &t.t("firewall.update"),
                            )
                            .clicked()
                            {
                                self.apply(ui.ctx(), Some(port));
                            }
                        }
                        None => {
                            ui.label(fonts::job(
                                icon::WARNING,
                                &t.t("firewall.none"),
                                fonts::regular(size::BODY),
                                p.text_secondary,
                            ));
                            if widgets::button(
                                ui,
                                p,
                                BKind::Secondary,
                                Some(icon::SHIELD_CHECK),
                                &t.t("firewall.allow"),
                            )
                            .clicked()
                            {
                                self.apply(ui.ctx(), Some(port));
                            }
                        }
                    }
                }
            }
            // inside the guide dialog itself the "Guide" button would only reopen it
            if !in_guide
                && widgets::button(
                    ui,
                    p,
                    BKind::Ghost,
                    Some(icon::INFO),
                    &t.t("firewall.guide"),
                )
                .clicked()
            {
                guide = true;
            }
        });
        match &self.last {
            Some(Ok(added)) => {
                ui.label(fonts::job(
                    icon::CHECK,
                    &t.t(if *added {
                        "firewall.done"
                    } else {
                        "firewall.removed"
                    }),
                    fonts::regular(size::CAPTION),
                    p.success,
                ));
            }
            Some(Err(FwError::Cancelled)) => {
                ui.label(fonts::job(
                    icon::INFO,
                    &format!("FW-02: {}", t.t("error.FW-02.what")),
                    fonts::regular(size::CAPTION),
                    p.text_secondary,
                ));
            }
            Some(Err(FwError::Failed(out))) => {
                ui.label(fonts::job(
                    icon::WARNING_OCTAGON,
                    &format!(
                        "FW-03: {}",
                        t.t_args("error.FW-03.what", &[("detail", out)])
                    ),
                    fonts::regular(size::CAPTION),
                    p.danger,
                ));
            }
            None => {}
        }
        guide
    }
}

pub enum Outcome {
    None,
    /// The firewall notice was closed; `true` = do not show again.
    FirewallClosed(bool),
    OpenLog,
}

impl Dialogs {
    /// First run (DESIGN 6.6): the firewall notice opens until "Do not show again" is ticked.
    pub fn with_firewall_notice(show: bool) -> Self {
        Self {
            firewall: show,
            ..Default::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        t: &I18n,
        items: &std::collections::HashMap<String, Item>,
        tex: &dyn Fn(&str) -> Option<egui::TextureHandle>,
        tx: &Sender<Command>,
        fw: &mut Firewall,
        server_port: u16,
    ) -> Outcome {
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut out = Outcome::None;

        // 4.7 already installed: default and Esc = Skip
        if let Some(it) = self.decision.as_ref().and_then(|id| items.get(id)).cloned() {
            let update = it.kind() == Kind::Update;
            let mut choice = None;
            modal(ctx, p, "decision", 480.0, |ui| {
                title(
                    ui,
                    icon::WARNING,
                    &t.t(if update {
                        "dialog.installed.update_title"
                    } else {
                        "dialog.installed.title"
                    }),
                    p.warning,
                );
                item_header(ui, p, &it, tex(&it.id).as_ref());
                ui.label(
                    RichText::new(t.t_args("dialog.installed.text", &[("id", &it.title_id)]))
                        .color(p.text_primary),
                );
                widgets::space(ui, 2);
                ui.label(
                    RichText::new(t.t(if update {
                        "dialog.installed.update_help"
                    } else {
                        "dialog.installed.reinstall_help"
                    }))
                    .color(p.text_secondary)
                    .size(size::CAPTION),
                );
                widgets::space(ui, 2);
                ui.checkbox(&mut self.apply_all, t.t("dialog.installed.apply_all"));
                widgets::space(ui, 4);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let again = if update {
                        t.t("dialog.installed.again")
                    } else {
                        t.t("act.reinstall")
                    };
                    if widgets::button(ui, p, BKind::Danger, Some(icon::ARROWS_CLOCKWISE), &again)
                        .clicked()
                    {
                        choice = Some(Decision::Reinstall);
                    }
                    let skip = widgets::button(
                        ui,
                        p,
                        BKind::Secondary,
                        Some(icon::SKIP),
                        &t.t("act.skip"),
                    );
                    if !ui.ctx().memory(|m| m.focused().is_some()) {
                        skip.request_focus();
                    }
                    if skip.clicked() || esc {
                        choice = Some(Decision::Skip);
                    }
                });
            });
            if let Some(decision) = choice {
                let _ = tx.send(Command::Decide {
                    id: it.id.clone(),
                    decision,
                    apply_all: self.apply_all,
                });
                self.decision = None;
                self.apply_all = false;
            }
        } else {
            self.decision = None;
        }

        // 4.8 error details
        if let Some(it) = self.error.as_ref().and_then(|id| items.get(id)).cloned()
            && let Some(e) = it.error.clone()
        {
            let mut close = esc;
            modal(ctx, p, "error", 640.0, |ui| {
                let name = it.title.clone().unwrap_or_else(|| it.title_id.clone());
                title(
                    ui,
                    icon::WARNING_OCTAGON,
                    &format!("{} · {name} ({})", e.code, it.title_id),
                    p.danger,
                );
                for (k, part) in [
                    ("log.what", "what"),
                    ("log.why", "why"),
                    ("log.action", "action"),
                ] {
                    ui.label(
                        RichText::new(t.t(k))
                            .font(fonts::semibold(size::BODY))
                            .color(p.text_primary),
                    );
                    ui.label(RichText::new(describe(t, &e, part)).color(p.text_primary));
                    widgets::space(ui, 1);
                }
                for (code, meaning) in
                    crate::errors::find_console_codes(&format!("{} {}", e.detail, e.technical))
                        .iter()
                        .map(|c| {
                            let key =
                                crate::errors::lookup_console_code(c).map_or("unknown", |k| k.key);
                            (
                                c.clone(),
                                t.t_args(&format!("console.{key}.meaning"), &[("code", c)]),
                            )
                        })
                {
                    ui.label(
                        RichText::new(format!("{code}: {meaning}"))
                            .color(p.text_secondary)
                            .size(size::CAPTION),
                    );
                }
                widgets::space(ui, 1);
                ui.checkbox(&mut self.show_tech, t.t("dialog.error.technical"));
                if self.show_tech {
                    ui.label(
                        RichText::new(format!("{}\n{}", e.detail, e.technical))
                            .font(fonts::mono(size::CAPTION))
                            .color(p.text_secondary),
                    );
                }
                if let Some(path) = &e.console_log {
                    ui.checkbox(&mut self.show_log, t.t("dialog.error.console_log"));
                    if self.show_log {
                        let text = std::fs::read_to_string(path).unwrap_or_default();
                        let tail: Vec<&str> = text
                            .lines()
                            .rev()
                            .take(200)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect();
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(tail.join("\n"))
                                        .font(fonts::mono(size::CAPTION))
                                        .color(p.text_secondary),
                                );
                            });
                    }
                }
                widgets::space(ui, 4);
                ui.horizontal(|ui| {
                    if widgets::button(
                        ui,
                        p,
                        BKind::Ghost,
                        Some(icon::COPY),
                        &t.t("dialog.error.copy"),
                    )
                    .clicked()
                    {
                        let txt = format!(
                            "{} {}\n{}: {}\n{}: {}\n{}: {}\n{}\n{}",
                            e.code,
                            it.title_id,
                            t.t("log.what"),
                            describe(t, &e, "what"),
                            t.t("log.why"),
                            describe(t, &e, "why"),
                            t.t("log.action"),
                            describe(t, &e, "action"),
                            e.detail,
                            e.technical
                        );
                        ui.ctx().copy_text(txt);
                    }
                    if widgets::button(
                        ui,
                        p,
                        BKind::Ghost,
                        Some(icon::SCROLL),
                        &t.t("dialog.error.open_log"),
                    )
                    .clicked()
                    {
                        out = Outcome::OpenLog;
                        close = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if matches!(it.state, crate::engine::queue::State::Failed)
                            && widgets::button(
                                ui,
                                p,
                                BKind::Primary,
                                Some(icon::ARROWS_CLOCKWISE),
                                &t.t("dialog.error.retry"),
                            )
                            .clicked()
                        {
                            let _ = tx.send(Command::Retry(it.id.clone()));
                            close = true;
                        }
                        if widgets::button(
                            ui,
                            p,
                            BKind::Secondary,
                            None,
                            &t.t("dialog.close.close"),
                        )
                        .clicked()
                        {
                            close = true;
                        }
                    });
                });
            });
            if close {
                self.error = None;
            }
        } else {
            self.error = None;
        }

        // 4.11 uninstall confirmation: always asked, default and Esc = Cancel
        if let Some(target) = self.uninstall.clone() {
            let mut answer = None;
            modal(ctx, p, "uninstall", 480.0, |ui| {
                title(ui, icon::WARNING_OCTAGON, &t.t("uninstall.title"), p.danger);
                match target.item.as_ref().and_then(|id| items.get(id)) {
                    Some(it) => item_header(ui, p, it, tex(&it.id).as_ref()),
                    None => {
                        ui.label(RichText::new(&target.id).font(fonts::mono(size::BODY)));
                        widgets::space(ui, 2);
                    }
                }
                let key = match target.kind {
                    UninstallKind::Game => "uninstall.text.game",
                    UninstallKind::Patch => "uninstall.text.patch",
                    UninstallKind::Dlc => "uninstall.text.dlc",
                };
                ui.label(RichText::new(t.t(key)).color(p.text_primary));
                widgets::space(ui, 4);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::button(
                        ui,
                        p,
                        BKind::Danger,
                        Some(icon::TRASH),
                        &t.t("uninstall.confirm"),
                    )
                    .clicked()
                    {
                        answer = Some(true);
                    }
                    let cancel =
                        widgets::button(ui, p, BKind::Secondary, None, &t.t("uninstall.cancel"));
                    if !ui.ctx().memory(|m| m.focused().is_some()) {
                        cancel.request_focus();
                    }
                    if cancel.clicked() || esc {
                        answer = Some(false);
                    }
                });
            });
            if let Some(yes) = answer {
                if yes {
                    let _ = tx.send(Command::Uninstall(target));
                }
                self.uninstall = None;
            }
        }

        // 4.12 uninstall by ID
        if self.uninstall.is_none()
            && let Some(b) = self.by_id.as_mut()
        {
            let mut close = esc;
            let mut confirm = None;
            modal(ctx, p, "by-id", 480.0, |ui| {
                title(ui, icon::TRASH, &t.t("uninstall.by_id"), p.text_primary);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.t("uninstall.type")).color(p.text_secondary));
                    for (k, label) in [
                        (UninstallKind::Game, "kind.game"),
                        (UninstallKind::Patch, "kind.update"),
                        (UninstallKind::Dlc, "kind.dlc"),
                    ] {
                        if ui.radio_value(&mut b.kind, k, t.t(label)).changed() {
                            b.result = None;
                        }
                    }
                });
                let label = if b.kind == UninstallKind::Dlc {
                    "uninstall.content_id"
                } else {
                    "uninstall.title_id"
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.t(label)).color(p.text_secondary));
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut b.text)
                            .desired_width(320.0)
                            .font(fonts::mono(size::BODY)),
                    );
                    if r.changed() {
                        b.text = b.text.trim().to_ascii_uppercase();
                        b.result = None;
                    }
                    match &b.result {
                        Some(Ok(true)) => {
                            ui.label(fonts::job(
                                icon::CHECK,
                                &t.t("uninstall.installed"),
                                fonts::regular(size::CAPTION),
                                p.success,
                            ));
                        }
                        Some(Ok(false)) => {
                            ui.label(fonts::job(
                                icon::INFO,
                                &t.t_args("uninstall.not_installed", &[("id", &b.text)]),
                                fonts::regular(size::CAPTION),
                                p.text_secondary,
                            ));
                        }
                        Some(Err(e)) => {
                            ui.label(fonts::job(
                                icon::WARNING_OCTAGON,
                                &format!("RPI-02: {e}"),
                                fonts::regular(size::CAPTION),
                                p.danger,
                            ));
                        }
                        None => {}
                    }
                });
                let valid = valid_id(b.kind, &b.text);
                if !b.text.is_empty() && !valid {
                    ui.label(fonts::job(
                        icon::WARNING_OCTAGON,
                        &format!(
                            "CFG-03: {} {}",
                            t.t("error.CFG-03.what"),
                            t.t("error.CFG-03.action")
                        ),
                        fonts::regular(size::CAPTION),
                        p.danger,
                    ));
                }
                widgets::space(ui, 4);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::button_enabled(
                        ui,
                        p,
                        BKind::Danger,
                        Some(icon::TRASH),
                        &format!("{}…", t.t("uninstall.confirm")),
                        valid && b.result == Some(Ok(true)),
                    )
                    .clicked()
                    {
                        confirm = Some(UninstallTarget {
                            kind: b.kind,
                            id: b.text.clone(),
                            item: None,
                        });
                    }
                    if widgets::button_enabled(
                        ui,
                        p,
                        BKind::Secondary,
                        Some(icon::SEARCH),
                        &t.t("uninstall.check"),
                        valid && !b.checking,
                    )
                    .clicked()
                    {
                        b.checking = true;
                        let _ = tx.send(Command::CheckInstalled(UninstallTarget {
                            kind: b.kind,
                            id: b.text.clone(),
                            item: None,
                        }));
                    }
                    if widgets::button(ui, p, BKind::Ghost, None, &t.t("uninstall.cancel"))
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
            if let Some(c) = confirm {
                self.uninstall = Some(c);
            }
            if close {
                self.by_id = None;
            }
        }

        // 4.9 firewall guide
        if self.firewall {
            let mut closed = None;
            fw.poll();
            let port_s = server_port.to_string();
            modal(ctx, p, "firewall", 600.0, |ui| {
                title(ui, icon::INFO, &t.t("firewall.dialog.title"), p.info);
                ui.label(
                    RichText::new(t.t_args("firewall.dialog.text", &[("port", &port_s)]))
                        .color(p.text_primary),
                );
                widgets::space(ui, 2);
                for k in [
                    "firewall.dialog.step1",
                    "firewall.dialog.step2",
                    "firewall.dialog.step3",
                    "firewall.dialog.step4",
                ] {
                    ui.label(RichText::new(t.t(k)).color(p.text_primary));
                }
                widgets::space(ui, 2);
                ui.label(
                    RichText::new(t.t_args("firewall.dialog.self", &[("port", &port_s)]))
                        .color(p.text_secondary)
                        .size(size::CAPTION),
                );
                widgets::space(ui, 2);
                fw.row(ui, p, t, server_port, true);
                widgets::space(ui, 4);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.dont_show, t.t("firewall.dialog.dont_show"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button(ui, p, BKind::Primary, None, &t.t("firewall.dialog.ok"))
                            .clicked()
                            || esc
                        {
                            closed = Some(self.dont_show);
                        }
                        if widgets::button(
                            ui,
                            p,
                            BKind::Ghost,
                            Some(icon::COPY),
                            &t.t("firewall.dialog.copy"),
                        )
                        .clicked()
                        {
                            let steps: Vec<String> = [
                                "firewall.dialog.text",
                                "firewall.dialog.step1",
                                "firewall.dialog.step2",
                                "firewall.dialog.step3",
                                "firewall.dialog.step4",
                            ]
                            .iter()
                            .map(|k| t.t_args(k, &[("port", &port_s)]))
                            .collect();
                            ui.ctx().copy_text(steps.join("\n"));
                        }
                    });
                });
            });
            if let Some(d) = closed {
                self.firewall = false;
                out = Outcome::FirewallClosed(d);
            }
        }
        out
    }

    /// Engine answered a CheckInstalled. Returns a message for the toast, if any.
    pub fn installed_result(
        &mut self,
        t: &I18n,
        target: UninstallTarget,
        result: Result<bool, String>,
    ) -> Option<String> {
        if target.item.is_none()
            && let Some(b) = self.by_id.as_mut()
            && b.text == target.id
        {
            b.checking = false;
            b.result = Some(result);
            return None;
        }
        match result {
            Ok(true) => {
                self.uninstall = Some(target);
                None
            }
            Ok(false) => Some(t.t_args("uninstall.not_installed", &[("id", &target.id)])),
            Err(e) => Some(format!(
                "{}: {} ({e})",
                Code::Rpi02,
                t.t("error.RPI-02.what")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_formats() {
        assert!(valid_id(UninstallKind::Game, "CUSA12345"));
        assert!(!valid_id(UninstallKind::Game, "cusa12345"));
        assert!(!valid_id(UninstallKind::Patch, "CUSA1234"));
        assert!(valid_id(
            UninstallKind::Dlc,
            "EP0002-CUSA00184_00-ANGRYBSTARWARSDL"
        ));
        assert!(!valid_id(UninstallKind::Dlc, "CUSA00184"));
        assert!(!valid_id(
            UninstallKind::Dlc,
            "EP0002-CUSA00184_00-angrybstarwarsdl"
        ));
    }
}
