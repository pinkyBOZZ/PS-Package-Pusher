//! Log tab (DESIGN 4.6): level filters, search, expandable error entries, copy, open folder.

use super::{
    fonts::{self, icon, size},
    theme::Palette,
    widgets::{self, Kind},
};
use crate::{
    i18n::I18n,
    logging::{Entry, Level, Logger},
};
use eframe::egui::{self, RichText, Ui};

pub struct LogView {
    show: [bool; 4],
    search: String,
    cache: Vec<Entry>,
    generation: u64,
    selected: Option<usize>,
    expanded: Option<usize>,
    /// Errors already seen on the Log tab (header badge counts the rest).
    pub seen_errors: usize,
    copied_at: Option<f64>,
}

impl Default for LogView {
    fn default() -> Self {
        Self {
            show: [true, true, true, false],
            search: String::new(),
            cache: Vec::new(),
            generation: u64::MAX,
            selected: None,
            expanded: None,
            seen_errors: 0,
            copied_at: None,
        }
    }
}

fn level_ix(l: Level) -> usize {
    match l {
        Level::Error => 0,
        Level::Warn => 1,
        Level::Info => 2,
        Level::Debug => 3,
    }
}

impl LogView {
    pub fn refresh(&mut self, log: &Logger) {
        let g = log.generation();
        if g != self.generation {
            self.cache = log.entries();
            self.generation = g;
        }
    }

    pub fn error_count(&self) -> usize {
        self.cache
            .iter()
            .filter(|e| e.level == Level::Error)
            .count()
    }

    fn visible(&self) -> Vec<usize> {
        let q = self.search.to_lowercase();
        (0..self.cache.len())
            .filter(|&i| {
                let e = &self.cache[i];
                self.show[level_ix(e.level)]
                    && (q.is_empty() || e.line().to_lowercase().contains(&q))
            })
            .collect()
    }

    fn entry_text(e: &Entry, t: &I18n) -> String {
        let mut s = e.line();
        if let Some(c) = e.code {
            s += &format!(
                "\n  {}: {}\n  {}: {}",
                t.t("log.what"),
                t.t(&format!("error.{c}.what")),
                t.t("log.action"),
                t.t(&format!("error.{c}.action"))
            );
        }
        s
    }

    pub fn ui(&mut self, ui: &mut Ui, p: &Palette, t: &I18n, log: &Logger) {
        self.refresh(log);
        self.seen_errors = self.error_count();
        let now = ui.input(|i| i.time);
        let vis = self.visible();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t.t("tab.log")).font(fonts::semibold(size::TITLE)));
            ui.add_space(16.0);
            for (i, k) in [
                "log.filter.error",
                "log.filter.warn",
                "log.filter.info",
                "log.filter.debug",
            ]
            .iter()
            .enumerate()
            {
                ui.checkbox(&mut self.show[i], t.t(k));
            }
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text(t.t("log.search"))
                    .desired_width(200.0),
            );
        });
        ui.horizontal(|ui| {
            let sel = self.selected.and_then(|i| self.cache.get(i));
            if widgets::button_enabled(
                ui,
                p,
                Kind::Secondary,
                Some(icon::COPY),
                &t.t("log.copy"),
                sel.is_some(),
            )
            .clicked()
                && let Some(e) = sel
            {
                ui.ctx().copy_text(Self::entry_text(e, t));
                self.copied_at = Some(now);
            }
            if widgets::button_enabled(
                ui,
                p,
                Kind::Secondary,
                Some(icon::COPY),
                &t.t("log.copy_all"),
                !vis.is_empty(),
            )
            .clicked()
            {
                let all: Vec<String> = vis
                    .iter()
                    .map(|&i| Self::entry_text(&self.cache[i], t))
                    .collect();
                ui.ctx().copy_text(all.join("\n"));
                self.copied_at = Some(now);
            }
            if widgets::button(
                ui,
                p,
                Kind::Ghost,
                Some(icon::FOLDER_OPEN),
                &t.t("log.open_folder"),
            )
            .clicked()
                && let Some(d) = log.dir()
            {
                let _ = std::process::Command::new("explorer").arg(d).spawn();
            }
            if self.copied_at.is_some_and(|c| now - c < 2.0) {
                ui.label(fonts::job(
                    icon::CHECK,
                    &t.t("copied"),
                    fonts::regular(size::BODY),
                    p.success,
                ));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(500));
            }
        });
        widgets::space(ui, 2);
        if vis.is_empty() {
            ui.label(RichText::new(t.t("log.empty")).color(p.text_secondary));
            return;
        }
        let row_h = 22.0;
        egui::Frame::new()
            .fill(p.bg_surface)
            .corner_radius(12)
            .inner_margin(8)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for &i in &vis {
                            let e = &self.cache[i];
                            let (lvl_c, lvl) = match e.level {
                                Level::Error => (p.danger, "ERROR"),
                                Level::Warn => (p.warning, "WARN"),
                                Level::Info => (p.text_secondary, "INFO"),
                                Level::Debug => (p.text_disabled, "DEBUG"),
                            };
                            let expandable = e.code.is_some() || !e.fields.is_empty();
                            let open = self.expanded == Some(i);
                            let selected = self.selected == Some(i);
                            let resp = ui
                                .horizontal(|ui| {
                                    ui.set_min_height(row_h);
                                    let caret = if !expandable {
                                        " "
                                    } else if open {
                                        icon::CARET_DOWN
                                    } else {
                                        icon::CARET_RIGHT
                                    };
                                    ui.label(
                                        fonts::icon_rt(caret, size::BODY).color(p.text_secondary),
                                    );
                                    ui.label(
                                        RichText::new(e.time.get(11..19).unwrap_or(&e.time))
                                            .font(fonts::mono(size::CAPTION))
                                            .color(p.text_secondary),
                                    );
                                    ui.add_sized(
                                        [56.0, row_h],
                                        egui::Label::new(
                                            RichText::new(t.t(&format!("log.level.{lvl}")))
                                                .font(fonts::semibold(size::CAPTION))
                                                .color(lvl_c),
                                        ),
                                    );
                                    if let Some(tid) = &e.title_id {
                                        ui.label(
                                            RichText::new(format!("[{tid}]"))
                                                .font(fonts::mono(size::CAPTION))
                                                .color(p.text_secondary),
                                        );
                                    }
                                    if let Some(c) = e.code {
                                        ui.label(
                                            RichText::new(c.as_str())
                                                .font(fonts::semibold(size::CAPTION))
                                                .color(lvl_c),
                                        );
                                    }
                                    let msg = match e.code {
                                        Some(c) => {
                                            t.t(&format!("error.{c}.what"))
                                                .split('{')
                                                .next()
                                                .unwrap_or_default()
                                                .trim_end()
                                                .to_string()
                                                + if e.msg.is_empty() { "" } else { " · " }
                                                + &e.msg
                                        }
                                        None => e.msg.clone(),
                                    };
                                    ui.add(
                                        egui::Label::new(RichText::new(msg).color(p.text_primary))
                                            .truncate(),
                                    );
                                })
                                .response
                                .interact(egui::Sense::click());
                            if selected {
                                ui.painter().rect_filled(
                                    resp.rect,
                                    6,
                                    p.bg_selected.gamma_multiply(0.5),
                                );
                            }
                            if resp.clicked() {
                                self.selected = Some(i);
                                if expandable {
                                    self.expanded = if open { None } else { Some(i) };
                                }
                            }
                            if open {
                                egui::Frame::new()
                                    .fill(p.bg_surface2)
                                    .corner_radius(8)
                                    .inner_margin(12)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        if let Some(c) = e.code {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}: {}",
                                                    t.t("log.what"),
                                                    t.t(&format!("error.{c}.what"))
                                                ))
                                                .color(p.text_primary),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}: {}",
                                                    t.t("log.action"),
                                                    t.t(&format!("error.{c}.action"))
                                                ))
                                                .color(p.text_primary),
                                            );
                                        }
                                        let details: Vec<String> = std::iter::once(e.msg.clone())
                                            .chain(e.fields.iter().map(|(k, v)| format!("{k}={v}")))
                                            .collect();
                                        ui.label(
                                            RichText::new(format!(
                                                "{}: {}",
                                                t.t("log.details"),
                                                details.join(" · ")
                                            ))
                                            .font(fonts::mono(size::CAPTION))
                                            .color(p.text_secondary),
                                        );
                                    });
                            }
                        }
                    });
            });
        if ui.input(|i| i.key_pressed(egui::Key::Enter))
            && let Some(s) = self.selected
        {
            self.expanded = if self.expanded == Some(s) {
                None
            } else {
                Some(s)
            };
        }
    }
}
