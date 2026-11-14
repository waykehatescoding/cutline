use crate::app::{App, ToastKind};
use crate::export::job::ExportJob;
use crate::export::{ExportOptions, Format, JobState};
use crate::icons::{self, Icon};
use crate::theme;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::Duration;

const HEIGHTS: [u32; 5] = [2160, 1440, 1080, 720, 480];
const FPS: [f64; 5] = [24.0, 25.0, 30.0, 50.0, 60.0];
const AUDIO_KBPS: [u32; 4] = [128, 192, 256, 320];

#[derive(Default)]
pub struct State {
    pub open: bool,
    was_open: bool,
    opts: Option<ExportOptions>,
    out_path: Option<PathBuf>,
    height: Option<u32>,
    fps: Option<f64>,
    error: Option<String>,
    result: Option<JobState>,
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    poll(app, ctx);
    if !app.export_dialog.open {
        app.export_dialog.was_open = false;
        return;
    }
    if !app.export_dialog.was_open {
        app.export_dialog.was_open = true;
        if app.export_job.is_none() && app.export_dialog.result.is_none() {
            init(app);
        }
    }
    let modal = egui::Modal::new(egui::Id::unique("export_dialog")).show(ctx, |ui| {
        ui.set_width(460.0);
        header(app, ui);
        ui.add_space(theme::SP_3);
        match app.export_job.as_mut().map(|j| j.poll().clone()) {
            Some(JobState::Running { progress, eta, speed }) => progress_view(app, ui, progress, eta, speed),
            _ => match app.export_dialog.result.clone() {
                Some(result) => result_view(app, ui, ctx, result),
                None => form(app, ui),
            },
        }
    });
    if modal.should_close() {
        app.export_dialog.open = false;
    }
}

fn poll(app: &mut App, ctx: &egui::Context) {
    let Some(job) = app.export_job.as_mut() else { return };
    let state = job.poll().clone();
    if matches!(state, JobState::Running { .. }) {
        ctx.request_repaint_after(Duration::from_millis(100));
        return;
    }
    app.export_job = None;
    if !app.export_dialog.open {
        match &state {
            JobState::Done { path, .. } => app.notify(format!("Exported {}", file_name(path)), ToastKind::Success),
            JobState::Failed(e) => app.notify(format!("Export failed: {}", e.lines().next().unwrap_or("")), ToastKind::Error),
            _ => {}
        }
    }
    app.export_dialog.result = Some(state);
}

fn init(app: &mut App) {
    let p = &app.project;
    let opts = ExportOptions {
        format: Format::Mp4H264,
        width: p.settings.width,
        height: p.settings.height,
        fps: p.settings.fps,
        quality: 70,
        audio_kbps: 192,
        range: p.range,
    };
    let dir = p.media.first().and_then(|m| m.path.parent().map(Path::to_path_buf)).unwrap_or_default();
    let name = if p.name.trim().is_empty() { "Untitled" } else { p.name.trim() };
    let d = &mut app.export_dialog;
    d.out_path = Some(dir.join(format!("{}.{}", name, opts.format.extension())));
    d.opts = Some(opts);
    d.height = None;
    d.fps = None;
    d.error = None;
}

fn header(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(theme::heading("Export"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icons::button(ui, Icon::Close, "Close").clicked() {
                app.export_dialog.open = false;
            }
        });
    });
}

fn form(app: &mut App, ui: &mut egui::Ui) {
    let (pw, ph, pfps) = (app.project.settings.width, app.project.settings.height, app.project.settings.fps);
    let duration = app.project.duration();
    let range = app.project.range;
    let ffmpeg_ok = app.ffmpeg_ok;
    let d = &mut app.export_dialog;
    let (Some(opts), Some(out)) = (d.opts.as_mut(), d.out_path.as_mut()) else { return };
    let audio_only = opts.format.audio_only();

    egui::Grid::new("export_form").num_columns(2).spacing([theme::SP_3, theme::SP_2]).show(ui, |ui| {
        ui.label(theme::muted("Format"));
        egui::ComboBox::from_id_salt("format").selected_text(opts.format.name()).width(200.0).show_ui(ui, |ui| {
            for f in Format::ALL {
                if ui.selectable_label(opts.format == f, f.name()).clicked() && opts.format != f {
                    opts.format = f;
                    out.set_extension(f.extension());
                }
            }
        });
        ui.end_row();

        ui.label(theme::muted("Resolution"));
        ui.add_enabled_ui(!audio_only, |ui| {
            let label = match d.height {
                Some(h) => format!("{}p ({}x{})", h, scaled_width(pw, ph, h), h),
                None => format!("Project ({}x{})", pw, ph),
            };
            egui::ComboBox::from_id_salt("resolution").selected_text(label).width(200.0).show_ui(ui, |ui| {
                if ui.selectable_label(d.height.is_none(), format!("Project ({}x{})", pw, ph)).clicked() {
                    d.height = None;
                }
                for h in HEIGHTS {
                    if ui.selectable_label(d.height == Some(h), format!("{}p ({}x{})", h, scaled_width(pw, ph, h), h)).clicked() {
                        d.height = Some(h);
                    }
                }
            });
        });
        (opts.width, opts.height) = match d.height {
            Some(h) => (scaled_width(pw, ph, h), h),
            None => (pw, ph),
        };
        ui.end_row();

        ui.label(theme::muted("Frame rate"));
        ui.add_enabled_ui(!audio_only, |ui| {
            let label = d.fps.map_or(format!("Project ({})", fps_text(pfps)), fps_text);
            egui::ComboBox::from_id_salt("fps").selected_text(label).width(200.0).show_ui(ui, |ui| {
                if ui.selectable_label(d.fps.is_none(), format!("Project ({})", fps_text(pfps))).clicked() {
                    d.fps = None;
                }
                for f in FPS {
                    if ui.selectable_label(d.fps == Some(f), fps_text(f)).clicked() {
                        d.fps = Some(f);
                    }
                }
            });
        });
        opts.fps = d.fps.unwrap_or(pfps);
        ui.end_row();

        ui.label(theme::muted("Quality"));
        ui.add_enabled_ui(opts.format != Format::Wav, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut opts.quality, 0..=100).show_value(false));
                ui.label(theme::small(quality_word(opts.quality)));
            });
        });
        ui.end_row();

        if opts.format.has_audio() && opts.format != Format::Wav {
            ui.label(theme::muted("Audio bitrate"));
            egui::ComboBox::from_id_salt("audio_kbps").selected_text(format!("{} kbps", opts.audio_kbps)).width(200.0).show_ui(ui, |ui| {
                for k in AUDIO_KBPS {
                    ui.selectable_value(&mut opts.audio_kbps, k, format!("{} kbps", k));
                }
            });
            ui.end_row();
        }

        ui.label(theme::muted("Range"));
        let label = if opts.range.is_some() { "In to out" } else { "Entire sequence" };
        egui::ComboBox::from_id_salt("range").selected_text(label).width(200.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut opts.range, None, "Entire sequence");
            ui.add_enabled_ui(range.is_some(), |ui| {
                if ui.selectable_label(opts.range.is_some(), "In to out").clicked() {
                    opts.range = range;
                }
            });
        });
        ui.end_row();

        ui.label(theme::muted("Output"));
        ui.horizontal(|ui| {
            ui.label(elide_left(out, 30));
            if ui.button("Browse").clicked() {
                let mut dialog = rfd::FileDialog::new().add_filter(opts.format.name(), &[opts.format.extension()]);
                if let Some(dir) = out.parent() {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(name) = out.file_name() {
                    dialog = dialog.set_file_name(name.to_string_lossy());
                }
                if let Some(mut path) = dialog.save_file() {
                    path.set_extension(opts.format.extension());
                    *out = path;
                }
            }
        });
        ui.end_row();
    });

    let span = opts.range.map_or(duration, |(a, b)| (b - a).max(0.0));
    ui.add_space(theme::SP_2);
    ui.label(
        theme::mono(format!("{}  ~{}", theme::timecode(span, opts.fps), size_text(estimate_bytes(opts, span)))).color(theme::TEXT_MUTED),
    );
    ui.add_space(theme::SP_3);

    let blocker = if !ffmpeg_ok {
        Some("ffmpeg was not found")
    } else if duration <= 0.0 {
        Some("The timeline is empty")
    } else {
        None
    };
    if let Some(reason) = blocker {
        ui.label(egui::RichText::new(reason).color(theme::DANGER));
    }
    if let Some(e) = &d.error {
        ui.label(egui::RichText::new(e).color(theme::DANGER));
    }
    let clicked = ui
        .add_enabled_ui(blocker.is_none(), |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| theme::primary_button(ui, "Export").clicked()).inner
        })
        .inner;
    if clicked {
        let (opts, out) = (opts.clone(), out.clone());
        match ExportJob::start(&app.project, &opts, out) {
            Ok(job) => {
                app.export_job = Some(job);
                app.export_dialog.error = None;
                app.export_dialog.result = None;
            }
            Err(e) => app.export_dialog.error = Some(e),
        }
    }
}

fn progress_view(app: &mut App, ui: &mut egui::Ui, progress: f32, eta: Option<f64>, speed: Option<f32>) {
    ui.label(theme::mono(format!("{:.0}%", progress * 100.0)).size(32.0).color(theme::TEXT));
    ui.add_space(theme::SP_2);
    ui.add(egui::ProgressBar::new(progress).fill(theme::ACCENT).desired_width(ui.available_width()).animate(false));
    let eta = eta.map_or("--:--:--".to_string(), hms);
    let speed = speed.map_or(String::new(), |s| format!("  {:.1}x", s));
    ui.add_space(theme::SP_2);
    ui.label(theme::mono(format!("ETA {}{}", eta, speed)).color(theme::TEXT_MUTED));
    ui.add_space(theme::SP_3);
    if ui.button("Cancel").clicked() {
        if let Some(job) = app.export_job.as_mut() {
            job.cancel();
        }
    }
}

fn result_view(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context, result: JobState) {
    let mut again = false;
    match &result {
        JobState::Done { path, secs } => {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(theme::ICON), egui::Sense::hover());
                icons::paint(ui.painter(), rect, Icon::Check, theme::SUCCESS);
                ui.label(theme::strong("Export finished"));
            });
            ui.label(file_name(path));
            ui.label(theme::mono(format!("Took {}", hms(*secs))).color(theme::TEXT_MUTED));
            ui.add_space(theme::SP_3);
            ui.horizontal(|ui| {
                if theme::primary_button(ui, "Open").clicked() {
                    open_file(path);
                }
                if ui.button("Show in folder").clicked() {
                    reveal(path);
                }
                again = ui.button("Export again").clicked();
            });
        }
        JobState::Failed(msg) => {
            ui.label(egui::RichText::new("Export failed").family(theme::semibold()).color(theme::DANGER));
            egui::Frame::new().fill(theme::BG_INPUT).corner_radius(theme::R_INNER).inner_margin(theme::SP_2).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(160.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.label(theme::mono(msg).color(theme::TEXT));
                });
            });
            ui.add_space(theme::SP_2);
            ui.horizontal(|ui| {
                if ui.button("Copy error").clicked() {
                    ctx.copy_text(msg.clone());
                }
                again = ui.button("Export again").clicked();
            });
        }
        _ => {
            ui.label(theme::muted("Export cancelled"));
            ui.add_space(theme::SP_2);
            again = ui.button("Export again").clicked();
        }
    }
    if again {
        app.export_dialog.result = None;
        app.export_dialog.error = None;
        if app.export_dialog.opts.is_none() {
            init(app);
        }
    }
}

fn scaled_width(pw: u32, ph: u32, h: u32) -> u32 {
    ((h as f64 * pw as f64 / ph.max(1) as f64 / 2.0).round() as u32 * 2).max(2)
}

fn fps_text(f: f64) -> String {
    format!("{} fps", f)
}

fn quality_word(q: u8) -> &'static str {
    match q {
        0..=29 => "Small file",
        30..=64 => "Balanced",
        65..=89 => "High",
        _ => "Visually lossless",
    }
}

fn estimate_bytes(o: &ExportOptions, secs: f64) -> f64 {
    let q = o.quality as f64 / 100.0;
    let bpp = match o.format {
        Format::Mp4H264 => 0.02 + q * 0.2,
        Format::Mp4H265 => 0.012 + q * 0.12,
        Format::WebmVp9 => 0.014 + q * 0.14,
        Format::MovProRes => 2.4,
        Format::Gif => 0.3 + q * 0.4,
        Format::Mp3 | Format::Wav => 0.0,
    };
    let video = if o.format.audio_only() { 0.0 } else { o.width as f64 * o.height as f64 * o.fps * bpp / 8.0 };
    let audio = match o.format {
        Format::Wav => 192_000.0,
        f if f.has_audio() => o.audio_kbps as f64 * 125.0,
        _ => 0.0,
    };
    (video + audio) * secs
}

fn size_text(b: f64) -> String {
    match b {
        b if b >= 1e9 => format!("{:.1} GB", b / 1e9),
        b if b >= 1e6 => format!("{:.0} MB", b / 1e6),
        b => format!("{:.0} KB", (b / 1e3).max(1.0)),
    }
}

fn hms(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn elide_left(p: &Path, max: usize) -> String {
    let s = p.to_string_lossy();
    let n = s.chars().count();
    if n <= max {
        return s.into_owned();
    }
    format!("...{}", s.chars().skip(n - max).collect::<String>())
}

fn reveal(path: &Path) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    #[cfg(not(windows))]
    {
        let dir = path.parent().unwrap_or(path);
        let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(dir).spawn();
    }
}

fn open_file(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer").arg(path).creation_flags(0x0800_0000).spawn();
    }
    #[cfg(not(windows))]
    let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(path).spawn();
}
