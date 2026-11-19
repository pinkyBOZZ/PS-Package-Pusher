//! Queue tab. Faza 2: empty state and a read-only list of saved items; the full list comes in Faza 3.

use super::{
    fonts::{self, icon, size},
    theme::Palette,
    widgets::{self, Kind},
};
use crate::{engine::queue::Item, i18n::I18n};
use eframe::egui::{self, RichText, Ui};

pub fn ui(ui: &mut Ui, p: &Palette, t: &I18n, items: &[&Item]) {
    if items.is_empty() {
        empty_state(ui, p, t);
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for it in items {
                widgets::card(ui, p, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            fonts::icon_rt(icon::PACKAGE, size::TITLE).color(p.text_secondary),
                        );
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(it.title.clone().unwrap_or_default())
                                    .font(fonts::semibold(size::HEADING)),
                            );
                            ui.label(
                                RichText::new(format!("{} · {:?}", it.title_id, it.state))
                                    .color(p.text_secondary)
                                    .size(size::CAPTION),
                            );
                        });
                    });
                });
                widgets::space(ui, 1);
            }
        });
}

fn empty_state(ui: &mut Ui, p: &Palette, t: &I18n) {
    let avail = ui.available_rect_before_wrap().shrink(8.0);
    let r = ui.allocate_rect(avail, egui::Sense::hover()).rect;
    ui.painter().rect_stroke(
        r,
        12,
        egui::Stroke::new(1.5, p.border_strong),
        egui::StrokeKind::Inside,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(r)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    child.add_space((r.height() / 2.0 - 90.0).max(16.0));
    child.label(fonts::icon_rt(icon::DOWNLOAD, 48.0).color(p.accent));
    child.add_space(8.0);
    child.label(RichText::new(t.t("queue.empty.title")).font(fonts::semibold(size::HEADING)));
    child.label(RichText::new(t.t("queue.empty.text")).color(p.text_secondary));
    child.add_space(16.0);
    child.horizontal(|ui| {
        ui.add_space((r.width() - 320.0).max(0.0) / 2.0);
        // adding arrives in Faza 3 (DESIGN 6.1); buttons are shown disabled so the layout is final
        widgets::button_enabled(
            ui,
            p,
            Kind::Primary,
            Some(icon::PLUS),
            &t.t("queue.add_files"),
            false,
        );
        widgets::button_enabled(
            ui,
            p,
            Kind::Secondary,
            Some(icon::FOLDER_PLUS),
            &t.t("queue.add_folder"),
            false,
        );
    });
}
