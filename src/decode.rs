use crate::model::*;
use eframe::egui;
use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Stdio};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

type Key = (MediaId, u64, u32, u32);

const CACHE_BYTES: usize = 256 << 20;
const IDLE: Duration = Duration::from_secs(2);
const AHEAD: usize = 8;

fn fit_size(sw: u32, sh: u32, mw: u32, mh: u32) -> (u32, u32) {
    let s = (mw as f64 / sw.max(1) as f64).min(mh as f64 / sh.max(1) as f64).min(1.0);
    let even = |v: u32| ((v as f64 * s) as u32 & !1).max(2);
    (even(sw), even(sh))
}

fn media_fps(media: &Media) -> f64 {
    if media.fps > 0.0 {
        media.fps
    } else {
        30.0
    }
}

struct Lru {
    map: HashMap<Key, (Arc<Frame>, u64)>,
    bytes: usize,
    tick: u64,
    cap: usize,
}

impl Lru {
    fn new(cap: usize) -> Self {
        Self { map: HashMap::new(), bytes: 0, tick: 0, cap }
    }

    fn get(&mut self, key: &Key) -> Option<Arc<Frame>> {
        self.tick += 1;
        let tick = self.tick;
        self.map.get_mut(key).map(|e| {
            e.1 = tick;
            e.0.clone()
        })
    }

    fn put(&mut self, key: Key, frame: Arc<Frame>) {
        self.tick += 1;
        self.bytes += frame.rgba.len();
        if let Some(old) = self.map.insert(key, (frame, self.tick)) {
            self.bytes -= old.0.rgba.len();
        }
        while self.bytes > self.cap && self.map.len() > 1 {
            let oldest = *self.map.iter().min_by_key(|(_, e)| e.1).unwrap().0;
            self.bytes -= self.map.remove(&oldest).unwrap().0.rgba.len();
        }
    }
}

struct Shared {
    lru: Lru,
    last: HashMap<MediaId, Arc<Frame>>,
}

struct Req {
    media: Media,
    t: f64,
    w: u32,
    h: u32,
    key: Key,
}

#[derive(Default)]
struct SlotState {
    req: Option<Req>,
    closed: bool,
}

#[derive(Default)]
struct Slot {
    state: Mutex<SlotState>,
    cv: Condvar,
}

struct Scrub {
    slot: Arc<Slot>,
    asked: Option<Key>,
}

impl Scrub {
    fn new(shared: Arc<Mutex<Shared>>, ctx: egui::Context) -> Self {
        let slot = Arc::new(Slot::default());
        let worker = slot.clone();
        std::thread::spawn(move || loop {
            let req = {
                let mut s = worker.state.lock().unwrap();
                loop {
                    if s.closed {
                        return;
                    }
                    if let Some(r) = s.req.take() {
                        break r;
                    }
                    s = worker.cv.wait(s).unwrap();
                }
            };
            if shared.lock().unwrap().lru.get(&req.key).is_some() {
                continue;
            }
            if let Ok(f) = grab_frame(&req.media, req.t, req.w, req.h) {
                let f = Arc::new(f);
                let mut sh = shared.lock().unwrap();
                sh.lru.put(req.key, f.clone());
                sh.last.insert(req.media.id, f);
                drop(sh);
                ctx.request_repaint();
            }
        });
        Self { slot, asked: None }
    }

    fn submit(&mut self, req: Req) {
        self.asked = Some(req.key);
        self.slot.state.lock().unwrap().req = Some(req);
        self.slot.cv.notify_one();
    }
}

impl Drop for Scrub {
    fn drop(&mut self) {
        self.slot.state.lock().unwrap().closed = true;
        self.slot.cv.notify_one();
    }
}

struct Stream {
    child: Child,
    rx: Receiver<Arc<Frame>>,
    start: f64,
    fps: f64,
    received: u64,
    w: u32,
    h: u32,
    last_t: f64,
    seen: Instant,
    cur: Option<Arc<Frame>>,
    next: Option<(f64, Arc<Frame>)>,
}

impl Stream {
    fn spawn(media: &Media, start: f64, w: u32, h: u32, ctx: egui::Context) -> Option<Self> {
        let fps = media_fps(media);
        let mut child = crate::ffmpeg::command("ffmpeg")
            .args(["-v", "error", "-hwaccel", "auto", "-threads", "0", "-ss"])
            .arg(format!("{:.3}", start.max(0.0)))
            .args(crate::ffmpeg::SAFE_INPUT)
            .arg("-i")
            .arg(&media.path)
            .args(["-an", "-sn", "-vf"])
            .arg(format!("scale={w}:{h}:flags=bilinear,fps={fps}"))
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut out = child.stdout.take()?;
        let (tx, rx) = sync_channel(AHEAD);
        std::thread::spawn(move || loop {
            let mut rgba = vec![0u8; (w * h * 4) as usize];
            if out.read_exact(&mut rgba).is_err() || tx.send(Arc::new(Frame { w, h, rgba })).is_err() {
                break;
            }
            ctx.request_repaint();
        });
        Some(Self { child, rx, start, fps, received: 0, w, h, last_t: start, seen: Instant::now(), cur: None, next: None })
    }

    fn advance(&mut self, t: f64) -> Option<Arc<Frame>> {
        loop {
            if self.next.is_none() {
                match self.rx.try_recv() {
                    Ok(f) => {
                        self.next = Some((self.start + self.received as f64 / self.fps, f));
                        self.received += 1;
                    }
                    Err(_) => break,
                }
            }
            match self.next.take() {
                Some((pts, f)) if pts <= t => self.cur = Some(f),
                other => {
                    self.next = other;
                    break;
                }
            }
        }
        self.cur.clone()
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Decoder {
    ctx: egui::Context,
    shared: Arc<Mutex<Shared>>,
    scrubs: HashMap<MediaId, Scrub>,
    streams: HashMap<MediaId, Stream>,
}

impl Decoder {
    pub fn new(ctx: egui::Context) -> Self {
        let shared = Shared { lru: Lru::new(CACHE_BYTES), last: HashMap::new() };
        Self { ctx, shared: Arc::new(Mutex::new(shared)), scrubs: HashMap::new(), streams: HashMap::new() }
    }

    pub fn frame(&mut self, media: &Media, t: f64, max_w: u32, max_h: u32, playing: bool) -> Option<Arc<Frame>> {
        let image = media.kind == MediaKind::Image;
        let (w, h) = fit_size(media.width, media.height, max_w, max_h);
        self.streams.retain(|_, s| s.seen.elapsed() < IDLE);
        if playing && !image {
            if let Some(f) = self.stream_frame(media, t, w, h) {
                self.shared.lock().unwrap().last.insert(media.id, f.clone());
                return Some(f);
            }
            if let Some(f) = self.shared.lock().unwrap().last.get(&media.id) {
                return Some(f.clone());
            }
        } else {
            self.streams.remove(&media.id);
        }
        let fps = media_fps(media);
        let idx = if image { 0 } else { (t.clamp(0.0, (media.duration - 1.0 / fps).max(0.0)) * fps).round() as u64 };
        let key = (media.id, idx, w, h);
        let mut shared = self.shared.lock().unwrap();
        if let Some(f) = shared.lru.get(&key) {
            shared.last.insert(media.id, f.clone());
            return Some(f);
        }
        let nearest = shared.last.get(&media.id).cloned();
        drop(shared);
        let (ctx, sh) = (&self.ctx, &self.shared);
        let scrub = self.scrubs.entry(media.id).or_insert_with(|| Scrub::new(sh.clone(), ctx.clone()));
        if scrub.asked != Some(key) {
            scrub.submit(Req { media: media.clone(), t: idx as f64 / fps, w, h, key });
        }
        nearest
    }

    fn stream_frame(&mut self, media: &Media, t: f64, w: u32, h: u32) -> Option<Arc<Frame>> {
        let restart = self.streams.get(&media.id).map_or(true, |s| s.w != w || s.h != h || t + 0.05 < s.last_t || t > s.last_t + 1.0);
        if restart {
            match Stream::spawn(media, t, w, h, self.ctx.clone()) {
                Some(s) => {
                    self.streams.insert(media.id, s);
                }
                None => {
                    self.streams.remove(&media.id);
                    return None;
                }
            }
        }
        let s = self.streams.get_mut(&media.id)?;
        s.seen = Instant::now();
        s.last_t = t;
        s.advance(t)
    }
}

pub fn grab_frame(media: &Media, t: f64, w: u32, h: u32) -> Result<Frame, String> {
    let mut cmd = crate::ffmpeg::command("ffmpeg");
    cmd.args(["-v", "error", "-hwaccel", "auto", "-threads", "0"]);
    if media.kind != MediaKind::Image {
        cmd.arg("-ss").arg(format!("{:.3}", t.max(0.0)));
    }
    let out = cmd
        .args(crate::ffmpeg::SAFE_INPUT)
        .arg("-i")
        .arg(&media.path)
        .args(["-frames:v", "1", "-an", "-sn", "-vf"])
        .arg(format!("scale={w}:{h}:flags=bilinear"))
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
        .output()
        .map_err(|e| e.to_string())?;
    if out.stdout.len() != (w * h * 4) as usize {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if msg.is_empty() { "no frame decoded".into() } else { msg });
    }
    Ok(Frame { w, h, rgba: out.stdout })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn clip(tag: &str) -> Option<Media> {
        if !crate::ffmpeg::available() {
            return None;
        }
        let path: PathBuf = std::env::temp_dir().join(format!("cutline_decode_{}_{}.mp4", tag, std::process::id()));
        let ok = crate::ffmpeg::command("ffmpeg")
            .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc=duration=3:size=160x90:rate=10", "-pix_fmt", "yuv420p"])
            .arg(&path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        ok.then(|| Media {
            id: MediaId(1),
            path,
            kind: MediaKind::Video,
            duration: 3.0,
            width: 160,
            height: 90,
            fps: 10.0,
            has_video: true,
            has_audio: false,
            audio_channels: 0,
            sample_rate: 0,
            codec: "h264".into(),
        })
    }

    #[test]
    fn fit_keeps_aspect_even_and_never_upscales() {
        assert_eq!(fit_size(1920, 1080, 960, 960), (960, 540));
        assert_eq!(fit_size(160, 90, 1920, 1080), (160, 90));
        assert_eq!(fit_size(101, 51, 1000, 1000), (100, 50));
    }

    #[test]
    fn lru_evicts_least_recently_used_over_cap() {
        let mut lru = Lru::new(2 * 16);
        let k = |i| (MediaId(1), i, 2, 2);
        lru.put(k(0), Arc::new(Frame::new(2, 2)));
        lru.put(k(1), Arc::new(Frame::new(2, 2)));
        assert!(lru.get(&k(0)).is_some());
        lru.put(k(2), Arc::new(Frame::new(2, 2)));
        assert!(lru.get(&k(1)).is_none());
        assert!(lru.get(&k(0)).is_some());
        assert!(lru.get(&k(2)).is_some());
    }

    #[test]
    fn grab_frame_returns_requested_dimensions() {
        let Some(m) = clip("grab") else { return };
        let f = grab_frame(&m, 1.0, 80, 44).unwrap();
        assert_eq!((f.w, f.h, f.rgba.len()), (80, 44, 80 * 44 * 4));
        assert!(f.rgba.iter().any(|&b| b != 0));
        let mut missing = m.clone();
        missing.path = PathBuf::from("does_not_exist.mp4");
        assert!(grab_frame(&missing, 0.0, 80, 44).is_err());
    }

    #[test]
    fn scrub_frame_arrives_in_background() {
        let Some(m) = clip("scrub") else { return };
        let mut d = Decoder::new(egui::Context::default());
        let start = Instant::now();
        let mut got = None;
        while start.elapsed() < Duration::from_secs(10) {
            if let Some(f) = d.frame(&m, 1.5, 1000, 1000, false) {
                if f.w == 160 && f.h == 90 {
                    got = Some(f);
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(got.is_some());
        let again = d.frame(&m, 1.5, 1000, 1000, false).unwrap();
        assert!(Arc::ptr_eq(&again, got.as_ref().unwrap()));
    }

    #[test]
    fn streaming_returns_increasing_frames() {
        let Some(m) = clip("stream") else { return };
        let mut d = Decoder::new(egui::Context::default());
        let begin = Instant::now();
        let mut distinct: Vec<Arc<Frame>> = vec![];
        while begin.elapsed() < Duration::from_millis(2000) {
            let t = begin.elapsed().as_secs_f64();
            if let Some(f) = d.frame(&m, t, 160, 90, true) {
                if distinct.last().map_or(true, |l| l.rgba != f.rgba) {
                    distinct.push(f);
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        assert!(distinct.len() >= 5, "only {} distinct frames", distinct.len());
    }
}
