//! Palette (DESIGN 2.2 tokens) for dark/light, linear-RGB lerp, mapping onto egui visuals.

use eframe::egui::{self, Color32, CornerRadius, Rgba, Stroke};

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// All colour tokens. Names follow DESIGN 2.2 (`bg.base` → `bg_base`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub bg_base: Color32,
    pub bg_surface: Color32,
    pub bg_surface2: Color32,
    pub bg_hover: Color32,
    pub bg_selected: Color32,
    pub overlay: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_disabled: Color32,
    pub accent: Color32,
    pub accent_on: Color32,
    /// magenta fill/graphics
    pub accent2: Color32,
    /// magenta usable as text
    pub accent2_text: Color32,
    pub purple_text: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub danger_on: Color32,
    pub info: Color32,
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub focus: Color32,
    pub progress_a: Color32,
    pub progress_b: Color32,
    pub track: Color32,
    pub badge_game: (Color32, Color32),
    pub badge_update: (Color32, Color32),
    pub badge_dlc: (Color32, Color32),
    pub banner_warning: Color32,
    pub banner_danger: Color32,
    pub banner_info: Color32,
    pub banner_success: Color32,
}

pub const SUNSET: [Color32; 3] = [hex(0xFFD23D), hex(0xFF9E3D), hex(0xFF2E97)];

pub const DARK: Palette = Palette {
    dark: true,
    bg_base: hex(0x0D0221),
    bg_surface: hex(0x1A0638),
    bg_surface2: hex(0x261052),
    bg_hover: hex(0x33166B),
    bg_selected: hex(0x2E1260),
    overlay: Color32::from_rgba_premultiplied(8, 1, 20, 153), // #0D0221 @ 60 %
    text_primary: hex(0xF2EAFF),
    text_secondary: hex(0xB8A8D9),
    text_disabled: hex(0x7A6A9E),
    accent: hex(0x00F0FF),
    accent_on: hex(0x0D0221),
    accent2: hex(0xFF2E97),
    accent2_text: hex(0xFF5CAD),
    purple_text: hex(0xB98CFF),
    success: hex(0x3DFFA8),
    warning: hex(0xFFD23D),
    danger: hex(0xFF5C7A),
    danger_on: hex(0x0D0221),
    info: hex(0x5CC8FF),
    border_subtle: hex(0x3A1F6B),
    border_strong: hex(0x8B74C2),
    focus: hex(0x00F0FF),
    progress_a: hex(0x00F0FF),
    progress_b: hex(0xFF2E97),
    track: hex(0x261052),
    badge_game: (hex(0x00F0FF), hex(0x0D0221)),
    badge_update: (hex(0xFF9E3D), hex(0x0D0221)),
    badge_dlc: (hex(0xB98CFF), hex(0x0D0221)),
    banner_warning: hex(0x2E2208),
    banner_danger: hex(0x3A0A1E),
    banner_info: hex(0x08283A),
    banner_success: hex(0x082E22),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    bg_base: hex(0xF7F3FF),
    bg_surface: hex(0xFFFFFF),
    bg_surface2: hex(0xEFE7FF),
    bg_hover: hex(0xF1EBFF),
    bg_selected: hex(0xEAE0FF),
    overlay: Color32::from_rgba_premultiplied(10, 2, 22, 102), // #1A0638 @ 40 %
    text_primary: hex(0x1A0638),
    text_secondary: hex(0x584878),
    text_disabled: hex(0x9A8DB8),
    accent: hex(0x006A73),
    accent_on: hex(0xFFFFFF),
    accent2: hex(0xB3005F),
    accent2_text: hex(0xB3005F),
    purple_text: hex(0x6A1FE0),
    success: hex(0x0A6640),
    warning: hex(0x8A5300),
    danger: hex(0xB0142F),
    danger_on: hex(0xFFFFFF),
    info: hex(0x005FA8),
    border_subtle: hex(0xDCD0F5),
    border_strong: hex(0x7F6CA8),
    focus: hex(0xB3005F),
    progress_a: hex(0x006A73),
    progress_b: hex(0xB3005F),
    track: hex(0xE3D6FF),
    badge_game: (hex(0x006A73), hex(0xFFFFFF)),
    badge_update: (hex(0x8A5300), hex(0xFFFFFF)),
    badge_dlc: (hex(0x6A1FE0), hex(0xFFFFFF)),
    banner_warning: hex(0xFFF4D6),
    banner_danger: hex(0xFFE3E8),
    banner_info: hex(0xE0F0FF),
    banner_success: hex(0xDDF7EA),
};

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let (a, b) = (Rgba::from(a), Rgba::from(b));
    Color32::from(a * (1.0 - t) + b * t)
}

impl Palette {
    /// Per-channel blend in linear RGB (DESIGN 2.2 "Pretapanje teme").
    pub fn lerp(&self, o: &Palette, t: f32) -> Palette {
        let t = t.clamp(0.0, 1.0);
        let m = |a, b| mix(a, b, t);
        let p = |a: (Color32, Color32), b: (Color32, Color32)| (mix(a.0, b.0, t), mix(a.1, b.1, t));
        Palette {
            dark: if t < 0.5 { self.dark } else { o.dark },
            bg_base: m(self.bg_base, o.bg_base),
            bg_surface: m(self.bg_surface, o.bg_surface),
            bg_surface2: m(self.bg_surface2, o.bg_surface2),
            bg_hover: m(self.bg_hover, o.bg_hover),
            bg_selected: m(self.bg_selected, o.bg_selected),
            overlay: m(self.overlay, o.overlay),
            text_primary: m(self.text_primary, o.text_primary),
            text_secondary: m(self.text_secondary, o.text_secondary),
            text_disabled: m(self.text_disabled, o.text_disabled),
            accent: m(self.accent, o.accent),
            accent_on: m(self.accent_on, o.accent_on),
            accent2: m(self.accent2, o.accent2),
            accent2_text: m(self.accent2_text, o.accent2_text),
            purple_text: m(self.purple_text, o.purple_text),
            success: m(self.success, o.success),
            warning: m(self.warning, o.warning),
            danger: m(self.danger, o.danger),
            danger_on: m(self.danger_on, o.danger_on),
            info: m(self.info, o.info),
            border_subtle: m(self.border_subtle, o.border_subtle),
            border_strong: m(self.border_strong, o.border_strong),
            focus: m(self.focus, o.focus),
            progress_a: m(self.progress_a, o.progress_a),
            progress_b: m(self.progress_b, o.progress_b),
            track: m(self.track, o.track),
            badge_game: p(self.badge_game, o.badge_game),
            badge_update: p(self.badge_update, o.badge_update),
            badge_dlc: p(self.badge_dlc, o.badge_dlc),
            banner_warning: m(self.banner_warning, o.banner_warning),
            banner_danger: m(self.banner_danger, o.banner_danger),
            banner_info: m(self.banner_info, o.banner_info),
            banner_success: m(self.banner_success, o.banner_success),
        }
    }

    /// egui's own widgets (text edits, checkboxes, scroll bars) take their colours from here.
    pub fn visuals(&self) -> egui::Visuals {
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.override_text_color = Some(self.text_primary);
        v.weak_text_color = Some(self.text_secondary);
        v.hyperlink_color = self.accent;
        v.panel_fill = self.bg_base;
        v.window_fill = self.bg_surface;
        v.window_stroke = Stroke::new(1.0, self.border_subtle);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(8);
        v.extreme_bg_color = self.bg_surface2;
        v.text_edit_bg_color = Some(self.bg_surface2);
        v.faint_bg_color = self.bg_surface;
        v.code_bg_color = self.bg_surface2;
        v.warn_fg_color = self.warning;
        v.error_fg_color = self.danger;
        v.selection.bg_fill = self.bg_selected;
        v.selection.stroke = Stroke::new(1.0, self.accent);
        let w = &mut v.widgets;
        let r = CornerRadius::same(8);
        w.noninteractive.bg_fill = self.bg_surface;
        w.noninteractive.weak_bg_fill = self.bg_surface;
        w.noninteractive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.text_primary);
        for (wv, bg, stroke) in [
            (&mut w.inactive, self.bg_surface2, self.border_strong),
            (&mut w.hovered, self.bg_hover, self.border_strong),
            (&mut w.active, self.bg_selected, self.accent),
            (&mut w.open, self.bg_selected, self.accent),
        ] {
            wv.bg_fill = bg;
            wv.weak_bg_fill = bg;
            wv.bg_stroke = Stroke::new(1.0, stroke);
            wv.fg_stroke = Stroke::new(1.5, self.text_primary);
            wv.corner_radius = r;
            wv.expansion = 0.0;
        }
        v
    }
}

/// Follows the Windows theme with a 300 ms cross-fade (DESIGN 9.7, `motion.theme`).
pub struct ThemeState {
    from: Palette,
    to: Palette,
    started: f64,
    pub current: Palette,
    applied: Option<Palette>,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            from: DARK,
            to: DARK,
            started: f64::NEG_INFINITY,
            current: DARK,
            applied: None,
        }
    }
}

impl ThemeState {
    /// Call once per frame. Returns true while the fade is running (caller requests repaint).
    /// `force_dark`: debug override (`--theme`), otherwise follows `ctx.system_theme()`.
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        now: f64,
        duration: f64,
        force_dark: Option<bool>,
    ) -> bool {
        let want = match force_dark
            .map(|d| {
                if d {
                    egui::Theme::Dark
                } else {
                    egui::Theme::Light
                }
            })
            .or(ctx.system_theme())
        {
            Some(egui::Theme::Light) => LIGHT,
            _ => DARK,
        };
        if want.dark != self.to.dark {
            self.from = self.current;
            self.to = want;
            self.started = if self.applied.is_none() {
                f64::NEG_INFINITY
            } else {
                now
            };
        }
        let t = if duration <= 0.0 {
            1.0
        } else {
            ((now - self.started) / duration).clamp(0.0, 1.0) as f32
        };
        let eased = egui::emath::easing::cubic_in_out(t);
        self.current = self.from.lerp(&self.to, eased);
        if self.applied != Some(self.current) {
            let v = self.current.visuals();
            ctx.set_visuals_of(egui::Theme::Dark, v.clone());
            ctx.set_visuals_of(egui::Theme::Light, v);
            self.applied = Some(self.current);
        }
        t < 1.0
    }
}

/// WCAG 2.x contrast ratio.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lum = |c: Color32| {
        let ch = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLAN 2 acceptance: text ≥ 4.5 and UI ≥ 3 against every background (DESIGN 2.2 tables).
    #[test]
    fn wcag_thresholds() {
        for p in [DARK, LIGHT] {
            let bgs = [p.bg_base, p.bg_surface, p.bg_surface2, p.bg_selected];
            for (name, fg, min) in [
                ("text_primary", p.text_primary, 4.5),
                ("text_secondary", p.text_secondary, 4.5),
                ("accent", p.accent, 4.5),
                ("accent2_text", p.accent2_text, 4.5),
                ("purple_text", p.purple_text, 4.5),
                ("success", p.success, 4.5),
                ("warning", p.warning, 4.5),
                ("danger", p.danger, 4.5),
                ("info", p.info, 4.5),
                ("border_strong", p.border_strong, 3.0),
                ("focus", p.focus, 3.0),
            ] {
                for bg in bgs {
                    let c = contrast(fg, bg);
                    assert!(
                        c >= min,
                        "{} {name} on {bg:?}: {c:.2} < {min}",
                        if p.dark { "dark" } else { "light" }
                    );
                }
            }
            for (fill, text) in [
                p.badge_game,
                p.badge_update,
                p.badge_dlc,
                (p.accent, p.accent_on),
                (p.danger, p.danger_on),
            ] {
                assert!(contrast(fill, text) >= 4.5);
            }
            for (bg, title) in [
                (p.banner_warning, p.warning),
                (p.banner_danger, p.danger),
                (p.banner_info, p.info),
                (p.banner_success, p.success),
            ] {
                assert!(
                    contrast(bg, title) >= 4.5
                        && contrast(bg, p.text_primary) >= 4.5
                        && contrast(bg, p.text_secondary) >= 4.5
                );
            }
        }
        // spot check against DESIGN's computed values
        assert!((contrast(DARK.accent2, DARK.bg_base) - 5.81).abs() < 0.01);
        assert!((contrast(LIGHT.accent, LIGHT.bg_surface) - 6.35).abs() < 0.01);
    }

    #[test]
    fn lerp_ends() {
        assert_eq!(DARK.lerp(&LIGHT, 0.0), DARK);
        assert_eq!(DARK.lerp(&LIGHT, 1.0).bg_base, LIGHT.bg_base);
        let mid = DARK.lerp(&LIGHT, 0.5).bg_base;
        assert!(mid != DARK.bg_base && mid != LIGHT.bg_base);
    }
}
