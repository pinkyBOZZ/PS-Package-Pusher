//! Fonts (DESIGN 2.3) and Phosphor icon glyphs (2.5, CP-0: TTF embedded directly).

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const SEMIBOLD: &str = "inter-semibold";
pub const DISPLAY: &str = "display";
pub const ICONS: &str = "icons";

pub fn install(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    let add = |f: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        f.font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(
        &mut f,
        "inter",
        include_bytes!("../../assets/fonts/Inter-Regular.ttf"),
    );
    add(
        &mut f,
        "inter-sb",
        include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"),
    );
    add(
        &mut f,
        "audiowide",
        include_bytes!("../../assets/fonts/Audiowide-Regular.ttf"),
    );
    add(
        &mut f,
        "phosphor",
        include_bytes!("../../assets/fonts/Phosphor.ttf"),
    );
    // Icons get their own family: Inter has glyphs in the private-use area (would shadow Phosphor)
    // and Phosphor maps ASCII letters to ligature glyphs (would hide text), so they cannot share one.
    f.families
        .get_mut(&FontFamily::Proportional)
        .expect("default family")
        .insert(0, "inter".into());
    f.families
        .insert(FontFamily::Name(SEMIBOLD.into()), vec!["inter-sb".into()]);
    f.families
        .insert(FontFamily::Name(ICONS.into()), vec!["phosphor".into()]);
    f.families.insert(
        FontFamily::Name(DISPLAY.into()),
        vec!["audiowide".into(), "inter".into()],
    );
    ctx.set_fonts(f);
}

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY.into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}
pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}
pub fn icons(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ICONS.into()))
}

/// Icon glyph as RichText in the icon font.
pub fn icon_rt(icon: &str, size: f32) -> egui::RichText {
    egui::RichText::new(icon).font(icons(size))
}

/// `icon  text` as one layout job: icon in Phosphor, text in `font`, same colour.
pub fn job(icon: &str, text: &str, font: FontId, color: egui::Color32) -> egui::text::LayoutJob {
    let mut j = egui::text::LayoutJob::default();
    let fmt = |f: FontId| egui::TextFormat {
        font_id: f,
        color,
        valign: egui::Align::Center,
        ..Default::default()
    };
    if !icon.is_empty() {
        j.append(icon, 0.0, fmt(icons(font.size)));
    }
    if !text.is_empty() {
        j.append(
            text,
            if icon.is_empty() {
                0.0
            } else {
                font.size * 0.5
            },
            fmt(font),
        );
    }
    j
}

/// Type scale (DESIGN 2.3), in points.
pub mod size {
    pub const DISPLAY: f32 = 26.0;
    pub const TITLE: f32 = 20.0;
    pub const HEADING: f32 = 16.0;
    pub const BODY: f32 = 14.0;
    pub const CAPTION: f32 = 12.0;
    pub const BADGE: f32 = 11.0;
    pub const MONO: f32 = 13.0;
}

/// Phosphor 2.1.2 regular codepoints (from the package's style.css).
pub mod icon {
    pub const SPINNER: &str = "\u{e66a}";
    pub const FILE_X: &str = "\u{e23c}";
    pub const PACKAGE: &str = "\u{e390}";
    pub const HOURGLASS: &str = "\u{e2b2}";
    pub const SEARCH: &str = "\u{e30c}";
    pub const WARNING: &str = "\u{e4e0}";
    pub const ARROWS_CLOCKWISE: &str = "\u{e094}";
    pub const UPLOAD: &str = "\u{e4c0}";
    pub const DOWNLOAD: &str = "\u{e20c}";
    pub const PAUSE: &str = "\u{e39e}";
    pub const PLAY: &str = "\u{e3d0}";
    pub const CHECK: &str = "\u{e182}";
    pub const WARNING_OCTAGON: &str = "\u{e4e4}";
    pub const PROHIBIT: &str = "\u{e3de}";
    pub const SKIP: &str = "\u{e5a6}";
    pub const TRASH: &str = "\u{e4a6}";
    pub const PLUS: &str = "\u{e3d4}";
    pub const FOLDER_PLUS: &str = "\u{e258}";
    pub const FOLDER_OPEN: &str = "\u{e256}";
    pub const GEAR: &str = "\u{e272}";
    pub const LIST_CHECKS: &str = "\u{eadc}";
    pub const SCROLL: &str = "\u{eb7a}";
    pub const COPY: &str = "\u{e1ca}";
    pub const X: &str = "\u{e4f6}";
    pub const CARET_DOWN: &str = "\u{e136}";
    pub const CARET_RIGHT: &str = "\u{e13a}";
    pub const CIRCLE: &str = "\u{e18a}";
    pub const PLUG: &str = "\u{eb5c}";
    pub const INFO: &str = "\u{e2ce}";
    pub const UNDO: &str = "\u{e038}";
    pub const SAVE: &str = "\u{e248}";
    pub const SHIELD_CHECK: &str = "\u{e40c}";
    pub const STOP: &str = "\u{e46c}";
}
