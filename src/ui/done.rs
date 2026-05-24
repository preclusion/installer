use egui::{Color32, RichText, Stroke, Ui};

use crate::app::{DoneState, Operation};

pub enum DoneAction {
    Launch,
    Close,
}

pub fn show(ui: &mut Ui, state: &DoneState) -> Option<DoneAction> {
    let mut action = None;

    ui.add_space(30.0);

    ui.vertical_centered(|ui| {
        draw_status_icon(ui, state.success, state.operation);
        ui.add_space(16.0);

        for line in state.message.lines() {
            ui.label(RichText::new(line).size(15.0).color(Color32::from_gray(210)));
        }

        if state.success && state.operation == Operation::Install {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Installed to: {}", state.install_dir.display()))
                    .size(11.0)
                    .color(Color32::from_gray(85))
                    .monospace(),
            );

            ui.add_space(14.0);
            draw_heart(ui);
            ui.add_space(6.0);
            ui.label(
                RichText::new("Thank you for using Kadr!")
                    .size(13.0)
                    .color(Color32::from_gray(130)),
            );
        }

        ui.add_space(24.0);

        // Centered button row — no manual padding, vertical_centered handles it
        ui.horizontal(|ui| {
            if state.success && state.operation != Operation::Uninstall {
                let launch_btn = egui::Button::new(
                    RichText::new("Launch Kadr").color(Color32::from_rgb(145, 190, 255)),
                )
                .fill(Color32::from_rgba_premultiplied(99, 155, 255, 45))
                .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 170)));

                if ui.add(launch_btn).clicked() {
                    action = Some(DoneAction::Launch);
                }
                ui.add_space(8.0);
            }

            if ui.button("Close").clicked() {
                action = Some(DoneAction::Close);
            }
        });
    });

    action
}

fn draw_heart(ui: &mut Ui) {
    let size = 28.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let color = Color32::from_rgb(155, 80, 220);
    let c = rect.center();

    // Two filled circles for the top bumps
    let r = size * 0.24;
    let bump_y = c.y - size * 0.08;
    p.circle_filled(egui::pos2(c.x - r * 0.72, bump_y), r, color);
    p.circle_filled(egui::pos2(c.x + r * 0.72, bump_y), r, color);

    // Filled triangle for the bottom point
    let tl = egui::pos2(c.x - size * 0.46, c.y);
    let tr = egui::pos2(c.x + size * 0.46, c.y);
    let bot = egui::pos2(c.x, c.y + size * 0.44);
    p.add(egui::Shape::convex_polygon(
        vec![tl, tr, bot],
        color,
        Stroke::NONE,
    ));

    // Bridge rect between bumps and triangle to fill the gap
    let bridge = egui::Rect::from_min_max(
        egui::pos2(c.x - size * 0.46, c.y - size * 0.02),
        egui::pos2(c.x + size * 0.46, c.y + size * 0.06),
    );
    p.rect_filled(bridge, 0.0, color);
}

fn draw_status_icon(ui: &mut Ui, success: bool, operation: Operation) {
    let size = 56.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let r = size / 2.0 - 2.0;

    let (ring_color, mark_color) = if success {
        (Color32::from_rgb(80, 190, 110), Color32::from_rgb(100, 215, 130))
    } else {
        (Color32::from_rgb(200, 80, 70), Color32::from_rgb(230, 100, 90))
    };

    p.circle_stroke(c, r, Stroke::new(2.5, ring_color));

    if success {
        let s = r * 0.42;
        let p1 = c + egui::vec2(-s, 0.0);
        let p2 = c + egui::vec2(-s * 0.25, s * 0.7);
        let p3 = c + egui::vec2(s, -s * 0.6);
        p.line_segment([p1, p2], Stroke::new(2.5, mark_color));
        p.line_segment([p2, p3], Stroke::new(2.5, mark_color));
    } else {
        let s = r * 0.38;
        p.line_segment([c + egui::vec2(-s, -s), c + egui::vec2(s, s)], Stroke::new(2.5, mark_color));
        p.line_segment([c + egui::vec2(s, -s), c + egui::vec2(-s, s)], Stroke::new(2.5, mark_color));
    }

    ui.add_space(4.0);
    ui.label(
        RichText::new(match operation {
            Operation::Install   => "installed",
            Operation::Update    => "updated",
            Operation::Uninstall => "removed",
        })
        .size(10.5)
        .color(Color32::from_gray(75)),
    );
}
