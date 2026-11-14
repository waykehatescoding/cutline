use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct MediaId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ClipId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TrackId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub settings: Settings,
    pub media: Vec<Media>,
    pub tracks: Vec<Track>,
    pub markers: Vec<Marker>,
    pub range: Option<(f64, f64)>,
    pub next_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub sample_rate: u32,
    pub background: [u8; 4],
}

impl Default for Settings {
    fn default() -> Self {
        Self { width: 1920, height: 1080, fps: 30.0, sample_rate: 48_000, background: [0, 0, 0, 255] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    Video,
    Audio,
    Image,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Media {
    pub id: MediaId,
    pub path: PathBuf,
    pub kind: MediaKind,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub has_video: bool,
    pub has_audio: bool,
    pub audio_channels: u16,
    pub sample_rate: u32,
    pub codec: String,
}

impl Media {
    pub fn file_name(&self) -> String {
        self.path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub kind: TrackKind,
    pub name: String,
    pub muted: bool,
    pub solo: bool,
    pub locked: bool,
    pub hidden: bool,
    pub volume: f32,
    pub clips: Vec<Clip>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClipSource {
    Media(MediaId),
    Text(TextStyle),
    Color([u8; 4]),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    pub text: String,
    pub size: f32,
    pub color: [u8; 4],
    pub background: Option<[u8; 4]>,
    pub bold: bool,
    pub align: TextAlign,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Clip {
    pub id: ClipId,
    pub name: String,
    pub source: ClipSource,
    pub start: f64,
    pub in_point: f64,
    pub out_point: f64,
    pub speed: f64,
    pub reverse: bool,
    pub volume: f32,
    pub audio_muted: bool,
    pub opacity: f32,
    pub transform: Transform,
    pub crop: Crop,
    pub effects: Vec<Effect>,
    pub fade_in: f64,
    pub fade_out: f64,
    pub transition_in: Option<Transition>,
    pub label: u8,
}

impl Clip {
    pub fn duration(&self) -> f64 {
        (self.out_point - self.in_point).max(0.0) / self.speed.max(1e-6)
    }
    pub fn end(&self) -> f64 {
        self.start + self.duration()
    }
    pub fn contains(&self, t: f64) -> bool {
        t >= self.start && t < self.end()
    }
    pub fn source_time(&self, t: f64) -> f64 {
        let local = (t - self.start).clamp(0.0, self.duration()) * self.speed;
        if self.reverse {
            self.out_point - local
        } else {
            self.in_point + local
        }
    }
    pub fn media_id(&self) -> Option<MediaId> {
        match self.source {
            ClipSource::Media(id) => Some(id),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rotation: f32,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl Default for Transform {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5, scale: 1.0, rotation: 0.0, flip_h: false, flip_v: false }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Crop {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Crop {
    pub fn is_none(&self) -> bool {
        self.left == 0.0 && self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rect01 {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect01 {
    pub const FULL: Rect01 = Rect01 { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    pub enabled: bool,
    pub kind: EffectKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EffectKind {
    Blur { radius: f32, region: Option<Rect01> },
    Pixelate { block: f32, region: Option<Rect01> },
    ColorAdjust { brightness: f32, contrast: f32, saturation: f32, gamma: f32 },
    WhiteBalance { temperature: f32, tint: f32 },
    HueShift { degrees: f32 },
    Grayscale,
    Sepia { amount: f32 },
    Invert,
    Vignette { strength: f32 },
    Sharpen { amount: f32 },
    Noise { amount: f32 },
    ChromaKey { color: [u8; 3], similarity: f32, smoothness: f32 },
}

impl EffectKind {
    pub fn name(&self) -> &'static str {
        match self {
            EffectKind::Blur { region: None, .. } => "Blur",
            EffectKind::Blur { .. } => "Blur region",
            EffectKind::Pixelate { region: None, .. } => "Pixelate",
            EffectKind::Pixelate { .. } => "Pixelate region",
            EffectKind::ColorAdjust { .. } => "Color",
            EffectKind::WhiteBalance { .. } => "White balance",
            EffectKind::HueShift { .. } => "Hue shift",
            EffectKind::Grayscale => "Black and white",
            EffectKind::Sepia { .. } => "Sepia",
            EffectKind::Invert => "Invert",
            EffectKind::Vignette { .. } => "Vignette",
            EffectKind::Sharpen { .. } => "Sharpen",
            EffectKind::Noise { .. } => "Film grain",
            EffectKind::ChromaKey { .. } => "Chroma key",
        }
    }

    pub fn catalog() -> Vec<EffectKind> {
        let center = Some(Rect01 { x: 0.35, y: 0.3, w: 0.3, h: 0.4 });
        vec![
            EffectKind::Blur { radius: 24.0, region: None },
            EffectKind::Blur { radius: 40.0, region: center },
            EffectKind::Pixelate { block: 24.0, region: None },
            EffectKind::Pixelate { block: 24.0, region: center },
            EffectKind::ColorAdjust { brightness: 0.0, contrast: 1.0, saturation: 1.0, gamma: 1.0 },
            EffectKind::WhiteBalance { temperature: 0.0, tint: 0.0 },
            EffectKind::HueShift { degrees: 0.0 },
            EffectKind::Grayscale,
            EffectKind::Sepia { amount: 1.0 },
            EffectKind::Invert,
            EffectKind::Vignette { strength: 0.5 },
            EffectKind::Sharpen { amount: 1.0 },
            EffectKind::Noise { amount: 0.2 },
            EffectKind::ChromaKey { color: [0, 255, 0], similarity: 0.3, smoothness: 0.1 },
        ]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransitionKind {
    Crossfade,
    FadeThroughBlack,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
    ZoomIn,
}

impl TransitionKind {
    pub const ALL: [TransitionKind; 7] = [
        TransitionKind::Crossfade,
        TransitionKind::FadeThroughBlack,
        TransitionKind::WipeLeft,
        TransitionKind::WipeRight,
        TransitionKind::SlideLeft,
        TransitionKind::SlideRight,
        TransitionKind::ZoomIn,
    ];
    pub fn name(&self) -> &'static str {
        match self {
            TransitionKind::Crossfade => "Cross dissolve",
            TransitionKind::FadeThroughBlack => "Dip to black",
            TransitionKind::WipeLeft => "Wipe left",
            TransitionKind::WipeRight => "Wipe right",
            TransitionKind::SlideLeft => "Slide left",
            TransitionKind::SlideRight => "Slide right",
            TransitionKind::ZoomIn => "Zoom in",
        }
    }
    pub fn xfade(&self) -> &'static str {
        match self {
            TransitionKind::Crossfade => "fade",
            TransitionKind::FadeThroughBlack => "fadeblack",
            TransitionKind::WipeLeft => "wipeleft",
            TransitionKind::WipeRight => "wiperight",
            TransitionKind::SlideLeft => "slideleft",
            TransitionKind::SlideRight => "slideright",
            TransitionKind::ZoomIn => "zoomin",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Marker {
    pub time: f64,
    pub label: String,
    pub color: u8,
}

impl Default for Project {
    fn default() -> Self {
        let mut p = Project {
            name: "Untitled".into(),
            settings: Settings::default(),
            media: vec![],
            tracks: vec![],
            markers: vec![],
            range: None,
            next_id: 1,
        };
        for (kind, name) in [(TrackKind::Video, "V2"), (TrackKind::Video, "V1"), (TrackKind::Audio, "A1"), (TrackKind::Audio, "A2")] {
            let id = TrackId(p.alloc_id());
            p.tracks.push(Track {
                id,
                kind,
                name: name.into(),
                muted: false,
                solo: false,
                locked: false,
                hidden: false,
                volume: 1.0,
                clips: vec![],
            });
        }
        p
    }
}

impl Project {
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn media(&self, id: MediaId) -> Option<&Media> {
        self.media.iter().find(|m| m.id == id)
    }
    pub fn duration(&self) -> f64 {
        self.tracks.iter().flat_map(|t| t.clips.iter()).map(|c| c.end()).fold(0.0, f64::max)
    }
    pub fn clip(&self, id: ClipId) -> Option<(usize, usize)> {
        self.tracks.iter().enumerate().find_map(|(ti, t)| t.clips.iter().position(|c| c.id == id).map(|ci| (ti, ci)))
    }
    pub fn clip_ref(&self, id: ClipId) -> Option<&Clip> {
        self.clip(id).map(|(t, c)| &self.tracks[t].clips[c])
    }
    pub fn clip_mut(&mut self, id: ClipId) -> Option<&mut Clip> {
        self.clip(id).map(move |(t, c)| &mut self.tracks[t].clips[c])
    }
    pub fn new_clip(&mut self, source: ClipSource, name: String, start: f64, in_point: f64, out_point: f64) -> Clip {
        Clip {
            id: ClipId(self.alloc_id()),
            name,
            source,
            start,
            in_point,
            out_point,
            speed: 1.0,
            reverse: false,
            volume: 1.0,
            audio_muted: false,
            opacity: 1.0,
            transform: Transform::default(),
            crop: Crop::default(),
            effects: vec![],
            fade_in: 0.0,
            fade_out: 0.0,
            transition_in: None,
            label: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    pub fn new(w: u32, h: u32) -> Self {
        Self { w, h, rgba: vec![0; (w * h * 4) as usize] }
    }
    pub fn filled(w: u32, h: u32, c: [u8; 4]) -> Self {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            rgba.extend_from_slice(&c);
        }
        Self { w, h, rgba }
    }
}
