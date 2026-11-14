use crate::decode::grab_frame;
use crate::model::*;
use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Condvar, Mutex};

const BUCKET: f64 = 2.0;
const MAX_TEXTURES: usize = 600;
const MAX_QUEUE: usize = 96;
const THUMB_W: u32 = 160;
const THUMB_H: u32 = 90;

type Key = (MediaId, i64);

struct Job {
    key: Key,
    media: Media,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    closed: bool,
}

struct Lru<V> {
    map: HashMap<Key, (V, u64)>,
    tick: u64,
    cap: usize,
}

impl<V: Clone> Lru<V> {
    fn new(cap: usize) -> Self {
        Self { map: HashMap::new(), tick: 0, cap }
    }

    fn get(&mut self, key: &Key) -> Option<V> {
        self.tick += 1;
        let tick = self.tick;
        self.map.get_mut(key).map(|e| {
            e.1 = tick;
            e.0.clone()
        })
    }

    fn insert(&mut self, key: Key, v: V) {
        self.tick += 1;
        self.map.insert(key, (v, self.tick));
        if self.map.len() > self.cap {
            if let Some(old) = self.map.iter().min_by_key(|(_, e)| e.1).map(|(k, _)| *k) {
                self.map.remove(&old);
            }
        }
    }
}

fn bucket(t: f64, duration: f64) -> i64 {
    (t.clamp(0.0, duration.max(0.0)) / BUCKET).floor() as i64
}

fn bucket_time(b: i64, duration: f64) -> f64 {
    (b as f64 * BUCKET).min((duration - 0.05).max(0.0))
}

fn fit(w: u32, h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (THUMB_W, THUMB_H);
    }
    let s = (THUMB_W as f64 / w as f64).min(THUMB_H as f64 / h as f64);
    (((w as f64 * s).round() as u32).max(2) & !1, ((h as f64 * s).round() as u32).max(2) & !1)
}

pub struct Thumbs {
    ctx: egui::Context,
    cache: Lru<egui::TextureHandle>,
    failed: HashSet<Key>,
    pending: HashSet<Key>,
    queue: Arc<(Mutex<Queue>, Condvar)>,
    results: Receiver<(Key, Result<Frame, String>)>,
}

impl Thumbs {
    pub fn new(ctx: egui::Context) -> Self {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let (tx, results) = channel();
        for _ in 0..2 {
            let queue = queue.clone();
            let tx = tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || loop {
                let job = {
                    let (lock, cv) = &*queue;
                    let mut q = lock.lock().unwrap();
                    loop {
                        if q.closed {
                            return;
                        }
                        if let Some(job) = q.jobs.pop_back() {
                            break job;
                        }
                        q = cv.wait(q).unwrap();
                    }
                };
                let (w, h) = fit(job.media.width, job.media.height);
                let t = match job.media.kind {
                    MediaKind::Image => 0.0,
                    _ => bucket_time(job.key.1, job.media.duration),
                };
                let res = grab_frame(&job.media, t, w, h);
                if tx.send((job.key, res)).is_err() {
                    return;
                }
                ctx.request_repaint();
            });
        }
        Self { ctx, cache: Lru::new(MAX_TEXTURES), failed: HashSet::new(), pending: HashSet::new(), queue, results }
    }

    fn drain(&mut self) {
        while let Ok((key, res)) = self.results.try_recv() {
            self.pending.remove(&key);
            match res {
                Ok(f) if f.w > 0 && f.h > 0 => {
                    let img = egui::ColorImage::from_rgba_unmultiplied([f.w as usize, f.h as usize], &f.rgba);
                    let name = format!("thumb_{}_{}", key.0 .0, key.1);
                    self.cache.insert(key, self.ctx.load_texture(name, img, egui::TextureOptions::LINEAR));
                }
                _ => {
                    self.failed.insert(key);
                }
            }
        }
    }

    pub fn get(&mut self, media: &Media, t: f64) -> Option<egui::TextureHandle> {
        if media.kind == MediaKind::Audio || !(media.has_video || media.kind == MediaKind::Image) {
            return None;
        }
        self.drain();
        let b = if media.kind == MediaKind::Image { 0 } else { bucket(t, media.duration) };
        let key = (media.id, b);
        if let Some(tex) = self.cache.get(&key) {
            return Some(tex);
        }
        if self.failed.contains(&key) || !self.pending.insert(key) {
            return None;
        }
        let (lock, cv) = &*self.queue;
        let mut q = lock.lock().unwrap();
        q.jobs.push_back(Job { key, media: media.clone() });
        if q.jobs.len() > MAX_QUEUE {
            if let Some(old) = q.jobs.pop_front() {
                self.pending.remove(&old.key);
            }
        }
        drop(q);
        cv.notify_one();
        None
    }

    pub fn poster(&mut self, media: &Media) -> Option<egui::TextureHandle> {
        let t = if media.kind == MediaKind::Image { 0.0 } else { media.duration * 0.1 };
        self.get(media, t)
    }
}

impl Drop for Thumbs {
    fn drop(&mut self) {
        let (lock, cv) = &*self.queue;
        let mut q = lock.lock().unwrap();
        q.closed = true;
        q.jobs.clear();
        drop(q);
        cv.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_quantizes_to_two_seconds() {
        assert_eq!(bucket(0.0, 10.0), 0);
        assert_eq!(bucket(1.99, 10.0), 0);
        assert_eq!(bucket(2.0, 10.0), 1);
        assert_eq!(bucket(5.5, 10.0), 2);
    }

    #[test]
    fn bucket_clamps_to_duration() {
        assert_eq!(bucket(-3.0, 10.0), 0);
        assert_eq!(bucket(99.0, 10.0), 5);
        assert_eq!(bucket(5.0, 0.0), 0);
    }

    #[test]
    fn bucket_time_stays_inside_media() {
        assert_eq!(bucket_time(2, 10.0), 4.0);
        assert!(bucket_time(5, 10.0) < 10.0);
        assert_eq!(bucket_time(3, 0.0), 0.0);
    }

    #[test]
    fn fit_keeps_aspect() {
        assert_eq!(fit(1920, 1080), (160, 90));
        assert_eq!(fit(1080, 1920), (50, 90));
        assert_eq!(fit(0, 0), (160, 90));
    }

    #[test]
    fn lru_evicts_least_recently_used() {
        let k = |i| (MediaId(1), i);
        let mut lru = Lru::new(2);
        lru.insert(k(0), 0);
        lru.insert(k(1), 1);
        assert_eq!(lru.get(&k(0)), Some(0));
        lru.insert(k(2), 2);
        assert_eq!(lru.get(&k(1)), None);
        assert_eq!(lru.get(&k(0)), Some(0));
        assert_eq!(lru.get(&k(2)), Some(2));
        assert_eq!(lru.map.len(), 2);
    }
}
