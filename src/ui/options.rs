use egui::{Color32, RichText, Stroke, Ui};

use crate::install::InstallOptions;

pub enum OptionsAction {
    Back,
    Install(InstallOptions),
}

pub fn show(ui: &mut Ui, opts: &mut InstallOptions) -> Option<OptionsAction> {
    let mut action = None;

    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .max_height(330.0)
        .show(ui, |ui| {
            section(ui, "Install Location");
            ui.horizontal(|ui| {
                let path_str = opts.install_dir.to_string_lossy().to_string();
                let mut editable = path_str;
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut editable)
                        .desired_width(380.0)
                        .font(egui::TextStyle::Monospace),
                );
                if resp.changed() {
                    opts.install_dir = std::path::PathBuf::from(&editable);
                }
                if ui.button("Browse…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title("Choose install directory")
                        .pick_folder()
                    {
                        opts.install_dir = dir;
                    }
                }
            });

            ui.add_space(12.0);
            section(ui, "Shortcuts");
            opt_check(ui, &mut opts.desktop_shortcut, "Desktop shortcut");
            opt_check(ui, &mut opts.start_menu_shortcut, "Start Menu shortcut");

            ui.add_space(12.0);
            section(ui, "Integration");
            opt_check(ui, &mut opts.add_to_path, "Add to PATH (use kadr from terminal)");
            opt_check(ui, &mut opts.context_menu, "Right-click context menu (Open with Kadr)");

            ui.add_space(12.0);
            section(ui, "Default Viewer");
            opt_check(ui, &mut opts.default_image_viewer, "Set as default image viewer");
            opt_check(ui, &mut opts.default_video_viewer, "Set as default video viewer");
        });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui.button("Back").clicked() {
            action = Some(OptionsAction::Back);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let btn = egui::Button::new(
                RichText::new("Install").color(Color32::from_rgb(145, 190, 255)),
            )
            .fill(Color32::from_rgba_premultiplied(99, 155, 255, 38))
            .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(99, 155, 255, 160)));
            if ui.add(btn).clicked() {
                action = Some(OptionsAction::Install(opts.clone()));
            }
        });
    });

    action
}

fn section(ui: &mut Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .size(11.0)
            .color(Color32::from_gray(100)),
    );
    ui.add_space(4.0);
}

fn opt_check(ui: &mut Ui, val: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        ui.checkbox(val, label);
    });
}
