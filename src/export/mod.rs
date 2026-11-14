pub mod graph;
pub mod job;

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Mp4H264,
    Mp4H265,
    WebmVp9,
    MovProRes,
    Gif,
    Mp3,
    Wav,
}

impl Format {
    pub const ALL: [Format; 7] = [Format::Mp4H264, Format::Mp4H265, Format::WebmVp9, Format::MovProRes, Format::Gif, Format::Mp3, Format::Wav];

    pub fn name(&self) -> &'static str {
        match self {
            Format::Mp4H264 => "MP4 (H.264)",
            Format::Mp4H265 => "MP4 (H.265)",
            Format::WebmVp9 => "WebM (VP9)",
            Format::MovProRes => "MOV (ProRes 422)",
            Format::Gif => "GIF",
            Format::Mp3 => "MP3 audio",
            Format::Wav => "WAV audio",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Format::Mp4H264 | Format::Mp4H265 => "mp4",
            Format::WebmVp9 => "webm",
            Format::MovProRes => "mov",
            Format::Gif => "gif",
            Format::Mp3 => "mp3",
            Format::Wav => "wav",
        }
    }

    pub fn audio_only(&self) -> bool {
        matches!(self, Format::Mp3 | Format::Wav)
    }

    pub fn has_audio(&self) -> bool {
        !matches!(self, Format::Gif)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExportOptions {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub quality: u8,
    pub audio_kbps: u32,
    pub range: Option<(f64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum JobState {
    Running { progress: f32, eta: Option<f64>, speed: Option<f32> },
    Done { path: PathBuf, secs: f64 },
    Failed(String),
    Cancelled,
}

pub fn font_file(bold: bool) -> PathBuf {
    let name = if bold { "cutline-Inter-SemiBold.ttf" } else { "cutline-Inter-Regular.ttf" };
    let path = std::env::temp_dir().join(name);
    if !path.exists() {
        let bytes: &[u8] = if bold {
            include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")
        } else {
            include_bytes!("../../assets/fonts/Inter-Regular.ttf")
        };
        let _ = std::fs::write(&path, bytes);
    }
    path
}
