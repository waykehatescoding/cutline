use super::{font_file, ExportOptions, Format};
use crate::model::*;
use std::path::{Path, PathBuf};
const INLINE_LIMIT: usize = 6000;

fn num(x: f64) -> String {
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

fn ev(x: f64) -> u32 {
    ((x / 2.0).round().max(1.0) as u32) * 2
}

fn hex(c: [u8; 3]) -> String {
    format!("0x{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn hex_alpha(c: [u8; 4], mul: f64) -> String {
    format!("{}@{}", hex([c[0], c[1], c[2]]), num(c[3] as f64 / 255.0 * mul))
}

fn esc_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").replace(':', "\\:")
}

fn temp_file(stem: &str, contents: &str) -> Result<PathBuf, String> {
    crate::ffmpeg::scratch_file(stem, "txt", contents.as_bytes()).map_err(|e| e.to_string())
}

fn visible(c: &Clip, t0: f64, dur: f64) -> bool {
    let s = c.start - t0;
    c.duration() > 1e-6 && s + c.duration() > 1e-6 && s < dur - 1e-6
}

struct Input<'a> {
    media: &'a Media,
    lo: f64,
    hi: f64,
    vuses: usize,
    vnext: usize,
    auses: usize,
    anext: usize,
}

struct VClip<'a> {
    c: &'a Clip,
    ext: f64,
    tr: Option<Transition>,
}

struct Chain {
    cur: String,
    pend: Vec<String>,
    w: u32,
    h: u32,
}

fn video_tracks(p: &Project, t0: f64, dur: f64) -> Vec<Vec<VClip<'_>>> {
    let mut out = vec![];
    for t in p.tracks.iter().rev().filter(|t| t.kind == TrackKind::Video && !t.hidden) {
        let mut list: Vec<VClip> = t
            .clips
            .iter()
            .filter(|c| visible(c, t0, dur))
            .filter(|c| match &c.source {
                ClipSource::Media(id) => p.media(*id).is_some_and(|m| m.has_video || m.kind == MediaKind::Image),
                _ => true,
            })
            .map(|c| VClip { c, ext: 0.0, tr: None })
            .collect();
        for i in 1..list.len() {
            let (prev, cur) = (list[i - 1].c, list[i].c);
            let plain = |c: &Clip| !matches!(c.source, ClipSource::Text(_));
            if let Some(tr) = cur.transition_in.filter(|t| t.duration > 1e-3) {
                if plain(prev) && plain(cur) && (prev.end() - cur.start).abs() < 1e-3 {
                    let d = tr.duration.min(cur.duration());
                    list[i].tr = Some(Transition { kind: tr.kind, duration: d });
                    list[i - 1].ext = d;
                }
            }
        }
        out.push(list);
    }
    out
}

fn audible(p: &Project, t0: f64, dur: f64) -> Vec<(&Track, &Clip)> {
    let solo = p.tracks.iter().any(|t| t.kind == TrackKind::Audio && t.solo);
    let mut out = vec![];
    for t in p.tracks.iter().filter(|t| t.kind == TrackKind::Audio && !t.muted && (!solo || t.solo)) {
        for c in t.clips.iter().filter(|c| !c.audio_muted && visible(c, t0, dur)) {
            if c.media_id().and_then(|id| p.media(id)).is_some_and(|m| m.has_audio) {
                out.push((t, c));
            }
        }
    }
    out
}

struct Build<'a> {
    p: &'a Project,
    w: u32,
    h: u32,
    fps: f64,
    t0: f64,
    dur: f64,
    base: usize,
    inputs: Vec<Input<'a>>,
    lines: Vec<String>,
    n: usize,
}

impl<'a> Build<'a> {
    fn new(p: &'a Project, w: u32, h: u32, fps: f64, t0: f64, dur: f64, base: usize) -> Self {
        Build { p, w, h, fps, t0, dur, base, inputs: vec![], lines: vec![], n: 0 }
    }

    fn reg(&mut self, id: MediaId, video: bool, lo: f64, hi: f64) {
        let Some(media) = self.p.media(id) else { return };
        let i = match self.inputs.iter().position(|x| x.media.id == id) {
            Some(i) => i,
            None => {
                self.inputs.push(Input { media, lo: f64::MAX, hi: 0.0, vuses: 0, vnext: 0, auses: 0, anext: 0 });
                self.inputs.len() - 1
            }
        };
        let inp = &mut self.inputs[i];
        inp.lo = inp.lo.min(lo.max(0.0));
        inp.hi = inp.hi.max(hi);
        if video {
            inp.vuses += 1;
        } else {
            inp.auses += 1;
        }
    }

    fn reg_video(&mut self, v: &VClip) {
        if let Some(id) = v.c.media_id() {
            let ext = v.ext * v.c.speed;
            if v.c.reverse {
                self.reg(id, true, v.c.in_point - ext, v.c.out_point);
            } else {
                self.reg(id, true, v.c.in_point, v.c.out_point + ext);
            }
        }
    }

    fn reg_audio(&mut self, c: &Clip) {
        if let Some(id) = c.media_id() {
            self.reg(id, false, c.in_point, c.out_point);
        }
    }

    fn ss(&self, i: usize) -> f64 {
        let inp = &self.inputs[i];
        if inp.media.kind == MediaKind::Image || inp.lo <= 1.0 {
            0.0
        } else {
            inp.lo - 0.5
        }
    }

    fn index(&self, id: MediaId) -> usize {
        self.inputs.iter().position(|x| x.media.id == id).unwrap()
    }

    fn take(&mut self, i: usize, video: bool) -> String {
        let at = self.base + i;
        let inp = &mut self.inputs[i];
        let (uses, next, tag) = if video { (inp.vuses, &mut inp.vnext, "v") } else { (inp.auses, &mut inp.anext, "a") };
        if uses == 1 {
            format!("{at}:{tag}")
        } else {
            *next += 1;
            format!("{tag}{at}_{}", *next - 1)
        }
    }

    fn splits(&mut self) {
        for (i, inp) in self.inputs.iter().enumerate() {
            let at = self.base + i;
            for (uses, tag, f) in [(inp.vuses, "v", "split"), (inp.auses, "a", "asplit")] {
                if uses > 1 {
                    let outs: String = (0..uses).map(|k| format!("[{tag}{at}_{k}]")).collect();
                    self.lines.push(format!("[{at}:{tag}]{f}={uses}{outs}"));
                }
            }
        }
    }

    fn input_args(&self) -> Vec<String> {
        let mut a = vec![];
        for (i, inp) in self.inputs.iter().enumerate() {
            if inp.media.kind == MediaKind::Image {
                a.extend(["-loop".into(), "1".into(), "-framerate".into(), num(self.fps), "-t".into(), num(inp.hi + 1.0)]);
            } else if self.ss(i) > 0.0 {
                a.extend(["-ss".into(), num(self.ss(i))]);
            }
            a.extend(crate::ffmpeg::SAFE_INPUT.map(String::from));
            a.extend(["-i".into(), inp.media.path.to_string_lossy().into_owned()]);
        }
        a
    }

    fn emit(&mut self, ins: &[&str], body: &str) -> String {
        self.n += 1;
        let out = format!("l{}", self.n);
        let ins: String = ins.iter().map(|s| format!("[{s}]")).collect();
        self.lines.push(format!("{ins}{body}[{out}]"));
        out
    }

    fn flush(&mut self, ch: &mut Chain) -> String {
        if !ch.pend.is_empty() {
            let body = ch.pend.join(",");
            ch.pend.clear();
            ch.cur = self.emit(&[&ch.cur.clone()], &body);
        }
        ch.cur.clone()
    }

    fn regional(&mut self, ch: &mut Chain, region: Option<Rect01>, f: impl Fn(u32, u32) -> String) {
        let Some(r) = region else {
            ch.pend.push(f(ch.w, ch.h));
            return;
        };
        let cur = self.flush(ch);
        let x = ((r.x as f64 * ch.w as f64) as u32 & !1).min(ch.w.saturating_sub(2));
        let y = ((r.y as f64 * ch.h as f64) as u32 & !1).min(ch.h.saturating_sub(2));
        let rw = (ev(r.w as f64 * ch.w as f64)).min(ch.w - x).max(2);
        let rh = (ev(r.h as f64 * ch.h as f64)).min(ch.h - y).max(2);
        self.n += 1;
        let (a, b) = (format!("s{}a", self.n), format!("s{}b", self.n));
        self.lines.push(format!("[{cur}]split[{a}][{b}]"));
        let piece = self.emit(&[&b], &format!("crop={rw}:{rh}:{x}:{y},{}", f(rw, rh)));
        ch.cur = self.emit(&[&a, &piece], &format!("overlay={x}:{y}:format=auto"));
    }

    fn effect(&mut self, ch: &mut Chain, kind: &EffectKind) {
        let scale = self.h as f64 / 1080.0;
        match kind {
            EffectKind::Blur { radius, region } => {
                let sigma = num((*radius as f64 * scale / 2.0).max(0.1));
                self.regional(ch, *region, |_, _| format!("gblur=sigma={sigma}"));
            }
            EffectKind::Pixelate { block, region } => {
                let b = (*block as f64 * scale).max(2.0);
                self.regional(ch, *region, |w, h| {
                    let (dw, dh) = (((w as f64 / b).ceil() as u32).max(1), ((h as f64 / b).ceil() as u32).max(1));
                    format!("scale={dw}:{dh}:flags=area,scale={w}:{h}:flags=neighbor")
                });
            }
            EffectKind::ColorAdjust { brightness, contrast, saturation, gamma } => {
                ch.pend.push(format!(
                    "eq=brightness={}:contrast={}:saturation={}:gamma={}",
                    num(*brightness as f64),
                    num(*contrast as f64),
                    num(*saturation as f64),
                    num(*gamma as f64)
                ));
            }
            EffectKind::WhiteBalance { temperature, tint } => {
                let t = (*temperature as f64).clamp(-1.0, 1.0) * 0.4;
                let g = -(*tint as f64).clamp(-1.0, 1.0) * 0.3;
                let (t, nt) = (num(t), num(-t));
                let g = num(g);
                ch.pend.push(format!("colorbalance=rs={t}:rm={t}:rh={t}:bs={nt}:bm={nt}:bh={nt}:gs={g}:gm={g}:gh={g}"));
            }
            EffectKind::HueShift { degrees } => ch.pend.push(format!("hue=h={}", num(*degrees as f64))),
            EffectKind::Grayscale => ch.pend.push("hue=s=0".into()),
            EffectKind::Sepia { amount } => {
                let a = (*amount as f64).clamp(0.0, 1.0);
                let m = |id: f64, s: f64| num(id * (1.0 - a) + s * a);
                ch.pend.push(format!(
                    "colorchannelmixer=rr={}:rg={}:rb={}:gr={}:gg={}:gb={}:br={}:bg={}:bb={}",
                    m(1.0, 0.393),
                    m(0.0, 0.769),
                    m(0.0, 0.189),
                    m(0.0, 0.349),
                    m(1.0, 0.686),
                    m(0.0, 0.168),
                    m(0.0, 0.272),
                    m(0.0, 0.534),
                    m(1.0, 0.131)
                ));
            }
            EffectKind::Invert => ch.pend.push("negate".into()),
            EffectKind::Vignette { strength } => {
                let angle = (1.4 - 1.1 * (*strength as f64).clamp(0.0, 1.0)).max(0.2);
                ch.pend.push(format!("vignette=angle={}", num(angle)));
            }
            EffectKind::Sharpen { amount } => ch.pend.push(format!("unsharp=5:5:{}:5:5:0", num((*amount as f64).clamp(0.0, 5.0)))),
            EffectKind::Noise { amount } => ch.pend.push(format!("noise=alls={}:allf=t", num((*amount as f64 * 100.0).clamp(0.0, 100.0)))),
            EffectKind::ChromaKey { color, similarity, smoothness } => {
                ch.pend.push(format!(
                    "chromakey=color={}:similarity={}:blend={}",
                    hex(*color),
                    num((*similarity as f64).max(0.01)),
                    num(*smoothness as f64)
                ));
            }
        }
    }

    fn item(&mut self, c: &Clip, ext: f64) -> Chain {
        let (ow, oh) = (self.w, self.h);
        let dur = c.duration();
        let mut ch = match &c.source {
            ClipSource::Media(id) => {
                let i = self.index(*id);
                let ss = self.ss(i);
                let pad = self.take(i, true);
                let media = self.inputs[i].media;
                let sp = c.speed.max(1e-6);
                let cap = if media.kind == MediaKind::Image || media.duration <= 0.0 { f64::MAX } else { media.duration };
                let (lo, hi, short) = if c.reverse {
                    let want = c.in_point - ext * sp;
                    let lo = want.max(0.0);
                    (lo, c.out_point, (lo - want) / sp)
                } else {
                    let want = c.out_point + ext * sp;
                    let hi = want.min(cap);
                    (c.in_point, hi, (want - hi) / sp)
                };
                let mut f = vec![format!("trim=start={}:end={}", num(lo - ss), num(hi - ss))];
                if c.reverse {
                    f.extend(["setpts=PTS-STARTPTS".into(), "reverse".into(), format!("setpts=PTS/{}", num(sp))]);
                } else {
                    f.push(format!("setpts=(PTS-STARTPTS)/{}", num(sp)));
                }
                f.push(format!("fps={}", num(self.fps)));
                if short > 1e-3 {
                    f.push(format!("tpad=stop_mode=clone:stop_duration={}", num(short)));
                }
                let (w, h) = if media.width >= 2 && media.height >= 2 { (media.width, media.height) } else { (ow, oh) };
                Chain { cur: pad, pend: f, w, h }
            }
            ClipSource::Color(col) => {
                let src = self.emit(&[], &format!("color=c={}:s={ow}x{oh}:r={}:d={}", hex_alpha(*col, 1.0), num(self.fps), num(dur + ext)));
                Chain { cur: src, pend: vec!["format=yuva420p".into()], w: ow, h: oh }
            }
            ClipSource::Text(_) => unreachable!(),
        };
        if !c.crop.is_none() {
            let k = &c.crop;
            let cw = ev(ch.w as f64 * (1.0 - k.left as f64 - k.right as f64)).min(ch.w);
            let chh = ev(ch.h as f64 * (1.0 - k.top as f64 - k.bottom as f64)).min(ch.h);
            let x = ((k.left as f64 * ch.w as f64) as u32 & !1).min(ch.w - cw);
            let y = ((k.top as f64 * ch.h as f64) as u32 & !1).min(ch.h - chh);
            ch.pend.push(format!("crop={cw}:{chh}:{x}:{y}"));
            ch.w = cw;
            ch.h = chh;
        }
        if c.transform.flip_h {
            ch.pend.push("hflip".into());
        }
        if c.transform.flip_v {
            ch.pend.push("vflip".into());
        }
        for e in c.effects.iter().filter(|e| e.enabled) {
            self.effect(&mut ch, &e.kind);
        }
        let fit = (ow as f64 / ch.w as f64).min(oh as f64 / ch.h as f64) * c.transform.scale as f64;
        let (tw, th) = (ev(ch.w as f64 * fit), ev(ch.h as f64 * fit));
        if (tw, th) != (ch.w, ch.h) {
            ch.pend.push(format!("scale={tw}:{th}:flags=bicubic"));
        }
        ch.pend.push("format=yuva420p".into());
        ch.w = tw;
        ch.h = th;
        if c.transform.rotation.abs() > 0.01 {
            let r = (c.transform.rotation as f64).to_radians();
            let (s, k) = (r.sin().abs(), r.cos().abs());
            let (rw, rh) = (ev(tw as f64 * k + th as f64 * s), ev(tw as f64 * s + th as f64 * k));
            ch.pend.push(format!("rotate=a={}:ow={rw}:oh={rh}:c=none", num(r)));
            ch.w = rw;
            ch.h = rh;
        }
        if c.opacity < 0.999 {
            ch.pend.push(format!("lut=a=val*{}", num(c.opacity.clamp(0.0, 1.0) as f64)));
        }
        if c.fade_in > 1e-3 {
            ch.pend.push(format!("fade=t=in:st=0:d={}:alpha=1", num(c.fade_in)));
        }
        if c.fade_out > 1e-3 {
            ch.pend.push(format!("fade=t=out:st={}:d={}:alpha=1", num((dur - c.fade_out).max(0.0)), num(c.fade_out)));
        }
        ch
    }

    fn pos(c: &Clip) -> (String, String) {
        (format!("{}*W-w/2", num(c.transform.x as f64)), format!("{}*H-h/2", num(c.transform.y as f64)))
    }

    fn text(&mut self, canvas: &mut String, c: &Clip, st: &TextStyle) -> Result<(), String> {
        let (s, e) = (c.start - self.t0, c.end() - self.t0);
        let fs = st.size as f64 * self.h as f64 / 1080.0 * c.transform.scale as f64;
        let lh = fs * 1.2;
        let tx = num(c.transform.x as f64);
        let x = match st.align {
            TextAlign::Left => format!("{tx}*W"),
            TextAlign::Center => format!("{tx}*W-text_w/2"),
            TextAlign::Right => format!("{tx}*W-text_w"),
        };
        let op = c.opacity as f64;
        let mut tail = String::new();
        if let Some(bg) = st.background {
            tail.push_str(&format!(":box=1:boxcolor={}:boxborderw={}", hex_alpha(bg, op), num((fs * 0.25).ceil())));
        }
        let mut terms = vec![];
        if c.fade_in > 1e-3 {
            terms.push(format!("(t-{})/{}", num(s), num(c.fade_in)));
        }
        if c.fade_out > 1e-3 {
            terms.push(format!("({}-t)/{}", num(e), num(c.fade_out)));
        }
        if let Some(first) = terms.first() {
            let inner = if terms.len() == 2 { format!("min({},{})", terms[0], terms[1]) } else { first.clone() };
            tail.push_str(&format!(":alpha='min(1,{inner})'"));
        }
        tail.push_str(&format!(":enable='between(t,{},{})'", num(s), num(e)));
        let lines: Vec<&str> = st.text.lines().collect();
        for (i, line) in lines.iter().enumerate().filter(|(_, l)| !l.is_empty()) {
            let file = temp_file("text", line)?;
            let y = format!("{}*H+({})", num(c.transform.y as f64), num((i as f64 - lines.len() as f64 / 2.0) * lh));
            let f = format!(
                "drawtext=fontfile='{}':textfile='{}':expansion=none:fontsize={}:fontcolor={}:x='{x}':y='{y}'{tail}",
                esc_path(&font_file(st.bold)),
                esc_path(&file),
                num(fs),
                hex_alpha(st.color, op)
            );
            *canvas = self.emit(&[&canvas.clone()], &f);
        }
        Ok(())
    }

    fn video(&mut self, tracks: &[Vec<VClip>], canvas: &mut String) -> Result<(), String> {
        for track in tracks {
            let mut i = 0;
            while i < track.len() {
                let v = &track[i];
                if let ClipSource::Text(st) = &v.c.source {
                    self.text(canvas, v.c, st)?;
                    i += 1;
                    continue;
                }
                let mut j = i;
                while j + 1 < track.len() && track[j + 1].tr.is_some() {
                    j += 1;
                }
                if j == i {
                    let (s, e) = (v.c.start - self.t0, v.c.end() - self.t0);
                    let mut ch = self.item(v.c, 0.0);
                    ch.pend.push(format!("setpts=PTS-STARTPTS+{}/TB", num(s)));
                    let item = self.flush(&mut ch);
                    let (x, y) = Self::pos(v.c);
                    let body = format!("overlay=x='{x}':y='{y}':eof_action=pass:enable='between(t,{},{})'", num(s), num(e));
                    *canvas = self.emit(&[&canvas.clone(), &item], &body);
                } else {
                    self.group(&track[i..=j], canvas);
                }
                i = j + 1;
            }
        }
        Ok(())
    }

    fn group(&mut self, items: &[VClip], canvas: &mut String) {
        let g0 = items[0].c.start;
        let mut cur = String::new();
        for v in items {
            let mut ch = self.item(v.c, v.ext);
            let item = self.flush(&mut ch);
            let len = v.c.duration() + v.ext;
            let blank =
                self.emit(&[], &format!("color=c=black@0:s={}x{}:r={}:d={},format=yuva420p", self.w, self.h, num(self.fps), num(len)));
            let (x, y) = Self::pos(v.c);
            let layer = self.emit(&[&blank, &item], &format!("overlay=x='{x}':y='{y}':shortest=1:format=auto"));
            cur = match v.tr {
                None => layer,
                Some(tr) => {
                    let body = format!("xfade=transition={}:duration={}:offset={}", tr.kind.xfade(), num(tr.duration), num(v.c.start - g0));
                    self.emit(&[&cur.clone(), &layer], &body)
                }
            };
        }
        let (s, e) = (g0 - self.t0, items[items.len() - 1].c.end() - self.t0);
        let shifted = self.emit(&[&cur], &format!("setpts=PTS-STARTPTS+{}/TB", num(s)));
        let body = format!("overlay=eof_action=pass:format=auto:enable='between(t,{},{})'", num(s), num(e));
        *canvas = self.emit(&[&canvas.clone(), &shifted], &body);
    }

    fn audio(&mut self, clips: &[(&Track, &Clip)], sr: u32, layout: &str) -> String {
        let mut labels = vec![];
        for (t, c) in clips {
            let i = self.index(c.media_id().unwrap());
            let ss = self.ss(i);
            let pad = self.take(i, false);
            let sp = c.speed.max(1e-6);
            let mut f = vec![format!("atrim=start={}:end={}", num(c.in_point - ss), num(c.out_point - ss)), "asetpts=PTS-STARTPTS".into()];
            if c.reverse {
                f.extend(["areverse".into(), "asetpts=PTS-STARTPTS".into()]);
            }
            let mut rest = sp;
            let mut factors = vec![];
            while rest > 2.0 {
                factors.push(2.0);
                rest /= 2.0;
            }
            while rest < 0.5 {
                factors.push(0.5);
                rest /= 0.5;
            }
            if (rest - 1.0).abs() > 1e-6 {
                factors.push(rest);
            }
            f.extend(factors.iter().map(|x| format!("atempo={}", num(*x))));
            let vol = c.volume as f64 * t.volume as f64;
            if (vol - 1.0).abs() > 1e-3 {
                f.push(format!("volume={}", num(vol)));
            }
            let dur = c.duration();
            if c.fade_in > 1e-3 {
                f.push(format!("afade=t=in:st=0:d={}", num(c.fade_in)));
            }
            if c.fade_out > 1e-3 {
                f.push(format!("afade=t=out:st={}:d={}", num((dur - c.fade_out).max(0.0)), num(c.fade_out)));
            }
            let s = c.start - self.t0;
            if s < -1e-6 {
                f.extend([format!("atrim=start={}", num(-s)), "asetpts=PTS-STARTPTS".into()]);
            } else if s > 1e-6 {
                f.push(format!("adelay={}:all=1", num((s * 1000.0).round())));
            }
            f.push(format!("aformat=sample_rates={sr}:sample_fmts=fltp:channel_layouts={layout}"));
            labels.push(self.emit(&[&pad], &f.join(",")));
        }
        let tail = format!("apad,atrim=end={}", num(self.dur));
        if labels.len() == 1 {
            self.emit(&[&labels[0]], &tail)
        } else {
            let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            self.emit(&refs, &format!("amix=inputs={}:normalize=0:duration=longest,{tail}", labels.len()))
        }
    }

    fn silence(&mut self, sr: u32) -> String {
        self.emit(&[], &format!("anullsrc=r={sr}:cl=stereo,atrim=end={}", num(self.dur)))
    }

    fn graph(&self) -> String {
        self.lines.join(";")
    }
}

fn filter_args(graph: String) -> Result<Vec<String>, String> {
    if graph.len() <= INLINE_LIMIT {
        return Ok(vec!["-filter_complex".into(), graph]);
    }
    let file = temp_file("graph", &graph)?;
    Ok(vec!["-/filter_complex".into(), file.to_string_lossy().into_owned()])
}

fn encoder_args(opts: &ExportOptions, sr: u32) -> Vec<String> {
    let q = opts.quality.min(100) as f64;
    let kbps = format!("{}k", opts.audio_kbps);
    let aac = ["-c:a", "aac", "-b:a", &kbps];
    let list: Vec<String> = match opts.format {
        Format::Mp4H264 => [
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            &num((35.0 - q * 0.17).round()),
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ]
        .iter()
        .chain(&aac)
        .map(|s| s.to_string())
        .collect(),
        Format::Mp4H265 => [
            "-c:v",
            "libx265",
            "-preset",
            "veryfast",
            "-crf",
            &num((38.0 - q * 0.18).round()),
            "-tag:v",
            "hvc1",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ]
        .iter()
        .chain(&aac)
        .map(|s| s.to_string())
        .collect(),
        Format::WebmVp9 => [
            "-c:v",
            "libvpx-vp9",
            "-b:v",
            "0",
            "-crf",
            &num((45.0 - q * 0.3).round()),
            "-row-mt",
            "1",
            "-deadline",
            "good",
            "-cpu-used",
            "4",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "libopus",
            "-b:a",
            &kbps,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        Format::MovProRes => {
            ["-c:v", "prores_ks", "-profile:v", "2", "-pix_fmt", "yuv422p10le", "-c:a", "pcm_s16le"].iter().map(|s| s.to_string()).collect()
        }
        Format::Gif => vec![],
        Format::Mp3 => ["-c:a", "libmp3lame", "-b:a", &kbps].iter().map(|s| s.to_string()).collect(),
        Format::Wav => ["-c:a", "pcm_s16le"].iter().map(|s| s.to_string()).collect(),
    };
    let mut a = list;
    if opts.format.has_audio() {
        a.extend(["-ar".into(), sr.to_string(), "-ac".into(), "2".into()]);
    }
    a
}

pub fn export_args(project: &Project, opts: &ExportOptions, out: &Path) -> Result<Vec<String>, String> {
    let (t0, t1) = opts.range.unwrap_or((0.0, project.duration()));
    let dur = t1 - t0;
    if dur <= 0.01 {
        return Err("Nothing to export".into());
    }
    let video = !opts.format.audio_only();
    let sr = project.settings.sample_rate;
    let (w, h) = (ev(opts.width as f64), ev(opts.height as f64));
    let mut b = Build::new(project, w, h, opts.fps, t0, dur, video as usize);
    let tracks = if video { video_tracks(project, t0, dur) } else { vec![] };
    for v in tracks.iter().flatten() {
        b.reg_video(v);
    }
    let clips = if opts.format.has_audio() { audible(project, t0, dur) } else { vec![] };
    for (_, c) in &clips {
        b.reg_audio(c);
    }
    b.splits();
    let mut args: Vec<String> = ["-hide_banner", "-y", "-nostats", "-progress", "pipe:1"].iter().map(|s| s.to_string()).collect();
    if video {
        let bg = project.settings.background;
        args.extend([
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            format!("color=c={}:s={w}x{h}:r={}:d={}", hex([bg[0], bg[1], bg[2]]), num(opts.fps), num(dur)),
        ]);
        let mut canvas = "0:v".to_string();
        b.video(&tracks, &mut canvas)?;
        if opts.format == Format::Gif {
            let fps = num(opts.fps.min(25.0));
            b.lines.push(format!("[{canvas}]fps={fps},split[g0][g1];[g0]palettegen[gp];[g1][gp]paletteuse[vout]"));
        } else {
            b.lines.push(format!("[{canvas}]null[vout]"));
        }
    }
    if opts.format.has_audio() {
        let label = if clips.is_empty() { b.silence(sr) } else { b.audio(&clips, sr, "stereo") };
        b.lines.push(format!("[{label}]anull[aout]"));
    }
    args.extend(b.input_args());
    args.extend(filter_args(b.graph())?);
    if video {
        args.extend(["-map".into(), "[vout]".into()]);
    }
    if opts.format.has_audio() {
        args.extend(["-map".into(), "[aout]".into()]);
    }
    args.extend(encoder_args(opts, sr));
    args.extend(["-t".into(), num(dur), out.to_string_lossy().into_owned()]);
    Ok(args)
}

pub fn audio_preview_args(project: &Project, from: f64, sample_rate: u32, channels: u16) -> Option<Vec<String>> {
    let dur = project.duration() - from;
    if dur <= 0.01 {
        return None;
    }
    let clips = audible(project, from, dur);
    if clips.is_empty() {
        return None;
    }
    let mut b = Build::new(project, 0, 0, 30.0, from, dur, 0);
    for (_, c) in &clips {
        b.reg_audio(c);
    }
    b.splits();
    let layout = if channels == 1 { "mono" } else { "stereo" };
    let label = b.audio(&clips, sample_rate, layout);
    b.lines.push(format!("[{label}]anull[aout]"));
    let mut args: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin"].iter().map(|s| s.to_string()).collect();
    args.extend(b.input_args());
    args.extend(filter_args(b.graph()).ok()?);
    args.extend(["-map", "[aout]", "-f", "f32le", "-ar"].iter().map(|s| s.to_string()));
    args.extend([sample_rate.to_string(), "-ac".into(), channels.to_string(), "pipe:1".into()]);
    Some(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    fn media(id: u64, path: &str, kind: MediaKind, dur: f64, has_v: bool, has_a: bool) -> Media {
        Media {
            id: MediaId(id),
            path: PathBuf::from(path),
            kind,
            duration: dur,
            width: 320,
            height: 240,
            fps: 30.0,
            has_video: has_v,
            has_audio: has_a,
            audio_channels: 2,
            sample_rate: 44100,
            codec: String::new(),
        }
    }

    fn opts(format: Format) -> ExportOptions {
        ExportOptions { format, width: 640, height: 360, fps: 30.0, quality: 70, audio_kbps: 128, range: None }
    }

    fn graph_of(args: &[String]) -> String {
        let i = args.iter().position(|a| a == "-filter_complex" || a == "-/filter_complex").unwrap();
        if args[i] == "-/filter_complex" {
            std::fs::read_to_string(&args[i + 1]).unwrap()
        } else {
            args[i + 1].clone()
        }
    }

    fn simple() -> Project {
        let mut p = Project::default();
        p.media.push(media(100, "a.mp4", MediaKind::Video, 10.0, true, true));
        let c = p.new_clip(ClipSource::Media(MediaId(100)), "a".into(), 0.0, 0.0, 4.0);
        p.tracks[1].clips.push(c);
        let a = p.new_clip(ClipSource::Media(MediaId(100)), "a".into(), 0.0, 0.0, 4.0);
        p.tracks[2].clips.push(a);
        p
    }

    fn text_clip(p: &mut Project, text: &str, start: f64, len: f64) -> Clip {
        p.new_clip(
            ClipSource::Text(TextStyle {
                text: text.into(),
                size: 48.0,
                color: [255, 255, 255, 255],
                background: Some([0, 0, 0, 128]),
                bold: true,
                align: TextAlign::Center,
            }),
            "t".into(),
            start,
            0.0,
            len,
        )
    }

    #[test]
    fn empty_project_is_an_error() {
        assert!(export_args(&Project::default(), &opts(Format::Mp4H264), Path::new("o.mp4")).is_err());
    }

    #[test]
    fn mp4_args_have_header_and_encoder() {
        let a = export_args(&simple(), &opts(Format::Mp4H264), Path::new("o.mp4")).unwrap();
        assert_eq!(&a[..5], ["-hide_banner", "-y", "-nostats", "-progress", "pipe:1"]);
        assert!(a.contains(&"libx264".to_string()) && a.contains(&"+faststart".to_string()));
        assert_eq!(a.last().unwrap(), "o.mp4");
    }

    #[test]
    fn graph_has_expected_filters() {
        let mut p = simple();
        {
            let c = &mut p.tracks[1].clips[0];
            c.speed = 2.0;
            c.reverse = true;
            c.crop = Crop { left: 0.1, top: 0.0, right: 0.1, bottom: 0.0 };
            c.effects.push(Effect {
                enabled: true,
                kind: EffectKind::Blur { radius: 10.0, region: Some(Rect01 { x: 0.1, y: 0.1, w: 0.3, h: 0.3 }) },
            });
            c.effects.push(Effect { enabled: true, kind: EffectKind::Pixelate { block: 16.0, region: None } });
        }
        p.tracks[2].clips[0].speed = 4.0;
        let t = text_clip(&mut p, "Hi: 100%", 1.0, 1.0);
        p.tracks[0].clips.push(t);
        let g = graph_of(&export_args(&p, &opts(Format::Mp4H264), Path::new("o.mp4")).unwrap());
        for key in ["trim=", "reverse", "crop=", "gblur", "neighbor", "drawtext", "atempo=2,atempo=2", "overlay"] {
            assert!(g.contains(key), "missing {key} in {g}");
        }
    }

    #[test]
    fn transition_uses_xfade() {
        let mut p = Project::default();
        p.media.push(media(100, "a.mp4", MediaKind::Video, 10.0, true, false));
        let a = p.new_clip(ClipSource::Media(MediaId(100)), "a".into(), 0.0, 0.0, 3.0);
        let mut b = p.new_clip(ClipSource::Media(MediaId(100)), "b".into(), 3.0, 4.0, 7.0);
        b.transition_in = Some(Transition { kind: TransitionKind::WipeLeft, duration: 1.0 });
        p.tracks[1].clips.push(a);
        p.tracks[1].clips.push(b);
        let g = graph_of(&export_args(&p, &opts(Format::Mp4H264), Path::new("o.mp4")).unwrap());
        assert!(g.contains("xfade=transition=wipeleft:duration=1:offset=3"), "{g}");
    }

    #[test]
    fn muted_or_unsoloed_tracks_are_silent() {
        let mut p = simple();
        p.tracks[2].muted = true;
        assert!(audio_preview_args(&p, 0.0, 48000, 2).is_none());
        p.tracks[2].muted = false;
        assert!(audio_preview_args(&p, 3.9, 48000, 2).is_some());
        assert!(audio_preview_args(&p, 4.5, 48000, 2).is_none());
        p.tracks[3].solo = true;
        assert!(audio_preview_args(&p, 0.0, 48000, 2).is_none());
    }

    #[test]
    fn preview_args_stream_raw_floats() {
        let a = audio_preview_args(&simple(), 1.0, 48000, 2).unwrap();
        assert!(a.windows(2).any(|w| w == ["-f", "f32le"]));
        assert_eq!(a.last().unwrap(), "pipe:1");
        assert!(a.contains(&"-nostdin".to_string()));
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cutline-graph-test-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn gen(dir: &Path, name: &str, args: &[&str]) -> PathBuf {
        let out = dir.join(name);
        let st = crate::ffmpeg::command("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(args)
            .arg(&out)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(st.success());
        out
    }

    fn probe(path: &Path) -> f64 {
        let o = crate::ffmpeg::command("ffprobe")
            .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
            .arg(path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).trim().parse().unwrap()
    }

    fn run(args: &[String]) {
        let o = crate::ffmpeg::command("ffmpeg").args(args).output().unwrap();
        assert!(o.status.success(), "{}\n{}", args.join(" "), String::from_utf8_lossy(&o.stderr));
    }

    fn fixtures(dir: &Path) -> Project {
        let v = gen(
            dir,
            "v.mp4",
            &[
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=6:size=320x240:rate=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=6",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest",
            ],
        );
        let img = gen(dir, "i.png", &["-f", "lavfi", "-i", "testsrc=size=200x200:rate=1", "-frames:v", "1"]);
        let mut p = Project::default();
        p.media.push(media(100, v.to_str().unwrap(), MediaKind::Video, 6.0, true, true));
        let mut im = media(101, img.to_str().unwrap(), MediaKind::Image, 5.0, true, false);
        im.width = 200;
        im.height = 200;
        p.media.push(im);
        p
    }

    #[test]
    fn integration_full_feature_export() {
        if !crate::ffmpeg::available() {
            return;
        }
        let dir = scratch("full");
        let mut p = fixtures(&dir);
        let m = ClipSource::Media(MediaId(100));
        let mut a = p.new_clip(m.clone(), "a".into(), 0.0, 0.0, 3.0);
        a.crop = Crop { left: 0.1, top: 0.1, right: 0.1, bottom: 0.1 };
        a.fade_in = 0.5;
        a.opacity = 0.9;
        a.effects.push(Effect {
            enabled: true,
            kind: EffectKind::Blur { radius: 12.0, region: Some(Rect01 { x: 0.1, y: 0.1, w: 0.4, h: 0.4 }) },
        });
        a.effects.push(Effect {
            enabled: true,
            kind: EffectKind::Pixelate { block: 20.0, region: Some(Rect01 { x: 0.5, y: 0.5, w: 0.4, h: 0.4 }) },
        });
        a.effects
            .push(Effect { enabled: true, kind: EffectKind::ColorAdjust { brightness: 0.05, contrast: 1.1, saturation: 1.2, gamma: 1.0 } });
        let mut b = p.new_clip(m.clone(), "b".into(), 3.0, 1.0, 5.0);
        b.speed = 2.0;
        b.transition_in = Some(Transition { kind: TransitionKind::Crossfade, duration: 0.5 });
        let mut c = p.new_clip(m.clone(), "c".into(), 5.0, 0.0, 2.0);
        c.reverse = true;
        c.transition_in = Some(Transition { kind: TransitionKind::SlideLeft, duration: 0.5 });
        c.transform.rotation = 10.0;
        c.transform.scale = 0.7;
        c.transform.flip_h = true;
        p.tracks[1].clips.extend([a, b, c]);
        let mut img = p.new_clip(ClipSource::Media(MediaId(101)), "img".into(), 1.0, 0.0, 2.0);
        img.transform.scale = 0.3;
        img.transform.x = 0.8;
        img.transform.y = 0.2;
        let mut col = p.new_clip(ClipSource::Color([255, 0, 0, 255]), "col".into(), 4.0, 0.0, 1.0);
        col.opacity = 0.4;
        let mut t = text_clip(&mut p, "Hello: 100% it's\nline two", 0.5, 3.0);
        t.fade_in = 0.3;
        let mut k = p.new_clip(m.clone(), "k".into(), 6.0, 0.0, 1.0);
        k.effects.push(Effect { enabled: true, kind: EffectKind::ChromaKey { color: [0, 255, 0], similarity: 0.3, smoothness: 0.1 } });
        for e in [
            EffectKind::Grayscale,
            EffectKind::Sepia { amount: 0.5 },
            EffectKind::Invert,
            EffectKind::Vignette { strength: 0.5 },
            EffectKind::Sharpen { amount: 1.0 },
            EffectKind::Noise { amount: 0.2 },
            EffectKind::HueShift { degrees: 30.0 },
            EffectKind::WhiteBalance { temperature: 0.3, tint: -0.2 },
        ] {
            k.effects.push(Effect { enabled: true, kind: e });
        }
        p.tracks[0].clips.extend([t, img, col, k]);
        let mut au = p.new_clip(m, "au".into(), 0.0, 0.5, 5.5);
        au.speed = 2.0;
        au.fade_out = 0.5;
        p.tracks[2].clips.push(au);
        let out = dir.join("out.mp4");
        run(&export_args(&p, &opts(Format::Mp4H264), &out).unwrap());
        let dur = probe(&out);
        assert!((dur - p.duration()).abs() < 0.2, "{dur} vs {}", p.duration());
    }

    #[test]
    fn integration_range_and_formats() {
        if !crate::ffmpeg::available() {
            return;
        }
        let dir = scratch("fmt");
        let mut p = fixtures(&dir);
        let a = p.new_clip(ClipSource::Media(MediaId(100)), "a".into(), 0.0, 0.0, 6.0);
        let mut au = p.new_clip(ClipSource::Media(MediaId(100)), "au".into(), 0.0, 0.0, 6.0);
        au.volume = 0.5;
        p.tracks[1].clips.push(a);
        p.tracks[2].clips.push(au);
        let mut o = opts(Format::Mp4H264);
        o.range = Some((2.0, 4.0));
        let out = dir.join("r.mp4");
        run(&export_args(&p, &o, &out).unwrap());
        assert!((probe(&out) - 2.0).abs() < 0.2);
        for (f, name) in [
            (Format::WebmVp9, "o.webm"),
            (Format::Gif, "o.gif"),
            (Format::Wav, "o.wav"),
            (Format::Mp3, "o.mp3"),
            (Format::MovProRes, "o.mov"),
            (Format::Mp4H265, "o265.mp4"),
        ] {
            let mut o = opts(f);
            o.range = Some((0.0, 2.0));
            let out = dir.join(name);
            run(&export_args(&p, &o, &out).unwrap());
            assert!((probe(&out) - 2.0).abs() < 0.3, "{name}");
        }
    }

    #[test]
    fn integration_long_graph_uses_file() {
        if !crate::ffmpeg::available() {
            return;
        }
        let dir = scratch("long");
        let mut p = Project::default();
        for i in 0..80 {
            let t = text_clip(&mut p, &format!("n{i}"), i as f64 * 0.05, 0.05);
            p.tracks[0].clips.push(t);
        }
        let out = dir.join("long.mp4");
        let args = export_args(&p, &opts(Format::Mp4H264), &out).unwrap();
        assert!(args.contains(&"-/filter_complex".to_string()));
        run(&args);
        assert!((probe(&out) - 4.0).abs() < 0.2);
    }

    #[test]
    fn integration_audio_preview_produces_samples() {
        if !crate::ffmpeg::available() {
            return;
        }
        let dir = scratch("prev");
        let mut p = fixtures(&dir);
        let au = p.new_clip(ClipSource::Media(MediaId(100)), "au".into(), 1.0, 0.0, 3.0);
        p.tracks[2].clips.push(au);
        let args = audio_preview_args(&p, 2.0, 48000, 2).unwrap();
        let o = crate::ffmpeg::command("ffmpeg").args(&args).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let secs = o.stdout.len() as f64 / (48000.0 * 2.0 * 4.0);
        assert!((secs - 2.0).abs() < 0.1, "{secs}");
    }
}
