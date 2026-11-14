use crate::model::*;

pub struct AudioEngine {}

impl AudioEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn play(&mut self, project: &Project, from: f64) {
        let _ = (project, from);
    }

    pub fn stop(&mut self) {}

    pub fn position(&self) -> Option<f64> {
        None
    }
}
