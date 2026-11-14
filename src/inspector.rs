use crate::app::App;
use crate::model::*;
use crate::{edit_ops, icons, theme};
use eframe::egui::{self, Align, CornerRadius, FontId, Layout, Sense, Stroke, StrokeKind, Ui};
use std::ops::RangeInclusive;

const LABEL_W: f32 = 92.0;

const RESOLUTIONS: [(&str, u32, u32); 6] = [
    ("3840 x 2160", 3840, 2160),
    ("2560 x 1440", 2560, 1440),
    ("1920 x 1080", 1920, 1080),
    ("1280 x 720", 1280, 720),
    ("1080 x 1920 vertical", 1080, 1920),
    ("1080 x 1080 square", 1080, 1080),
];

const FRAME_RATES: [f64; 8] = [23.976, 24.0, 25.0, 29.97, 30.0, 50.0, 59.94, 60.0];

enum Op {
    Speed(f64),
    Start(f64),
    End(f64),
    Transition(Option<TransitionKind>, f64),
}

#[derive(Default)]
struct Ed {
    ctl: f32,
    changed: bool,
    ck: bool,
    audio: bool,
    pending: bool,
    ops: Vec<Op>,
}

impl Ed {
    fn hit(&mut self) {
        self.changed = true;
        self.ck = true;
        self.audio = true;
    }

    fn note(&mut self, r: &egui::Response) {
        if r.drag_started() {
            self.pending = true;
        }
        if r.changed() {
            self.changed = true;
            if self.pending {
                self.pending = false;
                self.ck = true;
            } else if !r.dragged() {
                self.ck = true;
            }
            if !r.dragged() {
                self.audio = true;
            }
        }
        if r.drag_stopped() {
            self.audio = true;
        }
    }

    fn note_arm(&mut self, r: &egui::Response) {
        if r.clicked() || r.gained_focus() {
            self.pending = true;
        }
        if r.changed() {
            self.changed = true;
            if self.pending {
                self.pending = false;
                self.ck = true;
            }
        }
    }

    fn num(&mut self, ui: &mut Ui, v: &mut f64, range: RangeInclusive<f64>, speed: f64, suffix: &str, dec: usize) -> bool {
        let r = ui
            .scope(|ui| {
                ui.style_mut().override_font_id = Some(FontId::monospace(theme::SIZE_MONO));
                ui.spacing_mut().interact_size.x = 68.0;
                ui.add(egui::DragValue::new(v).range(range).speed(speed).suffix(suffix).fixed_decimals(dec).update_while_editing(false))
            })
            .inner;
        self.note(&r);
        r.changed()
    }

    fn val(&mut self, ui: &mut Ui, v: &mut f32, range: RangeInclusive<f32>, speed: f64, suffix: &str, dec: usize) {
        let mut d = *v as f64;
        if self.num(ui, &mut d, *range.start() as f64..=*range.end() as f64, speed, suffix, dec) {
            *v = d as f32;
        }
    }

    fn pct(&mut self, ui: &mut Ui, v: &mut f32, lo: f32, hi: f32) {
        let mut d = *v as f64 * 100.0;
        if self.num(ui, &mut d, lo as f64 * 100.0..=hi as f64 * 100.0, 0.5, " %", 1) {
            *v = (d / 100.0) as f32;
        }
    }

    fn slider(&mut self, ui: &mut Ui, v: &mut f32, range: RangeInclusive<f32>, suffix: &str, dec: usize) {
        let w = self.ctl;
        let r = ui
            .scope(|ui| {
                ui.style_mut().override_font_id = Some(FontId::monospace(theme::SIZE_MONO));
                ui.spacing_mut().slider_width = (w - 76.0).max(40.0);
                ui.add(egui::Slider::new(v, range).suffix(suffix).fixed_decimals(dec).update_while_editing(false))
            })
            .inner;
        self.note(&r);
    }

    fn check(&mut self, ui: &mut Ui, v: &mut bool) {
        let r = ui.checkbox(v, "");
        self.note(&r);
    }

    fn flag(&mut self, ui: &mut Ui, v: &mut bool, text: &str) {
        let mut r = ui.add(egui::Button::new(text).selected(*v));
        if r.clicked() {
            *v = !*v;
            r.mark_changed();
        }
        self.note(&r);
    }

    fn color(&mut self, ui: &mut Ui, c: &mut [u8; 4]) {
        let r = ui.color_edit_button_srgba_unmultiplied(c);
        self.note_arm(&r);
    }

    fn color3(&mut self, ui: &mut Ui, c: &mut [u8; 3]) {
        let r = ui.color_edit_button_srgb(c);
        self.note_arm(&r);
    }

    fn text(&mut self, ui: &mut Ui, s: &mut String, rows: usize) {
        let edit = if rows > 1 { egui::TextEdit::multiline(s).desired_rows(rows) } else { egui::TextEdit::singleline(s) };
        let r = ui.add(edit.desired_width(f32::INFINITY));
        self.note_arm(&r);
    }
}

fn row(ui: &mut Ui, ctl: f32, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.label(theme::muted(label));
    ui.allocate_ui_with_layout(egui::vec2(ctl, 24.0), Layout::right_to_left(Align::Center), f);
    ui.end_row();
}

fn grid(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, f: impl FnOnce(&mut Ui)) {
    egui::Grid::new(id).num_columns(2).min_col_width(LABEL_W).spacing([theme::SP_2, theme::SP_1 + 2.0]).show(ui, f);
}

fn section(ui: &mut Ui, title: &str, open: bool, f: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(theme::heading(title)).default_open(open).show_unindented(ui, |ui| {
        ui.add_space(theme::SP_1);
        f(ui);
        ui.add_space(theme::SP_1);
    });
    ui.add_space(theme::SP_2);
}

fn reset_row(ui: &mut Ui, ctl: f32) -> bool {
    let mut hit = false;
    row(ui, ctl, "", |ui| hit = ui.add(egui::Button::new(theme::small("Reset")).frame(false)).clicked());
    hit
}

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.visuals_mut().indent_has_left_vline = false;
        let pid = egui::Id::unique("inspector_pending");
        let mut ed = Ed {
            ctl: (ui.available_width() - LABEL_W - theme::SP_2).max(60.0),
            pending: ui.data(|d| d.get_temp(pid)).unwrap_or(false),
            ..Default::default()
        };
        if app.selection.len() > 1 {
            multi_panel(app, ui);
        } else if let Some(id) = app.primary_clip() {
            clip_panel(app, ui, &mut ed, id);
        } else if let Some(m) = app.selected_media.and_then(|id| app.project.media(id).cloned()) {
            media_panel(app, ui, &ed, &m);
        } else {
            project_panel(app, ui, &mut ed);
        }
        ui.data_mut(|d| d.insert_temp(pid, ed.pending));
        if ed.audio {
            app.restart_audio();
        }
    });
}

fn multi_panel(app: &mut App, ui: &mut Ui) {
    ui.label(theme::heading(format!("{} clips selected", app.selection.len())));
    ui.add_space(theme::SP_2);
    ui.horizontal(|ui| {
        for (text, ripple) in [("Delete", false), ("Ripple delete", true)] {
            if ui.button(text).clicked() {
                let ids = app.selection.clone();
                app.checkpoint();
                edit_ops::delete(&mut app.project, &ids, ripple);
                app.selection.clear();
                app.restart_audio();
            }
        }
    });
}

fn media_panel(app: &App, ui: &mut Ui, ed: &Ed, m: &Media) {
    let ctl = ed.ctl;
    ui.label(theme::heading(m.file_name()));
    ui.add_space(theme::SP_2);
    grid(ui, "media_info", |ui| {
        if m.has_video {
            row(ui, ctl, "Resolution", |ui| {
                ui.label(theme::mono(format!("{} x {}", m.width, m.height)));
            });
            row(ui, ctl, "Frame rate", |ui| {
                ui.label(theme::mono(format!("{:.3} fps", m.fps)));
            });
        }
        row(ui, ctl, "Duration", |ui| {
            ui.label(theme::mono(theme::timecode(m.duration, app.project.settings.fps)));
        });
        row(ui, ctl, "Codec", |ui| {
            ui.label(m.codec.as_str());
        });
        row(ui, ctl, "Audio", |ui| {
            if m.has_audio {
                ui.label(theme::mono(format!("{} ch, {} Hz", m.audio_channels, m.sample_rate)));
            } else {
                ui.label(theme::muted("None"));
            }
        });
    });
    ui.add_space(theme::SP_2);
    ui.label(theme::small(m.path.display().to_string()));
}

fn project_panel(app: &mut App, ui: &mut Ui, ed: &mut Ed) {
    let ctl = ed.ctl;
    let mut s = app.project.settings.clone();
    ui.label(theme::heading("Project settings"));
    ui.add_space(theme::SP_2);
    grid(ui, "project", |ui| {
        let current = RESOLUTIONS.iter().find(|r| r.1 == s.width && r.2 == s.height).map_or("Custom", |r| r.0);
        row(ui, ctl, "Resolution", |ui| {
            egui::ComboBox::from_id_salt("res").selected_text(current).show_ui(ui, |ui| {
                for (name, w, h) in RESOLUTIONS {
                    if ui.selectable_label(current == name, name).clicked() {
                        s.width = w;
                        s.height = h;
                        ed.hit();
                    }
                }
            });
        });
        let mut w = s.width as f64;
        row(ui, ctl, "Width", |ui| {
            if ed.num(ui, &mut w, 16.0..=8192.0, 2.0, " px", 0) {
                s.width = w as u32 & !1;
            }
        });
        let mut h = s.height as f64;
        row(ui, ctl, "Height", |ui| {
            if ed.num(ui, &mut h, 16.0..=8192.0, 2.0, " px", 0) {
                s.height = h as u32 & !1;
            }
        });
        row(ui, ctl, "Frame rate", |ui| {
            egui::ComboBox::from_id_salt("fps").selected_text(format!("{} fps", s.fps)).show_ui(ui, |ui| {
                for f in FRAME_RATES {
                    if ui.selectable_label((s.fps - f).abs() < 0.005, format!("{f} fps")).clicked() {
                        s.fps = f;
                        ed.hit();
                    }
                }
            });
        });
        row(ui, ctl, "Background", |ui| {
            let mut c = [s.background[0], s.background[1], s.background[2]];
            ed.color3(ui, &mut c);
            s.background = [c[0], c[1], c[2], 255];
        });
    });
    if ed.changed && s != app.project.settings {
        if ed.ck {
            app.checkpoint();
        }
        app.dirty = true;
        app.project.settings = s;
    }
    ui.add_space(theme::SP_4);
    ui.label(theme::small("Select a clip to edit its properties"));
}

fn source_label(c: &Clip, media: Option<&Media>) -> String {
    match &c.source {
        ClipSource::Media(_) => media.map(|m| m.file_name()).unwrap_or_else(|| "Missing media".into()),
        ClipSource::Text(_) => "Title".into(),
        ClipSource::Color(_) => "Color matte".into(),
    }
}

fn clip_panel(app: &mut App, ui: &mut Ui, ed: &mut Ed, id: ClipId) {
    let Some((ti, ci)) = app.project.clip(id) else { return };
    let mut clip = app.project.tracks[ti].clips[ci].clone();
    let audio_track = app.project.tracks[ti].kind == TrackKind::Audio;
    let media = clip.media_id().and_then(|m| app.project.media(m).cloned());
    let has_prev = app.project.tracks[ti].clips.iter().any(|c| c.id != id && (c.end() - clip.start).abs() < 1e-3);
    let fps = app.project.settings.fps;
    let ctl = ed.ctl;
    let visual = !audio_track && media.as_ref().is_none_or(|m| m.has_video || m.kind == MediaKind::Image);
    let has_audio = audio_track || media.as_ref().is_some_and(|m| m.has_audio);

    ed.text(ui, &mut clip.name, 1);
    ui.horizontal(|ui| {
        let source = source_label(&clip, media.as_ref());
        if source != clip.name {
            ui.label(theme::small(source));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(theme::mono(theme::timecode(clip.duration(), fps)).color(theme::TEXT_MUTED));
        });
    });
    ui.add_space(theme::SP_3);

    section(ui, "Timing", false, |ui| {
        grid(ui, "timing", |ui| {
            let mut start = clip.start;
            row(ui, ctl, "Start", |ui| {
                if ed.num(ui, &mut start, 0.0..=1e6, 0.05, " s", 2) {
                    ed.ops.push(Op::Start(start));
                }
            });
            let mut dur = clip.duration();
            row(ui, ctl, "Duration", |ui| {
                if ed.num(ui, &mut dur, 0.04..=1e6, 0.05, " s", 2) {
                    ed.ops.push(Op::End(clip.start + dur));
                }
            });
            let mut speed = clip.speed;
            row(ui, ctl, "Speed", |ui| {
                if ed.num(ui, &mut speed, 0.1..=8.0, 0.01, " x", 2) {
                    ed.ops.push(Op::Speed(speed));
                }
            });
            row(ui, ctl, "Reverse", |ui| ed.check(ui, &mut clip.reverse));
            row(ui, ctl, "Label", |ui| {
                ui.spacing_mut().item_spacing.x = theme::SP_1;
                for i in (0..theme::CLIP_LABELS.len() as u8).rev() {
                    let (rect, r) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::click());
                    ui.painter().rect_filled(rect, CornerRadius::same(theme::R_INNER), theme::CLIP_LABELS[i as usize]);
                    if clip.label == i {
                        ui.painter().rect_stroke(
                            rect.expand(2.0),
                            CornerRadius::same(theme::R_INNER + 2),
                            Stroke::new(1.5, theme::ACCENT),
                            StrokeKind::Outside,
                        );
                    }
                    if r.clicked() && clip.label != i {
                        clip.label = i;
                        ed.hit();
                    }
                }
            });
        });
    });

    if visual {
        section(ui, "Transform", true, |ui| {
            grid(ui, "transform", |ui| {
                let t = &mut clip.transform;
                row(ui, ctl, "Position X", |ui| ed.pct(ui, &mut t.x, -1.0, 2.0));
                row(ui, ctl, "Position Y", |ui| ed.pct(ui, &mut t.y, -1.0, 2.0));
                row(ui, ctl, "Scale", |ui| ed.pct(ui, &mut t.scale, 0.01, 10.0));
                row(ui, ctl, "Rotation", |ui| ed.val(ui, &mut t.rotation, -360.0..=360.0, 0.5, "\u{b0}", 1));
                row(ui, ctl, "Flip", |ui| {
                    ed.flag(ui, &mut t.flip_v, "Vertical");
                    ed.flag(ui, &mut t.flip_h, "Horizontal");
                });
                if reset_row(ui, ctl) {
                    *t = Transform::default();
                    ed.hit();
                }
            });
        });

        section(ui, "Crop", false, |ui| {
            grid(ui, "crop", |ui| {
                let c = &mut clip.crop;
                row(ui, ctl, "Left", |ui| ed.pct(ui, &mut c.left, 0.0, 0.95));
                row(ui, ctl, "Top", |ui| ed.pct(ui, &mut c.top, 0.0, 0.95));
                row(ui, ctl, "Right", |ui| ed.pct(ui, &mut c.right, 0.0, 0.95));
                row(ui, ctl, "Bottom", |ui| ed.pct(ui, &mut c.bottom, 0.0, 0.95));
                if reset_row(ui, ctl) {
                    *c = Crop::default();
                    ed.hit();
                }
            });
        });
    }

    let dur = clip.duration() as f32;
    section(ui, if visual { "Opacity and fades" } else { "Fades" }, false, |ui| {
        grid(ui, "fades", |ui| {
            if visual {
                row(ui, ctl, "Opacity", |ui| ed.pct(ui, &mut clip.opacity, 0.0, 1.0));
            }
            let mut fi = clip.fade_in as f32;
            row(ui, ctl, "Fade in", |ui| ed.val(ui, &mut fi, 0.0..=dur, 0.02, " s", 2));
            clip.fade_in = fi as f64;
            let mut fo = clip.fade_out as f32;
            row(ui, ctl, "Fade out", |ui| ed.val(ui, &mut fo, 0.0..=dur, 0.02, " s", 2));
            clip.fade_out = fo as f64;
        });
    });

    if has_audio {
        section(ui, "Audio", false, |ui| {
            grid(ui, "audio", |ui| {
                let mut db = if clip.volume <= 0.0 { -60.0 } else { (20.0 * clip.volume.log10()).max(-60.0) } as f64;
                row(ui, ctl, "Volume", |ui| {
                    let r = ui
                        .scope(|ui| {
                            ui.style_mut().override_font_id = Some(FontId::monospace(theme::SIZE_MONO));
                            ui.spacing_mut().interact_size.x = 68.0;
                            ui.add(
                                egui::DragValue::new(&mut db)
                                    .range(-60.0..=12.0)
                                    .speed(0.2)
                                    .suffix(" dB")
                                    .update_while_editing(false)
                                    .custom_formatter(|v, _| if v <= -60.0 { "-inf".into() } else { format!("{v:.1}") }),
                            )
                        })
                        .inner;
                    ed.note(&r);
                    if r.changed() {
                        clip.volume = if db <= -60.0 { 0.0 } else { 10f32.powf(db as f32 / 20.0) };
                    }
                });
                row(ui, ctl, "Mute", |ui| ed.check(ui, &mut clip.audio_muted));
            });
        });
    }

    match &mut clip.source {
        ClipSource::Text(s) => section(ui, "Text", true, |ui| {
            ed.text(ui, &mut s.text, 3);
            ui.add_space(theme::SP_2);
            grid(ui, "text", |ui| {
                row(ui, ctl, "Size", |ui| ed.val(ui, &mut s.size, 8.0..=400.0, 0.5, " px", 0));
                row(ui, ctl, "Color", |ui| ed.color(ui, &mut s.color));
                let mut bg = s.background.is_some();
                row(ui, ctl, "Background", |ui| {
                    ed.check(ui, &mut bg);
                    if let Some(c) = &mut s.background {
                        ed.color(ui, c);
                    }
                });
                if bg != s.background.is_some() {
                    s.background = bg.then_some([0, 0, 0, 160]);
                }
                row(ui, ctl, "Bold", |ui| ed.check(ui, &mut s.bold));
                row(ui, ctl, "Align", |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (a, name) in [(TextAlign::Right, "Right"), (TextAlign::Center, "Center"), (TextAlign::Left, "Left")] {
                        let mut r = ui.add(egui::Button::new(name).selected(s.align == a));
                        if r.clicked() && s.align != a {
                            s.align = a;
                            r.mark_changed();
                        }
                        ed.note(&r);
                    }
                });
            });
        }),
        ClipSource::Color(c) => section(ui, "Color", true, |ui| {
            grid(ui, "color", |ui| row(ui, ctl, "Matte", |ui| ed.color(ui, c)));
        }),
        ClipSource::Media(_) => {}
    }

    if visual && has_prev {
        section(ui, "Transition", false, |ui| {
            grid(ui, "transition", |ui| {
                let cur = clip.transition_in;
                let mut kind = cur.map(|t| t.kind);
                let mut tdur = cur.map_or(1.0, |t| t.duration);
                row(ui, ctl, "Type", |ui| {
                    egui::ComboBox::from_id_salt("transition_kind").selected_text(kind.map_or("None", |k| k.name())).show_ui(ui, |ui| {
                        ui.selectable_value(&mut kind, None, "None");
                        for k in TransitionKind::ALL {
                            ui.selectable_value(&mut kind, Some(k), k.name());
                        }
                    });
                });
                if kind != cur.map(|t| t.kind) {
                    ed.ops.push(Op::Transition(kind, tdur));
                    ed.hit();
                }
                if kind.is_some() {
                    row(ui, ctl, "Duration", |ui| {
                        if ed.num(ui, &mut tdur, 0.05..=10.0, 0.02, " s", 2) {
                            ed.ops.push(Op::Transition(kind, tdur));
                        }
                    });
                }
            });
        });
    }

    if visual {
        section(ui, "Effects", true, |ui| effects(ed, ui, &mut clip.effects));
    }

    if ed.changed {
        if ed.ck {
            app.checkpoint();
        }
        app.dirty = true;
        app.project.tracks[ti].clips[ci] = clip;
        for op in std::mem::take(&mut ed.ops) {
            match op {
                Op::Speed(s) => edit_ops::set_speed(&mut app.project, id, s),
                Op::Start(t) => {
                    edit_ops::move_clip(&mut app.project, id, t, ti);
                }
                Op::End(t) => edit_ops::trim_end(&mut app.project, id, t),
                Op::Transition(k, d) => edit_ops::set_transition(&mut app.project, id, k, d),
            }
        }
    }
}

fn effects(ed: &mut Ed, ui: &mut Ui, list: &mut Vec<Effect>) {
    if list.is_empty() {
        ui.label(theme::small("No effects. Add one below."));
        ui.add_space(theme::SP_1);
    }
    let n = list.len();
    let mut act: Option<(usize, bool)> = None;
    for (i, fx) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let (icon, tip) = if fx.enabled { (icons::Icon::Eye, "Disable") } else { (icons::Icon::EyeOff, "Enable") };
            if icons::toggle(ui, icon, fx.enabled, tip).clicked() {
                fx.enabled = !fx.enabled;
                ed.hit();
            }
            let name = fx.kind.name();
            ui.label(if fx.enabled { theme::strong(name) } else { theme::muted(name) });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icons::button(ui, icons::Icon::Trash, "Remove").clicked() {
                    act = Some((i, false));
                }
                if i + 1 < n && icons::button(ui, icons::Icon::ChevronDown, "Move down").clicked() {
                    act = Some((i, true));
                }
            });
        });
        if fx.enabled {
            grid(ui, ("fx", i), |ui| params(ed, ui, &mut fx.kind));
        }
        ui.add_space(theme::SP_2);
    }
    match act {
        Some((i, true)) => list.swap(i, i + 1),
        Some((i, false)) => {
            list.remove(i);
        }
        None => {}
    }
    if act.is_some() {
        ed.hit();
    }
    ui.menu_button("Add effect", |ui| {
        for kind in EffectKind::catalog() {
            if ui.button(kind.name()).clicked() {
                list.push(Effect { enabled: true, kind });
                ed.hit();
                ui.close();
            }
        }
    });
}

fn region_rows(ed: &mut Ed, ui: &mut Ui, region: &mut Option<Rect01>) {
    let ctl = ed.ctl;
    let mut whole = region.is_none();
    row(ui, ctl, "Whole frame", |ui| ed.check(ui, &mut whole));
    if whole != region.is_none() {
        *region = if whole { None } else { Some(Rect01 { x: 0.35, y: 0.3, w: 0.3, h: 0.4 }) };
    }
    if let Some(r) = region {
        row(ui, ctl, "X", |ui| ed.pct(ui, &mut r.x, 0.0, 1.0));
        row(ui, ctl, "Y", |ui| ed.pct(ui, &mut r.y, 0.0, 1.0));
        row(ui, ctl, "Width", |ui| ed.pct(ui, &mut r.w, 0.01, 1.0));
        row(ui, ctl, "Height", |ui| ed.pct(ui, &mut r.h, 0.01, 1.0));
    }
}

fn params(ed: &mut Ed, ui: &mut Ui, kind: &mut EffectKind) {
    let ctl = ed.ctl;
    match kind {
        EffectKind::Blur { radius, region } => {
            row(ui, ctl, "Radius", |ui| ed.slider(ui, radius, 0.0..=300.0, " px", 0));
            region_rows(ed, ui, region);
        }
        EffectKind::Pixelate { block, region } => {
            row(ui, ctl, "Block", |ui| ed.slider(ui, block, 2.0..=200.0, " px", 0));
            region_rows(ed, ui, region);
        }
        EffectKind::ColorAdjust { brightness, contrast, saturation, gamma } => {
            row(ui, ctl, "Brightness", |ui| ed.slider(ui, brightness, -1.0..=1.0, "", 2));
            row(ui, ctl, "Contrast", |ui| ed.slider(ui, contrast, 0.0..=2.0, "", 2));
            row(ui, ctl, "Saturation", |ui| ed.slider(ui, saturation, 0.0..=3.0, "", 2));
            row(ui, ctl, "Gamma", |ui| ed.slider(ui, gamma, 0.1..=3.0, "", 2));
        }
        EffectKind::WhiteBalance { temperature, tint } => {
            row(ui, ctl, "Temperature", |ui| ed.slider(ui, temperature, -1.0..=1.0, "", 2));
            row(ui, ctl, "Tint", |ui| ed.slider(ui, tint, -1.0..=1.0, "", 2));
        }
        EffectKind::HueShift { degrees } => row(ui, ctl, "Shift", |ui| ed.slider(ui, degrees, -180.0..=180.0, "\u{b0}", 0)),
        EffectKind::Sepia { amount } => row(ui, ctl, "Amount", |ui| ed.slider(ui, amount, 0.0..=1.0, "", 2)),
        EffectKind::Vignette { strength } => row(ui, ctl, "Strength", |ui| ed.slider(ui, strength, 0.0..=1.0, "", 2)),
        EffectKind::Sharpen { amount } => row(ui, ctl, "Amount", |ui| ed.slider(ui, amount, 0.0..=3.0, "", 2)),
        EffectKind::Noise { amount } => row(ui, ctl, "Amount", |ui| ed.slider(ui, amount, 0.0..=1.0, "", 2)),
        EffectKind::ChromaKey { color, similarity, smoothness } => {
            row(ui, ctl, "Key color", |ui| ed.color3(ui, color));
            row(ui, ctl, "Similarity", |ui| ed.slider(ui, similarity, 0.01..=1.0, "", 2));
            row(ui, ctl, "Smoothness", |ui| ed.slider(ui, smoothness, 0.0..=1.0, "", 2));
        }
        EffectKind::Grayscale | EffectKind::Invert => {}
    }
}
