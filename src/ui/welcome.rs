use std::path::Path;

use egui::{Color32, RichText, Stroke, Ui};

pub enum WelcomeAction {
    Install,
    Update,
    Remove,
}

pub fn show(
    ui: &mut Ui,
    existing_dir: Option<&Path>,
    installed_version: Option<&str>,
    remote_version: Option<&str>,
    total_install_size: Option<u64>,
) -> Option<WelcomeAction> {
    let mut action = None;

    ui.add_space(16.0);

    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("Kadr Image Viewer")
                .size(24.0)
                .color(Color32::from_gray(225))
                .strong(),
        );
        ui.add_space(6.0);

        if let Some(dir) = existing_dir {
            let installed_str = installed_version.unwrap_or("unknown");
            let this_str = remote_version.unwrap_or("…");
            ui.label(
                RichText::new(format!(
                    "Installed: v{}    →    This installer: v{}",
                    installed_str, this_str
                ))
                .size(11.5)
                .color(Color32::from_gray(90))
                .monospace(),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!("{}", dir.display()))
                    .size(10.5)
                    .color(Color32::from_gray(58))
                    .monospace(),
            );
        } else {
            ui.label(
                RichText::new(format!("v{}  ·  Fast, minimal image viewer", remote_version.unwrap_or("…")))
                    .size(13.0)
                    .color(Color32::from_gray(100)),
            );
        }

        ui.add_space(6.0);

        // Network + size info
        let size_str = if existing_dir.is_none() {
            match total_install_size {
                Some(total) => format!("Requires internet connection  ·  ~{} disk space", fmt_size_abs(total)),
                None => "Requires internet connection  ·  calculating size…".to_owned(),
            }
        } else {
            "Requires internet connection".to_owned()
        };
        ui.label(
            RichText::new(size_str)
                .size(11.0)
                .color(Color32::from_gray(70)),
        );
    });

    ui.add_space(22.0);

    // ── Buttons ───────────────────────────────────────────────────────────────
    let btn_w = 400.0;
    let left_pad = (ui.available_width() - btn_w) / 2.0;

    if existing_dir.is_some() {
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Update",
                "View release notes and update",
                Color32::from_rgb(99, 155, 255),
                Color32::from_rgba_premultiplied(99, 155, 255, 40),
            ) {
                action = Some(WelcomeAction::Update);
            }
        });
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Install",
                "Reinstall or change install options",
                Color32::from_rgb(140, 115, 185),
                Color32::from_rgba_premultiplied(140, 115, 185, 28),
            ) {
                action = Some(WelcomeAction::Install);
            }
        });
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Remove",
                "Uninstall Kadr from this computer",
                Color32::from_rgb(205, 80, 110),
                Color32::from_rgba_premultiplied(180, 60, 90, 22),
            ) {
                action = Some(WelcomeAction::Remove);
            }
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_space(left_pad);
            if action_row(
                ui, btn_w,
                "Install",
                "Set up Kadr on this computer",
                Color32::from_rgb(99, 155, 255),
                Color32::from_rgba_premultiplied(99, 155, 255, 38),
            ) {
                action = Some(WelcomeAction::Install);
            }
        });
    }

    action
}

fn fmt_size_abs(bytes: u64) -> String {
    if bytes < 1_000_000 {
        format!("{} KB", (bytes + 500) / 1_000)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }
}

fn action_row(
    ui: &mut Ui,
    width: f32,
    title: &str,
    subtitle: &str,
    accent: Color32,
    bg: Color32,
) -> bool {
    let height = 58.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    let fill = if response.hovered() {
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

    let bar = egui::Rect::from_min_size(rect.min, egui::vec2(6.0, height));
    ui.painter().rect_filled(bar, egui::epaint::CornerRadiusF32 { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }, accent);

    let text_x = rect.min.x + 22.0;
    ui.painter().text(
        egui::pos2(text_x, rect.min.y + 13.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(14.0),
        Color32::from_gray(220),
    );
    ui.painter().text(
        egui::pos2(text_x, rect.min.y + 32.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(11.5),
        Color32::from_gray(100),
    );

    response.clicked()
}
