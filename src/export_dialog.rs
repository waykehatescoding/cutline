use crate::app::App;
use eframe::egui;

#[derive(Default)]
pub struct State {
    pub open: bool,
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let _ = (app, ctx);
}
