use crate::model::*;
use eframe::egui;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

const PER_SEC: usize = 100;
const SAMPLE_RATE: usize = 8000;
const MAX_JOBS: usize = 2;

pub struct Peaks {
    pub per_sec: f64,
    pub data: Vec<f32>,
}

impl Peaks {
    pub fn range(&self, t0: f64, t1: f64) -> f32 {
        let lo = (t0 * self.per_sec).floor() as usize;
        let hi = ((t1 * self.per_sec).ceil() as usize).max(lo + 1).min(self.data.len());
        self.data.get(lo..hi).map_or(0.0, |s| s.iter().copied().fold(0.0, f32::max))
    }
}

struct Reducer {
    per_bucket: usize,
    filled: usize,
    peak: f32,
    data: Vec<f32>,
}

impl Reducer {
    fn new(per_bucket: usize) -> Self {
        Self { per_bucket, filled: 0, peak: 0.0, data: Vec::new() }
    }

    fn push(&mut self, samples: &[f32]) {
        for &x in samples {
            self.peak = self.peak.max(x.abs());
            self.filled += 1;
            if self.filled == self.per_bucket {
                self.data.push(self.peak.min(1.0));
                self.filled = 0;
                self.peak = 0.0;
            }
        }
    }

    fn finish(mut self) -> Vec<f32> {
        if self.filled > 0 {
            self.data.push(self.peak.min(1.0));
        }
        self.data
    }
}

fn decode(path: &PathBuf) -> Option<Peaks> {
    let mut child = crate::ffmpeg::command("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-vn", "-ac", "1", "-ar", "8000", "-f", "f32le", "pipe:1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let mut reducer = Reducer::new(SAMPLE_RATE / PER_SEC);
    let mut buf = vec![0u8; 1 << 16];
    let mut carry = 0;
    let mut samples = Vec::with_capacity(buf.len() / 4);
    loop {
        let n = match out.read(&mut buf[carry..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => carry + n,
        };
        let whole = n / 4 * 4;
        samples.clear();
        samples.extend(buf[..whole].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])));
        reducer.push(&samples);
        buf.copy_within(whole..n, 0);
        carry = n - whole;
    }
    let _ = child.kill();
    let _ = child.wait();
    let data = reducer.finish();
    (!data.is_empty()).then(|| Peaks { per_sec: PER_SEC as f64, data })
}

enum State {
    Queued,
    Running,
    Ready(Arc<Peaks>),
    Failed,
}

pub struct Waveforms {
    ctx: egui::Context,
    states: HashMap<MediaId, State>,
    queue: Vec<(MediaId, PathBuf)>,
    tx: Sender<(MediaId, Option<Peaks>)>,
    rx: Receiver<(MediaId, Option<Peaks>)>,
}

impl Waveforms {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = channel();
        Self { ctx, states: HashMap::new(), queue: Vec::new(), tx, rx }
    }

    pub fn get(&mut self, media: &Media) -> Option<Arc<Peaks>> {
        while let Ok((id, peaks)) = self.rx.try_recv() {
            self.states.insert(id, peaks.map_or(State::Failed, |p| State::Ready(Arc::new(p))));
        }
        match self.states.get(&media.id) {
            Some(State::Ready(p)) => return Some(p.clone()),
            Some(_) => {}
            None if media.has_audio => {
                self.states.insert(media.id, State::Queued);
                self.queue.push((media.id, media.path.clone()));
            }
            None => {}
        }
        self.start_jobs();
        None
    }

    fn start_jobs(&mut self) {
        let mut running = self.states.values().filter(|s| matches!(s, State::Running)).count();
        while running < MAX_JOBS && !self.queue.is_empty() {
            let (id, path) = self.queue.remove(0);
            self.states.insert(id, State::Running);
            running += 1;
            let tx = self.tx.clone();
            let ctx = self.ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send((id, decode(&path)));
                ctx.request_repaint();
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_takes_max_abs_per_bucket() {
        let mut r = Reducer::new(3);
        r.push(&[0.1, -0.5, 0.2, 0.3]);
        r.push(&[0.9, -0.1]);
        assert_eq!(r.finish(), vec![0.5, 0.9]);
    }

    #[test]
    fn reducer_flushes_partial_bucket() {
        let mut r = Reducer::new(4);
        r.push(&[0.25, -0.75]);
        assert_eq!(r.finish(), vec![0.75]);
    }

    #[test]
    fn range_returns_max_over_buckets() {
        let p = Peaks { per_sec: 100.0, data: vec![0.1, 0.9, 0.3, 0.2] };
        assert_eq!(p.range(0.0, 0.04), 0.9);
        assert_eq!(p.range(0.02, 0.04), 0.3);
        assert_eq!(p.range(0.025, 0.026), 0.3);
    }

    #[test]
    fn range_clamps_and_handles_empty() {
        let p = Peaks { per_sec: 100.0, data: vec![0.1, 0.4] };
        assert_eq!(p.range(-1.0, 10.0), 0.4);
        assert_eq!(p.range(5.0, 6.0), 0.0);
        assert_eq!(Peaks { per_sec: 100.0, data: vec![] }.range(0.0, 1.0), 0.0);
    }
}
