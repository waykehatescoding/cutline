use crate::model::*;
use eframe::egui;
use std::sync::Arc;

pub struct Peaks {
    pub per_sec: f64,
    pub data: Vec<f32>,
}

pub struct Waveforms {
    ctx: egui::Context,
}

impl Waveforms {
    pub fn new(ctx: egui::Context) -> Self {
        Self { ctx }
    }

    pub fn get(&mut self, media: &Media) -> Option<Arc<Peaks>> {
        let _ = (&self.ctx, media);
        None
    }
}
