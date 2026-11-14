use crate::decode::Decoder;
use crate::effects;
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

#[derive(Clone, Copy)]
struct Place {
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
    rot: f32,
    opacity: f32,
    x0: f32,
    x1: f32,
}

enum Source {
    Text,
    Missing,
    Pixels(Arc<Frame>, bool),
}

struct Cached {
    t: f64,
    w: u32,
    h: u32,
    playing: bool,
    settings: Settings,
    media: usize,
    tracks: Vec<(bool, Vec<Clip>)>,
    out: Composite,
}

impl Cached {
    fn matches(&self, p: &Project, t: f64, w: u32, h: u32) -> bool {
        self.t == t
            && self.w == w
            && self.h == h
            && self.settings == p.settings
            && self.media == p.media.len()
            && self.tracks.len() == p.tracks.len()
            && self.tracks.iter().zip(&p.tracks).all(|((hidden, clips), tr)| *hidden == tr.hidden && *clips == tr.clips)
    }
}

pub struct Compositor {
    last: Option<Cached>,
}

impl Compositor {
    pub fn new() -> Self {
        Self { last: None }
    }

    pub fn render(&mut self, project: &Project, t: f64, decoder: &mut Decoder, w: u32, h: u32, playing: bool) -> Composite {
        if let Some(c) = &self.last {
            if !playing && !c.playing && c.out.complete && c.matches(project, t, w, h) {
                return c.out.clone();
            }
        }
        let mut canvas = self.last.take().map(|c| c.out.frame).unwrap_or_else(|| Arc::new(Frame::new(0, 0)));
        match Arc::get_mut(&mut canvas) {
            Some(f) if f.w == w && f.h == h => fill(f, project.settings.background),
            _ => canvas = Arc::new(Frame::filled(w, h, project.settings.background)),
        }
        let mut texts = vec![];
        let mut complete = true;
        let seed = (t * project.settings.fps) as u32;
        let frame = Arc::get_mut(&mut canvas).unwrap();
        for track in project.tracks.iter().rev() {
            if track.kind != TrackKind::Video || track.hidden {
                continue;
            }
            let Some(ci) = track.clips.iter().position(|c| c.contains(t)) else { continue };
            let clip = &track.clips[ci];
            let opacity = clip.opacity * fade(clip, t);
            let incoming = source(project, decoder, clip, clip.source_time(t), w, h, playing, seed);
            let tr = clip.transition_in.filter(|tr| tr.duration > 1e-6 && t - clip.start < tr.duration);
            let prev = tr.and_then(|tr| {
                let prev = track.clips.get(ci.checked_sub(1)?)?;
                ((prev.end() - clip.start).abs() < 1e-3).then_some((tr, prev))
            });
            let Some((tr, prev)) = prev else {
                put(frame, &mut texts, &mut complete, incoming, clip, opacity);
                continue;
            };
            let p = ((t - clip.start) / tr.duration) as f32;
            let outgoing = source(project, decoder, prev, outgoing_time(project, prev, t - clip.start), w, h, playing, seed);
            match (outgoing, incoming) {
                (Source::Pixels(fa, ca), Source::Pixels(fb, cb)) => {
                    let (a, b) = (place(&fa, ca, prev, w, h, prev.opacity), place(&fb, cb, clip, w, h, opacity));
                    transition(frame, tr.kind, p, a, b, &fa, &fb);
                }
                (out, inc) => {
                    put(frame, &mut texts, &mut complete, out, prev, prev.opacity * (1.0 - p));
                    put(frame, &mut texts, &mut complete, inc, clip, opacity * p);
                }
            }
        }
        let out = Composite { frame: canvas, texts, complete };
        self.last = Some(Cached {
            t,
            w,
            h,
            playing,
            settings: project.settings.clone(),
            media: project.media.len(),
            tracks: project.tracks.iter().map(|tr| (tr.hidden, tr.clips.clone())).collect(),
            out: out.clone(),
        });
        out
    }
}

fn fill(f: &mut Frame, c: [u8; 4]) {
    for px in f.rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&c);
    }
}

fn fade(clip: &Clip, t: f64) -> f32 {
    let fi = if clip.fade_in > 0.0 { ((t - clip.start) / clip.fade_in).clamp(0.0, 1.0) } else { 1.0 };
    let fo = if clip.fade_out > 0.0 { ((clip.end() - t) / clip.fade_out).clamp(0.0, 1.0) } else { 1.0 };
    (fi * fo) as f32
}

fn outgoing_time(project: &Project, prev: &Clip, local: f64) -> f64 {
    let dir = if prev.reverse { -1.0 } else { 1.0 };
    let t = prev.source_time(prev.end()) + local * prev.speed * dir;
    match prev.media_id().and_then(|id| project.media(id)) {
        Some(m) => t.clamp(0.0, m.duration),
        None => t,
    }
}

fn source(project: &Project, decoder: &mut Decoder, clip: &Clip, st: f64, w: u32, h: u32, playing: bool, seed: u32) -> Source {
    let (base, color) = match &clip.source {
        ClipSource::Text(_) => return Source::Text,
        ClipSource::Color(c) => (Arc::new(Frame::filled(1, 1, *c)), true),
        ClipSource::Media(id) => {
            let Some(media) = project.media(*id) else { return Source::Missing };
            let Some(f) = decoder.frame(media, st, w, h, playing) else { return Source::Missing };
            (f, false)
        }
    };
    let mut f = base;
    if !color && !clip.crop.is_none() {
        f = Arc::new(effects::crop(&f, &clip.crop));
    }
    let scale = h as f32 / 1080.0;
    for e in clip.effects.iter().filter(|e| e.enabled) {
        effects::apply(Arc::make_mut(&mut f), &e.kind, scale, seed);
    }
    Source::Pixels(f, color)
}

fn place(f: &Frame, color: bool, clip: &Clip, w: u32, h: u32, opacity: f32) -> Place {
    let (w, h) = (w as f32, h as f32);
    let tf = &clip.transform;
    let (bx, by) = if color {
        (w, h)
    } else {
        let s = (w / f.w as f32).min(h / f.h as f32);
        (s, s)
    };
    let sign = |flip: bool| if flip { -1.0 } else { 1.0 };
    Place {
        cx: tf.x * w,
        cy: tf.y * h,
        sx: bx * tf.scale * sign(tf.flip_h),
        sy: by * tf.scale * sign(tf.flip_v),
        rot: tf.rotation.to_radians(),
        opacity,
        x0: 0.0,
        x1: w,
    }
}

fn put(canvas: &mut Frame, texts: &mut Vec<TextOverlay>, complete: &mut bool, src: Source, clip: &Clip, opacity: f32) {
    match (src, &clip.source) {
        (Source::Pixels(f, color), _) => {
            let p = place(&f, color, clip, canvas.w, canvas.h, opacity);
            blit(canvas, &f, &p);
        }
        (Source::Text, ClipSource::Text(style)) => {
            texts.push(TextOverlay { clip: clip.id, style: style.clone(), transform: clip.transform, opacity })
        }
        _ => *complete = false,
    }
}

fn dim(canvas: &mut Frame, amount: f32) {
    let black = Frame::filled(1, 1, [0, 0, 0, 255]);
    let (w, h) = (canvas.w as f32, canvas.h as f32);
    blit(canvas, &black, &Place { cx: w / 2.0, cy: h / 2.0, sx: w, sy: h, rot: 0.0, opacity: amount, x0: 0.0, x1: w });
}

fn transition(canvas: &mut Frame, kind: TransitionKind, p: f32, mut a: Place, mut b: Place, fa: &Frame, fb: &Frame) {
    let w = canvas.w as f32;
    match kind {
        TransitionKind::Crossfade => {
            blit(canvas, fa, &a);
            b.opacity *= p;
            blit(canvas, fb, &b);
        }
        TransitionKind::FadeThroughBlack => {
            if p < 0.5 {
                blit(canvas, fa, &a);
                dim(canvas, p * 2.0);
            } else {
                blit(canvas, fb, &b);
                dim(canvas, (1.0 - p) * 2.0);
            }
        }
        TransitionKind::WipeLeft => {
            blit(canvas, fa, &a);
            b.x0 = w * (1.0 - p);
            blit(canvas, fb, &b);
        }
        TransitionKind::WipeRight => {
            blit(canvas, fa, &a);
            b.x1 = w * p;
            blit(canvas, fb, &b);
        }
        TransitionKind::SlideLeft => {
            a.cx -= p * w;
            b.cx += (1.0 - p) * w;
            blit(canvas, fa, &a);
            blit(canvas, fb, &b);
        }
        TransitionKind::SlideRight => {
            a.cx += p * w;
            b.cx -= (1.0 - p) * w;
            blit(canvas, fa, &a);
            blit(canvas, fb, &b);
        }
        TransitionKind::ZoomIn => {
            blit(canvas, fb, &b);
            a.sx *= 1.0 + p;
            a.sy *= 1.0 + p;
            a.opacity *= 1.0 - p;
            blit(canvas, fa, &a);
        }
    }
}

fn blit(canvas: &mut Frame, src: &Frame, p: &Place) {
    if src.w == 0 || src.h == 0 || p.opacity <= 0.0 || p.sx == 0.0 || p.sy == 0.0 {
        return;
    }
    let (sin, cos) = p.rot.sin_cos();
    let (hw, hh) = (src.w as f32 * p.sx.abs() / 2.0, src.h as f32 * p.sy.abs() / 2.0);
    let ex = hw * cos.abs() + hh * sin.abs();
    let ey = hw * sin.abs() + hh * cos.abs();
    let xa = (p.cx - ex).max(p.x0).floor().max(0.0) as usize;
    let xb = ((p.cx + ex).min(p.x1).ceil().max(0.0) as usize).min(canvas.w as usize);
    let ya = (p.cy - ey).floor().max(0.0) as usize;
    let yb = ((p.cy + ey).ceil().max(0.0) as usize).min(canvas.h as usize);
    if xa >= xb || ya >= yb {
        return;
    }
    let cw = canvas.w as usize;
    let rows = &mut canvas.rgba[ya * cw * 4..yb * cw * 4];
    let threads = if cw * canvas.h as usize > 100_000 { std::thread::available_parallelism().map_or(1, |n| n.get()).min(8) } else { 1 };
    let per = (yb - ya).div_ceil(threads);
    let run = |chunk: &mut [u8], first: usize| blit_rows(chunk, first, cw, xa, xb, src, p, sin, cos);
    if threads == 1 {
        run(rows, ya);
        return;
    }
    std::thread::scope(|s| {
        for (i, chunk) in rows.chunks_mut(per * cw * 4).enumerate() {
            let run = &run;
            s.spawn(move || run(chunk, ya + i * per));
        }
    });
}

fn blit_rows(rows: &mut [u8], first: usize, cw: usize, xa: usize, xb: usize, src: &Frame, p: &Place, sin: f32, cos: f32) {
    let (sw, sh) = (src.w as usize, src.h as usize);
    let (fw, fh) = (sw as f32, sh as f32);
    let (du, dv) = (cos / p.sx, -sin / p.sy);
    for (r, row) in rows.chunks_exact_mut(cw * 4).enumerate() {
        let y = (first + r) as f32 + 0.5 - p.cy;
        let x = xa as f32 + 0.5 - p.cx;
        let mut u = (x * cos + y * sin) / p.sx + fw / 2.0;
        let mut v = (-x * sin + y * cos) / p.sy + fh / 2.0;
        for dst in row[xa * 4..xb * 4].chunks_exact_mut(4) {
            if u >= 0.0 && u < fw && v >= 0.0 && v < fh {
                let (uu, vv) = (u - 0.5, v - 0.5);
                let (x0, y0) = (uu.floor(), vv.floor());
                let (fx, fy) = (uu - x0, vv - y0);
                let cx = |x: f32| (x.max(0.0) as usize).min(sw - 1);
                let cy = |y: f32| (y.max(0.0) as usize).min(sh - 1);
                let (xl, xr, yt, yb) = (cx(x0), cx(x0 + 1.0), cy(y0), cy(y0 + 1.0));
                let at = |x: usize, y: usize, c: usize| src.rgba[(y * sw + x) * 4 + c] as f32;
                let mix = |c: usize| {
                    let top = at(xl, yt, c) * (1.0 - fx) + at(xr, yt, c) * fx;
                    let bot = at(xl, yb, c) * (1.0 - fx) + at(xr, yb, c) * fx;
                    top * (1.0 - fy) + bot * fy
                };
                let a = mix(3) / 255.0 * p.opacity;
                if a > 0.0 {
                    for c in 0..3 {
                        let d = dst[c] as f32;
                        dst[c] = (d + (mix(c) - d) * a + 0.5) as u8;
                    }
                    let da = dst[3] as f32;
                    dst[3] = (da + (255.0 - da) * a + 0.5) as u8;
                }
            }
            u += du;
            v += dv;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    fn full(w: u32, h: u32) -> Place {
        Place { cx: w as f32 / 2.0, cy: h as f32 / 2.0, sx: 1.0, sy: 1.0, rot: 0.0, opacity: 1.0, x0: 0.0, x1: w as f32 }
    }

    fn px(f: &Frame, x: u32) -> [u8; 4] {
        let i = (x * 4) as usize;
        [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2], f.rgba[i + 3]]
    }

    #[test]
    fn blit_identity_copies_pixels() {
        let mut src = Frame::new(2, 2);
        src.rgba.copy_from_slice(&[1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255]);
        let mut dst = Frame::filled(2, 2, [0, 0, 0, 255]);
        blit(&mut dst, &src, &full(2, 2));
        assert_eq!(dst.rgba, src.rgba);
    }

    #[test]
    fn blit_opacity_mixes() {
        let src = Frame::filled(2, 2, [200, 100, 50, 255]);
        let mut dst = Frame::filled(2, 2, [0, 0, 0, 255]);
        let mut p = full(2, 2);
        p.opacity = 0.5;
        blit(&mut dst, &src, &p);
        let c = px(&dst, 0);
        assert!((c[0] as i32 - 100).abs() <= 1 && (c[1] as i32 - 50).abs() <= 1 && (c[2] as i32 - 25).abs() <= 1);
    }

    #[test]
    fn blit_zero_alpha_is_invisible() {
        let src = Frame::filled(2, 2, [200, 100, 50, 0]);
        let mut dst = Frame::filled(2, 2, [9, 9, 9, 255]);
        blit(&mut dst, &src, &full(2, 2));
        assert_eq!(px(&dst, 0), [9, 9, 9, 255]);
    }

    #[test]
    fn blit_scales_up_to_cover() {
        let src = Frame::filled(2, 2, [255, 0, 0, 255]);
        let mut dst = Frame::filled(4, 4, [0, 0, 0, 255]);
        let mut p = full(4, 4);
        p.sx = 2.0;
        p.sy = 2.0;
        blit(&mut dst, &src, &p);
        assert!(dst.rgba.chunks(4).all(|c| c == [255, 0, 0, 255]));
    }

    #[test]
    fn blit_large_frame_threaded() {
        let src = Frame::filled(400, 300, [10, 20, 30, 255]);
        let mut dst = Frame::filled(400, 300, [0, 0, 0, 255]);
        blit(&mut dst, &src, &full(400, 300));
        assert!(dst.rgba.chunks(4).all(|c| c == [10, 20, 30, 255]));
    }

    fn blend(kind: TransitionKind, p: f32) -> Frame {
        let a = Frame::filled(4, 1, [200, 0, 0, 255]);
        let b = Frame::filled(4, 1, [0, 100, 0, 255]);
        let mut canvas = Frame::filled(4, 1, [0, 0, 0, 255]);
        transition(&mut canvas, kind, p, full(4, 1), full(4, 1), &a, &b);
        canvas
    }

    #[test]
    fn crossfade_endpoints_and_middle() {
        assert_eq!(px(&blend(TransitionKind::Crossfade, 0.0), 0), [200, 0, 0, 255]);
        assert_eq!(px(&blend(TransitionKind::Crossfade, 1.0), 0), [0, 100, 0, 255]);
        let m = px(&blend(TransitionKind::Crossfade, 0.5), 0);
        assert!((m[0] as i32 - 100).abs() <= 1 && (m[1] as i32 - 50).abs() <= 1);
    }

    #[test]
    fn dip_to_black_is_black_in_the_middle() {
        assert_eq!(px(&blend(TransitionKind::FadeThroughBlack, 0.5), 0), [0, 0, 0, 255]);
        assert_eq!(px(&blend(TransitionKind::FadeThroughBlack, 0.0), 0), [200, 0, 0, 255]);
    }

    #[test]
    fn wipe_left_reveals_from_the_right() {
        let f = blend(TransitionKind::WipeLeft, 0.5);
        assert_eq!(px(&f, 0), [200, 0, 0, 255]);
        assert_eq!(px(&f, 3), [0, 100, 0, 255]);
    }

    #[test]
    fn slide_left_moves_both_images() {
        let f = blend(TransitionKind::SlideLeft, 0.5);
        assert_eq!(px(&f, 0), [200, 0, 0, 255]);
        assert_eq!(px(&f, 3), [0, 100, 0, 255]);
    }

    fn text_project() -> Project {
        let mut p = Project::default();
        let style = TextStyle { text: "hi".into(), size: 48.0, color: [255; 4], background: None, bold: false, align: TextAlign::Center };
        let c = p.new_clip(ClipSource::Text(style), "t".into(), 0.0, 0.0, 5.0);
        p.tracks[0].clips.push(c);
        p
    }

    #[test]
    fn text_clip_becomes_overlay() {
        let p = text_project();
        let mut d = Decoder::new(egui::Context::default());
        let out = Compositor::new().render(&p, 1.0, &mut d, 64, 36, false);
        assert!(out.complete);
        assert_eq!(out.texts.len(), 1);
        assert_eq!(out.texts[0].style.text, "hi");
        assert_eq!(out.frame.rgba[..4], p.settings.background);
    }

    #[test]
    fn unchanged_render_hits_cache() {
        let mut p = text_project();
        let mut d = Decoder::new(egui::Context::default());
        let mut c = Compositor::new();
        let a = c.render(&p, 1.0, &mut d, 64, 36, false);
        let b = c.render(&p, 1.0, &mut d, 64, 36, false);
        assert!(Arc::ptr_eq(&a.frame, &b.frame));
        p.tracks[0].clips[0].opacity = 0.5;
        let e = c.render(&p, 1.0, &mut d, 64, 36, false);
        assert_eq!(e.texts[0].opacity, 0.5);
    }
}
