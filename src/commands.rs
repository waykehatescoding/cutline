use crate::app::{App, ToastKind, Tool};
use crate::export::{self, JobState};
use crate::icons::{self, Icon};
use crate::model::*;
use crate::updater::Status;
use crate::{edit_ops, project_io, shell, theme};
use eframe::egui::{self, vec2, Align, Align2, Event, FontId, Id, Key, Layout, Modifiers, Sense, Stroke};
use std::sync::Mutex;
use std::time::Duration;

static PENDING: Mutex<Vec<(String, ToastKind)>> = Mutex::new(Vec::new());

const N: Modifiers = Modifiers::NONE;
const C: Modifiers = Modifiers::COMMAND;
const S: Modifiers = Modifiers::SHIFT;
const CS: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    New,
    Open,
    Save,
    SaveAs,
    Import,
    Export,
    OverwriteOriginal,
    ExportFrame,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    RippleDelete,
    SelectAll,
    Deselect,
    Split,
    SplitAll,
    AddTitle,
    AddMatte,
    SpeedUp,
    SpeedDown,
    Reverse,
    MuteAudio,
    CloseGaps,
    AddVideoTrack,
    AddAudioTrack,
    SetIn,
    SetOut,
    ClearInOut,
    AddMarker,
    PrevEdit,
    NextEdit,
    GoStart,
    GoEnd,
    StepBack,
    StepForward,
    JumpBack,
    JumpForward,
    PlayPause,
    ShuttleBack,
    Pause,
    Play,
    SelectTool,
    RazorTool,
    SlipTool,
    Snapping,
    RippleMode,
    Loop,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    Palette,
    Shortcuts,
    ExplorerIntegration,
    CheckUpdates,
    About,
}

use Command::*;

const MENUS: &[(&str, &[&[Command]])] = &[
    ("File", &[&[New, Open], &[Save, SaveAs], &[Import], &[Export, OverwriteOriginal, ExportFrame], &[Quit]]),
    ("Edit", &[&[Undo, Redo], &[Cut, Copy, Paste, Duplicate], &[Delete, RippleDelete], &[SelectAll, Deselect]]),
    (
        "Clip",
        &[&[Split, SplitAll], &[AddTitle, AddMatte], &[SpeedUp, SpeedDown, Reverse, MuteAudio], &[CloseGaps, AddVideoTrack, AddAudioTrack]],
    ),
    (
        "Sequence",
        &[
            &[SetIn, SetOut, ClearInOut, AddMarker],
            &[PlayPause, ShuttleBack, Pause, Play, Loop],
            &[PrevEdit, NextEdit, GoStart, GoEnd],
            &[StepBack, StepForward, JumpBack, JumpForward],
        ],
    ),
    ("View", &[&[SelectTool, RazorTool, SlipTool], &[Snapping, RippleMode], &[ZoomIn, ZoomOut, ZoomFit], &[Palette]]),
    ("Help", &[&[Shortcuts, ExplorerIntegration], &[CheckUpdates, About]]),
];

fn key_label(m: Modifiers, k: Key) -> String {
    let mut s = String::new();
    if m.command {
        s += "Ctrl+";
    }
    if m.alt {
        s += "Alt+";
    }
    if m.shift {
        s += "Shift+";
    }
    s + k.symbol_or_name()
}

impl Command {
    pub fn all() -> impl Iterator<Item = Command> {
        MENUS.iter().flat_map(|(_, groups)| groups.iter().flat_map(|g| g.iter().copied()))
    }

    pub fn name(self) -> &'static str {
        match self {
            New => "New project",
            Open => "Open project",
            Save => "Save",
            SaveAs => "Save as",
            Import => "Import media",
            Export => "Export",
            OverwriteOriginal => "Save over original",
            ExportFrame => "Export frame as PNG",
            Quit => "Quit",
            Undo => "Undo",
            Redo => "Redo",
            Cut => "Cut",
            Copy => "Copy",
            Paste => "Paste at playhead",
            Duplicate => "Duplicate",
            Delete => "Delete",
            RippleDelete => "Ripple delete",
            SelectAll => "Select all",
            Deselect => "Deselect",
            Split => "Split at playhead",
            SplitAll => "Split all tracks",
            AddTitle => "Add title",
            AddMatte => "Add color matte",
            SpeedUp => "Speed up",
            SpeedDown => "Speed down",
            Reverse => "Reverse",
            MuteAudio => "Mute clip audio",
            CloseGaps => "Close gaps",
            AddVideoTrack => "Add video track",
            AddAudioTrack => "Add audio track",
            SetIn => "Set in point",
            SetOut => "Set out point",
            ClearInOut => "Clear in and out",
            AddMarker => "Add marker",
            PrevEdit => "Previous edit",
            NextEdit => "Next edit",
            GoStart => "Go to start",
            GoEnd => "Go to end",
            StepBack => "Step frame back",
            StepForward => "Step frame forward",
            JumpBack => "Jump back 1 second",
            JumpForward => "Jump forward 1 second",
            PlayPause => "Play or pause",
            ShuttleBack => "Back 1 second",
            Pause => "Pause",
            Play => "Play",
            SelectTool => "Select tool",
            RazorTool => "Razor tool",
            SlipTool => "Slip tool",
            Snapping => "Snapping",
            RippleMode => "Ripple editing",
            Loop => "Loop playback",
            ZoomIn => "Zoom in",
            ZoomOut => "Zoom out",
            ZoomFit => "Zoom to fit",
            Palette => "Command palette",
            Shortcuts => "Keyboard shortcuts",
            ExplorerIntegration => "Show Edit this video in Explorer",
            CheckUpdates => "Check for updates",
            About => "About Cutline",
        }
    }

    fn keys(self) -> &'static [(Modifiers, Key)] {
        match self {
            New => &[(C, Key::N)],
            Open => &[(C, Key::O)],
            Save => &[(C, Key::S)],
            SaveAs => &[(CS, Key::S)],
            Import => &[(C, Key::I)],
            Export => &[(C, Key::E)],
            ExportFrame => &[(CS, Key::E)],
            Quit => &[(C, Key::Q)],
            Undo => &[(C, Key::Z)],
            Redo => &[(C, Key::Y), (CS, Key::Z)],
            Cut => &[(C, Key::X)],
            Copy => &[(C, Key::C)],
            Paste => &[(C, Key::V)],
            Duplicate => &[(C, Key::D)],
            Delete => &[(N, Key::Delete), (N, Key::Backspace)],
            RippleDelete => &[(C, Key::Delete)],
            SelectAll => &[(C, Key::A)],
            Deselect => &[(CS, Key::A)],
            Split => &[(C, Key::K), (N, Key::S)],
            SplitAll => &[(CS, Key::K)],
            AddTitle => &[(C, Key::T)],
            ClearInOut => &[(CS, Key::X)],
            SetIn => &[(N, Key::I)],
            SetOut => &[(N, Key::O)],
            AddMarker => &[(N, Key::M)],
            PrevEdit => &[(N, Key::ArrowUp)],
            NextEdit => &[(N, Key::ArrowDown)],
            GoStart => &[(N, Key::Home)],
            GoEnd => &[(N, Key::End)],
            StepBack => &[(N, Key::ArrowLeft)],
            StepForward => &[(N, Key::ArrowRight)],
            JumpBack => &[(S, Key::ArrowLeft)],
            JumpForward => &[(S, Key::ArrowRight)],
            PlayPause => &[(N, Key::Space)],
            ShuttleBack => &[(N, Key::J)],
            Pause => &[(N, Key::K)],
            Play => &[(N, Key::L)],
            SelectTool => &[(N, Key::V)],
            RazorTool => &[(N, Key::C)],
            SlipTool => &[(N, Key::Y)],
            Snapping => &[(N, Key::N)],
            Loop => &[(C, Key::L)],
            ZoomIn => &[(N, Key::Equals)],
            ZoomOut => &[(N, Key::Minus)],
            ZoomFit => &[(N, Key::Backslash)],
            Palette => &[(C, Key::P)],
            Shortcuts => &[(N, Key::F1)],
            _ => &[],
        }
    }

    pub fn shortcut(self) -> Option<String> {
        self.keys().first().map(|&(m, k)| key_label(m, k))
    }

    fn shortcut_list(self) -> String {
        self.keys().iter().map(|&(m, k)| key_label(m, k)).collect::<Vec<_>>().join(" / ")
    }

    fn tooltip(self) -> String {
        match self.shortcut() {
            Some(s) => format!("{} ({s})", self.name()),
            None => self.name().to_string(),
        }
    }

    fn checked(self, app: &App) -> Option<bool> {
        match self {
            Snapping => Some(app.view.snap),
            RippleMode => Some(app.view.ripple),
            Loop => Some(app.loop_playback),
            SelectTool => Some(app.tool == Tool::Select),
            RazorTool => Some(app.tool == Tool::Razor),
            SlipTool => Some(app.tool == Tool::Slip),
            ExplorerIntegration => Some(shell::is_registered()),
            _ => None,
        }
    }

    pub fn enabled(self, app: &App) -> bool {
        let selected = !app.selection.is_empty();
        let has_clips = app.project.duration() > 0.0;
        match self {
            Export => has_clips && app.ffmpeg_ok,
            OverwriteOriginal => {
                has_clips && app.ffmpeg_ok && app.export_job.is_none() && app.original.as_deref().and_then(export::format_for).is_some()
            }
            ExportFrame => has_clips && app.ffmpeg_ok,
            Undo => app.history.can_undo(),
            Redo => app.history.can_redo(),
            Cut | Copy | Duplicate | Delete | RippleDelete | Deselect => selected,
            Paste => !app.clipboard.is_empty(),
            SelectAll | Split | SplitAll | Play | PlayPause => has_clips,
            SpeedUp | SpeedDown | Reverse | MuteAudio => app.primary_clip().is_some(),
            ClearInOut => app.project.range.is_some(),
            _ => true,
        }
    }

    pub fn run(self, app: &mut App) {
        match self {
            New => app.new_project(),
            Open => app.open_project_dialog(),
            Save => app.save(),
            SaveAs => app.save_as(),
            Import => app.import_dialog(),
            Export => app.export_dialog.open = true,
            OverwriteOriginal => app.export_dialog.confirm_replace = true,
            ExportFrame => export_frame(app),
            Quit => app.ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Undo => app.undo(),
            Redo => app.redo(),
            Cut => {
                app.clipboard = edit_ops::copy(&app.project, &app.selection);
                edit(app, |a| {
                    let ids = std::mem::take(&mut a.selection);
                    edit_ops::delete(&mut a.project, &ids, a.view.ripple);
                });
            }
            Copy => app.clipboard = edit_ops::copy(&app.project, &app.selection),
            Paste => {
                let clips = app.clipboard.clone();
                edit(app, |a| a.selection = edit_ops::paste(&mut a.project, &clips, a.playhead));
            }
            Duplicate => edit(app, |a| a.selection = edit_ops::duplicate(&mut a.project, &a.selection.clone())),
            Delete => delete_selected(app, app.view.ripple),
            RippleDelete => delete_selected(app, true),
            SelectAll => app.selection = app.project.tracks.iter().flat_map(|t| t.clips.iter().map(|c| c.id)).collect(),
            Deselect => app.selection.clear(),
            Split => split(app),
            SplitAll => edit(app, |a| {
                edit_ops::split_all_at(&mut a.project, a.playhead);
            }),
            AddTitle => edit(app, |a| a.selection.extend(edit_ops::add_text(&mut a.project, a.playhead))),
            AddMatte => edit(app, |a| a.selection.extend(edit_ops::add_matte(&mut a.project, [0, 0, 0, 255], a.playhead))),
            SpeedUp => scale_speed(app, 2.0),
            SpeedDown => scale_speed(app, 0.5),
            Reverse => each_selected(app, |c| c.reverse = !c.reverse),
            MuteAudio => each_selected(app, |c| c.audio_muted = !c.audio_muted),
            CloseGaps => edit(app, |a| {
                let mut tracks: Vec<usize> = a.selection.iter().filter_map(|id| a.project.clip(*id).map(|p| p.0)).collect();
                if tracks.is_empty() {
                    tracks = (0..a.project.tracks.len()).collect();
                }
                tracks.sort_unstable();
                tracks.dedup();
                for t in tracks {
                    edit_ops::close_gaps(&mut a.project, t);
                }
            }),
            AddVideoTrack => edit(app, |a| {
                edit_ops::add_track(&mut a.project, TrackKind::Video);
            }),
            AddAudioTrack => edit(app, |a| {
                edit_ops::add_track(&mut a.project, TrackKind::Audio);
            }),
            SetIn => {
                let t = app.playhead;
                let end = app.project.range.map(|r| r.1).filter(|e| *e > t).unwrap_or_else(|| app.project.duration().max(t));
                app.checkpoint();
                app.project.range = Some((t, end));
            }
            SetOut => {
                let t = app.playhead;
                let start = app.project.range.map(|r| r.0).filter(|s| *s < t).unwrap_or(0.0);
                app.checkpoint();
                app.project.range = Some((start, t));
            }
            ClearInOut => {
                app.checkpoint();
                app.project.range = None;
            }
            AddMarker => edit(app, |a| edit_ops::add_marker(&mut a.project, a.playhead)),
            PrevEdit => seek_edit(app, false),
            NextEdit => seek_edit(app, true),
            GoStart => app.seek(0.0),
            GoEnd => app.seek(app.project.duration()),
            StepBack => app.seek(app.playhead - 1.0 / app.project.settings.fps.max(1.0)),
            StepForward => app.seek(app.playhead + 1.0 / app.project.settings.fps.max(1.0)),
            JumpBack => app.seek(app.playhead - 1.0),
            JumpForward => app.seek(app.playhead + 1.0),
            PlayPause => app.toggle_play(),
            ShuttleBack => app.seek(app.playhead - 1.0),
            Pause => app.stop(),
            Play => app.play(),
            SelectTool => app.tool = Tool::Select,
            RazorTool => app.tool = Tool::Razor,
            SlipTool => app.tool = Tool::Slip,
            Snapping => app.view.snap = !app.view.snap,
            RippleMode => app.view.ripple = !app.view.ripple,
            Loop => app.loop_playback = !app.loop_playback,
            ZoomIn => app.view.px_per_sec = (app.view.px_per_sec * 1.25).min(2000.0),
            ZoomOut => app.view.px_per_sec = (app.view.px_per_sec / 1.25).max(1.0),
            ZoomFit => {
                let d = app.project.duration();
                if d > 0.0 {
                    let width = (app.ctx.content_rect().width() - 300.0).max(200.0);
                    app.view.px_per_sec = (width as f64 / d).clamp(1.0, 2000.0) as f32;
                    app.view.scroll_x = 0.0;
                }
            }
            Palette => app.palette_open = true,
            Shortcuts => set_flag(&app.ctx, shortcuts_id(), true),
            ExplorerIntegration => toggle_explorer(app),
            CheckUpdates => {
                set_flag(&app.ctx, manual_check_id(), true);
                app.updater.check();
            }
            About => set_flag(&app.ctx, about_id(), true),
        }
    }
}

fn edit(app: &mut App, f: impl FnOnce(&mut App)) {
    app.checkpoint();
    f(app);
    app.restart_audio();
}

fn each_selected(app: &mut App, f: impl Fn(&mut Clip)) {
    let ids = app.selection.clone();
    edit(app, |a| {
        for id in ids {
            if let Some(c) = a.project.clip_mut(id) {
                f(c);
            }
        }
    });
}

fn scale_speed(app: &mut App, factor: f64) {
    let ids = app.selection.clone();
    edit(app, |a| {
        for id in ids {
            if let Some(speed) = a.project.clip_ref(id).map(|c| (c.speed * factor).clamp(0.25, 8.0)) {
                edit_ops::set_speed(&mut a.project, id, speed);
            }
        }
    });
}

fn delete_selected(app: &mut App, ripple: bool) {
    edit(app, |a| {
        let ids = std::mem::take(&mut a.selection);
        edit_ops::delete(&mut a.project, &ids, ripple);
    });
}

fn split(app: &mut App) {
    let t = app.playhead;
    let mut ids: Vec<ClipId> =
        app.selection.iter().copied().filter(|id| app.project.clip_ref(*id).is_some_and(|c| c.contains(t))).collect();
    if ids.is_empty() {
        ids = edit_ops::clips_at(&app.project, t);
    }
    if ids.is_empty() {
        return;
    }
    edit(app, |a| {
        for id in ids {
            edit_ops::split_at(&mut a.project, id, t);
        }
    });
}

fn seek_edit(app: &mut App, forward: bool) {
    let target = edit_ops::next_edit(&app.project, app.playhead, forward);
    app.seek(target.unwrap_or(if forward { app.project.duration() } else { 0.0 }));
}

fn export_frame(app: &mut App) {
    let name = format!("{}.png", app.project.name);
    let Some(path) = rfd::FileDialog::new().add_filter("PNG image", &["png"]).set_file_name(name).save_file() else {
        return;
    };
    let project = app.project.clone();
    let t = app.playhead;
    let ctx = app.ctx.clone();
    std::thread::spawn(move || {
        let result = match export::job::export_frame(&project, t, &path) {
            Ok(()) => {
                (format!("Saved {}", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()), ToastKind::Success)
            }
            Err(e) => (e, ToastKind::Error),
        };
        PENDING.lock().unwrap().push(result);
        ctx.request_repaint();
    });
}

fn toggle_explorer(app: &mut App) {
    let result = if shell::is_registered() {
        shell::unregister().map(|_| "Removed from the Explorer menu")
    } else {
        shell::register().map(|_| "Added to the Explorer menu")
    };
    match result {
        Ok(text) => app.notify(text, ToastKind::Success),
        Err(e) => app.notify(e, ToastKind::Error),
    }
}

fn shortcuts_id() -> Id {
    Id::unique("cutline_shortcuts_open")
}

fn about_id() -> Id {
    Id::unique("cutline_about_open")
}

fn manual_check_id() -> Id {
    Id::unique("cutline_manual_update_check")
}

fn update_status(app: &mut App, ui: &mut egui::Ui) {
    let manual = app.ctx.data(|d| d.get_temp::<bool>(manual_check_id())).unwrap_or(false);
    match app.updater.status() {
        Status::Checking if manual => {
            ui.add(egui::Spinner::new().size(10.0));
            ui.label(theme::small("Checking for updates"));
        }
        Status::Downloading => {
            ui.add(egui::Spinner::new().size(10.0));
            ui.label(theme::small("Downloading update"));
        }
        Status::Available { version, .. } => {
            let text = egui::RichText::new(format!("Update to {version}")).size(theme::SIZE_SMALL).color(theme::ACCENT);
            if ui.add(egui::Button::new(text).frame(false)).on_hover_text("Download and install, then restart Cutline").clicked() {
                if app.dirty {
                    app.notify("Save your project before updating", ToastKind::Error);
                } else {
                    app.updater.install();
                }
            }
        }
        Status::UpToDate if manual => {
            set_flag(&app.ctx, manual_check_id(), false);
            app.notify(format!("Cutline {} is up to date", env!("CARGO_PKG_VERSION")), ToastKind::Success);
        }
        Status::Failed(e) if manual => {
            set_flag(&app.ctx, manual_check_id(), false);
            app.notify(format!("Update failed: {e}"), ToastKind::Error);
        }
        _ => {}
    }
}

fn set_flag(ctx: &egui::Context, id: Id, on: bool) {
    ctx.data_mut(|d| d.insert_temp(id, on));
}

fn menu_item(app: &mut App, ui: &mut egui::Ui, cmd: Command) {
    let mut button = egui::Button::new(cmd.name());
    if let Some(s) = cmd.shortcut() {
        button = button.shortcut_text(theme::small(s));
    }
    if cmd.checked(app) == Some(true) {
        button = button.selected(true);
    }
    if ui.add_enabled(cmd.enabled(app), button).clicked() {
        cmd.run(app);
    }
}

fn recent_menu(app: &mut App, ui: &mut egui::Ui) {
    ui.menu_button("Open recent", |ui| {
        let recent = project_io::recent();
        if recent.is_empty() {
            ui.add_enabled(false, egui::Button::new("No recent projects"));
        }
        for path in recent {
            let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                app.open_project(path);
            }
        }
    });
}

pub const TITLE_H: f32 = 34.0;
const CAPTION_W: f32 = 46.0;

pub fn menu_bar(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let bar = ui.max_rect();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let drag = ui.interact(bar, Id::unique("cutline_title_drag"), Sense::click_and_drag());
    if drag.double_clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    } else if drag.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    let file = app.original.as_ref().filter(|_| app.project_path.is_none()).and_then(|p| p.file_name()).map(|n| n.to_string_lossy());
    let title = match (&file, app.project.name.trim()) {
        (Some(f), _) => f.as_ref(),
        (None, "") => "Untitled",
        (None, name) => name,
    };
    let galley = ui.painter().layout_no_wrap(format!("{title} - Cutline"), FontId::proportional(theme::SIZE_BODY), theme::TEXT_MUTED);
    let pos = bar.center() - galley.size() / 2.0;
    if app.dirty {
        ui.painter().circle_filled(egui::pos2(pos.x - theme::SP_2, bar.center().y), 3.0, theme::ACCENT);
    }
    ui.painter().galley(pos, galley, theme::TEXT_MUTED);

    ui.horizontal_centered(|ui| {
        let (icon, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        let tex = app_icon(&ctx);
        ui.painter().image(tex.id(), icon, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        ui.add_space(theme::SP_1);
        egui::MenuBar::new().ui(ui, |ui| {
            for (name, groups) in MENUS {
                ui.menu_button(*name, |ui| {
                    ui.set_min_width(260.0);
                    for (i, group) in groups.iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        for &cmd in *group {
                            menu_item(app, ui, cmd);
                            if cmd == Open {
                                recent_menu(app, ui);
                            }
                        }
                    }
                });
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            caption_buttons(ui, maximized);
            ui.spacing_mut().item_spacing.x = theme::SP_2;
            ui.add_space(theme::SP_2);
            bar_right(app, ui);
        });
    });
}

fn app_icon(ctx: &egui::Context) -> egui::TextureHandle {
    let id = Id::unique("cutline_app_icon");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let img = egui::ColorImage::from_rgba_unmultiplied([64, 64], include_bytes!("../assets/icon64.rgba"));
    let t = ctx.load_texture("cutline-icon", img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

fn caption_buttons(ui: &mut egui::Ui, maximized: bool) {
    let h = ui.available_height().max(TITLE_H);
    let buttons = [
        (Icon::Close, "Close", egui::ViewportCommand::Close),
        if maximized {
            (Icon::Restore, "Restore", egui::ViewportCommand::Maximized(false))
        } else {
            (Icon::Maximize, "Maximize", egui::ViewportCommand::Maximized(true))
        },
        (Icon::Minimize, "Minimize", egui::ViewportCommand::Minimized(true)),
    ];
    for (icon, label, cmd) in buttons {
        let (rect, resp) = ui.allocate_exact_size(vec2(CAPTION_W, h), Sense::click());
        let close = icon == Icon::Close;
        let (bg, fg) = match (resp.hovered(), close) {
            (true, true) => (theme::DANGER, egui::Color32::WHITE),
            (true, false) => (theme::BG_HOVER, theme::TEXT),
            _ => (egui::Color32::TRANSPARENT, theme::TEXT_MUTED),
        };
        ui.painter().rect_filled(rect, 0.0, bg);
        icons::paint(ui.painter(), egui::Rect::from_center_size(rect.center(), vec2(10.0, 10.0)), icon, fg);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
        if resp.clicked() {
            ui.ctx().send_viewport_cmd(cmd);
        }
    }
}

pub fn window_frame(ctx: &egui::Context) {
    let (maximized, rect, pointer, pressed) =
        ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.content_rect(), i.pointer.hover_pos(), i.pointer.primary_pressed()));
    if maximized {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::unique("cutline_frame")));
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::BORDER_STRONG), egui::StrokeKind::Inside);
    let Some(p) = pointer else { return };
    let e = 5.0;
    let (l, r, t, b) = (p.x - rect.left() < e, rect.right() - p.x < e, p.y - rect.top() < e, rect.bottom() - p.y < e);
    use egui::ResizeDirection as D;
    let dir = match (l, r, t, b) {
        (true, _, true, _) => D::NorthWest,
        (_, true, true, _) => D::NorthEast,
        (true, _, _, true) => D::SouthWest,
        (_, true, _, true) => D::SouthEast,
        (true, ..) => D::West,
        (_, true, ..) => D::East,
        (_, _, true, _) => D::North,
        (.., true) => D::South,
        _ => return,
    };
    ctx.set_cursor_icon(match dir {
        D::NorthWest | D::SouthEast => egui::CursorIcon::ResizeNwSe,
        D::NorthEast | D::SouthWest => egui::CursorIcon::ResizeNeSw,
        D::West | D::East => egui::CursorIcon::ResizeHorizontal,
        D::North | D::South => egui::CursorIcon::ResizeVertical,
    });
    if pressed {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
    }
}

fn bar_right(app: &mut App, ui: &mut egui::Ui) {
    let ready = Export.enabled(app);
    ui.add_enabled_ui(ready, |ui| {
        if theme::primary_button(ui, "Export").on_hover_text(Export.tooltip()).clicked() {
            Export.run(app);
        }
    });
    if app.original.is_some() {
        ui.add_enabled_ui(OverwriteOriginal.enabled(app), |ui| {
            if ui.button("Save over original").on_hover_text(OverwriteOriginal.tooltip()).clicked() {
                OverwriteOriginal.run(app);
            }
        });
    }
    ui.add_space(theme::SP_1);
    for (cmd, icon) in [(Redo, Icon::Redo), (Undo, Icon::Undo)] {
        ui.add_enabled_ui(cmd.enabled(app), |ui| {
            if icons::button(ui, icon, &cmd.tooltip()).clicked() {
                cmd.run(app);
            }
        });
    }
}

pub fn status_bar(app: &mut App, ui: &mut egui::Ui) {
    let progress = app.export_job.as_mut().and_then(|job| match job.poll() {
        JobState::Running { progress, .. } => Some(*progress),
        _ => None,
    });
    ui.horizontal(|ui| {
        if let Some(p) = progress {
            let (rect, _) = ui.allocate_exact_size(vec2(72.0, 4.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, theme::BG_ACTIVE);
            ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, vec2(rect.width() * p.clamp(0.0, 1.0), 4.0)), 2.0, theme::ACCENT);
            ui.label(theme::mono(format!("Exporting {}%", (p * 100.0).round() as u32)).size(theme::SIZE_SMALL).color(theme::TEXT_MUTED));
        } else if !app.ffmpeg_ok {
            ui.label(egui::RichText::new("ffmpeg not found").size(theme::SIZE_SMALL).color(theme::DANGER));
        }
        update_status(app, ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let s = &app.project.settings;
            ui.label(theme::mono(format!("{}x{}  {} fps", s.width, s.height, s.fps)).size(theme::SIZE_SMALL).color(theme::TEXT_MUTED));
            match app.selection.len() {
                0 => {}
                1 => {
                    ui.label(theme::small("1 clip selected"));
                }
                n => {
                    ui.label(theme::small(format!("{n} clips selected")));
                }
            }
        });
    });
}

pub fn handle_shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.text_edit_focused() {
        return;
    }
    let (key_down, clip) = ctx.input(|i| {
        let key_down = i.events.iter().any(|e| matches!(e, Event::Key { pressed: true, .. }));
        let clip = i.events.iter().find_map(|e| match e {
            Event::Cut => Some(Cut),
            Event::Copy => Some(Copy),
            Event::Paste(_) => Some(Paste),
            _ => None,
        });
        (key_down, clip)
    });
    if let Some(cmd) = clip {
        if cmd.enabled(app) {
            cmd.run(app);
        }
        return;
    }
    if !key_down {
        return;
    }
    let weight = |m: Modifiers| m.command as u8 + m.shift as u8 + m.alt as u8;
    for w in (0..=3).rev() {
        for cmd in Command::all() {
            for &(m, k) in cmd.keys() {
                if weight(m) == w && ctx.input_mut(|i| i.consume_key(m, k)) {
                    if cmd.enabled(app) {
                        cmd.run(app);
                    }
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Default)]
struct PaletteState {
    query: String,
    sel: usize,
    focused: bool,
}

fn fuzzy(query: &str, name: &str) -> Option<i32> {
    let name = name.to_lowercase();
    let mut chars = name.char_indices();
    let mut last: i32 = -1;
    let mut penalty = 0;
    for q in query.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        let (i, _) = chars.find(|(_, c)| *c == q)?;
        penalty += i as i32 - last - 1;
        last = i as i32;
    }
    Some(-penalty)
}

pub fn palette(app: &mut App, ctx: &egui::Context) {
    windows(ctx);
    let id = Id::unique("cutline_palette");
    if !app.palette_open {
        ctx.data_mut(|d| d.remove::<PaletteState>(id));
        return;
    }
    let mut st = ctx.data(|d| d.get_temp::<PaletteState>(id)).unwrap_or_default();
    let (down, up, enter, esc) = ctx.input_mut(|i| {
        (i.consume_key(N, Key::ArrowDown), i.consume_key(N, Key::ArrowUp), i.consume_key(N, Key::Enter), i.consume_key(N, Key::Escape))
    });
    if esc {
        app.palette_open = false;
        return;
    }
    let moved = down || up;
    let mut run = None;
    let area = egui::Area::new(id).order(egui::Order::Foreground).anchor(Align2::CENTER_TOP, [0.0, theme::SP_5 * 3.0]).show(ctx, |ui| {
        egui::Frame::new()
            .fill(theme::BG_RAISED)
            .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
            .corner_radius(theme::R_OUTER)
            .shadow(ctx.global_style().visuals.popup_shadow)
            .inner_margin(theme::PAD as i8)
            .show(ui, |ui| {
                ui.set_width(480.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut st.query)
                        .hint_text("Type a command")
                        .desired_width(f32::INFINITY)
                        .margin(vec2(theme::SP_2, theme::SP_2)),
                );
                if !st.focused {
                    edit.request_focus();
                    st.focused = true;
                }
                if edit.changed() {
                    st.sel = 0;
                }
                let mut hits: Vec<(i32, Command)> = Command::all().filter_map(|c| fuzzy(&st.query, c.name()).map(|s| (s, c))).collect();
                hits.sort_by_key(|h| -h.0);
                if down {
                    st.sel += 1;
                }
                if up {
                    st.sel = st.sel.saturating_sub(1);
                }
                st.sel = st.sel.min(hits.len().saturating_sub(1));
                if enter {
                    run = hits.get(st.sel).map(|h| h.1).filter(|c| c.enabled(app));
                }
                ui.add_space(theme::SP_1);
                if hits.is_empty() {
                    ui.add_space(theme::SP_1);
                    ui.label(theme::muted("No matching commands"));
                    ui.add_space(theme::SP_1);
                }
                egui::ScrollArea::vertical().max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (i, (_, cmd)) in hits.iter().enumerate() {
                        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
                        let on = cmd.enabled(app);
                        let selected = i == st.sel;
                        if selected || (resp.hovered() && on) {
                            let fill = if selected { theme::BG_ACTIVE } else { theme::BG_HOVER };
                            ui.painter().rect_filled(rect, theme::R_INNER, fill);
                        }
                        let color = if on { theme::TEXT } else { theme::TEXT_FAINT };
                        ui.painter().text(
                            rect.left_center() + vec2(theme::SP_2, 0.0),
                            Align2::LEFT_CENTER,
                            cmd.name(),
                            FontId::proportional(theme::SIZE_BODY),
                            color,
                        );
                        if let Some(s) = cmd.shortcut() {
                            let muted = if on { theme::TEXT_MUTED } else { theme::TEXT_FAINT };
                            ui.painter().text(
                                rect.right_center() - vec2(theme::SP_2, 0.0),
                                Align2::RIGHT_CENTER,
                                s,
                                FontId::proportional(theme::SIZE_SMALL),
                                muted,
                            );
                        }
                        if selected && moved {
                            ui.scroll_to_rect(rect, None);
                        }
                        if resp.clicked() && on {
                            run = Some(*cmd);
                        }
                    }
                });
            });
    });
    let outside = ctx.input(|i| i.pointer.any_pressed()) && ctx.pointer_interact_pos().is_some_and(|p| !area.response.rect.contains(p));
    ctx.data_mut(|d| d.insert_temp(id, st));
    if run.is_some() || outside {
        app.palette_open = false;
    }
    if let Some(cmd) = run {
        cmd.run(app);
    }
}

fn windows(ctx: &egui::Context) {
    let mut open = ctx.data(|d| d.get_temp::<bool>(shortcuts_id())).unwrap_or(false);
    if open {
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                    egui::Grid::new("cutline_shortcuts_grid").num_columns(2).spacing([theme::SP_5, theme::SP_1]).show(ui, |ui| {
                        for (i, (menu, groups)) in MENUS.iter().enumerate() {
                            if i > 0 {
                                ui.add_space(theme::SP_2);
                                ui.end_row();
                            }
                            ui.label(theme::heading(*menu));
                            ui.end_row();
                            for cmd in groups.iter().flat_map(|g| g.iter()).filter(|c| !c.keys().is_empty()) {
                                ui.label(cmd.name());
                                ui.label(theme::mono(cmd.shortcut_list()));
                                ui.end_row();
                            }
                        }
                    });
                });
            });
        set_flag(ctx, shortcuts_id(), open);
    }
    let mut open = ctx.data(|d| d.get_temp::<bool>(about_id())).unwrap_or(false);
    if open {
        egui::Window::new("About Cutline")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(theme::heading("Cutline"));
                ui.horizontal(|ui| {
                    ui.label(theme::muted("Version"));
                    ui.label(theme::mono(env!("CARGO_PKG_VERSION")));
                });
                ui.add_space(theme::SP_1);
                ui.label("A fast, lightweight non-linear video editor.");
            });
        set_flag(ctx, about_id(), open);
    }
}

pub fn toasts(app: &mut App, ctx: &egui::Context) {
    let pending: Vec<_> = PENDING.lock().unwrap().drain(..).collect();
    for (text, kind) in pending {
        app.notify(text, kind);
    }
    if app.toasts.is_empty() {
        return;
    }
    let now = app.now();
    let ttl = |k: ToastKind| if k == ToastKind::Error { 8.0 } else { 4.0 };
    app.toasts.retain(|t| now - t.born < ttl(t.kind));
    let mut next = app.toasts.iter().map(|t| ttl(t.kind) - (now - t.born)).fold(f64::MAX, f64::min);
    let mut dismissed = vec![];
    let mut y = theme::SP_4 + 28.0;
    for (i, toast) in app.toasts.iter().enumerate().rev() {
        let age = now - toast.born;
        if age < 0.3 {
            next = next.min(0.016);
        }
        let id = Id::unique(("cutline_toast", toast.born.to_bits(), toast.text.len()));
        let shown = ctx.animate_value_with_time(id, if age < 0.03 { 0.0 } else { 1.0 }, 0.18);
        let stripe = match toast.kind {
            ToastKind::Success => theme::SUCCESS,
            ToastKind::Error => theme::DANGER,
            ToastKind::Info => theme::INFO,
        };
        let offset = [-theme::SP_4 + (1.0 - shown) * theme::SP_5, -y];
        let inner =
            egui::Area::new(id).sense(Sense::click()).order(egui::Order::Tooltip).anchor(Align2::RIGHT_BOTTOM, offset).show(ctx, |ui| {
                ui.set_opacity(shown);
                let frame = egui::Frame::new()
                    .fill(theme::BG_RAISED)
                    .corner_radius(theme::R_OUTER)
                    .shadow(ctx.global_style().visuals.popup_shadow)
                    .inner_margin(egui::Margin {
                        left: theme::SP_3 as i8 + 3,
                        right: theme::SP_3 as i8,
                        top: theme::SP_2 as i8,
                        bottom: theme::SP_2 as i8,
                    });
                let shown_frame = frame.show(ui, |ui| {
                    ui.set_max_width(320.0);
                    ui.add(egui::Label::new(egui::RichText::new(&toast.text).color(theme::TEXT)).wrap());
                });
                let rect = shown_frame.response.rect;
                let r = theme::R_OUTER;
                let radius = egui::CornerRadius { nw: r, sw: r, ne: 0, se: 0 };
                let strip = egui::Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
                ui.painter().rect_filled(strip, radius, stripe);
                rect
            });
        let click = inner.response;
        if click.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if click.clicked() {
            dismissed.push(i);
        }
        y += inner.inner.height() + theme::SP_2;
    }
    for i in dismissed {
        app.toasts.remove(i);
    }
    ctx.request_repaint_after(Duration::from_secs_f64(next.max(0.016)));
}
