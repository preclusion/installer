use std::sync::mpsc;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{
    install::{InstallOptions, InstallProgress},
    ui::{done, options, patchnotes, progress, welcome},
};

pub enum Page {
    Welcome,
    UpdateChecking(mpsc::Receiver<crate::install::UpdateCheckResult>),
    PatchNotes(PatchNotesState),
    Options(InstallOptions),
    Progress(ProgressState),
    Done(DoneState),
}

pub struct PatchNotesState {
    pub pending: Vec<crate::install::PendingUpdate>,
    pub kadr_version: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Operation {
    Install,
    Update,
    Uninstall,
}

pub struct ProgressState {
    pub rx: mpsc::Receiver<InstallProgress>,
    pub log: Vec<String>,
    pub fraction: f32,
    pub finished: bool,
    pub error: Option<String>,
    pub options: InstallOptions,
    pub operation: Operation,
}

pub struct DoneState {
    pub success: bool,
    pub message: String,
    pub install_dir: std::path::PathBuf,
    pub operation: Operation,
}

pub struct InstallerApp {
    pub page: Page,
    pub existing_install: Option<crate::install::ExistingInstall>,
    pub remote_sizes: Arc<Mutex<Option<HashMap<String, u64>>>>,
    pub remote_kadr_version: Arc<Mutex<Option<String>>>,
}

impl InstallerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let remote_sizes: Arc<Mutex<Option<HashMap<String, u64>>>> = Arc::new(Mutex::new(None));
        let sizes_ref = Arc::clone(&remote_sizes);
        let ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            let sizes = crate::install::fetch_remote_sizes();
            *sizes_ref.lock().unwrap() = Some(sizes);
            ctx.request_repaint();
        });

        let remote_kadr_version: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let ver_ref = Arc::clone(&remote_kadr_version);
        let ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            if let Some(r) = crate::install::fetch_release_version() {
                *ver_ref.lock().unwrap() = Some(r);
                ctx.request_repaint();
            }
        });

        Self {
            page: Page::Welcome,
            existing_install: crate::install::detect_existing_install(),
            remote_sizes,
            remote_kadr_version,
        }
    }
}

impl eframe::App for InstallerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        apply_theme(&ctx);

        let frame = egui::Frame::default().fill(egui::Color32::from_rgb(11, 10, 16));
        egui::CentralPanel::default()
            .frame(frame)
            .show_inside(ui, |ui| {
                let kadr_ver = self.remote_kadr_version.lock().unwrap().clone();
                draw_header(ui, kadr_ver.as_deref());
                ui.add_space(8.0);

                match &mut self.page {
                    Page::Welcome => {
                        let existing_dir = self.existing_install.as_ref().map(|e| e.dir.as_path());
                        let installed_ver = self.existing_install.as_ref().and_then(|e| e.version.as_deref());
                        let remote_ver = self.remote_kadr_version.lock().unwrap().clone();
                        let remote = self.remote_sizes.lock().unwrap().clone();
                        let total_size = remote.as_ref().map(|m| m.values().sum::<u64>());
                        if let Some(action) = welcome::show(ui, existing_dir, installed_ver, remote_ver.as_deref(), total_size) {
                            match action {
                                welcome::WelcomeAction::Install => {
                                    self.page = Page::Options(InstallOptions::default());
                                }
                                welcome::WelcomeAction::Update => {
                                    let (tx, rx) = mpsc::channel();
                                    if let Some(existing) = &self.existing_install {
                                        let dir = existing.dir.clone();
                                        let ctx = ctx.clone();
                                        std::thread::spawn(move || {
                                            let result = crate::install::get_pending_updates(&dir);
                                            let _ = tx.send(result);
                                            ctx.request_repaint();
                                        });
                                    } else {
                                        let _ = tx.send(crate::install::UpdateCheckResult { pending: vec![], kadr_version: None });
                                    }
                                    self.page = Page::UpdateChecking(rx);
                                }
                                welcome::WelcomeAction::Remove => {
                                    if let Some(existing) = &self.existing_install {
                                        let (tx, rx) = mpsc::channel();
                                        let dir = existing.dir.clone();
                                        let dir2 = dir.clone();
                                        std::thread::spawn(move || {
                                            crate::uninstall::run_uninstall(&dir, tx);
                                        });
                                        let mut opts = InstallOptions::default();
                                        opts.install_dir = dir2;
                                        self.page = Page::Progress(ProgressState {
                                            rx,
                                            log: Vec::new(),
                                            fraction: 0.0,
                                            finished: false,
                                            error: None,
                                            options: opts,
                                            operation: Operation::Uninstall,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    Page::UpdateChecking(rx) => {
                        if let Ok(result) = rx.try_recv() {
                            self.page = Page::PatchNotes(PatchNotesState {
                                pending: result.pending,
                                kadr_version: result.kadr_version,
                            });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("Checking for updates…");
                            });
                            ctx.request_repaint_after(std::time::Duration::from_millis(50));
                        }
                    }

                    Page::PatchNotes(state) => {
                        let all_up_to_date = state.pending.is_empty();
                        let remote = self.remote_sizes.lock().unwrap().clone();
                        let size_delta = remote.as_ref().map(|remote_map| {
                            let install_dir = self.existing_install.as_ref().map(|e| e.dir.as_path());
                            state.pending.iter().map(|update| {
                                let filename = crate::install::filename_from_url(update.entry.url);
                                let remote_size = remote_map.get(filename).copied().unwrap_or(0) as i64;
                                let local_size = install_dir
                                    .and_then(|d| std::fs::metadata(d.join(filename)).ok())
                                    .map(|m| m.len() as i64)
                                    .unwrap_or(0);
                                remote_size - local_size
                            }).sum::<i64>()
                        });
                        if let Some(action) = patchnotes::show(ui, size_delta, all_up_to_date, state.kadr_version.as_deref()) {
                            match action {
                                patchnotes::PatchNotesAction::Back => {
                                    self.page = Page::Welcome;
                                }
                                patchnotes::PatchNotesAction::Confirm => {
                                    if let Some(existing) = &self.existing_install {
                                        let (tx, rx) = mpsc::channel();
                                        let dir = existing.dir.clone();
                                        let dir2 = dir.clone();
                                        std::thread::spawn(move || {
                                            crate::install::run_update(&dir, tx);
                                        });
                                        let mut opts = InstallOptions::default();
                                        opts.install_dir = dir2;
                                        self.page = Page::Progress(ProgressState {
                                            rx,
                                            log: Vec::new(),
                                            fraction: 0.0,
                                            finished: false,
                                            error: None,
                                            options: opts,
                                            operation: Operation::Update,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    Page::Options(opts) => {
                        if let Some(action) = options::show(ui, opts) {
                            match action {
                                options::OptionsAction::Back => {
                                    self.page = Page::Welcome;
                                }
                                options::OptionsAction::Install(opts) => {
                                    let (tx, rx) = mpsc::channel();
                                    let opts_clone = opts.clone();
                                    std::thread::spawn(move || {
                                        crate::install::run_install(&opts_clone, tx);
                                    });
                                    self.page = Page::Progress(ProgressState {
                                        rx,
                                        log: Vec::new(),
                                        fraction: 0.0,
                                        finished: false,
                                        error: None,
                                        options: opts,
                                        operation: Operation::Install,
                                    });
                                }
                            }
                        }
                    }

                    Page::Progress(state) => {
                        while let Ok(msg) = state.rx.try_recv() {
                            match msg {
                                InstallProgress::Log(s) => state.log.push(s),
                                InstallProgress::Step(f) => state.fraction = f,
                                InstallProgress::Done => {
                                    state.fraction = 1.0;
                                    state.finished = true;
                                }
                                InstallProgress::Error(e) => {
                                    state.error = Some(e);
                                    state.finished = true;
                                }
                            }
                        }

                        if state.finished {
                            ctx.request_repaint();
                        } else {
                            ctx.request_repaint_after(std::time::Duration::from_millis(50));
                        }

                        if let Some(action) = progress::show(ui, state) {
                            match action {
                                progress::ProgressAction::Continue => {
                                    let success = state.error.is_none();
                                    let msg = if success {
                                        match state.operation {
                                            Operation::Install => "Kadr was installed successfully!".to_owned(),
                                            Operation::Update  => "Kadr was updated successfully!".to_owned(),
                                            Operation::Uninstall => "Kadr was uninstalled successfully.".to_owned(),
                                        }
                                    } else {
                                        format!("{} failed:\n{}",
                                            match state.operation {
                                                Operation::Install   => "Installation",
                                                Operation::Update    => "Update",
                                                Operation::Uninstall => "Uninstall",
                                            },
                                            state.error.as_deref().unwrap_or("unknown error"))
                                    };
                                    let dir = state.options.install_dir.clone();
                                    let op = state.operation;
                                    if op == Operation::Uninstall && success {
                                        self.existing_install = None;
                                    }
                                    self.page = Page::Done(DoneState {
                                        success,
                                        message: msg,
                                        install_dir: dir,
                                        operation: op,
                                    });
                                }
                            }
                        }
                    }

                    Page::Done(state) => {
                        if let Some(action) = done::show(ui, state) {
                            match action {
                                done::DoneAction::Launch => {
                                    let exe = state.install_dir.join("kadr.exe");
                                    let _ = std::process::Command::new(exe).spawn();
                                    std::process::exit(0);
                                }
                                done::DoneAction::Close => {
                                    std::process::exit(0);
                                }
                            }
                        }
                    }
                }
            });
    }
}

fn draw_header(ui: &mut egui::Ui, kadr_version: Option<&str>) {
    let available_w = ui.available_width();
    let height = 52.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(available_w, height), egui::Sense::hover());
    let p = ui.painter();

    p.rect_filled(rect, 0.0, egui::Color32::from_rgb(15, 13, 22));

    p.text(
        rect.min + egui::vec2(24.0, 12.0),
        egui::Align2::LEFT_TOP,
        "kadr",
        egui::FontId::proportional(22.0),
        egui::Color32::from_rgb(99, 155, 255),
    );
    p.text(
        rect.min + egui::vec2(74.0, 17.0),
        egui::Align2::LEFT_TOP,
        "installer",
        egui::FontId::proportional(13.0),
        egui::Color32::from_gray(85),
    );

    let ver_text = format!(
        "kadr v{}   installer v{}",
        kadr_version.unwrap_or("…"),
        env!("CARGO_PKG_VERSION"),
    );
    p.text(
        rect.right_center() - egui::vec2(18.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        &ver_text,
        egui::FontId::monospace(10.5),
        egui::Color32::from_gray(80),
    );

    p.hline(
        rect.left()..=rect.right(),
        rect.bottom(),
        egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(99, 155, 255, 50)),
    );
}

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = egui::Color32::from_rgb(11, 10, 16);
    style.visuals.window_fill = egui::Color32::from_rgb(16, 14, 22);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(8, 7, 12);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(22, 20, 30);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(30, 27, 42);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(40, 36, 58);
    style.visuals.override_text_color = Some(egui::Color32::from_gray(210));
    style.visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_gray(35));
    ctx.set_global_style(style);
}
