use crate::model::*;
use crate::{
    audio, commands, compositor, decode, edit_ops, effects_panel, export, export_dialog, history, inspector, media, media_bin, monitor,
    project_io, theme, thumbs, timeline, updater, waveform,
};
use eframe::egui;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Razor,
    Slip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    Media,
    Effects,
}

#[derive(Clone, Debug)]
pub enum DragPayload {
    Media(MediaId),
    Effect(EffectKind),
    Transition(TransitionKind),
    Title,
    Matte([u8; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub born: f64,
}

#[derive(Clone, Debug)]
pub struct TimelineView {
    pub px_per_sec: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub snap: bool,
    pub ripple: bool,
    pub follow: bool,
    pub track_height: f32,
}

impl Default for TimelineView {
    fn default() -> Self {
        Self { px_per_sec: 60.0, scroll_x: 0.0, scroll_y: 0.0, snap: true, ripple: false, follow: true, track_height: 56.0 }
    }
}

pub struct App {
    pub ctx: egui::Context,
    pub project: Project,
    pub history: history::History,
    pub project_path: Option<PathBuf>,
    pub dirty: bool,
    pub selection: Vec<ClipId>,
    pub selected_media: Option<MediaId>,
    pub clipboard: Vec<(usize, Clip)>,
    pub playhead: f64,
    pub playing: bool,
    pub loop_playback: bool,
    pub play_anchor: Option<(f64, f64)>,
    pub tool: Tool,
    pub left_tab: LeftTab,
    pub view: TimelineView,
    pub decoder: decode::Decoder,
    pub thumbs: thumbs::Thumbs,
    pub waves: waveform::Waveforms,
    pub audio: audio::AudioEngine,
    pub importer: media::Importer,
    pub compositor: compositor::Compositor,
    pub export_dialog: export_dialog::State,
    pub export_job: Option<export::job::ExportJob>,
    pub palette_open: bool,
    pub toasts: Vec<Toast>,
    pub ffmpeg_ok: bool,
    pub last_autosave: f64,
    pub updater: updater::Updater,
    pub original: Option<PathBuf>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Vec<PathBuf>) -> Self {
        theme::apply(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let mut app = Self {
            project: Project::default(),
            history: history::History::default(),
            project_path: None,
            dirty: false,
            selection: vec![],
            selected_media: None,
            clipboard: vec![],
            playhead: 0.0,
            playing: false,
            loop_playback: false,
            play_anchor: None,
            tool: Tool::Select,
            left_tab: LeftTab::Media,
            view: TimelineView::default(),
            decoder: decode::Decoder::new(ctx.clone()),
            thumbs: thumbs::Thumbs::new(ctx.clone()),
            waves: waveform::Waveforms::new(ctx.clone()),
            audio: audio::AudioEngine::new(),
            importer: media::Importer::new(ctx.clone()),
            compositor: compositor::Compositor::new(),
            export_dialog: export_dialog::State::default(),
            export_job: None,
            palette_open: false,
            toasts: vec![],
            ffmpeg_ok: crate::ffmpeg::available(),
            last_autosave: 0.0,
            updater: updater::Updater::new(ctx.clone()),
            original: None,
            ctx,
        };
        app.updater.check();
        if !app.ffmpeg_ok {
            app.notify("ffmpeg was not found. Install it or place ffmpeg.exe next to cutline.exe", ToastKind::Error);
        }
        for path in open {
            if path.extension().is_some_and(|e| e == "cutline") {
                app.open_project(path);
            } else {
                app.importer.import(vec![path]);
            }
        }
        app
    }

    pub fn now(&self) -> f64 {
        self.ctx.input(|i| i.time)
    }

    pub fn notify(&mut self, text: impl Into<String>, kind: ToastKind) {
        let born = self.now();
        self.toasts.push(Toast { text: text.into(), kind, born });
    }

    pub fn checkpoint(&mut self) {
        self.history.checkpoint(&self.project);
        self.dirty = true;
    }

    pub fn undo(&mut self) {
        if self.history.undo(&mut self.project) {
            self.dirty = true;
            self.prune_selection();
            self.restart_audio();
        }
    }

    pub fn redo(&mut self) {
        if self.history.redo(&mut self.project) {
            self.dirty = true;
            self.prune_selection();
            self.restart_audio();
        }
    }

    fn prune_selection(&mut self) {
        let p = &self.project;
        self.selection.retain(|id| p.clip(*id).is_some());
    }

    pub fn primary_clip(&self) -> Option<ClipId> {
        self.selection.last().copied().filter(|id| self.project.clip(*id).is_some())
    }

    pub fn end_time(&self) -> f64 {
        self.project.range.map(|r| r.1).unwrap_or_else(|| self.project.duration())
    }

    pub fn seek(&mut self, t: f64) {
        self.playhead = t.clamp(0.0, self.project.duration().max(0.0));
        if self.playing {
            self.restart_audio();
        }
    }

    pub fn toggle_play(&mut self) {
        if self.playing {
            self.stop();
        } else {
            self.play();
        }
    }

    pub fn play(&mut self) {
        if self.project.duration() <= 0.0 {
            return;
        }
        if self.playhead >= self.end_time() - 1e-3 {
            self.playhead = self.project.range.map(|r| r.0).unwrap_or(0.0);
        }
        self.playing = true;
        self.restart_audio();
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.play_anchor = None;
        self.audio.stop();
    }

    pub fn restart_audio(&mut self) {
        if !self.playing {
            return;
        }
        self.play_anchor = Some((self.playhead, self.now()));
        self.audio.play(&self.project, self.playhead);
    }

    fn tick_playback(&mut self) {
        if !self.playing {
            return;
        }
        let t = self.audio.position().unwrap_or_else(|| match self.play_anchor {
            Some((start, at)) => start + (self.now() - at),
            None => self.playhead,
        });
        let end = self.end_time();
        if t >= end {
            if self.loop_playback {
                self.playhead = self.project.range.map(|r| r.0).unwrap_or(0.0);
                self.restart_audio();
            } else {
                self.playhead = end;
                self.stop();
            }
        } else {
            self.playhead = t;
        }
        self.ctx.request_repaint();
    }

    pub fn import_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().add_filter("Media", media::EXTENSIONS).add_filter("All files", &["*"]).pick_files() {
            self.importer.import(paths);
        }
    }

    fn poll_imports(&mut self) {
        for result in self.importer.poll() {
            match result {
                Ok(mut m) => {
                    if self.project.media.iter().any(|x| x.path == m.path) {
                        continue;
                    }
                    self.checkpoint();
                    m.id = MediaId(self.project.alloc_id());
                    let first = self.project.media.is_empty() && self.project.duration() == 0.0;
                    let id = m.id;
                    if first {
                        self.original = Some(m.path.clone());
                        self.dirty = false;
                    }
                    if first && m.has_video {
                        self.project.settings.width = m.width.max(2) & !1;
                        self.project.settings.height = m.height.max(2) & !1;
                        if m.fps > 0.0 {
                            self.project.settings.fps = m.fps;
                        }
                    }
                    self.project.media.push(m);
                    if first {
                        self.selection = edit_ops::add_media(&mut self.project, id, None, 0.0);
                    }
                    self.selected_media = Some(id);
                }
                Err(e) => self.notify(e, ToastKind::Error),
            }
        }
    }

    pub fn reset_engines(&mut self) {
        self.stop();
        self.decoder = decode::Decoder::new(self.ctx.clone());
        self.thumbs = thumbs::Thumbs::new(self.ctx.clone());
        self.waves = waveform::Waveforms::new(self.ctx.clone());
    }

    pub fn new_project(&mut self) {
        self.reset_engines();
        self.original = None;
        self.project = Project::default();
        self.history = history::History::default();
        self.project_path = None;
        self.dirty = false;
        self.selection.clear();
        self.selected_media = None;
        self.playhead = 0.0;
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("Cutline project", &["cutline"]).pick_file() {
            self.open_project(path);
        }
    }

    pub fn open_project(&mut self, path: PathBuf) {
        match project_io::load(&path).and_then(|mut p| project_io::verify_media(&mut p).map(|_| p)) {
            Ok(p) => {
                self.new_project();
                self.project = p;
                project_io::push_recent(&path);
                self.project_path = Some(path);
            }
            Err(e) => self.notify(e, ToastKind::Error),
        }
    }

    pub fn save(&mut self) {
        match self.project_path.clone() {
            Some(path) => self.save_to(path),
            None => self.save_as(),
        }
    }

    pub fn save_as(&mut self) {
        let name = format!("{}.cutline", self.project.name);
        if let Some(path) = rfd::FileDialog::new().add_filter("Cutline project", &["cutline"]).set_file_name(name).save_file() {
            self.save_to(path);
        }
    }

    fn save_to(&mut self, path: PathBuf) {
        if let Some(stem) = path.file_stem() {
            self.project.name = stem.to_string_lossy().into_owned();
        }
        match project_io::save(&self.project, &path) {
            Ok(()) => {
                self.dirty = false;
                project_io::push_recent(&path);
                self.project_path = Some(path);
                self.notify("Project saved", ToastKind::Success);
            }
            Err(e) => self.notify(e, ToastKind::Error),
        }
    }

    fn autosave(&mut self) {
        let now = self.now();
        if self.dirty && now - self.last_autosave > 60.0 {
            self.last_autosave = now;
            let _ = project_io::save(&self.project, &project_io::autosave_path());
        }
    }

    fn left_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab, name) in [(LeftTab::Media, "Media"), (LeftTab::Effects, "Effects")] {
                let on = self.left_tab == tab;
                let text = if on { theme::strong(name) } else { theme::muted(name) };
                if ui.add(egui::Button::new(text).selected(on).frame_when_inactive(false)).clicked() {
                    self.left_tab = tab;
                }
            }
        });
        ui.add_space(theme::SP_1);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_imports();
        let dropped: Vec<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() {
            self.importer.import(dropped);
        }
        commands::handle_shortcuts(self, &ctx);
        self.tick_playback();
        self.autosave();

        let gutter = egui::Margin::same(3);
        egui::Panel::top("menu")
            .exact_size(commands::TITLE_H)
            .frame(egui::Frame::new().fill(theme::BG_APP).inner_margin(egui::Margin { left: theme::SP_2 as i8, ..Default::default() }))
            .show(ui, |ui| commands::menu_bar(self, ui));
        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(theme::BG_APP).inner_margin(egui::Margin::symmetric(theme::SP_3 as i8, 3)))
            .show(ui, |ui| commands::status_bar(self, ui));
        egui::Panel::bottom("timeline")
            .resizable(true)
            .default_size(320.0)
            .min_size(180.0)
            .frame(theme::panel_frame().outer_margin(gutter))
            .show(ui, |ui| timeline::show(self, ui));
        egui::Panel::left("left")
            .resizable(true)
            .default_size(300.0)
            .min_size(220.0)
            .frame(theme::panel_frame().outer_margin(gutter).inner_margin(theme::SP_2 as i8))
            .show(ui, |ui| {
                self.left_tabs(ui);
                match self.left_tab {
                    LeftTab::Media => media_bin::show(self, ui),
                    LeftTab::Effects => effects_panel::show(self, ui),
                }
            });
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(300.0)
            .min_size(240.0)
            .frame(theme::panel_frame().outer_margin(gutter).inner_margin(theme::SP_2 as i8))
            .show(ui, |ui| inspector::show(self, ui));
        egui::CentralPanel::default().frame(theme::panel_frame().outer_margin(gutter)).show(ui, |ui| monitor::show(self, ui));

        export_dialog::show(self, &ctx);
        commands::window_frame(&ctx);
        commands::palette(self, &ctx);
        commands::toasts(self, &ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BG_APP.to_normalized_gamma_f32()
    }
}
