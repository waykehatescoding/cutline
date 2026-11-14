use crate::model::*;
use eframe::egui;

pub struct Thumbs {
    ctx: egui::Context,
}

impl Thumbs {
    pub fn new(ctx: egui::Context) -> Self {
        Self { ctx }
    }

    pub fn get(&mut self, media: &Media, t: f64) -> Option<egui::TextureHandle> {
        let _ = (&self.ctx, media, t);
        None
    }

    pub fn poster(&mut self, media: &Media) -> Option<egui::TextureHandle> {
        self.get(media, media.duration * 0.1)
    }
}
