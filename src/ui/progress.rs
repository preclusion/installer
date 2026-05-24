use egui::{Color32, RichText, Stroke, Ui};

use crate::app::ProgressState;

pub enum ProgressAction {
    Continue,
}

pub fn show(ui: &mut Ui, state: &ProgressState) -> Option<ProgressAction> {
    let mut action = None;

    ui.add_space(16.0);

    ui.vertical_centered(|ui| {
        let label = if state.finished {
            if state.error.is_some() { "Installation failed" } else { "Installing… done!" }
        } else {
            "Installing…"
        };
        ui.label(RichText::new(label).size(16.0).color(Color32::from_gray(200)));
    });

    ui.add_space(12.0);

    // Progress bar
    let bar_height = 8.0;
    let (bar_rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width() - 40.0, bar_height),
        egui::Sense::hover(),
    );
    let full = bar_rect;
    let filled = egui::Rect::from_min_size(
        full.min,
        egui::vec2(full.width() * state.fraction, full.height()),
    );

    ui.painter().rect_filled(full, 4.0, Color32::from_rgb(22, 22, 28));
    ui.painter().rect_filled(filled, 4.0, Color32::from_rgb(99, 155, 255));

    ui.add_space(12.0);

    // Log area
    egui::ScrollArea::vertical()
        .id_salt("install_log")
        .max_height(200.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for line in &state.log {
                let color = if line.to_lowercase().contains("error") || line.to_lowercase().contains("fail") {
                    Color32::from_rgb(255, 120, 100)
                } else if line == "Done!" || line.contains("complete") {
                    Color32::from_rgb(120, 210, 140)
                } else {
                    Color32::from_gray(150)
                };
                ui.label(RichText::new(line).size(11.5).color(color).monospace());
            }
        });

    if state.finished {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (btn_text, btn_color) = if state.error.is_some() {
                    ("Close", Color32::from_gray(160))
                } else {
                    ("Continue", Color32::from_rgb(145, 190, 255))
                };
                let btn = egui::Button::new(RichText::new(btn_text).color(btn_color))
                    .fill(Color32::from_rgba_premultiplied(99, 155, 255, 38))
                    .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 130)));
                if ui.add(btn).clicked() {
                    action = Some(ProgressAction::Continue);
                }
            });
        });
    }

    action
}
