use crate::model::*;
use eframe::egui;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

pub const EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "flv", "ts", "mp3", "wav", "flac", "aac", "m4a", "ogg", "opus", "png", "jpg", "jpeg",
    "bmp", "gif", "webp",
];

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp"];
const MAX_JOBS: usize = 4;

type Done = (PathBuf, Result<Media, String>);

pub struct Importer {
    ctx: egui::Context,
    queue: VecDeque<PathBuf>,
    pending: Vec<PathBuf>,
    running: usize,
    early: Vec<Result<Media, String>>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
}

impl Importer {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = channel();
        Self { ctx, queue: VecDeque::new(), pending: vec![], running: 0, early: vec![], tx, rx }
    }

    pub fn import(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            let files = if path.is_dir() { list_dir(&path) } else { vec![path] };
            for file in files {
                if is_supported(&file) {
                    self.pending.push(file.clone());
                    self.queue.push_back(file);
                } else {
                    self.early.push(Err(format!("Unsupported file: {}", display_name(&file))));
                }
            }
        }
        self.spawn_jobs();
        if !self.early.is_empty() {
            self.ctx.request_repaint();
        }
    }

    pub fn poll(&mut self) -> Vec<Result<Media, String>> {
        let mut out = std::mem::take(&mut self.early);
        while let Ok((path, result)) = self.rx.try_recv() {
            self.running -= 1;
            if let Some(i) = self.pending.iter().position(|p| *p == path) {
                self.pending.remove(i);
            }
            out.push(result);
        }
        self.spawn_jobs();
        out
    }

    pub fn pending(&self) -> &[PathBuf] {
        &self.pending
    }

    fn spawn_jobs(&mut self) {
        while self.running < MAX_JOBS {
            let Some(path) = self.queue.pop_front() else { break };
            self.running += 1;
            let tx = self.tx.clone();
            let ctx = self.ctx.clone();
            std::thread::spawn(move || {
                let result = probe(&path);
                let _ = tx.send((path, result));
                ctx.request_repaint();
            });
        }
    }
}

fn list_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> =
        std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file() && is_supported(p)).collect();
    files.sort();
    files
}

fn display_name(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

pub fn is_supported(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn probe(path: &Path) -> Result<Media, String> {
    let name = display_name(path);
    let out = crate::ffmpeg::command("ffprobe")
        .args(["-v", "error", "-print_format", "json", "-show_streams", "-show_format"])
        .arg(path)
        .output()
        .map_err(|e| format!("Could not run ffprobe for {name}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("Could not read {name}: {}", err.lines().next().unwrap_or("unknown error")));
    }
    parse_probe(path, &String::from_utf8_lossy(&out.stdout))
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn parse_rate(s: &str) -> f64 {
    let (n, d) = s.split_once('/').unwrap_or((s, "1"));
    match (n.parse::<f64>(), d.parse::<f64>()) {
        (Ok(n), Ok(d)) if d > 0.0 => n / d,
        _ => 0.0,
    }
}

fn rotation(stream: &Value) -> f64 {
    let side = stream["side_data_list"].as_array().and_then(|l| l.iter().find_map(|d| d.get("rotation").and_then(num)));
    side.or_else(|| num(&stream["tags"]["rotate"])).unwrap_or(0.0)
}

pub fn parse_probe(path: &Path, json: &str) -> Result<Media, String> {
    let name = display_name(path);
    let root: Value = serde_json::from_str(json).map_err(|e| format!("Could not read {name}: {e}"))?;
    let streams = root["streams"].as_array().map(Vec::as_slice).unwrap_or_default();
    let video = streams.iter().find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1);
    let audio = streams.iter().find(|s| s["codec_type"] == "audio");
    if video.is_none() && audio.is_none() {
        return Err(format!("No audio or video streams in {name}"));
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let duration = num(&root["format"]["duration"]).or_else(|| video.or(audio).and_then(|s| num(&s["duration"]))).unwrap_or(0.0);
    let codec_of = |s: Option<&Value>| s.and_then(|s| s["codec_name"].as_str()).unwrap_or_default().to_string();
    let (kind, duration) = match video {
        None => (MediaKind::Audio, duration),
        Some(v) => {
            let still = matches!(codec_of(Some(v)).as_str(), "png" | "mjpeg" | "bmp") && duration < 0.5;
            if IMAGE_EXTENSIONS.contains(&ext.as_str()) || still {
                (MediaKind::Image, 5.0)
            } else {
                (MediaKind::Video, duration)
            }
        }
    };
    let (mut width, mut height, mut fps) = (0, 0, 0.0);
    if let Some(v) = video {
        width = v["width"].as_u64().unwrap_or(0) as u32;
        height = v["height"].as_u64().unwrap_or(0) as u32;
        if (rotation(v).abs() % 180.0 - 90.0).abs() < 1.0 {
            std::mem::swap(&mut width, &mut height);
        }
        if kind == MediaKind::Video {
            let rate = |k: &str| v[k].as_str().map_or(0.0, parse_rate);
            fps = [rate("avg_frame_rate"), rate("r_frame_rate")].into_iter().find(|r| *r > 0.0).unwrap_or(30.0);
        }
    }
    Ok(Media {
        id: MediaId(0),
        path: path.to_path_buf(),
        kind,
        duration,
        width,
        height,
        fps,
        has_video: video.is_some(),
        has_audio: audio.is_some(),
        audio_channels: audio.and_then(|a| a["channels"].as_u64()).unwrap_or(0) as u16,
        sample_rate: audio.and_then(|a| num(&a["sample_rate"])).unwrap_or(0.0) as u32,
        codec: codec_of(video.or(audio)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIDEO_AUDIO: &str = r#"{"streams":[{"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"avg_frame_rate":"30000/1001","r_frame_rate":"30000/1001","duration":"12.5"},{"codec_type":"audio","codec_name":"aac","channels":2,"sample_rate":"48000"}],"format":{"duration":"12.512000"}}"#;
    const COVER_ART: &str = r#"{"streams":[{"codec_type":"audio","codec_name":"mp3","channels":2,"sample_rate":"44100"},{"codec_type":"video","codec_name":"mjpeg","width":500,"height":500,"avg_frame_rate":"0/0","r_frame_rate":"90000/1","disposition":{"attached_pic":1}}],"format":{"duration":"200.5"}}"#;
    const IMAGE: &str = r#"{"streams":[{"codec_type":"video","codec_name":"png","width":800,"height":600,"avg_frame_rate":"0/0","r_frame_rate":"25/1"}],"format":{"duration":"0.040000"}}"#;
    const ROTATED: &str = r#"{"streams":[{"codec_type":"video","codec_name":"hevc","width":1920,"height":1080,"avg_frame_rate":"30/1","side_data_list":[{"side_data_type":"Display Matrix","rotation":-90}]},{"codec_type":"audio","codec_name":"aac","channels":1,"sample_rate":"44100"}],"format":{"duration":"3.0"}}"#;

    fn parse(json: &str, name: &str) -> Media {
        parse_probe(Path::new(name), json).unwrap()
    }

    #[test]
    fn video_with_audio() {
        let m = parse(VIDEO_AUDIO, "a.mp4");
        assert_eq!(m.kind, MediaKind::Video);
        assert_eq!((m.width, m.height), (1920, 1080));
        assert!((m.fps - 29.97).abs() < 0.01);
        assert!((m.duration - 12.512).abs() < 1e-6);
        assert!(m.has_video && m.has_audio);
        assert_eq!((m.audio_channels, m.sample_rate), (2, 48000));
        assert_eq!(m.codec, "h264");
        assert_eq!(m.id, MediaId(0));
    }

    #[test]
    fn audio_with_cover_art_is_audio() {
        let m = parse(COVER_ART, "a.mp3");
        assert_eq!(m.kind, MediaKind::Audio);
        assert!(!m.has_video && m.has_audio);
        assert_eq!(m.codec, "mp3");
        assert_eq!(m.sample_rate, 44100);
        assert!((m.duration - 200.5).abs() < 1e-6);
    }

    #[test]
    fn single_frame_image() {
        let m = parse(IMAGE, "a.png");
        assert_eq!(m.kind, MediaKind::Image);
        assert_eq!(m.duration, 5.0);
        assert_eq!((m.width, m.height), (800, 600));
        assert!(m.has_video && !m.has_audio);
    }

    #[test]
    fn rotated_video_swaps_dimensions() {
        let m = parse(ROTATED, "a.mov");
        assert_eq!((m.width, m.height), (1080, 1920));
        assert_eq!(m.fps, 30.0);
        assert_eq!(m.audio_channels, 1);
    }

    #[test]
    fn no_streams_is_error() {
        assert!(parse_probe(Path::new("a.mp4"), r#"{"streams":[],"format":{}}"#).is_err());
    }
}
