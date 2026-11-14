use crate::model::*;

pub fn apply(frame: &mut Frame, effect: &EffectKind, scale: f32, seed: u32) {
    let _ = (frame, effect, scale, seed);
}

pub fn crop(frame: &Frame, crop: &Crop) -> Frame {
    let _ = crop;
    frame.clone()
}
