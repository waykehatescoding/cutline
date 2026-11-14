use crate::model::*;
use eframe::egui;
use std::sync::Arc;

pub struct Decoder {
    ctx: egui::Context,
}

impl Decoder {
    pub fn new(ctx: egui::Context) -> Self {
        Self { ctx }
    }

    pub fn frame(&mut self, media: &Media, t: f64, max_w: u32, max_h: u32, playing: bool) -> Option<Arc<Frame>> {
        let _ = (&self.ctx, media, t, max_w, max_h, playing);
        None
    }

    pub fn clear(&mut self) {}
}

pub fn grab_frame(media: &Media, t: f64, w: u32, h: u32) -> Result<Frame, String> {
    let _ = (media, t);
    Ok(Frame::new(w, h))
}
