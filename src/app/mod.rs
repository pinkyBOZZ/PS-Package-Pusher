//! UI layer, UI thread only (DESIGN 3, 4). Never does I/O or network on the UI thread (7.3).

pub mod anim;
pub mod dialogs;
pub mod fonts;
pub mod log_view;
pub mod queue_view;
pub mod settings_view;
pub mod splash;
pub mod theme;
pub mod widgets;

use crate::{
    config::{self, Config},
    engine::{
        self, EngineOptions, EngineTimings,
        commands::{Banner, Command, Event},
        queue::{Item, State},
    },
    i18n::{I18n, Lang},
    logging::Logger,
    rpi::RpiDialect,
};
use eframe::egui::{self, Align, Layout, RichText, Ui, vec2};
use fonts::{icon, size};
use settings_view::{Reach, SettingsView};
use std::{
    collections::HashMap,
    net::SocketAddrV4,
    path::PathBuf,
    sync::{Arc, mpsc},
};
use theme::Palette;
use widgets::{BannerKind, Kind};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Queue,
    Settings,
    Log,
    Gallery,
}

impl Tab {
    fn index(self) -> i32 {
        self as i32
    }
}

pub struct App {
    theme: theme::ThemeState,
    t: I18n,
    cfg: Config,
    data_dir: PathBuf,
    log: Logger,
    debug: bool,
    engine_tx: mpsc::Sender<Command>,
    engine_join: Option<std::thread::JoinHandle<()>>,
    events: mpsc::Receiver<Event>,
    items: HashMap<String, Item>,
    order: Vec<String>,
    banner: Option<Banner>,
    server: Option<SocketAddrV4>,
    tab: Tab,
    tab_since: f64,
    tab_from: i32,
    settings: SettingsView,
    log_view: LogView,
    reach: Option<Reach>,
    logo: Option<egui::TextureHandle>,
    frames: u64,
    /// Debug-only override for screenshots (`--theme light|dark`); the app follows Windows otherwise.
    force_theme: Option<bool>,
}

use log_view::LogView;

impl App {
    /// `start_tab` is a debug aid for screenshots (`--tab settings|log|gallery`).
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cfg: Config,
        data_dir: PathBuf,
        log: Logger,
        debug: bool,
        start_tab: Option<&str>,
        theme: Option<&str>,
    ) -> Self {
        fonts::install(&cc.egui_ctx);
        anim::set_reduced(!crate::winapi::animations_enabled());
        cc.egui_ctx
            .options_mut(|o| o.theme_preference = egui::ThemePreference::System);
        let (etx, erx) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let h = engine::spawn(
            EngineOptions {
                data_dir: Some(data_dir.clone()),
                config: cfg.clone(),
                timings: EngineTimings::default(),
                dialect: RpiDialect::default(),
                log: log.clone(),
            },
            etx,
            Arc::new(move || ctx.request_repaint()),
        );
        let logo = image::load_from_memory(include_bytes!("../../assets/logo/png/icon-32.png"))
            .ok()
            .map(|img| {
                let rgba = img.to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                cc.egui_ctx
                    .load_texture("logo", ci, egui::TextureOptions::LINEAR)
            });
        log.info(
            "app",
            format!("PS Package Pusher {} started", env!("CARGO_PKG_VERSION")),
        );
        Self {
            theme: theme::ThemeState::default(),
            t: I18n::new(Lang::from_code(&cfg.language)),
            settings: SettingsView::new(&cfg),
            cfg,
            data_dir,
            log,
            debug,
            engine_tx: h.tx,
            engine_join: Some(h.join),
            events: erx,
            items: HashMap::new(),
            order: Vec::new(),
            banner: None,
            server: None,
            tab: match start_tab {
                Some("settings") => Tab::Settings,
                Some("log") => Tab::Log,
                Some("gallery") if debug => Tab::Gallery,
                _ => Tab::Queue,
            },
            tab_since: f64::NEG_INFINITY,
            tab_from: 0,
            log_view: LogView::default(),
            reach: None,
            logo,
            frames: 0,
            force_theme: if debug {
                theme.map(|t| t == "dark")
            } else {
                None
            },
        }
    }

    fn drain_events(&mut self) {
        while let Ok(e) = self.events.try_recv() {
            match e {
                Event::ItemUpsert(i) => {
                    if !self.order.contains(&i.id) {
                        self.order.push(i.id.clone());
                    }
                    self.items.insert(i.id.clone(), *i);
                }
                Event::ItemRemoved(id) => {
                    self.items.remove(&id);
                    self.order.retain(|x| *x != id);
                }
                Event::Order(o) => self.order = o,
                Event::Banner(b) => {
                    if matches!(b, Some(Banner::Global(ref r)) if r.code.as_str().starts_with("NET"))
                    {
                        self.reach = Some(Reach::Unreachable);
                    }
                    self.banner = b;
                }
                Event::ServerStatus(s) => self.server = s,
                Event::AskDecision(_) | Event::QueueRunning(_) | Event::Info(_) => {}
            }
        }
    }

    fn switch(&mut self, tab: Tab, now: f64) {
        if tab != self.tab {
            self.tab_from = self.tab.index();
            self.tab = tab;
            self.tab_since = now;
        }
    }

    fn busy(&self) -> bool {
        self.items.values().any(|i| {
            matches!(
                i.state,
                State::Transferring | State::Installing | State::Paused | State::Starting
            )
        })
    }

    fn header(&mut self, ui: &mut Ui, p: &Palette, now: f64) {
        let full = ui.max_rect();
        widgets::sunset_stripe(ui, full);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.set_min_height(44.0);
            ui.add_space(12.0);
            if let Some(l) = &self.logo {
                ui.add(egui::Image::new(l).fit_to_exact_size(vec2(24.0, 24.0)));
            }
            ui.label(
                RichText::new(self.t.t("app.name"))
                    .font(fonts::display(18.0))
                    .color(p.text_primary),
            );
            ui.add_space(24.0);
            let pending = self
                .items
                .values()
                .filter(|i| !i.state.is_finished() && i.state != State::Invalid)
                .count();
            let unread = self
                .log_view
                .error_count()
                .saturating_sub(self.log_view.seen_errors);
            let mut tabs = vec![
                (Tab::Queue, "tab.queue", pending, false),
                (Tab::Settings, "tab.settings", 0, false),
                (Tab::Log, "tab.log", unread, true),
            ];
            if self.debug {
                tabs.push((Tab::Gallery, "tab.gallery", 0, false));
            }
            for (tab, key, count, danger) in tabs {
                let active = self.tab == tab;
                let mut text = self.t.t(key);
                if count > 0 && !danger {
                    text += &format!(" ({count})");
                }
                let color = if active { p.accent } else { p.text_secondary };
                let r = ui.add(
                    egui::Button::new(
                        RichText::new(text)
                            .font(fonts::semibold(size::BODY))
                            .color(color),
                    )
                    .frame(false)
                    .min_size(vec2(32.0, 32.0)),
                );
                if count > 0 && danger {
                    widgets::badge(ui, (p.danger, p.danger_on), &count.to_string());
                }
                let underline =
                    ui.ctx()
                        .animate_bool_with_time(r.id.with("u"), active, anim::dur(anim::TAB));
                if underline > 0.0 {
                    let w = r.rect.width() * underline;
                    let y = r.rect.bottom() + 2.0;
                    let x0 = r.rect.center().x - w / 2.0;
                    if p.dark {
                        ui.painter().line_segment(
                            [egui::pos2(x0, y), egui::pos2(x0 + w, y)],
                            egui::Stroke::new(6.0, p.accent.gamma_multiply(0.15)),
                        );
                    }
                    ui.painter().line_segment(
                        [egui::pos2(x0, y), egui::pos2(x0 + w, y)],
                        egui::Stroke::new(2.0, p.accent),
                    );
                }
                if r.clicked() {
                    self.switch(tab, now);
                }
                ui.add_space(8.0);
                if r.has_focus() {
                    widgets::focus_ring(ui, r.rect, p, 6.0);
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                let configured = self.cfg.console_ip().is_some();
                let (txt, ic) = match (configured, self.settings.testing(), self.reach) {
                    (false, _, _) => (self.t.t("console.state.not_configured"), icon::PLUG),
                    (_, true, _) => (self.t.t("console.state.checking"), icon::SPINNER),
                    (_, _, Some(Reach::Ok(ms))) => (
                        self.t
                            .t_args("console.state.ok", &[("ms", &ms.to_string())]),
                        icon::CIRCLE,
                    ),
                    (_, _, Some(Reach::Unreachable)) => {
                        (self.t.t("console.state.unreachable"), icon::WARNING_OCTAGON)
                    }
                    _ => (self.t.t("console.state.unknown"), icon::CIRCLE),
                };
                let ip = self.cfg.console_ip.clone().unwrap_or_default();
                let color = settings_view::reach_color(p, self.reach, configured);
                let r = ui.add(
                    egui::Button::new(fonts::job(
                        ic,
                        &format!("{ip}  {txt}"),
                        fonts::regular(size::CAPTION),
                        color,
                    ))
                    .frame(false),
                );
                if r.clicked() {
                    self.switch(Tab::Settings, now);
                }
            });
        });
        let y = ui.min_rect().bottom() + 4.0;
        ui.painter().line_segment(
            [egui::pos2(full.left(), y), egui::pos2(full.right(), y)],
            egui::Stroke::new(1.0, p.border_subtle),
        );
        ui.add_space(8.0);
    }

    fn banner_ui(&mut self, ui: &mut Ui, p: &Palette, now: f64) {
        let Some(b) = self.banner.clone() else {
            if self.cfg.console_ip().is_none() {
                if widgets::banner(
                    ui,
                    p,
                    BannerKind::Info,
                    &self.t.t("banner.not_configured.title"),
                    &self.t.t("banner.not_configured.text"),
                    Some(&self.t.t("banner.open_settings")),
                ) {
                    self.switch(Tab::Settings, now);
                }
                widgets::space(ui, 2);
            }
            return;
        };
        let (kind, title, text) = match &b {
            Banner::ConsoleNotConfigured => (
                BannerKind::Info,
                self.t.t("banner.not_configured.title"),
                self.t.t("banner.not_configured.text"),
            ),
            Banner::Global(r) => (
                BannerKind::Danger,
                format!(
                    "{}: {}",
                    r.code,
                    self.t.t(&format!("error.{}.what", r.code))
                ),
                format!(
                    "{} {}",
                    self.t.t(&format!("error.{}.action", r.code)),
                    r.detail
                ),
            ),
        };
        if widgets::banner(
            ui,
            p,
            kind,
            &title,
            &text,
            Some(&self.t.t("banner.open_settings")),
        ) {
            self.switch(Tab::Settings, now);
        }
        widgets::space(ui, 2);
    }

    fn gallery(&mut self, ui: &mut Ui, p: &Palette) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Token gallery (--debug)").font(fonts::semibold(size::TITLE)),
                );
                let sw = [
                    ("bg.base", p.bg_base),
                    ("bg.surface", p.bg_surface),
                    ("bg.surface2", p.bg_surface2),
                    ("bg.hover", p.bg_hover),
                    ("bg.selected", p.bg_selected),
                    ("text.primary", p.text_primary),
                    ("text.secondary", p.text_secondary),
                    ("text.disabled", p.text_disabled),
                    ("accent", p.accent),
                    ("accent2", p.accent2),
                    ("purple.text", p.purple_text),
                    ("success", p.success),
                    ("warning", p.warning),
                    ("danger", p.danger),
                    ("info", p.info),
                    ("border.subtle", p.border_subtle),
                    ("border.strong", p.border_strong),
                    ("focus", p.focus),
                    ("track", p.track),
                ];
                for row in sw.chunks(7) {
                    ui.horizontal(|ui| {
                        for &(n, c) in row {
                            ui.vertical(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(vec2(96.0, 36.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, 8, c);
                                ui.painter().rect_stroke(
                                    r,
                                    8,
                                    egui::Stroke::new(1.0, p.border_subtle),
                                    egui::StrokeKind::Inside,
                                );
                                ui.label(
                                    RichText::new(n).size(size::CAPTION).color(p.text_secondary),
                                );
                            });
                        }
                    });
                }
                widgets::space(ui, 3);
                ui.horizontal(|ui| {
                    widgets::button(ui, p, Kind::Primary, Some(icon::PLAY), "Primary");
                    widgets::button(ui, p, Kind::Secondary, Some(icon::PAUSE), "Secondary");
                    widgets::button(ui, p, Kind::Danger, Some(icon::TRASH), "Danger");
                    widgets::button(ui, p, Kind::Ghost, Some(icon::FOLDER_OPEN), "Ghost");
                    widgets::button_enabled(ui, p, Kind::Primary, None, "Disabled", false);
                });
                widgets::space(ui, 2);
                ui.horizontal(|ui| {
                    widgets::badge(ui, p.badge_game, "Igra");
                    widgets::badge(ui, p.badge_update, "Update");
                    widgets::badge(ui, p.badge_dlc, "DLC");
                    widgets::badge(ui, (p.danger, p.danger_on), "3");
                });
                widgets::space(ui, 2);
                for (c, i, txt) in [
                    (p.info, icon::SEARCH, "Provjera…"),
                    (p.accent, icon::DOWNLOAD, "Šalje se"),
                    (p.warning, icon::PAUSE, "Pauzirano"),
                    (p.success, icon::CHECK, "Završeno"),
                    (p.danger, icon::WARNING_OCTAGON, "Greška"),
                    (p.text_secondary, icon::SKIP, "Preskočeno"),
                ] {
                    widgets::status(ui, c, i, txt);
                }
                widgets::space(ui, 2);
                let t = (ui.input(|i| i.time) * 0.1).fract() as f32;
                widgets::progress(ui, p, Some(t), 360.0, ui.scope_id().with("g1"));
                widgets::progress(ui, p, None, 360.0, ui.scope_id().with("g2"));
                widgets::space(ui, 2);
                for k in [
                    BannerKind::Info,
                    BannerKind::Warning,
                    BannerKind::Danger,
                    BannerKind::Success,
                ] {
                    widgets::banner(
                        ui,
                        p,
                        k,
                        "Banner title",
                        "Banner text with the secondary line.",
                        Some("Action"),
                    );
                    widgets::space(ui, 1);
                }
                ui.label(
                    RichText::new("PS PACKAGE PUSHER")
                        .font(fonts::display(size::DISPLAY))
                        .color(p.accent),
                );
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frames += 1;
        let now = ctx.input(|i| i.time);
        self.drain_events();
        if let Some(r) = self.settings.poll() {
            self.reach = Some(r);
        }
        if self
            .theme
            .update(&ctx, now, anim::dur(anim::THEME) as f64, self.force_theme)
        {
            ctx.request_repaint();
        }
        let p = self.theme.current;
        // position from the outer rect, size from the inner rect (the viewport builder takes inner size)
        if let (Some(o), Some(inner)) =
            ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect))
        {
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            if !maximized {
                self.cfg.window = Some(config::WindowState {
                    x: o.left(),
                    y: o.top(),
                    w: inner.width(),
                    h: inner.height(),
                    maximized,
                });
            } else if let Some(w) = self.cfg.window.as_mut() {
                w.maximized = true;
            }
        }
        // keyboard (DESIGN 12)
        let (n1, n2, n3, tab_key) = ctx.input(|i| {
            let c = i.modifiers.ctrl;
            (
                c && i.key_pressed(egui::Key::Num1),
                c && i.key_pressed(egui::Key::Num2),
                c && i.key_pressed(egui::Key::Num3),
                c && i.key_pressed(egui::Key::Tab),
            )
        });
        if n1 {
            self.switch(Tab::Queue, now);
        } else if n2 {
            self.switch(Tab::Settings, now);
        } else if n3 {
            self.switch(Tab::Log, now);
        } else if tab_key {
            let next = match self.tab {
                Tab::Queue => Tab::Settings,
                Tab::Settings => Tab::Log,
                Tab::Log if self.debug => Tab::Gallery,
                _ => Tab::Queue,
            };
            self.switch(next, now);
        }
        self.log_view.refresh(&self.log);

        egui::Frame::new().fill(p.bg_base).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            self.header(ui, &p, now);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(16, 0))
                .show(ui, |ui| {
                    self.banner_ui(ui, &p, now);
                    // tab transition: fade + 12 px slide from the side of the previous tab (motion.tab)
                    let (alpha, dx) = match anim::progress(
                        &ctx,
                        self.tab_since,
                        anim::TAB,
                        egui::emath::easing::cubic_out,
                    ) {
                        Some(k) => (
                            k,
                            (1.0 - k)
                                * 12.0
                                * if self.tab.index() >= self.tab_from {
                                    1.0
                                } else {
                                    -1.0
                                },
                        ),
                        None => (1.0, 0.0),
                    };
                    let rect = ui.available_rect_before_wrap().translate(vec2(dx, 0.0));
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(Layout::top_down(Align::Min)),
                    );
                    child.multiply_opacity(alpha);
                    match self.tab {
                        Tab::Queue => {
                            let items: Vec<&Item> = self
                                .order
                                .iter()
                                .filter_map(|id| self.items.get(id))
                                .collect();
                            queue_view::ui(&mut child, &p, &self.t, &items);
                        }
                        Tab::Settings => {
                            let busy = self.busy();
                            if let Some(new) = self.settings.ui(
                                &mut child,
                                &p,
                                &self.t,
                                &self.cfg,
                                self.server,
                                busy,
                                &self.data_dir,
                            ) {
                                let lang_changed = new.language != self.cfg.language;
                                let mut new = new;
                                new.window = self.cfg.window;
                                self.cfg = new;
                                if let Err(e) = config::save(&self.data_dir, &self.cfg) {
                                    self.log
                                        .error("app", "cannot save config.json")
                                        .field("err", e);
                                }
                                self.log.info("app", "settings saved");
                                let _ = self
                                    .engine_tx
                                    .send(Command::ApplySettings(Box::new(self.cfg.clone())));
                                if lang_changed {
                                    self.t = I18n::new(Lang::from_code(&self.cfg.language));
                                }
                            }
                            // language applies immediately, even before Save (DESIGN 4.5)
                            if self.settings.draft.language != self.t.lang.code() {
                                self.t = I18n::new(Lang::from_code(&self.settings.draft.language));
                            }
                        }
                        Tab::Log => self.log_view.ui(&mut child, &p, &self.t, &self.log),
                        Tab::Gallery => self.gallery(&mut child, &p),
                    }
                });
        });
        if self.debug {
            egui::Area::new(egui::Id::unique("dbg"))
                .anchor(egui::Align2::RIGHT_BOTTOM, vec2(-8.0, -8.0))
                .show(&ctx, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("frames {}", self.frames))
                                .font(fonts::mono(size::CAPTION))
                                .color(p.text_secondary),
                        )
                        .extend(),
                    );
                });
        }
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        self.theme.current.bg_base.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.engine_tx.send(Command::Shutdown);
        if let Some(j) = self.engine_join.take() {
            let _ = j.join();
        }
        if let Err(e) = config::save(&self.data_dir, &self.cfg) {
            self.log
                .error("app", "cannot save config.json")
                .field("err", e);
        }
        self.log.info("app", "exit");
    }
}
