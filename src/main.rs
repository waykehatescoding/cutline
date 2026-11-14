#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod commands;
mod compositor;
mod decode;
mod edit_ops;
mod effects;
mod effects_panel;
mod export;
mod export_dialog;
mod ffmpeg;
mod history;
mod icons;
mod inspector;
mod media;
mod media_bin;
mod model;
mod monitor;
mod project_io;
mod shell;
mod theme;
mod thumbs;
mod timeline;
mod updater;
mod waveform;

use eframe::egui;

fn main() -> eframe::Result {
    let open: Vec<std::path::PathBuf> = std::env::args_os().skip(1).map(Into::into).collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cutline")
            .with_inner_size([1600.0, 940.0])
            .with_min_inner_size([1024.0, 640.0])
            .with_drag_and_drop(true)
            .with_icon(egui::IconData { rgba: include_bytes!("../assets/icon64.rgba").to_vec(), width: 64, height: 64 }),
        ..Default::default()
    };
    eframe::run_native("Cutline", options, Box::new(|cc| Ok(Box::new(app::App::new(cc, open)))))
}
