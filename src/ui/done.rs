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

        // Center button row manually — ui.horizontal inside vertical_centered
        // always takes full available width, so items start at the left edge
        // unless we pad explicitly.
        let show_launch = state.success && state.operation != Operation::Uninstall;
        let launch_w = 100.0f32;
        let close_w  =  60.0f32;
        let gap      =   8.0f32;
        let total    = if show_launch { launch_w + gap + close_w } else { close_w };
        let pad      = ((ui.available_width() - total) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            ui.add_space(pad);
            if show_launch {
                let launch_btn = egui::Button::new(
                    RichText::new("Launch Kadr").color(Color32::from_rgb(145, 190, 255)),
                )
                .min_size(egui::vec2(launch_w, 0.0))
                .fill(Color32::from_rgba_premultiplied(99, 155, 255, 45))
                .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 170)));

                if ui.add(launch_btn).clicked() {
                    action = Some(DoneAction::Launch);
                }
                ui.add_space(gap);
            }

            if ui.add(egui::Button::new("Close").min_size(egui::vec2(close_w, 0.0))).clicked() {
                action = Some(DoneAction::Close);
            }
        });
    });

    action
}

fn draw_heart(ui: &mut Ui) {
    let size = 36.0f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());

    let cx = rect.center().x;
    let cy = rect.center().y + 1.5;
    let scale = 0.88f32;

    // Slight clockwise lean — real hand-drawn hearts are never perfectly upright.
    let lean = 0.055f32; // ≈ 3°
    let (sl, cl) = (lean.sin(), lean.cos());

    // Index 0 maps to t = π (bottom tip) so the polygon starts at a convex point.
    let n = 72usize;
    let pts: Vec<egui::Pos2> = (0..n).map(|i| {
        let t = std::f32::consts::TAU * (i as f32 / n as f32) + std::f32::consts::PI;

        let xf = 16.0 * t.sin().powi(3);
        let yf_sym = -(13.0 * t.cos()
            - 5.0 * (2.0 * t).cos()
            - 2.0 * (3.0 * t).cos()
            -       (4.0 * t).cos());

        // Lift the top-center V-dip to remove the fill tessellation artifact.
        // ((t/2).cos())^4  ==  1 at the top-center dip,  0 at the bottom tip.
        let top_lift = ((t * 0.5).cos()).powi(4);
        let yf = yf_sym + 3.2 * top_lift;

        // Two *independent* wobble curves break the left↔right mirror symmetry.
        // Amplitudes in heart-formula units; at this scale ≈ 0.88 px/unit visible.
        let w_r = (t * 2.6 + 0.9).sin() * 1.25
                + (t * 4.1 - 0.4).cos() * 0.52
                + (t * 1.2 + 0.2).sin() * 0.28;
        let w_l = (t * 1.9 - 1.1).sin() * 0.98
                + (t * 3.5 + 0.6).cos() * 0.58
                + (t * 5.0 - 0.8).sin() * 0.22;

        // tanh blend: smooth left↔right transition, no hard seam at x = 0.
        let blend = (xf * 0.22).tanh(); // −1 = full-left, +1 = full-right
        let w = w_l * (1.0 - blend) * 0.5 + w_r * (1.0 + blend) * 0.5;

        let angle = yf.atan2(xf);
        let px = (xf + w * angle.cos()) * scale;
        let py = (yf + w * angle.sin()) * scale;

        // Apply lean (rotation around cx/cy).
        egui::pos2(
            cx + px * cl - py * sl,
            cy + px * sl + py * cl,
        )
    }).collect();

    ui.painter().add(egui::Shape::Path(egui::epaint::PathShape {
        points: pts,
        closed: true,
        fill: Color32::from_rgba_premultiplied(218, 62, 98, 225),
        stroke: egui::epaint::PathStroke::new(1.5, Color32::from_rgb(242, 92, 118)),
    }));
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
