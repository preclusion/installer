#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod install;
mod uninstall;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Kadr Installer")
            .with_inner_size([660.0, 520.0])
            .with_min_inner_size([660.0, 520.0])
            .with_max_inner_size([660.0, 520.0])
            .with_resizable(false)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Kadr Installer",
        options,
        Box::new(|cc| Ok(Box::new(app::InstallerApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    // 32×32 RGBA painted icon (same design as kadr icon)
    let size = 32usize;
    let mut pixels = vec![0u8; size * size * 4];
    let fi = size as f32;
    let fi_half = fi / 2.0;

    for y in 0..size {
        for x in 0..size {
            let cx = x as f32 - fi_half + 0.5;
            let cy = y as f32 - fi_half + 0.5;

            let bg_sdf = rrect_sdf(cx, cy, fi_half - 1.0, fi_half - 1.0, fi_half * 0.22);
            let alpha = ((-bg_sdf).clamp(0.0, 1.0) * 255.0) as u8;

            // bg color: rgb(12, 12, 16)
            let mut r = (12u8, 12u8, 16u8, alpha);

            if bg_sdf < 0.0 {
                // frame ring
                let frame_outer = fi_half * 0.78;
                let frame_inner = fi_half * 0.60;
                let frame_sdf_o = rrect_sdf(cx, cy, frame_outer, frame_outer, fi_half * 0.10);
                let frame_sdf_i = rrect_sdf(cx, cy, frame_inner, frame_inner, fi_half * 0.06);
                let on_frame = frame_sdf_o < 0.0 && frame_sdf_i >= 0.0;

                // sun
                let sun_r = fi_half * 0.18;
                let sun_cx = -fi_half * 0.28;
                let sun_cy = -fi_half * 0.20;
                let on_sun = ((cx - sun_cx).powi(2) + (cy - sun_cy).powi(2)).sqrt() < sun_r;

                // mountain
                let in_content = frame_sdf_i < 0.0;
                let mnt_slope = 1.30f32;
                let mnt_lift = fi_half * 0.30;
                let on_mnt = in_content && !on_sun && cy > cx.abs() * mnt_slope - mnt_lift;

                if on_frame || on_sun || on_mnt {
                    r = (192, 196, 210, alpha);
                }
            }

            let idx = (y * size + x) * 4;
            pixels[idx]     = r.0;
            pixels[idx + 1] = r.1;
            pixels[idx + 2] = r.2;
            pixels[idx + 3] = r.3;
        }
    }

    egui::IconData { rgba: pixels, width: size as u32, height: size as u32 }
}

fn rrect_sdf(cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = cx.abs() - hw + r;
    let qy = cy.abs() - hh + r;
    qx.max(0.0).hypot(qy.max(0.0)) + qx.min(0.0).max(qy.min(0.0)) - r
}
