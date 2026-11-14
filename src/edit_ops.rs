use crate::model::*;

pub fn add_media(p: &mut Project, media: MediaId, track: Option<usize>, at: f64) -> Vec<ClipId> {
    let _ = (p, media, track, at);
    vec![]
}

pub fn add_clip(p: &mut Project, track: usize, clip: Clip) -> ClipId {
    let id = clip.id;
    p.tracks[track].clips.push(clip);
    id
}

pub fn split_at(p: &mut Project, id: ClipId, t: f64) -> Option<ClipId> {
    let _ = (p, id, t);
    None
}

pub fn split_all_at(p: &mut Project, t: f64) -> Vec<ClipId> {
    let _ = (p, t);
    vec![]
}

pub fn delete(p: &mut Project, ids: &[ClipId], ripple: bool) {
    let _ = (p, ids, ripple);
}

pub fn move_clip(p: &mut Project, id: ClipId, new_start: f64, new_track: usize) -> bool {
    let _ = (p, id, new_start, new_track);
    false
}

pub fn trim_start(p: &mut Project, id: ClipId, new_start: f64) {
    let _ = (p, id, new_start);
}

pub fn trim_end(p: &mut Project, id: ClipId, new_end: f64) {
    let _ = (p, id, new_end);
}

pub fn slip(p: &mut Project, id: ClipId, delta: f64) {
    let _ = (p, id, delta);
}

pub fn duplicate(p: &mut Project, ids: &[ClipId]) -> Vec<ClipId> {
    let _ = (p, ids);
    vec![]
}

pub fn copy(p: &Project, ids: &[ClipId]) -> Vec<(usize, Clip)> {
    let _ = (p, ids);
    vec![]
}

pub fn paste(p: &mut Project, clips: &[(usize, Clip)], at: f64) -> Vec<ClipId> {
    let _ = (p, clips, at);
    vec![]
}

pub fn snap_points(p: &Project, exclude: &[ClipId], playhead: f64) -> Vec<f64> {
    let _ = (p, exclude, playhead);
    vec![]
}

pub fn snap(t: f64, points: &[f64], tolerance: f64) -> Option<f64> {
    let _ = (t, points, tolerance);
    None
}

pub fn close_gaps(p: &mut Project, track: usize) {
    let _ = (p, track);
}

pub fn add_track(p: &mut Project, kind: TrackKind) -> usize {
    let _ = (p, kind);
    0
}

pub fn remove_track(p: &mut Project, index: usize) {
    let _ = (p, index);
}

pub fn remove_media(p: &mut Project, id: MediaId) {
    let _ = (p, id);
}

pub fn set_speed(p: &mut Project, id: ClipId, speed: f64) {
    let _ = (p, id, speed);
}

pub fn add_text(p: &mut Project, at: f64) -> Option<ClipId> {
    let _ = (p, at);
    None
}

pub fn add_matte(p: &mut Project, color: [u8; 4], at: f64) -> Option<ClipId> {
    let _ = (p, color, at);
    None
}

pub fn set_transition(p: &mut Project, id: ClipId, kind: Option<TransitionKind>, duration: f64) {
    let _ = (p, id, kind, duration);
}

pub fn clips_at(p: &Project, t: f64) -> Vec<ClipId> {
    let _ = (p, t);
    vec![]
}

pub fn next_edit(p: &Project, t: f64, forward: bool) -> Option<f64> {
    let _ = (p, t, forward);
    None
}

pub fn add_marker(p: &mut Project, t: f64) {
    let _ = (p, t);
}
