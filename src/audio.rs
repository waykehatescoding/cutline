use crate::model::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::collections::VecDeque;
use std::io::Read;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

type Ring = Arc<Mutex<VecDeque<f32>>>;

const RING_SECONDS: f64 = 0.5;

fn push_bytes(ring: &mut VecDeque<f32>, bytes: &[u8]) -> usize {
    let used = bytes.len() / 4 * 4;
    ring.extend(bytes[..used].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])));
    used
}

fn drain_into<T: FromSample<f32> + Copy>(ring: &mut VecDeque<f32>, out: &mut [T], silence: T, gain: f32) -> usize {
    let taken = ring.len().min(out.len());
    for (o, s) in out.iter_mut().zip(ring.drain(..taken)) {
        *o = T::from_sample_(s * gain);
    }
    out[taken..].fill(silence);
    taken
}

struct Playback {
    stream: cpal::Stream,
    child: Child,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    played: Arc<AtomicU64>,
    from: f64,
    sample_rate: f64,
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(r) = self.reader.take() {
            let _ = r.join();
        }
    }
}

fn stream_for<T: SizedSample + FromSample<f32> + Send + 'static>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    ring: Ring,
    started: Arc<AtomicBool>,
    played: Arc<AtomicU64>,
    gain: Arc<AtomicU32>,
) -> Option<cpal::Stream> {
    let channels = config.channels as u64;
    let silence = T::from_sample_(0.0f32);
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                let Ok(mut ring) = ring.try_lock() else {
                    out.fill(silence);
                    if started.load(Ordering::Relaxed) {
                        played.fetch_add(out.len() as u64 / channels, Ordering::Relaxed);
                    }
                    return;
                };
                drain_into(&mut ring, out, silence, f32::from_bits(gain.load(Ordering::Relaxed)));
                if started.load(Ordering::Relaxed) {
                    played.fetch_add(out.len() as u64 / channels, Ordering::Relaxed);
                }
            },
            |_| {},
            None,
        )
        .ok()
}

fn start(project: &Project, from: f64, gain: Arc<AtomicU32>) -> Option<Playback> {
    let device = cpal::default_host().default_output_device()?;
    let supported = device.default_output_config().ok()?;
    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let args = crate::export::graph::audio_preview_args(project, from, sample_rate, channels)?;
    let ring: Ring = Arc::new(Mutex::new(VecDeque::new()));
    let started = Arc::new(AtomicBool::new(false));
    let played = Arc::new(AtomicU64::new(0));
    let config = supported.config();
    let (r, s, p, g) = (ring.clone(), started.clone(), played.clone(), gain);
    let stream = match supported.sample_format() {
        SampleFormat::F32 => stream_for::<f32>(&device, config, r, s, p, g),
        SampleFormat::I16 => stream_for::<i16>(&device, config, r, s, p, g),
        SampleFormat::U16 => stream_for::<u16>(&device, config, r, s, p, g),
        SampleFormat::I32 => stream_for::<i32>(&device, config, r, s, p, g),
        _ => None,
    }?;
    let mut child = crate::ffmpeg::command("ffmpeg")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let stop = Arc::new(AtomicBool::new(false));
    let cap = (sample_rate as f64 * channels as f64 * RING_SECONDS) as usize;
    let reader = {
        let stop = stop.clone();
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 16384];
            let mut carry = 0;
            while !stop.load(Ordering::Relaxed) {
                if ring.lock().map_or(true, |r| r.len() >= cap) {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                let n = match stdout.read(&mut buf[carry..]) {
                    Ok(n) if n > 0 => n,
                    _ => break,
                };
                if let Ok(mut r) = ring.lock() {
                    let used = push_bytes(&mut r, &buf[..carry + n]);
                    buf.copy_within(used..carry + n, 0);
                    carry = carry + n - used;
                }
                started.store(true, Ordering::Relaxed);
            }
            started.store(true, Ordering::Relaxed);
        })
    };
    let playback = Playback { stream, child, stop, reader: Some(reader), played, from, sample_rate: sample_rate as f64 };
    playback.stream.play().ok()?;
    Some(playback)
}

pub struct AudioEngine {
    playback: Option<Playback>,
    gain: Arc<AtomicU32>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self { playback: None, gain: Arc::new(AtomicU32::new(1.0f32.to_bits())) }
    }

    pub fn play(&mut self, project: &Project, from: f64) {
        self.stop();
        self.playback = start(project, from, self.gain.clone());
    }

    pub fn set_volume(&self, gain: f32) {
        self.gain.store(gain.to_bits(), Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        self.playback = None;
    }

    pub fn position(&self) -> Option<f64> {
        let p = self.playback.as_ref()?;
        Some(p.from + p.played.load(Ordering::Relaxed) as f64 / p.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_bytes_keeps_partial_sample() {
        let mut ring = VecDeque::new();
        let mut raw = 0.5f32.to_le_bytes().to_vec();
        raw.extend_from_slice(&(-0.25f32).to_le_bytes()[..2]);
        assert_eq!(push_bytes(&mut ring, &raw), 4);
        assert_eq!(ring, VecDeque::from([0.5]));
    }

    #[test]
    fn drain_pads_with_silence_and_reports_taken() {
        let mut ring = VecDeque::from([1.0f32, -1.0, 0.0]);
        let mut out = [7i16; 5];
        assert_eq!(drain_into(&mut ring, &mut out, 0, 1.0), 3);
        assert_eq!(out, [i16::MAX, -i16::MAX - 1, 0, 0, 0]);
        assert!(ring.is_empty());
    }

    #[test]
    fn drain_u16_silence_is_midpoint() {
        let mut ring = VecDeque::new();
        let mut out = [1u16; 2];
        drain_into(&mut ring, &mut out, u16::from_sample_(0.0f32), 1.0);
        assert_eq!(out, [32768, 32768]);
    }
}
