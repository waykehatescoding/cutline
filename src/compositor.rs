use crate::decode::Decoder;
use crate::model::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TextOverlay {
    pub clip: ClipId,
    pub style: TextStyle,
    pub transform: Transform,
    pub opacity: f32,
}

#[derive(Clone, Debug)]
pub struct Composite {
    pub frame: Arc<Frame>,
    pub texts: Vec<TextOverlay>,
    pub complete: bool,
}

pub struct Compositor {}

impl Compositor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&mut self, project: &Project, t: f64, decoder: &mut Decoder, w: u32, h: u32, playing: bool) -> Composite {
        let _ = (t, decoder, playing);
        Composite { frame: Arc::new(Frame::filled(w, h, project.settings.background)), texts: vec![], complete: true }
    }
}
