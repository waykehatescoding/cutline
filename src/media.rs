use crate::model::*;
use eframe::egui;
use std::path::{Path, PathBuf};

pub const EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "flv", "ts", "mp3", "wav", "flac", "aac", "m4a", "ogg", "opus", "png", "jpg", "jpeg",
    "bmp", "gif", "webp",
];

pub struct Importer {
    ctx: egui::Context,
}

impl Importer {
    pub fn new(ctx: egui::Context) -> Self {
        Self { ctx }
    }

    pub fn import(&mut self, paths: Vec<PathBuf>) {
        let _ = (&self.ctx, paths);
    }

    pub fn poll(&mut self) -> Vec<Result<Media, String>> {
        vec![]
    }

    pub fn pending(&self) -> &[PathBuf] {
        &[]
    }
}

pub fn is_supported(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn probe(path: &Path) -> Result<Media, String> {
    Err(format!("cannot probe {}", path.display()))
}
