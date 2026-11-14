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
    pub const ALL: [Format; 7] =
        [Format::Mp4H264, Format::Mp4H265, Format::WebmVp9, Format::MovProRes, Format::Gif, Format::Mp3, Format::Wav];

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
    static FONTS: std::sync::OnceLock<[PathBuf; 2]> = std::sync::OnceLock::new();
    let fonts = FONTS.get_or_init(|| {
        let write = |stem, bytes: &[u8]| crate::ffmpeg::scratch_file(stem, "ttf", bytes).unwrap_or_default();
        [
            write("inter", include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
            write("inter-semibold", include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")),
        ]
    });
    fonts[bold as usize].clone()
}

pub fn format_for(path: &std::path::Path) -> Option<Format> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" | "mov" | "mkv" => Some(Format::Mp4H264),
        "webm" => Some(Format::WebmVp9),
        "gif" => Some(Format::Gif),
        "mp3" => Some(Format::Mp3),
        "wav" => Some(Format::Wav),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn format_follows_original_extension() {
        assert_eq!(format_for(Path::new("a.MP4")), Some(Format::Mp4H264));
        assert_eq!(format_for(Path::new("a.mkv")), Some(Format::Mp4H264));
        assert_eq!(format_for(Path::new("a.webm")), Some(Format::WebmVp9));
        assert_eq!(format_for(Path::new("a.flac")), None);
        assert_eq!(format_for(Path::new("a")), None);
    }
}
