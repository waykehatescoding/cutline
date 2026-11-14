use crate::model::*;
use std::thread;

type Lut = [u8; 256];

pub fn apply(frame: &mut Frame, effect: &EffectKind, scale: f32, seed: u32) {
    match *effect {
        EffectKind::Blur { radius, region } => blur(frame, radius * scale, region),
        EffectKind::Pixelate { block, region } => pixelate(frame, (block * scale).round() as usize, region),
        EffectKind::ColorAdjust { brightness, contrast, saturation, gamma } => {
            let inv_gamma = 1.0 / gamma.max(0.01);
            let t = table(|x| (((x - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0)).powf(inv_gamma));
            luts(frame, [t, t, t]);
            if saturation != 1.0 {
                pixels(frame, |p| {
                    let l = 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
                    for c in &mut p[..3] {
                        *c = (l + (*c as f32 - l) * saturation).round().clamp(0.0, 255.0) as u8;
                    }
                });
            }
        }
        EffectKind::WhiteBalance { temperature, tint } => {
            let gain = |g: f32| table(move |x| x * g);
            luts(frame, [gain(1.0 + 0.25 * temperature), gain(1.0 - 0.25 * tint), gain(1.0 - 0.25 * temperature)]);
        }
        EffectKind::HueShift { degrees } => {
            let (s, c) = degrees.to_radians().sin_cos();
            matrix(
                frame,
                [
                    [0.213 + c * 0.787 - s * 0.213, 0.715 - c * 0.715 - s * 0.715, 0.072 - c * 0.072 + s * 0.928],
                    [0.213 - c * 0.213 + s * 0.143, 0.715 + c * 0.285 + s * 0.140, 0.072 - c * 0.072 - s * 0.283],
                    [0.213 - c * 0.213 - s * 0.787, 0.715 - c * 0.715 + s * 0.715, 0.072 + c * 0.928 + s * 0.072],
                ],
            );
        }
        EffectKind::Grayscale => matrix(frame, [[0.2126, 0.7152, 0.0722]; 3]),
        EffectKind::Sepia { amount } => {
            let a = amount.clamp(0.0, 1.0);
            let sepia = [[0.393, 0.769, 0.189], [0.349, 0.686, 0.168], [0.272, 0.534, 0.131]];
            let mut m = [[0.0; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    m[i][j] = sepia[i][j] * a + if i == j { 1.0 - a } else { 0.0 };
                }
            }
            matrix(frame, m);
        }
        EffectKind::Invert => {
            let t = table(|x| 1.0 - x);
            luts(frame, [t, t, t]);
        }
        EffectKind::Vignette { strength } => vignette(frame, strength),
        EffectKind::Sharpen { amount } => sharpen(frame, amount, scale),
        EffectKind::Noise { amount } => noise(frame, amount, seed),
        EffectKind::ChromaKey { color, similarity, smoothness } => chroma_key(frame, color, similarity, smoothness),
    }
}

pub fn crop(frame: &Frame, crop: &Crop) -> Frame {
    let (w, h) = (frame.w as f32, frame.h as f32);
    let x0 = (crop.left * w).round().clamp(0.0, w - 1.0) as usize;
    let y0 = (crop.top * h).round().clamp(0.0, h - 1.0) as usize;
    let x1 = (w - crop.right * w).round().clamp(x0 as f32 + 1.0, w) as usize;
    let y1 = (h - crop.bottom * h).round().clamp(y0 as f32 + 1.0, h) as usize;
    let mut out = Frame::new((x1 - x0) as u32, (y1 - y0) as u32);
    let stride = frame.w as usize * 4;
    for (i, row) in out.rgba.chunks_exact_mut((x1 - x0) * 4).enumerate() {
        row.copy_from_slice(&frame.rgba[(y0 + i) * stride + x0 * 4..][..row.len()]);
    }
    out
}

fn par_rows(buf: &mut [u8], row_len: usize, f: impl Fn(usize, &mut [u8]) + Sync) {
    let rows = buf.len() / row_len;
    let n = thread::available_parallelism().map_or(1, |n| n.get());
    if buf.len() < 1 << 16 || n == 1 {
        return f(0, buf);
    }
    let per = rows.div_ceil(n);
    thread::scope(|s| {
        for (i, chunk) in buf.chunks_mut(per * row_len).enumerate() {
            let f = &f;
            s.spawn(move || f(i * per, chunk));
        }
    });
}

fn pixels(frame: &mut Frame, f: impl Fn(&mut [u8]) + Sync) {
    let row = frame.w as usize * 4;
    par_rows(&mut frame.rgba, row, |_, rows| rows.chunks_exact_mut(4).for_each(&f));
}

fn table(f: impl Fn(f32) -> f32) -> Lut {
    std::array::from_fn(|i| (f(i as f32 / 255.0) * 255.0).round().clamp(0.0, 255.0) as u8)
}

fn luts(frame: &mut Frame, t: [Lut; 3]) {
    pixels(frame, |p| {
        p[0] = t[0][p[0] as usize];
        p[1] = t[1][p[1] as usize];
        p[2] = t[2][p[2] as usize];
    });
}

fn matrix(frame: &mut Frame, m: [[f32; 3]; 3]) {
    pixels(frame, |p| {
        let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
        for i in 0..3 {
            p[i] = (m[i][0] * r + m[i][1] * g + m[i][2] * b).round().clamp(0.0, 255.0) as u8;
        }
    });
}

fn region_px(frame: &Frame, region: Option<Rect01>) -> (usize, usize, usize, usize) {
    let r = region.unwrap_or(Rect01::FULL);
    let (w, h) = (frame.w as f32, frame.h as f32);
    let x0 = (r.x * w).round().clamp(0.0, w - 1.0);
    let y0 = (r.y * h).round().clamp(0.0, h - 1.0);
    let x1 = ((r.x + r.w) * w).round().clamp(x0 + 1.0, w);
    let y1 = ((r.y + r.h) * h).round().clamp(y0 + 1.0, h);
    (x0 as usize, y0 as usize, x1 as usize, y1 as usize)
}

fn box_rows(buf: &mut [u8], w: usize, r: usize) {
    par_rows(buf, w * 4, |_, rows| {
        let mut tmp = vec![0u8; w * 4];
        let n = (2 * r + 1) as i32;
        let last = w as isize - 1;
        let r = r as isize;
        for row in rows.chunks_exact_mut(w * 4) {
            tmp.copy_from_slice(row);
            let px = |i: isize| &tmp[i.clamp(0, last) as usize * 4..][..3];
            let mut sum = [0i32; 3];
            for k in -r..=r {
                for (s, v) in sum.iter_mut().zip(px(k)) {
                    *s += *v as i32;
                }
            }
            for x in 0..w {
                let (add, sub) = (px(x as isize + r + 1), px(x as isize - r));
                for c in 0..3 {
                    row[x * 4 + c] = ((sum[c] + n / 2) / n) as u8;
                    sum[c] += add[c] as i32 - sub[c] as i32;
                }
            }
        }
    });
}

fn transpose(src: &[u8], dst: &mut [u8], w: usize, h: usize) {
    par_rows(dst, h * 4, |first, rows| {
        for (j, row) in rows.chunks_exact_mut(h * 4).enumerate() {
            let x = first + j;
            for y in 0..h {
                row[y * 4..y * 4 + 4].copy_from_slice(&src[(y * w + x) * 4..][..4]);
            }
        }
    });
}

fn box_blur(buf: &mut [u8], w: usize, h: usize, r: usize) {
    let r = r.min(w.max(h));
    for _ in 0..3 {
        box_rows(buf, w, r);
    }
    let mut t = vec![0u8; buf.len()];
    transpose(buf, &mut t, w, h);
    for _ in 0..3 {
        box_rows(&mut t, h, r);
    }
    transpose(&t, buf, h, w);
}

fn blur(frame: &mut Frame, radius: f32, region: Option<Rect01>) {
    let r = (radius * 0.5).round() as usize;
    if r == 0 {
        return;
    }
    let (x0, y0, x1, y1) = region_px(frame, region);
    let (w, h, stride) = (x1 - x0, y1 - y0, frame.w as usize * 4);
    if w == frame.w as usize && h == frame.h as usize {
        return box_blur(&mut frame.rgba, w, h, r);
    }
    let mut sub = vec![0u8; w * h * 4];
    for (i, row) in sub.chunks_exact_mut(w * 4).enumerate() {
        row.copy_from_slice(&frame.rgba[(y0 + i) * stride + x0 * 4..][..w * 4]);
    }
    box_blur(&mut sub, w, h, r);
    for (i, row) in sub.chunks_exact(w * 4).enumerate() {
        frame.rgba[(y0 + i) * stride + x0 * 4..][..w * 4].copy_from_slice(row);
    }
}

fn pixelate(frame: &mut Frame, block: usize, region: Option<Rect01>) {
    if block < 2 {
        return;
    }
    let (x0, y0, x1, y1) = region_px(frame, region);
    let stride = frame.w as usize * 4;
    for by in (y0..y1).step_by(block) {
        let bh = (y1 - by).min(block);
        for bx in (x0..x1).step_by(block) {
            let bw = (x1 - bx).min(block);
            let mut sum = [0u32; 3];
            for y in by..by + bh {
                for p in frame.rgba[y * stride + bx * 4..][..bw * 4].chunks_exact(4) {
                    for c in 0..3 {
                        sum[c] += p[c] as u32;
                    }
                }
            }
            let n = (bw * bh) as u32;
            let avg = sum.map(|s| ((s + n / 2) / n) as u8);
            for y in by..by + bh {
                for p in frame.rgba[y * stride + bx * 4..][..bw * 4].chunks_exact_mut(4) {
                    p[..3].copy_from_slice(&avg);
                }
            }
        }
    }
}

fn vignette(frame: &mut Frame, strength: f32) {
    if strength == 0.0 {
        return;
    }
    let (w, h) = (frame.w as usize, frame.h as usize);
    let norm = |i: usize, n: usize| {
        let half = (n as f32 - 1.0) / 2.0;
        let d = (i as f32 - half) / half.max(1.0);
        d * d
    };
    let cols: Vec<f32> = (0..w).map(|x| norm(x, w)).collect();
    par_rows(&mut frame.rgba, w * 4, |first, rows| {
        for (j, row) in rows.chunks_exact_mut(w * 4).enumerate() {
            let dy = norm(first + j, h);
            for (p, dx) in row.chunks_exact_mut(4).zip(&cols) {
                let d = (dx + dy) * 0.5;
                let f = (1.0 - strength * d * d).max(0.0);
                for c in &mut p[..3] {
                    *c = (*c as f32 * f).round() as u8;
                }
            }
        }
    });
}

fn sharpen(frame: &mut Frame, amount: f32, scale: f32) {
    if amount == 0.0 {
        return;
    }
    let (w, h) = (frame.w as usize, frame.h as usize);
    let mut blurred = frame.rgba.clone();
    box_blur(&mut blurred, w, h, scale.round().max(1.0) as usize);
    par_rows(&mut frame.rgba, w * 4, |first, rows| {
        let base = &blurred[first * w * 4..];
        for (p, b) in rows.chunks_exact_mut(4).zip(base.chunks_exact(4)) {
            for c in 0..3 {
                let v = p[c] as f32;
                p[c] = (v + amount * (v - b[c] as f32)).round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}

fn noise(frame: &mut Frame, amount: f32, seed: u32) {
    if amount == 0.0 {
        return;
    }
    let w = frame.w as usize;
    let salt = seed.wrapping_mul(0x9e3779b9);
    par_rows(&mut frame.rgba, w * 4, |first, rows| {
        for (i, p) in rows.chunks_exact_mut(4).enumerate() {
            let n = (hash(((first * w + i) as u32).wrapping_add(salt)) >> 8) as f32 / 8388607.5 - 1.0;
            let d = n * amount * 80.0;
            for c in &mut p[..3] {
                *c = (*c as f32 + d).round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}

fn chroma(r: f32, g: f32, b: f32) -> (f32, f32) {
    (-0.168736 * r - 0.331264 * g + 0.5 * b, 0.5 * r - 0.418688 * g - 0.081312 * b)
}

fn chroma_key(frame: &mut Frame, color: [u8; 3], similarity: f32, smoothness: f32) {
    let (kb, kr) = chroma(color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0);
    let smooth = smoothness.max(1e-4);
    pixels(frame, |p| {
        let (cb, cr) = chroma(p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
        let d = ((cb - kb).powi(2) + (cr - kr).powi(2)).sqrt() / std::f32::consts::SQRT_2;
        p[3] = (p[3] as f32 * ((d - similarity) / smooth).clamp(0.0, 1.0)).round() as u8;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn gradient(w: u32, h: u32) -> Frame {
        let mut f = Frame::new(w, h);
        for (i, p) in f.rgba.chunks_exact_mut(4).enumerate() {
            let x = (i as u32 % w) as u8;
            let y = (i as u32 / w) as u8;
            p.copy_from_slice(&[x.wrapping_mul(17), y.wrapping_mul(23), x ^ y, 200]);
        }
        f
    }

    fn run(f: &mut Frame, e: EffectKind) {
        apply(f, &e, 1.0, 7);
    }

    #[test]
    fn invert_twice_is_identity() {
        let mut f = gradient(5, 4);
        let orig = f.rgba.clone();
        run(&mut f, EffectKind::Invert);
        assert_ne!(f.rgba, orig);
        run(&mut f, EffectKind::Invert);
        assert_eq!(f.rgba, orig);
    }

    #[test]
    fn blur_flat_color_stays_flat() {
        let mut f = Frame::filled(9, 7, [10, 120, 250, 255]);
        run(&mut f, EffectKind::Blur { radius: 6.0, region: None });
        assert!(f.rgba.chunks_exact(4).all(|p| p == [10, 120, 250, 255]));
    }

    #[test]
    fn blur_spreads_and_keeps_alpha() {
        let mut f = Frame::filled(9, 9, [0, 0, 0, 255]);
        f.rgba[(4 * 9 + 4) * 4..(4 * 9 + 4) * 4 + 3].fill(255);
        run(&mut f, EffectKind::Blur { radius: 4.0, region: None });
        assert!(f.rgba[(4 * 9 + 5) * 4] > 0);
        assert!(f.rgba[(4 * 9 + 4) * 4] < 255);
        assert!(f.rgba.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn region_blur_leaves_outside_untouched() {
        let mut f = gradient(20, 20);
        let orig = f.rgba.clone();
        let region = Rect01 { x: 0.25, y: 0.25, w: 0.5, h: 0.5 };
        run(&mut f, EffectKind::Blur { radius: 6.0, region: Some(region) });
        let mut inside_changed = false;
        for y in 0..20usize {
            for x in 0..20usize {
                let i = (y * 20 + x) * 4;
                let inside = (5..15).contains(&x) && (5..15).contains(&y);
                if inside {
                    inside_changed |= f.rgba[i..i + 4] != orig[i..i + 4];
                } else {
                    assert_eq!(f.rgba[i..i + 4], orig[i..i + 4]);
                }
            }
        }
        assert!(inside_changed);
    }

    #[test]
    fn pixelate_blocks_are_uniform() {
        let mut f = gradient(8, 8);
        run(&mut f, EffectKind::Pixelate { block: 4.0, region: None });
        for by in 0..2usize {
            for bx in 0..2usize {
                let first = &f.rgba[(by * 4 * 8 + bx * 4) * 4..][..4];
                for y in 0..4 {
                    for x in 0..4 {
                        let i = ((by * 4 + y) * 8 + bx * 4 + x) * 4;
                        assert_eq!(&f.rgba[i..i + 4], first);
                    }
                }
            }
        }
    }

    #[test]
    fn pixelate_region_leaves_outside_untouched() {
        let mut f = gradient(16, 16);
        let orig = f.rgba.clone();
        let region = Rect01 { x: 0.5, y: 0.5, w: 0.5, h: 0.5 };
        run(&mut f, EffectKind::Pixelate { block: 4.0, region: Some(region) });
        assert_eq!(f.rgba[..4], orig[..4]);
        assert_ne!(f.rgba, orig);
    }

    #[test]
    fn crop_dimensions() {
        let f = gradient(100, 50);
        let c = crop(&f, &Crop { left: 0.1, top: 0.2, right: 0.3, bottom: 0.0 });
        assert_eq!((c.w, c.h), (60, 40));
        assert_eq!(c.rgba.len(), 60 * 40 * 4);
        assert_eq!(c.rgba[..4], f.rgba[(10 * 100 + 10) * 4..][..4]);
    }

    #[test]
    fn crop_never_below_one_pixel() {
        let f = gradient(10, 10);
        let c = crop(&f, &Crop { left: 0.9, top: 0.9, right: 0.9, bottom: 0.9 });
        assert_eq!((c.w, c.h), (1, 1));
    }

    #[test]
    fn identity_params_change_nothing() {
        let orig = gradient(6, 6);
        for e in [
            EffectKind::ColorAdjust { brightness: 0.0, contrast: 1.0, saturation: 1.0, gamma: 1.0 },
            EffectKind::WhiteBalance { temperature: 0.0, tint: 0.0 },
            EffectKind::HueShift { degrees: 0.0 },
            EffectKind::Sepia { amount: 0.0 },
            EffectKind::Vignette { strength: 0.0 },
            EffectKind::Sharpen { amount: 0.0 },
            EffectKind::Noise { amount: 0.0 },
        ] {
            let mut f = orig.clone();
            run(&mut f, e.clone());
            assert_eq!(f.rgba, orig.rgba, "{:?}", e);
        }
    }

    #[test]
    fn grayscale_equalizes_channels() {
        let mut f = gradient(6, 6);
        run(&mut f, EffectKind::Grayscale);
        assert!(f.rgba.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]));
    }

    #[test]
    fn color_adjust_moves_brightness_and_saturation() {
        let mut f = Frame::filled(2, 2, [100, 100, 100, 255]);
        run(&mut f, EffectKind::ColorAdjust { brightness: 0.2, contrast: 1.0, saturation: 1.0, gamma: 1.0 });
        assert!(f.rgba[0] > 140);
        let mut g = Frame::filled(2, 2, [200, 50, 50, 255]);
        run(&mut g, EffectKind::ColorAdjust { brightness: 0.0, contrast: 1.0, saturation: 0.0, gamma: 1.0 });
        assert!(g.rgba[0] == g.rgba[1] && g.rgba[1] == g.rgba[2]);
    }

    #[test]
    fn white_balance_warms() {
        let mut f = Frame::filled(2, 2, [100, 100, 100, 255]);
        run(&mut f, EffectKind::WhiteBalance { temperature: 1.0, tint: 0.0 });
        assert!(f.rgba[0] > 100 && f.rgba[2] < 100);
    }

    #[test]
    fn hue_shift_rotates_red_towards_green() {
        let mut f = Frame::filled(2, 2, [255, 0, 0, 255]);
        run(&mut f, EffectKind::HueShift { degrees: 120.0 });
        assert!(f.rgba[1] > f.rgba[0] && f.rgba[1] > f.rgba[2]);
    }

    #[test]
    fn vignette_darkens_corners_only() {
        let mut f = Frame::filled(21, 21, [200, 200, 200, 255]);
        run(&mut f, EffectKind::Vignette { strength: 1.0 });
        assert_eq!(f.rgba[(10 * 21 + 10) * 4], 200);
        assert!(f.rgba[0] < 100);
    }

    #[test]
    fn sharpen_increases_edge_contrast() {
        let mut f = Frame::filled(8, 1, [50, 50, 50, 255]);
        for x in 4..8 {
            f.rgba[x * 4..x * 4 + 3].fill(150);
        }
        run(&mut f, EffectKind::Sharpen { amount: 1.0 });
        assert!(f.rgba[3 * 4] < 50 && f.rgba[4 * 4] > 150);
    }

    #[test]
    fn noise_is_deterministic_per_seed() {
        let base = Frame::filled(8, 8, [128, 128, 128, 255]);
        let (mut a, mut b, mut c) = (base.clone(), base.clone(), base.clone());
        apply(&mut a, &EffectKind::Noise { amount: 0.5 }, 1.0, 3);
        apply(&mut b, &EffectKind::Noise { amount: 0.5 }, 1.0, 3);
        apply(&mut c, &EffectKind::Noise { amount: 0.5 }, 1.0, 4);
        assert_eq!(a.rgba, b.rgba);
        assert_ne!(a.rgba, c.rgba);
        assert_ne!(a.rgba, base.rgba);
    }

    #[test]
    fn chroma_key_clears_key_color_only() {
        let mut f = Frame::new(2, 1);
        f.rgba.copy_from_slice(&[0, 255, 0, 255, 255, 0, 0, 255]);
        run(&mut f, EffectKind::ChromaKey { color: [0, 255, 0], similarity: 0.3, smoothness: 0.1 });
        assert_eq!(f.rgba[3], 0);
        assert_eq!(f.rgba[7], 255);
    }

    #[test]
    fn effects_keep_alpha() {
        let orig = gradient(6, 6);
        for e in EffectKind::catalog() {
            if matches!(e, EffectKind::ChromaKey { .. }) {
                continue;
            }
            let mut f = orig.clone();
            run(&mut f, e);
            assert!(f.rgba.chunks_exact(4).all(|p| p[3] == 200));
        }
    }

    #[test]
    #[ignore]
    fn blur_1080p_radius_200_timing() {
        let mut f = gradient(1920, 1080);
        let t = Instant::now();
        apply(&mut f, &EffectKind::Blur { radius: 200.0, region: None }, 1.0, 0);
        let ms = t.elapsed().as_millis();
        println!("blur 1080p r200: {ms}ms");
        assert!(ms < 50);
    }
}
