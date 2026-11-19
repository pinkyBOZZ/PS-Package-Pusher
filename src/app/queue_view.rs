//! Queue tab (DESIGN 4.2–4.4): toolbar, list rows, drop zone, reorder, item animations.

use super::{
    anim,
    fonts::{self, icon, size},
    theme::Palette,
    widgets::{self, Kind as BKind},
};
use crate::{
    engine::{
        commands::Command,
        queue::{Decision, Item, Pre, State},
    },
    i18n::I18n,
    pkg::Kind,
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Rect, RichText, Sense, Stroke, Ui, pos2, vec2,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::Sender,
};

pub const ROW_H: f32 = 72.0;

/// A live item, or a removed one still collapsing (with its removal time).
type Row<'a> = (Option<&'a Item>, Option<(&'a Item, f64)>);
const GAP: f32 = 8.0;
const GRIP: &str = "\u{eae2}"; // Phosphor dots-six-vertical

#[derive(Default)]
pub struct QueueView {
    textures: HashMap<String, egui::TextureHandle>,
    pub selected: Option<String>,
    first_seen: HashMap<String, f64>,
    primed: bool,
    ghosts: Vec<(usize, Item, f64)>,
    last_y: HashMap<String, f32>,
    moved: HashMap<String, (f32, f64)>,
    pausing: HashSet<String>,
    toast: Option<(String, f64)>,
    pub confirm_remove: Option<String>,
    pub details: Option<String>,
}

/// What the view asks the app to do outside of engine commands.
pub enum Action {
    None,
    AddFiles,
    AddFolder,
}

pub fn fmt_eta(t: &I18n, secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    if s < 60 {
        t.t_args("eta.sec", &[("n", &s.max(1).to_string())])
    } else if s < 3600 {
        t.t_args("eta.min", &[("n", &s.div_ceil(60).to_string())])
    } else {
        t.t_args(
            "eta.hour",
            &[
                ("h", &(s / 3600).to_string()),
                ("m", &format!("{:02}", (s % 3600) / 60)),
            ],
        )
    }
}

pub fn state_style(p: &Palette, s: State) -> (Color32, &'static str) {
    use State::*;
    match s {
        Parsing => (p.info, icon::SPINNER),
        Invalid => (p.danger, icon::FILE_X),
        Ready => (p.text_secondary, icon::PACKAGE),
        Queued => (p.text_secondary, icon::HOURGLASS),
        Checking => (p.info, icon::SEARCH),
        AwaitingDecision => (p.warning, icon::WARNING),
        Uninstalling => (p.warning, icon::ARROWS_CLOCKWISE),
        Starting => (p.accent, icon::UPLOAD),
        Transferring | Installing => (p.accent, icon::DOWNLOAD),
        Paused => (p.warning, icon::PAUSE),
        Done => (p.success, icon::CHECK),
        Failed => (p.danger, icon::WARNING_OCTAGON),
        Cancelled => (p.text_secondary, icon::PROHIBIT),
        Skipped => (p.text_secondary, icon::SKIP),
    }
}

impl QueueView {
    pub fn info(&mut self, msg: String, now: f64) {
        self.toast = Some((msg, now));
    }

    pub fn removed(&mut self, index: usize, item: Item, now: f64) {
        if !anim::reduced() {
            self.ghosts.push((index, item, now));
        }
    }

    fn texture(&mut self, ctx: &egui::Context, it: &Item) -> Option<egui::TextureHandle> {
        if let Some(t) = self.textures.get(&it.id) {
            return Some(t.clone());
        }
        let ic = it.icon.as_ref()?;
        let img = egui::ColorImage::from_rgba_unmultiplied(
            [ic.width as usize, ic.height as usize],
            &ic.rgba,
        );
        let tex = ctx.load_texture(format!("icon-{}", it.id), img, egui::TextureOptions::LINEAR);
        self.textures.insert(it.id.clone(), tex.clone());
        Some(tex)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        p: &Palette,
        t: &I18n,
        items: &[&Item],
        tx: &Sender<Command>,
        running: bool,
        hovering_files: usize,
    ) -> Action {
        let now = ui.input(|i| i.time);
        let ctx = ui.ctx().clone();
        let mut action = Action::None;
        self.pausing.retain(|id| {
            items
                .iter()
                .any(|i| &i.id == id && i.state == State::Transferring)
        });
        self.textures
            .retain(|k, _| items.iter().any(|i| &i.id == k));

        // keyboard (DESIGN 12)
        let (ctrl_o, ctrl_shift_o, ctrl_enter) = ui.input(|i| {
            let c = i.modifiers.ctrl;
            (
                c && !i.modifiers.shift && i.key_pressed(egui::Key::O),
                c && i.modifiers.shift && i.key_pressed(egui::Key::O),
                c && i.key_pressed(egui::Key::Enter),
            )
        });
        if ctrl_o {
            action = Action::AddFiles;
        }
        if ctrl_shift_o {
            action = Action::AddFolder;
        }
        let can_start = !running && items.iter().any(|i| i.state == State::Ready);
        if ctrl_enter && can_start {
            let _ = tx.send(Command::StartQueue);
        }

        // toolbar + footer (DESIGN 4.3)
        ui.horizontal(|ui| {
            if widgets::button(
                ui,
                p,
                BKind::Secondary,
                Some(icon::PLUS),
                &t.t("queue.add_files"),
            )
            .clicked()
            {
                action = Action::AddFiles;
            }
            if widgets::button(
                ui,
                p,
                BKind::Secondary,
                Some(icon::FOLDER_PLUS),
                &t.t("queue.add_folder"),
            )
            .clicked()
            {
                action = Action::AddFolder;
            }
            ui.add_space(12.0);
            if widgets::button_enabled(
                ui,
                p,
                BKind::Primary,
                Some(icon::PLAY),
                &t.t("toolbar.start"),
                can_start,
            )
            .clicked()
            {
                let _ = tx.send(Command::StartQueue);
            }
            if widgets::button_enabled(
                ui,
                p,
                BKind::Secondary,
                Some(icon::STOP),
                &t.t("toolbar.stop_after"),
                running,
            )
            .clicked()
            {
                let _ = tx.send(Command::StopAfterCurrent);
            }
            let finished = items
                .iter()
                .any(|i| matches!(i.state, State::Done | State::Skipped | State::Cancelled));
            if widgets::button_enabled(
                ui,
                p,
                BKind::Ghost,
                Some(icon::TRASH),
                &t.t("toolbar.clear"),
                finished,
            )
            .clicked()
            {
                let _ = tx.send(Command::ClearFinished);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let open: Vec<&&Item> = items
                    .iter()
                    .filter(|i| !i.state.is_finished() && i.state != State::Invalid)
                    .collect();
                if open.is_empty() {
                    return;
                }
                let left: u64 = open
                    .iter()
                    .map(|i| i.total_size().saturating_sub(i.live.transferred))
                    .sum();
                let speed = items
                    .iter()
                    .find_map(|i| i.live.speed.filter(|_| i.state == State::Transferring));
                let mut s = format!(
                    "{} · {}",
                    t.plural("queue.packages", open.len() as u64),
                    t.t_args("footer.left", &[("size", &t.size(left))])
                );
                if let Some(sp) = speed.filter(|s| *s > 0.0) {
                    s += &format!(
                        " · {}",
                        t.t_args(
                            "footer.eta",
                            &[("eta", fmt_eta(t, left as f64 / sp).trim_start_matches('~'))]
                        )
                    );
                }
                ui.label(RichText::new(s).color(p.text_secondary).size(size::CAPTION));
            });
        });
        widgets::space(ui, 2);

        self.ghosts.retain(|g| now - g.2 < 0.30);
        if items.is_empty() && self.ghosts.is_empty() {
            self.empty_state(ui, p, t, hovering_files);
            self.primed = true;
            self.toast_ui(ui, p, now);
            return action;
        }

        // display list: live items + collapsing ghosts at their last index (motion.item.out)
        let ghosts = self.ghosts.clone();
        let mut rows: Vec<Row> = items.iter().map(|i| (Some(*i), None)).collect();
        for (idx, item, at) in &ghosts {
            if !items.iter().any(|i| i.id == item.id) {
                rows.insert((*idx).min(rows.len()), (None, Some((item, *at))));
            }
        }
        if !self.ghosts.is_empty() {
            ctx.request_repaint();
        }
        let order_ix: HashMap<String, usize> = items
            .iter()
            .enumerate()
            .map(|(i, it)| (it.id.clone(), i))
            .collect();
        self.keyboard(ui, items, &order_ix, tx);

        let mut pos_no = 0usize;
        let mut drop_to: Option<(String, usize)> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = ui.available_width();
                for (live, ghost) in rows {
                    let (it, height_k, alpha) = match (live, ghost) {
                        (Some(it), _) => {
                            let seen = *self
                                .first_seen
                                .entry(it.id.clone())
                                .or_insert(if self.primed { now } else { f64::NEG_INFINITY });
                            let k = anim::progress(
                                &ctx,
                                seen,
                                anim::ITEM_IN,
                                egui::emath::easing::cubic_out,
                            )
                            .unwrap_or(1.0);
                            (it, k, k)
                        }
                        (None, Some((it, at))) => {
                            let tt = ((now - at) / 0.30) as f32;
                            let a = (1.0 - tt / 0.4).clamp(0.0, 1.0);
                            let h = 1.0
                                - egui::emath::easing::cubic_in_out(
                                    ((tt - 0.4) / 0.6).clamp(0.0, 1.0),
                                );
                            (it, h, a)
                        }
                        _ => continue,
                    };
                    let is_ghost = live.is_none();
                    let unfinished = !it.state.is_finished() && it.state != State::Invalid;
                    if unfinished && !is_ghost {
                        pos_no += 1;
                    }
                    let h = ROW_H * height_k;
                    let (rect, resp) = ui.allocate_exact_size(
                        vec2(width, h + GAP * height_k),
                        if is_ghost {
                            Sense::hover()
                        } else {
                            Sense::click()
                        },
                    );
                    if h < 2.0 {
                        continue;
                    }
                    let row = Rect::from_min_size(rect.min, vec2(width, ROW_H));
                    let dy = if is_ghost {
                        0.0
                    } else {
                        self.reorder_offset(&ctx, &it.id, row.top(), height_k, now)
                    };
                    let draw = row.translate(vec2(0.0, dy));
                    let mut cui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(draw)
                            .layout(egui::Layout::left_to_right(egui::Align::Min)),
                    );
                    cui.set_clip_rect(ui.clip_rect().intersect(Rect::from_min_max(
                        pos2(rect.left(), rect.top().min(draw.top())),
                        pos2(rect.right(), (rect.top() + h).max(draw.top() + h)),
                    )));
                    cui.multiply_opacity(alpha);
                    let selected = !is_ghost && self.selected.as_deref() == Some(it.id.as_str());
                    if resp.clicked() {
                        self.selected = Some(it.id.clone());
                    }
                    if let Some(d) = self.row(
                        &mut cui,
                        ui,
                        p,
                        t,
                        it,
                        items,
                        draw,
                        selected,
                        resp.hovered(),
                        unfinished.then_some(pos_no),
                        is_ghost,
                        tx,
                        &order_ix,
                        &resp,
                    ) {
                        drop_to = Some(d);
                    }
                    if !is_ghost {
                        resp.context_menu(|ui| self.menu(ui, t, it, items, tx));
                    }
                }
            });
        if let Some((id, to)) = drop_to {
            let from = order_ix.get(&id).copied().unwrap_or(0);
            let _ = tx.send(Command::Move {
                id,
                to: if to > from { to - 1 } else { to },
            });
        }
        self.primed = true;
        if hovering_files > 0 {
            self.drop_overlay(ui, p, t, hovering_files);
        }
        self.toast_ui(ui, p, now);
        action
    }

    fn keyboard(
        &mut self,
        ui: &Ui,
        items: &[&Item],
        order_ix: &HashMap<String, usize>,
        tx: &Sender<Command>,
    ) {
        let ctx = ui.ctx();
        if ctx.memory(|m| m.focused().is_some()) {
            return; // a text field has focus
        }
        let (up, down, alt, home, end, del, space) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.modifiers.alt,
                i.key_pressed(egui::Key::Home),
                i.key_pressed(egui::Key::End),
                i.key_pressed(egui::Key::Delete),
                i.key_pressed(egui::Key::Space),
            )
        });
        let Some(si) = self
            .selected
            .as_ref()
            .and_then(|s| order_ix.get(s).copied())
        else {
            if (up || down) && !items.is_empty() {
                self.selected = Some(items[0].id.clone());
            }
            return;
        };
        let it = items[si];
        if alt && !it.state.is_active() {
            let to = if up {
                Some(si.saturating_sub(1))
            } else if down {
                Some(si + 1)
            } else if home {
                Some(0)
            } else if end {
                Some(items.len())
            } else {
                None
            };
            if let Some(to) = to {
                let _ = tx.send(Command::Move {
                    id: it.id.clone(),
                    to,
                });
            }
        } else if up && si > 0 {
            self.selected = Some(items[si - 1].id.clone());
        } else if down && si + 1 < items.len() {
            self.selected = Some(items[si + 1].id.clone());
        }
        if del {
            self.remove(it, tx);
        }
        if space {
            match it.state {
                State::Transferring => {
                    self.pausing.insert(it.id.clone());
                    let _ = tx.send(Command::Pause(it.id.clone()));
                }
                State::Paused => {
                    let _ = tx.send(Command::Resume(it.id.clone()));
                }
                _ => {}
            }
        }
    }

    /// motion.reorder: a moved row slides from its previous y to the new one.
    fn reorder_offset(
        &mut self,
        ctx: &egui::Context,
        id: &str,
        top: f32,
        height_k: f32,
        now: f64,
    ) -> f32 {
        let ease = egui::emath::easing::cubic_in_out;
        if let Some(&prev) = self.last_y.get(id)
            && (prev - top).abs() > 1.0
            && height_k >= 1.0
        {
            let cur = self.moved.get(id).map_or(0.0, |(d, s)| {
                d * (1.0 - anim::progress(ctx, *s, anim::REORDER, ease).unwrap_or(1.0))
            });
            if !anim::reduced() {
                self.moved.insert(id.to_string(), (prev + cur - top, now));
            }
        }
        self.last_y.insert(id.to_string(), top);
        match self
            .moved
            .get(id)
            .map(|(d, s)| (*d, anim::progress(ctx, *s, anim::REORDER, ease)))
        {
            Some((d, Some(k))) => d * (1.0 - k),
            Some((_, None)) => {
                self.moved.remove(id);
                0.0
            }
            None => 0.0,
        }
    }

    /// One 72 px row (DESIGN 4.3). Returns a drop target `(dragged id, index)` when a drag ends here.
    #[allow(clippy::too_many_arguments)]
    fn row(
        &mut self,
        cui: &mut Ui,
        ui: &Ui,
        p: &Palette,
        t: &I18n,
        it: &Item,
        items: &[&Item],
        draw: Rect,
        selected: bool,
        hovered: bool,
        number: Option<usize>,
        is_ghost: bool,
        tx: &Sender<Command>,
        order_ix: &HashMap<String, usize>,
        resp: &egui::Response,
    ) -> Option<(String, usize)> {
        let ctx = ui.ctx().clone();
        let painter = cui.painter().clone();
        let bg = if selected {
            p.bg_selected
        } else if hovered {
            p.bg_hover
        } else {
            p.bg_surface
        };
        painter.rect_filled(draw, CornerRadius::same(12), bg);
        if selected {
            painter.rect_filled(
                Rect::from_min_size(draw.min + vec2(0.0, 12.0), vec2(3.0, ROW_H - 24.0)),
                2,
                p.accent,
            );
        }
        let mut drop = None;
        // drag handle; the active item is locked (DESIGN 4.3)
        let handle = Rect::from_min_size(draw.min + vec2(6.0, 20.0), vec2(20.0, 32.0));
        if !is_ghost && !it.state.is_active() {
            let hr = ui
                .interact(handle, egui::Id::unique(("drag", &it.id)), Sense::drag())
                .on_hover_cursor(egui::CursorIcon::Grab);
            hr.dnd_set_drag_payload(it.id.clone());
            if hovered || hr.hovered() || hr.dragged() {
                painter.text(
                    handle.center(),
                    Align2::CENTER_CENTER,
                    GRIP,
                    fonts::icons(size::HEADING),
                    p.text_secondary,
                );
            }
        }
        if !is_ghost {
            let pointer_below = ui
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|q| q.y >= draw.center().y);
            if let Some(dragged) = resp.dnd_hover_payload::<String>()
                && *dragged != it.id
            {
                let y = if pointer_below {
                    draw.bottom() + GAP / 2.0
                } else {
                    draw.top() - GAP / 2.0
                };
                ui.painter().line_segment(
                    [pos2(draw.left() + 8.0, y), pos2(draw.right() - 8.0, y)],
                    Stroke::new(2.0, p.accent),
                );
            }
            if let Some(dragged) = resp.dnd_release_payload::<String>()
                && let Some(&ix) = order_ix.get(&it.id)
            {
                drop = Some(((*dragged).clone(), ix + usize::from(pointer_below)));
            }
        }
        // #n for unfinished, status icon otherwise
        let (sc, sic) = state_style(p, it.state);
        let num_pos = draw.min + vec2(42.0, ROW_H / 2.0);
        match number {
            Some(n) => painter.text(
                num_pos,
                Align2::CENTER_CENTER,
                format!("#{n}"),
                fonts::semibold(size::CAPTION),
                p.text_secondary,
            ),
            None => painter.text(
                num_pos,
                Align2::CENTER_CENTER,
                sic,
                fonts::icons(size::HEADING),
                sc,
            ),
        };
        // icon 48×48
        let ir = Rect::from_min_size(draw.min + vec2(62.0, 12.0), vec2(48.0, 48.0));
        match (!is_ghost).then(|| self.texture(&ctx, it)).flatten() {
            Some(tex) => egui::Image::new(&tex).corner_radius(6).paint_at(cui, ir),
            None => {
                painter.rect_filled(ir, 6, p.bg_surface2);
                painter.text(
                    ir.center(),
                    Align2::CENTER_CENTER,
                    icon::PACKAGE,
                    fonts::icons(24.0),
                    p.text_secondary,
                );
            }
        }
        let x0 = ir.right() + 12.0;
        let right_col = draw.right() - 16.0;
        // line 1: title, type badge, version, size
        let title = it.title.clone().unwrap_or_else(|| it.title_id.clone());
        let max_title = (right_col - x0 - 260.0).max(80.0);
        let tg = painter.layout_no_wrap(title, fonts::semibold(size::HEADING), p.text_primary);
        let tw = tg.size().x.min(max_title);
        painter
            .with_clip_rect(Rect::from_min_size(
                pos2(x0, draw.top() + 6.0),
                vec2(tw, 24.0),
            ))
            .galley(pos2(x0, draw.top() + 8.0), tg, p.text_primary);
        let mut bx = x0 + tw + 10.0;
        let badge = match it.kind() {
            Kind::Game => Some((p.badge_game, t.t("kind.game"))),
            Kind::Update => Some((p.badge_update, t.t("kind.update"))),
            Kind::Dlc => Some((p.badge_dlc, t.t("kind.dlc"))),
            Kind::Other(c) if c != "?" => Some(((p.bg_surface2, p.text_secondary), c)),
            _ => None,
        };
        if let Some((colors, label)) = badge {
            let g = painter.layout_no_wrap(
                label.to_uppercase(),
                fonts::semibold(size::BADGE),
                colors.1,
            );
            let br = Rect::from_min_size(pos2(bx, draw.top() + 10.0), g.size() + vec2(14.0, 6.0));
            painter.rect_filled(br, br.height() / 2.0, colors.0);
            painter.galley(br.center() - g.size() / 2.0, g, colors.1);
            bx = br.right() + 8.0;
        }
        if let Some(v) = &it.version {
            painter.text(
                pos2(bx, draw.top() + 18.0),
                Align2::LEFT_CENTER,
                format!("v{v}"),
                fonts::regular(size::CAPTION),
                p.text_secondary,
            );
        }
        if it.total_size() > 0 {
            painter.text(
                pos2(right_col, draw.top() + 18.0),
                Align2::RIGHT_CENTER,
                t.size(it.total_size()),
                fonts::regular(size::CAPTION),
                p.text_secondary,
            );
        }
        // line 2: Title ID · context (error, precheck info, split, Content ID)
        let ctx_line = if let Some(e) = &it.error {
            format!("{} {}", e.code, crate::errors::describe(t, e, "what"))
        } else if let Some(pre) = it
            .pre
            .as_ref()
            .filter(|_| matches!(it.state, State::Ready | State::Queued))
        {
            match pre {
                Pre::NotInstalled => t.t("pre.not_installed"),
                Pre::Installed => t.t("pre.installed"),
                Pre::BaseMissing => t.t("pre.base_missing"),
                Pre::WaitsFor(b) => {
                    let n = items
                        .iter()
                        .filter(|i| !i.state.is_finished() && i.state != State::Invalid)
                        .position(|i| &i.id == b)
                        .map_or(0, |x| x + 1);
                    t.t_args("pre.waits_for", &[("n", &n.to_string())])
                }
            }
        } else if let Some(n) = it.note.as_deref().and_then(|n| n.strip_prefix("split: ")) {
            t.t_args("item.split", &[("n", n)])
        } else {
            it.content_id.clone()
        };
        let line2 = if it.title_id.is_empty() {
            ctx_line
        } else {
            format!("{} · {}", it.title_id, ctx_line)
        };
        let warn_pre =
            matches!(it.pre, Some(Pre::BaseMissing | Pre::Installed)) && it.state == State::Ready;
        let l2c = if it.error.is_some() {
            p.danger
        } else if warn_pre {
            p.warning
        } else {
            p.text_secondary
        };
        painter
            .with_clip_rect(Rect::from_min_max(
                pos2(x0, draw.top()),
                pos2(right_col, draw.bottom()),
            ))
            .text(
                pos2(x0, draw.top() + 36.0),
                Align2::LEFT_CENTER,
                line2,
                fonts::regular(size::CAPTION),
                l2c,
            );
        // line 3: status, progress, numbers
        let st_text = if self.pausing.contains(&it.id) {
            t.t("act.pausing")
        } else {
            t.t(&format!("state.{:?}", it.state))
        };
        let sg = painter.layout_job(fonts::job(sic, &st_text, fonts::regular(size::CAPTION), sc));
        let sw = sg.size().x;
        painter.galley(pos2(x0, draw.top() + 56.0 - sg.size().y / 2.0), sg, sc);
        let mut px = x0 + sw.max(110.0) + 12.0;
        if matches!(
            it.state,
            State::Checking
                | State::Starting
                | State::Transferring
                | State::Installing
                | State::Paused
                | State::Uninstalling
        ) {
            let bw = 200.0;
            let bar = Rect::from_min_size(pos2(px, draw.top() + 52.0), vec2(bw, 8.0));
            let mut bui = cui.new_child(egui::UiBuilder::new().max_rect(bar));
            let frac = match it.state {
                State::Installing => Some(1.0),
                State::Transferring | State::Paused => it.live.fraction.map(|f| f as f32),
                _ => None,
            };
            widgets::progress(&mut bui, p, frac, bw, egui::Id::unique(("prog", &it.id)));
            px += bw + 12.0;
            // fixed-width columns so the numbers do not dance (DESIGN 2.3)
            let mut nums: Vec<(f32, String)> = Vec::new();
            if let Some(f) = it
                .live
                .fraction
                .filter(|_| matches!(it.state, State::Transferring | State::Paused))
            {
                nums.push((44.0, format!("{:>3.0} %", f * 100.0)));
            }
            if let Some(s) = it.live.speed.filter(|_| it.state == State::Transferring) {
                nums.push((84.0, t.t_args("unit.speed", &[("size", &t.size(s as u64))])));
            }
            if let Some(e) = it.live.eta.filter(|_| it.state == State::Transferring) {
                nums.push((76.0, fmt_eta(t, e)));
            }
            for (w, n) in nums {
                painter.text(
                    pos2(px + w, draw.top() + 56.0),
                    Align2::RIGHT_CENTER,
                    n,
                    fonts::mono(size::CAPTION),
                    p.text_secondary,
                );
                px += w + 10.0;
            }
        }
        if !is_ghost {
            let ar = Rect::from_min_max(
                pos2(px.max(right_col - 330.0), draw.top() + 38.0),
                pos2(right_col + 4.0, draw.bottom() - 2.0),
            );
            let mut aui = cui.new_child(
                egui::UiBuilder::new()
                    .max_rect(ar)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            self.actions(&mut aui, p, t, it, tx);
        }
        drop
    }

    fn remove(&mut self, it: &Item, tx: &Sender<Command>) {
        if it.state.is_active() {
            self.confirm_remove = Some(it.id.clone());
        } else {
            let _ = tx.send(Command::Remove(it.id.clone()));
        }
    }

    fn actions(&mut self, ui: &mut Ui, p: &Palette, t: &I18n, it: &Item, tx: &Sender<Command>) {
        let id = it.id.clone();
        let btn = |ui: &mut Ui, kind, ic, label: &str| {
            widgets::button(ui, p, kind, Some(ic), label).clicked()
        };
        match it.state {
            State::Transferring => {
                if btn(ui, BKind::Ghost, icon::PROHIBIT, &t.t("act.cancel")) {
                    let _ = tx.send(Command::Cancel(id.clone()));
                }
                let pausing = self.pausing.contains(&id);
                if widgets::button_enabled(
                    ui,
                    p,
                    BKind::Secondary,
                    Some(icon::PAUSE),
                    &t.t(if pausing { "act.pausing" } else { "act.pause" }),
                    !pausing,
                )
                .clicked()
                {
                    self.pausing.insert(id.clone());
                    let _ = tx.send(Command::Pause(id));
                }
            }
            State::Paused => {
                if btn(ui, BKind::Ghost, icon::PROHIBIT, &t.t("act.cancel")) {
                    let _ = tx.send(Command::Cancel(id.clone()));
                }
                if btn(ui, BKind::Secondary, icon::PLAY, &t.t("act.resume")) {
                    let _ = tx.send(Command::Resume(id));
                }
            }
            State::Starting | State::Installing => {
                if btn(ui, BKind::Ghost, icon::PROHIBIT, &t.t("act.cancel")) {
                    let _ = tx.send(Command::Cancel(id));
                }
            }
            State::AwaitingDecision => {
                // shortcut: inline choice; Faza 4 replaces it with dialog 4.7 (Skip default, apply-to-all)
                if btn(
                    ui,
                    BKind::Danger,
                    icon::ARROWS_CLOCKWISE,
                    &t.t("act.reinstall"),
                ) {
                    let _ = tx.send(Command::Decide {
                        id: id.clone(),
                        decision: Decision::Reinstall,
                        apply_all: false,
                    });
                }
                if btn(ui, BKind::Secondary, icon::SKIP, &t.t("act.skip")) {
                    let _ = tx.send(Command::Decide {
                        id,
                        decision: Decision::Skip,
                        apply_all: false,
                    });
                }
            }
            State::Failed | State::Cancelled | State::Skipped => {
                if btn(ui, BKind::Ghost, icon::TRASH, &t.t("act.remove")) {
                    let _ = tx.send(Command::Remove(id.clone()));
                }
                if btn(
                    ui,
                    BKind::Secondary,
                    icon::ARROWS_CLOCKWISE,
                    &t.t("act.retry"),
                ) {
                    let _ = tx.send(Command::Retry(id.clone()));
                }
                if it.error.is_some() && btn(ui, BKind::Ghost, icon::INFO, &t.t("act.details")) {
                    self.details = Some(id);
                }
            }
            State::Invalid => {
                if btn(ui, BKind::Ghost, icon::TRASH, &t.t("act.remove")) {
                    let _ = tx.send(Command::Remove(id.clone()));
                }
                if btn(ui, BKind::Ghost, icon::INFO, &t.t("act.details")) {
                    self.details = Some(id);
                }
            }
            State::Ready | State::Queued | State::Done => {
                if btn(ui, BKind::Ghost, icon::TRASH, &t.t("act.remove")) {
                    let _ = tx.send(Command::Remove(id));
                }
            }
            State::Parsing | State::Checking | State::Uninstalling => {}
        }
    }

    fn menu(&mut self, ui: &mut Ui, t: &I18n, it: &Item, items: &[&Item], tx: &Sender<Command>) {
        let ix = items.iter().position(|i| i.id == it.id).unwrap_or(0);
        let movable = !it.state.is_active();
        for (key, to) in [
            ("menu.top", 0),
            ("menu.up", ix.saturating_sub(1)),
            ("menu.down", ix + 1),
            ("menu.bottom", items.len()),
        ] {
            if ui
                .add_enabled(movable, egui::Button::new(t.t(key)))
                .clicked()
            {
                let _ = tx.send(Command::Move {
                    id: it.id.clone(),
                    to,
                });
                ui.close();
            }
        }
        ui.separator();
        if let Some(part) = it.parts.first()
            && ui.button(t.t("menu.open_location")).clicked()
        {
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", part.path.display()))
                .spawn();
            ui.close();
        }
        if !it.title_id.is_empty() && ui.button(t.t("menu.copy_title_id")).clicked() {
            ui.ctx().copy_text(it.title_id.clone());
            ui.close();
        }
        ui.separator();
        if ui.button(t.t("act.remove")).clicked() {
            self.remove(it, tx);
            ui.close();
        }
    }

    fn empty_state(&mut self, ui: &mut Ui, p: &Palette, t: &I18n, hovering: usize) {
        let r = ui
            .allocate_rect(ui.available_rect_before_wrap().shrink(4.0), Sense::hover())
            .rect;
        dashed_rect(ui, r, p.border_strong);
        let mut c = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(r)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        c.add_space((r.height() / 2.0 - 70.0).max(16.0));
        c.label(fonts::icon_rt(icon::DOWNLOAD, 48.0).color(p.accent));
        c.add_space(8.0);
        c.label(RichText::new(t.t("queue.empty.title")).font(fonts::semibold(size::HEADING)));
        c.label(RichText::new(t.t("queue.empty.hint")).color(p.text_secondary));
        c.label(
            RichText::new(t.t("queue.empty.text"))
                .color(p.text_secondary)
                .size(size::CAPTION),
        );
        if hovering > 0 {
            self.drop_overlay(ui, p, t, hovering);
        }
    }

    /// Drag-over overlay (DESIGN 4.4).
    fn drop_overlay(&self, ui: &mut Ui, p: &Palette, t: &I18n, n: usize) {
        let r = ui.clip_rect().shrink(8.0);
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("drop"),
        ));
        painter.rect_filled(r, 12, p.overlay);
        if p.dark {
            painter.rect_stroke(
                r.expand(3.0),
                14,
                Stroke::new(4.0, p.accent.gamma_multiply(0.25)),
                egui::StrokeKind::Outside,
            );
        }
        painter.rect_stroke(r, 12, Stroke::new(2.0, p.accent), egui::StrokeKind::Inside);
        painter.text(
            r.center() - vec2(0.0, 36.0),
            Align2::CENTER_CENTER,
            icon::DOWNLOAD,
            fonts::icons(48.0),
            p.accent,
        );
        painter.text(
            r.center() + vec2(0.0, 14.0),
            Align2::CENTER_CENTER,
            t.t("drop.title"),
            fonts::semibold(size::HEADING),
            p.text_primary,
        );
        painter.text(
            r.center() + vec2(0.0, 38.0),
            Align2::CENTER_CENTER,
            t.plural("drop.count", n as u64),
            fonts::regular(size::BODY),
            p.text_secondary,
        );
    }

    fn toast_ui(&mut self, ui: &mut Ui, p: &Palette, now: f64) {
        let Some((msg, at)) = &self.toast else { return };
        if now - at > 4.0 {
            self.toast = None;
            return;
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(250));
        let r = ui.clip_rect();
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("toast"),
        ));
        let g = painter.layout_job(fonts::job(
            icon::INFO,
            msg,
            fonts::regular(size::BODY),
            p.text_primary,
        ));
        let tr = Rect::from_center_size(
            pos2(r.center().x, r.bottom() - 36.0),
            g.size() + vec2(32.0, 16.0),
        );
        painter.rect_filled(tr, 10, p.bg_surface2);
        painter.rect_stroke(
            tr,
            10,
            Stroke::new(1.0, p.border_strong),
            egui::StrokeKind::Inside,
        );
        painter.galley(tr.center() - g.size() / 2.0, g, p.text_primary);
    }
}

fn dashed_rect(ui: &Ui, r: Rect, c: Color32) {
    let s = Stroke::new(2.0, c);
    let pts = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for w in pts.windows(2) {
        ui.painter()
            .extend(egui::Shape::dashed_line(&[w[0], w[1]], s, 8.0, 6.0));
    }
}

/// Dropped files and how many are hovering (DESIGN 4.4). `.pkg` filtering happens in the engine.
pub fn dropped(ctx: &egui::Context) -> (Vec<PathBuf>, usize) {
    ctx.input(|i| {
        (
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect(),
            i.raw.hovered_files.len(),
        )
    })
}
