use std::path::PathBuf;

use egui::{Color32, RichText, Stroke, Ui};

pub enum WelcomeAction {
    Install,
    Update,
    Remove,
}

pub fn show(ui: &mut Ui, existing: Option<&PathBuf>) -> Option<WelcomeAction> {
    let mut action = None;

    ui.add_space(20.0);

    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("Kadr Image Viewer")
                .size(26.0)
                .color(Color32::from_gray(225))
                .strong(),
        );
        ui.add_space(6.0);

        if let Some(dir) = existing {
            ui.label(
                RichText::new(format!("Installed at  {}", dir.display()))
                    .size(11.5)
                    .color(Color32::from_gray(85))
                    .monospace(),
            );
        } else {
            ui.label(
                RichText::new("Fast, minimal image viewer for Windows")
                    .size(13.0)
                    .color(Color32::from_gray(110)),
            );
        }
    });

    ui.add_space(28.0);

    let btn_w = 400.0;
    let left_pad = (ui.available_width() - btn_w) / 2.0;

    if existing.is_some() {
        // Update — primary action when already installed
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Update",
                "Replace the binary, keep all settings and shortcuts",
                Color32::from_rgb(99, 155, 255),
                Color32::from_rgba_premultiplied(99, 155, 255, 40),
            ) {
                action = Some(WelcomeAction::Update);
            }
        });
        ui.add_space(8.0);

        // Install — reinstall / change options
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Install",
                "Reinstall or change install options",
                Color32::from_gray(160),
                Color32::from_rgba_premultiplied(255, 255, 255, 8),
            ) {
                action = Some(WelcomeAction::Install);
            }
        });
        ui.add_space(8.0);

        // Remove — destructive, subtle
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Remove",
                "Uninstall Kadr from this computer",
                Color32::from_rgb(220, 100, 90),
                Color32::from_rgba_premultiplied(200, 60, 50, 20),
            ) {
                action = Some(WelcomeAction::Remove);
            }
        });
    } else {
        // Fresh install — single prominent button
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Install",
                "Set up Kadr on this computer",
                Color32::from_rgb(99, 155, 255),
                Color32::from_rgba_premultiplied(99, 155, 255, 40),
            ) {
                action = Some(WelcomeAction::Install);
            }
        });
    }

    action
}

fn action_row(
    ui: &mut Ui,
    width: f32,
    title: &str,
    subtitle: &str,
    accent: Color32,
    bg: Color32,
) -> bool {
    let height = 64.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let fill = if response.hovered() {
        // brighten slightly on hover
        egui::Color32::from_rgba_premultiplied(
            bg.r().saturating_add(15),
            bg.g().saturating_add(15),
            bg.b().saturating_add(15),
            bg.a().saturating_add(10),
        )
    } else {
        bg
    };

    let stroke_alpha = if response.hovered() { 160u8 } else { 60u8 };
    let stroke_color = Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), stroke_alpha);

    ui.painter().rect(rect, 6.0, fill, Stroke::new(1.0, stroke_color), egui::StrokeKind::Outside);

    // Accent left bar
    let bar = egui::Rect::from_min_size(rect.min, egui::vec2(3.0, height));
    ui.painter().rect_filled(bar, egui::epaint::CornerRadiusF32 { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }, accent);

    let text_x = rect.min.x + 20.0;
    ui.painter().text(
        egui::pos2(text_x, rect.min.y + 16.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(15.0),
        Color32::from_gray(220),
    );
    ui.painter().text(
        egui::pos2(text_x, rect.min.y + 36.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(12.0),
        Color32::from_gray(105),
    );

    response.clicked()
}
