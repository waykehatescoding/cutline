use crate::app::{App, DragPayload, Tool};
use crate::edit_ops;
use crate::icons::{self, Icon};
use crate::model::*;
use crate::theme;
use crate::thumbs::Thumbs;
use crate::waveform::Waveforms;
use eframe::egui::{
    self, epaint, pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Painter, PointerButton, Pos2, Rect,
    Sense, Shape, Stroke, StrokeKind, Ui,
};

const HEADER_W: f32 = 150.0;
const RULER_H: f32 = 26.0;
const BAR_H: f32 = 12.0;
const GROUP_GAP: f32 = 6.0;
const NAME_H: f32 = 16.0;
const EDGE: f32 = 6.0;
const SNAP_PX: f32 = 8.0;
const MIN_PPS: f32 = 2.0;
const MAX_PPS: f32 = 1500.0;
const TICK_STEPS: [f64; 15] = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Body,
    Left,
    Right,
    FadeIn,
    FadeOut,
}

#[derive(Clone)]
enum Drag {
    Move { orig: Vec<(ClipId, f64, usize)>, primary: ClipId, grab: f64, pts: Vec<f64>, snap: Option<f64> },
    Trim { id: ClipId, end: bool, off: f64, pts: Vec<f64>, snap: Option<f64> },
    Fade { id: ClipId, out: bool },
    Slip { id: ClipId, last: f64 },
    Marquee { origin: Pos2, base: Vec<ClipId> },
}

#[derive(Clone, Copy, Default)]
struct Target {
    clip: Option<ClipId>,
    track: Option<usize>,
    t: f64,
}

struct Geo {
    lane: Rect,
    pps: f32,
    sx: f32,
    sy: f32,
    rows: Vec<(f32, f32)>,
}

impl Geo {
    fn x(&self, t: f64) -> f32 {
        self.lane.left() + (t * self.pps as f64 - self.sx as f64) as f32
    }

    fn t(&self, x: f32) -> f64 {
        ((x - self.lane.left()) as f64 + self.sx as f64) / self.pps as f64
    }

    fn row(&self, i: usize) -> Rect {
        let (y, h) = self.rows[i];
        Rect::from_min_size(pos2(self.lane.left(), self.lane.top() - self.sy + y), vec2(self.lane.width(), h))
    }

    fn track_at(&self, y: f32) -> Option<usize> {
        (0..self.rows.len()).find(|&i| {
            let r = self.row(i);
            y >= r.top() && y < r.bottom()
        })
    }

    fn track_near(&self, y: f32) -> usize {
        (0..self.rows.len()).min_by(|&a, &b| (self.row(a).center().y - y).abs().total_cmp(&(self.row(b).center().y - y).abs())).unwrap_or(0)
    }

    fn clip_rect(&self, i: usize, c: &Clip) -> Rect {
        let r = self.row(i);
        Rect::from_min_max(pos2(self.x(c.start), r.top() + 2.0), pos2(self.x(c.end()), r.bottom() - 2.0))
    }

    fn fade_handles(&self, c: &Clip, r: Rect) -> (Pos2, Pos2) {
        let y = r.top() + NAME_H + 4.0;
        (pos2(r.left() + (c.fade_in as f32 * self.pps).max(9.0), y), pos2(r.right() - (c.fade_out as f32 * self.pps).max(9.0), y))
    }
}

fn rows(p: &Project, th: f32) -> (Vec<(f32, f32)>, f32) {
    let mut y = 0.0;
    let mut prev = None;
    let mut out = Vec::with_capacity(p.tracks.len());
    for t in &p.tracks {
        if prev.is_some_and(|k| k != t.kind) {
            y += GROUP_GAP;
        }
        let h = if t.kind == TrackKind::Video { th } else { (th * 0.8).round() };
        out.push((y, h));
        y += h;
        prev = Some(t.kind);
    }
    (out, y)
}

fn radius() -> CornerRadius {
    CornerRadius::same(theme::R_INNER)
}

fn shade(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * f) as u8, (c.g() as f32 * f) as u8, (c.b() as f32 * f) as u8)
}

fn clip_color(c: &Clip, kind: TrackKind) -> Color32 {
    if c.label != 0 {
        return theme::CLIP_LABELS[c.label as usize % theme::CLIP_LABELS.len()];
    }
    match &c.source {
        ClipSource::Text(_) => theme::CLIP_TEXT,
        ClipSource::Color(k) => Color32::from_rgb(k[0], k[1], k[2]).lerp_to_gamma(theme::CLIP_MATTE, 0.4),
        ClipSource::Media(_) if kind == TrackKind::Audio => theme::CLIP_AUDIO,
        ClipSource::Media(_) => theme::CLIP_VIDEO,
    }
}

fn zoom(app: &mut App, new: f32, anchor: f32) {
    let v = &mut app.view;
    let new = new.clamp(MIN_PPS, MAX_PPS);
    let t = (v.scroll_x + anchor) / v.px_per_sec;
    v.px_per_sec = new;
    v.scroll_x = (t * new - anchor).max(0.0);
}

fn item(ui: &mut Ui, label: &str) -> bool {
    let hit = ui.button(label).clicked();
    if hit {
        ui.close();
    }
    hit
}

fn toolbar(app: &mut App, ui: &mut Ui, lane_w: f32) {
    let fps = app.project.settings.fps;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::SP_1;
        ui.label(theme::mono(theme::timecode(app.playhead, fps)).size(16.0).color(theme::TEXT));
        ui.add_space(theme::SP_3);
        for (tool, icon, tip) in
            [(Tool::Select, Icon::Select, "Select"), (Tool::Razor, Icon::Razor, "Razor"), (Tool::Slip, Icon::Slip, "Slip")]
        {
            if icons::toggle(ui, icon, app.tool == tool, tip).clicked() {
                app.tool = tool;
            }
        }
        ui.add_space(theme::SP_2);
        if icons::toggle(ui, Icon::Magnet, app.view.snap, "Snapping").clicked() {
            app.view.snap = !app.view.snap;
        }
        if icons::toggle(ui, Icon::Ripple, app.view.ripple, "Ripple edit").clicked() {
            app.view.ripple = !app.view.ripple;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let r = icons::button(ui, Icon::Plus, "Add track");
            egui::Popup::menu(&r).show(|ui| {
                if item(ui, "Add video track") {
                    app.checkpoint();
                    edit_ops::add_track(&mut app.project, TrackKind::Video);
                }
                if item(ui, "Add audio track") {
                    app.checkpoint();
                    edit_ops::add_track(&mut app.project, TrackKind::Audio);
                }
            });
            if icons::button(ui, Icon::Fit, "Fit project").clicked() {
                let dur = app.project.duration().max(1.0) as f32;
                app.view.px_per_sec = ((lane_w - 40.0) / dur).clamp(MIN_PPS, MAX_PPS);
                app.view.scroll_x = 0.0;
            }
            if icons::button(ui, Icon::ZoomIn, "Zoom in").clicked() {
                zoom(app, app.view.px_per_sec * 1.4, lane_w * 0.5);
            }
            let mut pps = app.view.px_per_sec;
            ui.spacing_mut().slider_width = 110.0;
            if ui.add(egui::Slider::new(&mut pps, MIN_PPS..=MAX_PPS).logarithmic(true).show_value(false)).changed() {
                zoom(app, pps, lane_w * 0.5);
            }
            if icons::button(ui, Icon::ZoomOut, "Zoom out").clicked() {
                zoom(app, app.view.px_per_sec / 1.4, lane_w * 0.5);
            }
        });
    });
}

fn hit(app: &App, g: &Geo, pos: Pos2) -> Option<(usize, ClipId, Part)> {
    let ti = g.track_at(pos.y)?;
    for c in &app.project.tracks[ti].clips {
        let r = g.clip_rect(ti, c);
        if app.selection.contains(&c.id) && r.width() > 40.0 {
            let (a, b) = g.fade_handles(c, r);
            if a.distance(pos) < 7.0 {
                return Some((ti, c.id, Part::FadeIn));
            }
            if b.distance(pos) < 7.0 {
                return Some((ti, c.id, Part::FadeOut));
            }
        }
        if r.contains(pos) {
            let part = if r.width() <= 16.0 {
                Part::Body
            } else if pos.x - r.left() < EDGE {
                Part::Left
            } else if r.right() - pos.x < EDGE {
                Part::Right
            } else {
                Part::Body
            };
            return Some((ti, c.id, part));
        }
    }
    None
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let area_w = ui.available_width();
    toolbar(app, ui, area_w - HEADER_W);
    ui.add_space(theme::SP_1);
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, egui::Sense::hover());
    if area.width() < HEADER_W + 60.0 || area.height() < RULER_H + BAR_H + 24.0 {
        return;
    }
    let lane = Rect::from_min_max(pos2(area.left() + HEADER_W, area.top() + RULER_H), pos2(area.right(), area.bottom() - BAR_H));
    let ruler = Rect::from_min_max(pos2(lane.left(), area.top()), pos2(lane.right(), lane.top()));
    let hdr = Rect::from_min_max(pos2(area.left(), lane.top()), pos2(lane.left(), lane.bottom()));
    let bar = Rect::from_min_max(pos2(lane.left(), lane.bottom()), pos2(lane.right(), area.bottom()));
    let (rows, total_h) = rows(&app.project, app.view.track_height);
    let ctx = ui.ctx().clone();
    let mods = ui.input(|i| i.modifiers);

    if let Some(hp) = ui.input(|i| i.pointer.hover_pos()).filter(|p| area.contains(*p)) {
        let (zd, sd) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
        if zd != 1.0 {
            zoom(app, app.view.px_per_sec * zd, (hp.x - lane.left()).max(0.0));
        } else if mods.shift {
            app.view.scroll_x -= sd.x + sd.y;
        } else {
            app.view.scroll_x -= sd.x;
            app.view.scroll_y -= sd.y;
        }
    }
    let dur = app.project.duration();
    let content_w = (dur as f32 * app.view.px_per_sec + lane.width() * 0.5).max(lane.width());
    if app.playing && app.view.follow && !ui.input(|i| i.pointer.any_down()) {
        let x = app.playhead as f32 * app.view.px_per_sec - app.view.scroll_x;
        if x > lane.width() - 30.0 || x < 0.0 {
            app.view.scroll_x = app.playhead as f32 * app.view.px_per_sec - lane.width() * 0.1;
        }
    }
    app.view.scroll_x = app.view.scroll_x.clamp(0.0, content_w - lane.width());
    app.view.scroll_y = app.view.scroll_y.clamp(0.0, (total_h - lane.height()).max(0.0));
    let g = Geo { lane, pps: app.view.px_per_sec, sx: app.view.scroll_x, sy: app.view.scroll_y, rows };
    let fps = app.project.settings.fps;
    let painter = ui.painter_at(area);
    let lanes = painter.with_clip_rect(lane);

    let bar_resp = ui.interact(bar, Id::unique("tl_bar"), Sense::click_and_drag());
    if content_w > lane.width() {
        let tw = (lane.width() * lane.width() / content_w).max(32.0);
        let tx = bar.left() + g.sx / (content_w - lane.width()) * (lane.width() - tw);
        if bar_resp.dragged() {
            app.view.scroll_x += bar_resp.drag_delta().x * (content_w - lane.width()) / (lane.width() - tw);
        }
        let col = if bar_resp.hovered() || bar_resp.dragged() { theme::BORDER_STRONG } else { theme::BG_ACTIVE };
        painter.rect_filled(Rect::from_min_size(pos2(tx, bar.top() + 3.0), vec2(tw, BAR_H - 6.0)), 3, col);
    }

    let ruler_resp = ui.interact(ruler, Id::unique("tl_ruler"), Sense::click_and_drag());
    let tid = Id::unique("tl_target");
    if ruler_resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down()) {
        if let Some(p) = ruler_resp.interact_pointer_pos().or(ruler_resp.hover_pos()) {
            app.seek(g.t(p.x.clamp(lane.left(), lane.right())));
        }
    }
    if ruler_resp.secondary_clicked() {
        if let Some(p) = ruler_resp.interact_pointer_pos() {
            ctx.data_mut(|d| d.insert_temp(tid, Target { clip: None, track: None, t: g.t(p.x) }));
        }
    }
    ruler_resp.context_menu(|ui| {
        let t = ui.data(|d| d.get_temp::<Target>(tid)).unwrap_or_default().t;
        if item(ui, "Add marker here") {
            app.checkpoint();
            edit_ops::add_marker(&mut app.project, t);
        }
    });

    let resp = ui.interact(lane, Id::unique("tl_lanes"), Sense::click_and_drag());
    let did = Id::unique("tl_drag");
    let ptr = ui.input(|i| i.pointer.latest_pos());
    let mut drag: Option<Drag> = ctx.data(|d| d.get_temp(did));
    let tol = (SNAP_PX / g.pps) as f64;

    if resp.hovered() && ui.input(|i| i.pointer.primary_pressed()) && app.tool != Tool::Razor {
        match ptr.and_then(|p| hit(app, &g, p)) {
            Some((_, id, _)) => {
                if mods.command {
                    match app.selection.iter().position(|s| *s == id) {
                        Some(i) => {
                            app.selection.remove(i);
                        }
                        None => app.selection.push(id),
                    }
                } else if mods.shift {
                    if !app.selection.contains(&id) {
                        app.selection.push(id);
                    }
                } else if !app.selection.contains(&id) {
                    app.selection = vec![id];
                }
            }
            None => {
                if !mods.shift && !mods.command {
                    app.selection.clear();
                }
            }
        }
    }

    if resp.drag_started_by(PointerButton::Primary) {
        let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(lane.center());
        let h = hit(app, &g, origin);
        drag = None;
        let free = |app: &App, ti: usize| !app.project.tracks[ti].locked;
        match (app.tool, h) {
            (Tool::Select, Some((ti, id, part))) if free(app, ti) && !mods.command => {
                app.checkpoint();
                let c = app.project.clip_ref(id).cloned();
                if let Some(c) = c {
                    drag = Some(match part {
                        Part::Body => {
                            let mut ids = app.selection.clone();
                            let mut primary = id;
                            if mods.alt {
                                let old = ids.clone();
                                ids = edit_ops::duplicate(&mut app.project, &old);
                                primary = old.iter().position(|s| *s == id).and_then(|i| ids.get(i).copied()).unwrap_or(id);
                                app.selection = ids.clone();
                            }
                            let orig: Vec<_> = ids
                                .iter()
                                .filter_map(|i| {
                                    let (t, k) = app.project.clip(*i)?;
                                    (!app.project.tracks[t].locked).then(|| (*i, app.project.tracks[t].clips[k].start, t))
                                })
                                .collect();
                            let pts = edit_ops::snap_points(&app.project, &ids, app.playhead);
                            Drag::Move { orig, primary, grab: g.t(origin.x), pts, snap: None }
                        }
                        Part::Left | Part::Right => {
                            let end = part == Part::Right;
                            let edge = if end { c.end() } else { c.start };
                            Drag::Trim {
                                id,
                                end,
                                off: edge - g.t(origin.x),
                                pts: edit_ops::snap_points(&app.project, &[id], app.playhead),
                                snap: None,
                            }
                        }
                        Part::FadeIn => Drag::Fade { id, out: false },
                        Part::FadeOut => Drag::Fade { id, out: true },
                    });
                }
            }
            (Tool::Select, None) => drag = Some(Drag::Marquee { origin, base: if mods.shift { app.selection.clone() } else { vec![] } }),
            (Tool::Slip, Some((ti, id, _))) if free(app, ti) => {
                app.checkpoint();
                drag = Some(Drag::Slip { id, last: g.t(origin.x) });
            }
            _ => {}
        }
    }

    let mut edited = false;
    if let (Some(d), Some(pos)) = (drag.as_mut(), ptr) {
        if resp.dragged() {
            let now = g.t(pos.x);
            match d {
                Drag::Move { orig, primary, grab, pts, snap } => {
                    if let (Some(&(_, o_start, o_track)), Some(c)) = (orig.iter().find(|o| o.0 == *primary), app.project.clip_ref(*primary))
                    {
                        let len = c.duration();
                        let min_start = orig.iter().map(|o| o.1).fold(f64::MAX, f64::min);
                        let mut dt = (now - *grab).max(-min_start);
                        *snap = None;
                        if app.view.snap {
                            if let Some(s) = edit_ops::snap(o_start + dt, pts, tol) {
                                dt += s - (o_start + dt);
                                *snap = Some(s);
                            } else if let Some(s) = edit_ops::snap(o_start + dt + len, pts, tol) {
                                dt += s - (o_start + dt + len);
                                *snap = Some(s);
                            }
                        }
                        let dtrack = g.track_near(pos.y) as isize - o_track as isize;
                        let mut order = orig.clone();
                        order.sort_by(|a, b| a.1.total_cmp(&b.1));
                        if dt > 0.0 {
                            order.reverse();
                        }
                        for (id, s, t) in order {
                            let nt = t as isize + dtrack;
                            let tracks = &app.project.tracks;
                            let ok = nt >= 0 && (nt as usize) < tracks.len() && tracks[nt as usize].kind == tracks[t].kind;
                            edit_ops::move_clip(&mut app.project, id, s + dt, if ok { nt as usize } else { t });
                        }
                        edited = true;
                    }
                }
                Drag::Trim { id, end, off, pts, snap } => {
                    let mut t = now + *off;
                    *snap = None;
                    if app.view.snap {
                        if let Some(s) = edit_ops::snap(t, pts, tol) {
                            t = s;
                            *snap = Some(s);
                        }
                    }
                    if *end {
                        edit_ops::trim_end(&mut app.project, *id, t);
                    } else {
                        edit_ops::trim_start(&mut app.project, *id, t);
                    }
                    edited = true;
                }
                Drag::Fade { id, out } => {
                    if let Some(c) = app.project.clip_mut(*id) {
                        let len = c.duration();
                        let v = if *out { c.end() - now } else { now - c.start };
                        let v = if v < 0.03 { 0.0 } else { v };
                        if *out {
                            c.fade_out = v.clamp(0.0, (len - c.fade_in).max(0.0));
                        } else {
                            c.fade_in = v.clamp(0.0, (len - c.fade_out).max(0.0));
                        }
                    }
                }
                Drag::Slip { id, last } => {
                    edit_ops::slip(&mut app.project, *id, -(now - *last));
                    *last = now;
                    edited = true;
                }
                Drag::Marquee { origin, base } => {
                    let r = Rect::from_two_pos(*origin, pos);
                    let mut sel = base.clone();
                    for (ti, tr) in app.project.tracks.iter().enumerate() {
                        if !g.row(ti).intersects(r) {
                            continue;
                        }
                        for c in &tr.clips {
                            if g.clip_rect(ti, c).intersects(r) && !sel.contains(&c.id) {
                                sel.push(c.id);
                            }
                        }
                    }
                    app.selection = sel;
                }
            }
        }
    }
    if edited {
        app.dirty = true;
    }
    let finished = resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down());
    if finished {
        if drag.is_some() {
            app.restart_audio();
        }
        drag = None;
        ctx.data_mut(|d| d.remove::<Drag>(did));
    } else if let Some(d) = &drag {
        ctx.data_mut(|data| data.insert_temp(did, d.clone()));
    }

    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = g.t(pos.x);
            match app.tool {
                Tool::Razor => {
                    let t = if (t - app.playhead).abs() < tol { app.playhead } else { t };
                    if mods.shift {
                        app.checkpoint();
                        edit_ops::split_all_at(&mut app.project, t);
                    } else if let Some((_, id, _)) = hit(app, &g, pos) {
                        app.checkpoint();
                        edit_ops::split_at(&mut app.project, id, t);
                    }
                }
                Tool::Select => {
                    if !mods.command && !mods.shift && app.selection.len() > 1 {
                        if let Some((_, id, _)) = hit(app, &g, pos) {
                            app.selection = vec![id];
                        }
                    }
                }
                Tool::Slip => {}
            }
        }
    }

    if resp.secondary_clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let h = hit(app, &g, pos);
            if let Some((_, id, _)) = h {
                if !app.selection.contains(&id) {
                    app.selection = vec![id];
                }
            }
            let target = Target { clip: h.map(|h| h.1), track: g.track_at(pos.y), t: g.t(pos.x) };
            ctx.data_mut(|d| d.insert_temp(tid, target));
        }
    }
    resp.context_menu(|ui| {
        let tg = ui.data(|d| d.get_temp::<Target>(tid)).unwrap_or_default();
        match tg.clip {
            Some(id) => clip_menu(app, ui, id),
            None => lane_menu(app, ui, tg.track, tg.t),
        }
    });

    let mut hl = None;
    let mut ghost: Option<(Rect, String)> = None;
    if let Some(pl) = resp.dnd_release_payload::<DragPayload>() {
        if let Some(pos) = ptr {
            let t = g.t(pos.x).max(0.0);
            let h = hit(app, &g, pos);
            app.checkpoint();
            match &*pl {
                DragPayload::Media(m) => {
                    let track = match app.project.media(*m) {
                        Some(media) => drop_track(&app.project, &g, pos.y, media.has_video),
                        None => None,
                    };
                    let ids = edit_ops::add_media(&mut app.project, *m, track, t);
                    if !ids.is_empty() {
                        app.selection = ids;
                    }
                }
                DragPayload::Effect(kind) => {
                    if let Some(c) = h.and_then(|h| app.project.clip_mut(h.1)) {
                        c.effects.push(Effect { enabled: true, kind: kind.clone() });
                        app.selection = vec![c.id];
                    }
                }
                DragPayload::Transition(kind) => {
                    if let Some((_, id, _)) = h {
                        let d = app.project.clip_ref(id).map(|c| (c.duration() * 0.5).min(1.0)).unwrap_or(1.0);
                        edit_ops::set_transition(&mut app.project, id, Some(*kind), d);
                        app.selection = vec![id];
                    }
                }
                DragPayload::Title => {
                    if let Some(id) = edit_ops::add_text(&mut app.project, t) {
                        app.selection = vec![id];
                    }
                }
                DragPayload::Matte(color) => {
                    if let Some(id) = edit_ops::add_matte(&mut app.project, *color, t) {
                        app.selection = vec![id];
                    }
                }
            }
        }
    } else if let (Some(pl), Some(pos)) = (resp.dnd_hover_payload::<DragPayload>(), ptr) {
        let mut t = g.t(pos.x).max(0.0);
        if app.view.snap {
            let pts = edit_ops::snap_points(&app.project, &[], app.playhead);
            if let Some(s) = edit_ops::snap(t, &pts, tol) {
                t = s;
            }
        }
        match &*pl {
            DragPayload::Media(m) => {
                if let Some(media) = app.project.media(*m) {
                    if let Some(ti) = drop_track(&app.project, &g, pos.y, media.has_video).or_else(|| g.track_at(pos.y)) {
                        let r = g.row(ti);
                        let rect =
                            Rect::from_min_max(pos2(g.x(t), r.top() + 2.0), pos2(g.x(t + media.duration.max(0.1)), r.bottom() - 2.0));
                        ghost = Some((rect, media.file_name()));
                    }
                }
            }
            DragPayload::Title | DragPayload::Matte(_) => {
                let want = app.project.tracks.iter().position(|t| t.kind == TrackKind::Video);
                if let Some(ti) = g.track_at(pos.y).filter(|i| app.project.tracks[*i].kind == TrackKind::Video).or(want) {
                    let r = g.row(ti);
                    let (len, name) = if matches!(*pl, DragPayload::Title) { (3.0, "Title") } else { (5.0, "Matte") };
                    ghost = Some((Rect::from_min_max(pos2(g.x(t), r.top() + 2.0), pos2(g.x(t + len), r.bottom() - 2.0)), name.into()));
                }
            }
            DragPayload::Effect(_) | DragPayload::Transition(_) => hl = hit(app, &g, pos).map(|h| h.1),
        }
    }

    let hov_hit = if resp.hovered() && drag.is_none() { ptr.and_then(|p| hit(app, &g, p)) } else { None };
    if matches!(drag, Some(Drag::Move { .. })) {
        ctx.set_cursor_icon(CursorIcon::Grabbing);
    } else if let Some((_, _, part)) = hov_hit {
        match (app.tool, part) {
            (Tool::Razor, _) => ctx.set_cursor_icon(CursorIcon::Crosshair),
            (Tool::Slip, _) | (_, Part::Left | Part::Right | Part::FadeIn | Part::FadeOut) => {
                ctx.set_cursor_icon(CursorIcon::ResizeHorizontal)
            }
            _ => {}
        }
    } else if app.tool == Tool::Razor && resp.hovered() {
        ctx.set_cursor_icon(CursorIcon::Crosshair);
    }
    let hov_id = hov_hit.map(|h| h.1);

    for i in 0..app.project.tracks.len() {
        let r = g.row(i);
        if r.bottom() < lane.top() || r.top() > lane.bottom() {
            continue;
        }
        lanes.rect_filled(r, 0, theme::BG_INPUT);
        if app.project.tracks[i].locked {
            lanes.rect_filled(r, 0, theme::BG_APP.gamma_multiply(0.55));
        }
        lanes.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, theme::BORDER));
        if i > 0 && g.rows[i].0 > g.rows[i - 1].0 + g.rows[i - 1].1 + 0.5 {
            lanes.hline(r.x_range(), r.top() - GROUP_GAP * 0.5, Stroke::new(1.0, theme::BORDER_STRONG));
        }
    }
    if let Some((a, b)) = app.project.range {
        let band = Rect::from_min_max(pos2(g.x(a), lane.top()), pos2(g.x(b), lane.bottom()));
        lanes.rect_filled(band, 0, theme::ACCENT.gamma_multiply(0.07));
        painter.rect_filled(
            Rect::from_min_max(pos2(g.x(a), ruler.top()), pos2(g.x(b), ruler.bottom())).intersect(ruler),
            0,
            theme::ACCENT.gamma_multiply(0.22),
        );
    }
    for m in &app.project.markers {
        let x = g.x(m.time);
        if x >= lane.left() && x <= lane.right() {
            lanes.vline(x.floor() + 0.5, lane.y_range(), Stroke::new(1.0, theme::CLIP_LABELS[m.color as usize % 8].gamma_multiply(0.35)));
        }
    }

    draw_ruler(&painter, ruler, &g, fps, &app.project);

    if app.project.tracks.iter().all(|t| t.clips.is_empty()) && drag.is_none() && ghost.is_none() {
        lanes.text(
            pos2(lane.center().x, lane.top() + app.view.track_height * 1.5),
            Align2::CENTER_CENTER,
            "Drag media here, or press Ctrl+I to import",
            FontId::proportional(theme::SIZE_BODY),
            theme::TEXT_MUTED,
        );
    }

    let (vis_l, vis_r) = (g.t(lane.left()), g.t(lane.right()));
    {
        let App { project, thumbs, waves, selection, .. } = &mut *app;
        for (ti, tr) in project.tracks.iter().enumerate() {
            let row = g.row(ti);
            if row.bottom() < lane.top() || row.top() > lane.bottom() {
                continue;
            }
            let dim = tr.hidden || tr.muted;
            for c in &tr.clips {
                if c.end() < vis_l || c.start > vis_r {
                    continue;
                }
                let r = g.clip_rect(ti, c);
                let sel = selection.contains(&c.id);
                draw_clip(thumbs, waves, project, &lanes, &g, tr.kind, dim, c, r, sel, hov_id == Some(c.id));
                if hl == Some(c.id) {
                    lanes.rect_filled(r, radius(), theme::ACCENT.gamma_multiply(0.25));
                    lanes.rect_stroke(r, radius(), Stroke::new(1.5, theme::ACCENT), StrokeKind::Inside);
                }
            }
        }
    }

    if let Some((rect, name)) = &ghost {
        lanes.rect_filled(*rect, radius(), theme::ACCENT.gamma_multiply(0.25));
        lanes.rect_stroke(*rect, radius(), Stroke::new(1.0, theme::ACCENT), StrokeKind::Inside);
        lanes.with_clip_rect(rect.intersect(lane)).text(
            pos2(rect.left().max(lane.left()) + 6.0, rect.top() + NAME_H * 0.5 + 1.0),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(theme::SIZE_SMALL),
            theme::TEXT,
        );
    }
    if app.tool == Tool::Razor && resp.hovered() && drag.is_none() {
        if let Some(p) = ptr {
            let x = p.x.floor() + 0.5;
            let span = match g.track_at(p.y) {
                Some(i) if !mods.shift => g.row(i).y_range(),
                _ => lane.y_range(),
            };
            lanes.vline(x, span, Stroke::new(1.0, theme::TEXT));
        }
    }
    let snap_t = match &drag {
        Some(Drag::Move { snap, .. }) | Some(Drag::Trim { snap, .. }) => *snap,
        _ => None,
    };
    if let Some(s) = snap_t {
        lanes.vline(g.x(s).floor() + 0.5, lane.y_range(), Stroke::new(1.0, theme::ACCENT_HOVER));
    }
    if let (Some(Drag::Marquee { origin, .. }), Some(p)) = (&drag, ptr) {
        let r = Rect::from_two_pos(*origin, p).intersect(lane);
        lanes.rect_filled(r, 0, theme::ACCENT.gamma_multiply(0.12));
        lanes.rect_stroke(r, 0, Stroke::new(1.0, theme::ACCENT), StrokeKind::Inside);
    }

    let px = g.x(app.playhead);
    if px >= lane.left() - 6.0 && px <= lane.right() + 6.0 {
        let both = painter.with_clip_rect(Rect::from_min_max(ruler.min, lane.max));
        let x = px.floor() + 0.5;
        both.vline(x, ruler.bottom()..=lane.bottom(), Stroke::new(1.0, theme::ACCENT));
        let b = ruler.bottom();
        both.add(Shape::convex_polygon(
            vec![pos2(x - 5.5, b - 11.0), pos2(x + 5.5, b - 11.0), pos2(x + 5.5, b - 5.0), pos2(x, b), pos2(x - 5.5, b - 5.0)],
            theme::ACCENT,
            Stroke::NONE,
        ));
    }

    draw_headers(app, ui, &painter, &g, hdr, area);
    painter.vline(lane.left() - 0.5, area.y_range(), Stroke::new(1.0, theme::BORDER));
    painter.hline(area.x_range(), lane.top() - 0.5, Stroke::new(1.0, theme::BORDER));
}

fn drop_track(p: &Project, g: &Geo, y: f32, video: bool) -> Option<usize> {
    let kind = if video { TrackKind::Video } else { TrackKind::Audio };
    g.track_at(y).filter(|i| p.tracks[*i].kind == kind)
}

fn draw_ruler(painter: &Painter, ruler: Rect, g: &Geo, fps: f64, p: &Project) {
    let rp = painter.with_clip_rect(ruler);
    let pps = g.pps as f64;
    let step = TICK_STEPS.iter().copied().find(|s| s * pps >= 110.0).unwrap_or(3600.0);
    let n = if step * pps / 5.0 >= 7.0 {
        5
    } else if step * pps / 2.0 >= 7.0 {
        2
    } else {
        1
    };
    let minor = step / n as f64;
    let first = (g.t(ruler.left()) / minor).floor() as i64;
    let last = (g.t(ruler.right()) / minor).ceil() as i64;
    let font = FontId::monospace(theme::SIZE_SMALL);
    for k in first.max(0)..=last {
        let t = k as f64 * minor;
        let x = g.x(t).floor() + 0.5;
        if k % n as i64 == 0 {
            rp.vline(x, ruler.bottom() - 9.0..=ruler.bottom(), Stroke::new(1.0, theme::TEXT_MUTED));
            rp.text(pos2(x + 4.0, ruler.top() + 3.0), Align2::LEFT_TOP, theme::timecode(t, fps), font.clone(), theme::TEXT_MUTED);
        } else {
            rp.vline(x, ruler.bottom() - 5.0..=ruler.bottom(), Stroke::new(1.0, theme::BORDER_STRONG));
        }
    }
    for m in &p.markers {
        let x = g.x(m.time).floor() + 0.5;
        if x < ruler.left() - 10.0 || x > ruler.right() {
            continue;
        }
        let col = theme::CLIP_LABELS[m.color as usize % theme::CLIP_LABELS.len()];
        rp.vline(x, ruler.top() + 2.0..=ruler.bottom(), Stroke::new(1.0, col));
        rp.add(Shape::convex_polygon(
            vec![pos2(x, ruler.top() + 2.0), pos2(x + 9.0, ruler.top() + 6.0), pos2(x, ruler.top() + 10.0)],
            col,
            Stroke::NONE,
        ));
    }
}

fn draw_headers(app: &mut App, ui: &mut Ui, painter: &Painter, g: &Geo, hdr: Rect, area: Rect) {
    let hp = painter.with_clip_rect(hdr);
    for ti in 0..app.project.tracks.len() {
        let row = g.row(ti);
        if row.bottom() < hdr.top() || row.top() > hdr.bottom() {
            continue;
        }
        let r = Rect::from_min_max(pos2(area.left(), row.top()), pos2(hdr.right() - 1.0, row.bottom()));
        hp.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, theme::BORDER));
        let tid = app.project.tracks[ti].id;
        let rr = ui.interact(r.intersect(hdr), Id::unique(("tl_head", tid)), Sense::click());
        let mut slots = vec![];
        let mut x = r.right() - theme::SP_1;
        for k in 0..3 {
            if k == 1 && app.project.tracks[ti].kind == TrackKind::Video {
                continue;
            }
            x -= 22.0;
            slots.push(Rect::from_min_size(pos2(x, r.center().y - 11.0), vec2(22.0, 22.0)));
            x -= 1.0;
        }
        let name_clip = Rect::from_min_max(r.min, pos2(x - 2.0, r.max.y)).intersect(hdr);
        hp.with_clip_rect(name_clip).text(
            pos2(r.left() + theme::SP_3, r.center().y),
            Align2::LEFT_CENTER,
            &app.project.tracks[ti].name,
            FontId::new(theme::SIZE_BODY, theme::semibold()),
            theme::TEXT,
        );
        let t = &app.project.tracks[ti];
        let (video, hidden, muted, solo, locked) = (t.kind == TrackKind::Video, t.hidden, t.muted, t.solo, t.locked);
        let mut flips = [false; 4];
        let lock_icon = if locked { Icon::Lock } else { Icon::Unlock };
        let mut slot = slots.iter();
        if let Some(s) = slot.next() {
            flips[3] =
                head_btn(ui, &hp, *s, Id::unique(("lock", tid)), locked, if locked { "Unlock track" } else { "Lock track" }, |p, r, c| {
                    icons::paint(p, r, lock_icon, c)
                });
        }
        if video {
            if let Some(s) = slot.next() {
                let icon = if hidden { Icon::EyeOff } else { Icon::Eye };
                flips[0] =
                    head_btn(ui, &hp, *s, Id::unique(("vis", tid)), hidden, if hidden { "Show track" } else { "Hide track" }, |p, r, c| {
                        icons::paint(p, r, icon, c)
                    });
            }
        } else {
            if let Some(s) = slot.next() {
                flips[2] = head_btn(ui, &hp, *s, Id::unique(("solo", tid)), solo, "Solo", |p, r, c| {
                    p.text(r.center(), Align2::CENTER_CENTER, "S", FontId::new(theme::SIZE_SMALL, theme::semibold()), c);
                });
            }
            if let Some(s) = slot.next() {
                let icon = if muted { Icon::SpeakerOff } else { Icon::Speaker };
                flips[1] = head_btn(
                    ui,
                    &hp,
                    *s,
                    Id::unique(("mute", tid)),
                    muted,
                    if muted { "Unmute track" } else { "Mute track" },
                    |p, r, c| icons::paint(p, r, icon, c),
                );
            }
        }
        if flips.iter().any(|f| *f) {
            app.checkpoint();
            let t = &mut app.project.tracks[ti];
            if flips[0] {
                t.hidden = !t.hidden;
            }
            if flips[1] {
                t.muted = !t.muted;
            }
            if flips[2] {
                t.solo = !t.solo;
            }
            if flips[3] {
                t.locked = !t.locked;
            }
            app.restart_audio();
        }
        let mut removed = false;
        rr.context_menu(|ui| removed = track_menu(app, ui, ti));
        if removed {
            let p = &app.project;
            app.selection.retain(|id| p.clip(*id).is_some());
            break;
        }
    }
}

fn head_btn(ui: &mut Ui, hp: &Painter, rect: Rect, id: Id, on: bool, tip: &str, paint: impl FnOnce(&Painter, Rect, Color32)) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        hp.rect_filled(rect, radius(), theme::BG_HOVER);
    }
    let col = if on {
        theme::ACCENT
    } else if resp.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_MUTED
    };
    paint(hp, rect, col);
    resp.on_hover_text(tip).clicked()
}

fn draw_clip(
    th: &mut Thumbs,
    wv: &mut Waveforms,
    p: &Project,
    pt: &Painter,
    g: &Geo,
    kind: TrackKind,
    dim: bool,
    c: &Clip,
    r: Rect,
    sel: bool,
    hov: bool,
) {
    let lane = g.lane;
    let vr = Rect::from_min_max(pos2(r.left().max(lane.left() - 8.0), r.top()), pos2(r.right().min(lane.right() + 8.0), r.bottom()));
    let mut col = clip_color(c, kind);
    if hov {
        col = col.lerp_to_gamma(Color32::WHITE, 0.1);
    }
    if dim {
        col = col.gamma_multiply(0.5);
    }
    pt.rect_filled(vr, radius(), col);
    let media = c.media_id().and_then(|id| p.media(id));
    if let Some(m) = media {
        if kind == TrackKind::Video && m.has_video && m.height > 0 && r.height() > NAME_H + 8.0 {
            let tw = (r.height() * m.width as f32 / m.height as f32).max(16.0);
            let a = ((vr.left() - r.left()) / tw).floor().max(0.0) as i32;
            let b = ((vr.right() - r.left()) / tw).ceil() as i32;
            let tint = if dim { Color32::WHITE.gamma_multiply(0.5) } else { Color32::WHITE };
            for i in a..b {
                let x0 = r.left() + i as f32 * tw;
                let x1 = (x0 + tw).min(r.right());
                let tc = g.t((x0 + tw * 0.5).min(r.right() - 0.01));
                if let Some(tex) = th.get(m, c.source_time(tc)) {
                    let slot = Rect::from_min_max(pos2(x0, r.top()), pos2(x1, r.bottom()));
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2((x1 - x0) / tw, 1.0));
                    pt.image(tex.id(), slot, uv, tint);
                }
            }
        } else if kind == TrackKind::Audio && m.has_audio {
            if let Some(pk) = wv.get(m) {
                draw_wave(pt, &pk, c, g, r, vr, col, dim || c.audio_muted);
            }
        }
    }
    let strip = Rect::from_min_max(vr.min, pos2(vr.right(), (vr.top() + NAME_H).min(vr.bottom())));
    let rad = radius();
    pt.rect_filled(strip, CornerRadius { nw: rad.nw, ne: rad.ne, sw: 0, se: 0 }, shade(col, 0.68));
    if let Some(tr) = c.transition_in {
        let w = (tr.duration as f32 * g.pps).min(r.width());
        let band =
            Rect::from_min_max(pos2(r.left().max(lane.left() - 8.0), strip.bottom()), pos2((r.left() + w).max(r.left()), r.bottom()));
        if band.width() > 0.0 {
            pt.rect_filled(band, 0, Color32::from_white_alpha(34));
        }
        let (x0, cy) = (r.left() + 4.0, strip.center().y);
        for (a, b) in [(x0, x0 + 5.0), (x0 + 10.0, x0 + 5.0)] {
            pt.add(Shape::convex_polygon(vec![pos2(a, cy - 4.5), pos2(a, cy + 4.5), pos2(b, cy)], theme::TEXT, Stroke::NONE));
        }
    }
    if c.fade_in > 0.0 {
        let w = c.fade_in as f32 * g.pps;
        let pts = vec![pos2(r.left(), strip.bottom()), pos2(r.left() + w, strip.bottom()), pos2(r.left(), r.bottom())];
        pt.add(Shape::convex_polygon(pts, Color32::from_black_alpha(70), Stroke::NONE));
        pt.line_segment(
            [pos2(r.left(), r.bottom()), pos2(r.left() + w, strip.bottom())],
            Stroke::new(1.0, theme::TEXT.gamma_multiply(0.8)),
        );
    }
    if c.fade_out > 0.0 {
        let w = c.fade_out as f32 * g.pps;
        let pts = vec![pos2(r.right(), strip.bottom()), pos2(r.right() - w, strip.bottom()), pos2(r.right(), r.bottom())];
        pt.add(Shape::convex_polygon(pts, Color32::from_black_alpha(70), Stroke::NONE));
        pt.line_segment(
            [pos2(r.right(), r.bottom()), pos2(r.right() - w, strip.bottom())],
            Stroke::new(1.0, theme::TEXT.gamma_multiply(0.8)),
        );
    }
    let mut rx = r.right().min(lane.right()) - 6.0;
    if r.width() > 80.0 {
        if (c.speed - 1.0).abs() > 1e-6 {
            let s = if (c.speed * 10.0).fract().abs() < 1e-6 { format!("{:.1}x", c.speed) } else { format!("{:.2}x", c.speed) };
            let tr = pt.text(pos2(rx, strip.center().y), Align2::RIGHT_CENTER, s, FontId::monospace(theme::SIZE_SMALL), theme::TEXT);
            rx = tr.left() - 6.0;
        }
        if !c.effects.is_empty() {
            let ir = Rect::from_center_size(pos2(rx - 7.0, strip.center().y), vec2(14.0, 14.0));
            icons::paint(pt, ir, Icon::Effect, theme::TEXT);
            rx = ir.left() - 4.0;
        }
    }
    let shift = if c.transition_in.is_some() && r.left() > lane.left() - 14.0 { 16.0 } else { 0.0 };
    let tl = r.left().max(lane.left()) + 6.0 + shift;
    let tclip = Rect::from_min_max(pos2(tl - 2.0, strip.top()), pos2(rx.max(tl), strip.bottom())).intersect(vr);
    pt.with_clip_rect(tclip).text(
        pos2(tl, strip.center().y),
        Align2::LEFT_CENTER,
        &c.name,
        FontId::proportional(theme::SIZE_SMALL),
        theme::TEXT,
    );
    if sel && r.width() > 40.0 {
        let (a, b) = g.fade_handles(c, r);
        for h in [a, b] {
            if lane.expand(4.0).contains(h) {
                pt.circle_filled(h, 3.5, theme::ACCENT);
                pt.circle_stroke(h, 3.5, Stroke::new(1.0, theme::ON_ACCENT));
            }
        }
    }
    if sel {
        pt.rect_stroke(vr, radius(), Stroke::new(1.5, theme::ACCENT), StrokeKind::Inside);
    } else {
        pt.rect_stroke(vr, radius(), Stroke::new(1.0, Color32::from_black_alpha(90)), StrokeKind::Inside);
    }
}

fn draw_wave(pt: &Painter, peaks: &crate::waveform::Peaks, c: &Clip, g: &Geo, r: Rect, vr: Rect, base: Color32, dim: bool) {
    let top = r.top() + NAME_H;
    let cy = (top + r.bottom()) * 0.5;
    let half = ((r.bottom() - top) * 0.5 - 2.0).max(1.0);
    let x0 = vr.left().max(g.lane.left()).floor();
    let x1 = vr.right().min(g.lane.right()).ceil();
    let col = base.lerp_to_gamma(Color32::WHITE, 0.55).gamma_multiply(if dim { 0.4 } else { 0.9 });
    let mut mesh = epaint::Mesh::default();
    let mut x = x0;
    let mut n = 0u32;
    while x < x1 {
        let (a, b) = (c.source_time(g.t(x)), c.source_time(g.t(x + 1.0)));
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        if lo * peaks.per_sec >= peaks.data.len() as f64 {
            break;
        }
        let peak = peaks.range(lo, hi).min(1.0);
        let amp = (peak * half).max(0.5);
        mesh.colored_vertex(pos2(x, cy - amp), col);
        mesh.colored_vertex(pos2(x, cy + amp), col);
        if n > 0 {
            let v = n * 2;
            mesh.add_triangle(v - 2, v - 1, v);
            mesh.add_triangle(v - 1, v, v + 1);
        }
        n += 1;
        x += 1.0;
    }
    if n > 1 {
        pt.add(Shape::mesh(mesh));
    }
}

fn selected_or(app: &App, id: ClipId) -> Vec<ClipId> {
    if app.selection.contains(&id) {
        app.selection.clone()
    } else {
        vec![id]
    }
}

fn clip_menu(app: &mut App, ui: &mut Ui, id: ClipId) {
    let ids = selected_or(app, id);
    if item(ui, "Split at playhead") {
        app.checkpoint();
        edit_ops::split_at(&mut app.project, id, app.playhead);
    }
    ui.separator();
    if item(ui, "Delete") {
        app.checkpoint();
        edit_ops::delete(&mut app.project, &ids, false);
        app.selection.retain(|s| !ids.contains(s));
    }
    if item(ui, "Ripple delete") {
        app.checkpoint();
        edit_ops::delete(&mut app.project, &ids, true);
        app.selection.retain(|s| !ids.contains(s));
    }
    if item(ui, "Duplicate") {
        app.checkpoint();
        let new = edit_ops::duplicate(&mut app.project, &ids);
        if !new.is_empty() {
            app.selection = new;
        }
    }
    if item(ui, "Copy") {
        app.clipboard = edit_ops::copy(&app.project, &ids);
    }
    ui.separator();
    ui.menu_button("Speed", |ui| {
        for s in [0.25, 0.5, 1.0, 1.5, 2.0, 4.0] {
            if item(ui, &format!("{}x", s)) {
                app.checkpoint();
                for i in &ids {
                    edit_ops::set_speed(&mut app.project, *i, s);
                }
            }
        }
    });
    if item(ui, "Reverse") {
        app.checkpoint();
        for i in &ids {
            if let Some(c) = app.project.clip_mut(*i) {
                c.reverse = !c.reverse;
            }
        }
    }
    let muted = app.project.clip_ref(id).is_some_and(|c| c.audio_muted);
    if item(ui, if muted { "Unmute audio" } else { "Mute audio" }) {
        app.checkpoint();
        for i in &ids {
            if let Some(c) = app.project.clip_mut(*i) {
                c.audio_muted = !muted;
            }
        }
    }
    ui.menu_button("Label color", |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme::SP_1;
            let current = app.project.clip_ref(id).map(|c| c.label).unwrap_or(0);
            for (n, col) in theme::CLIP_LABELS.iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                ui.painter().rect_filled(rect, radius(), *col);
                if current as usize == n || resp.hovered() {
                    ui.painter().rect_stroke(rect, radius(), Stroke::new(1.5, theme::TEXT), StrokeKind::Inside);
                }
                if resp.clicked() {
                    app.checkpoint();
                    for i in &ids {
                        if let Some(c) = app.project.clip_mut(*i) {
                            c.label = n as u8;
                        }
                    }
                    ui.close();
                }
            }
        });
    });
    ui.menu_button("Transition", |ui| {
        let current = app.project.clip_ref(id).and_then(|c| c.transition_in);
        let d = current.map(|t| t.duration).unwrap_or(1.0);
        for k in TransitionKind::ALL {
            if item(ui, k.name()) {
                app.checkpoint();
                edit_ops::set_transition(&mut app.project, id, Some(k), d);
            }
        }
        ui.separator();
        if item(ui, "None") {
            app.checkpoint();
            edit_ops::set_transition(&mut app.project, id, None, 0.0);
        }
    });
    if item(ui, "Clear effects") {
        app.checkpoint();
        for i in &ids {
            if let Some(c) = app.project.clip_mut(*i) {
                c.effects.clear();
            }
        }
    }
    ui.menu_button("Rename", |ui| {
        let mut name = app.project.clip_ref(id).map(|c| c.name.clone()).unwrap_or_default();
        let r = ui.text_edit_singleline(&mut name);
        if r.gained_focus() {
            app.checkpoint();
        }
        if !r.has_focus() && !r.lost_focus() {
            r.request_focus();
        }
        if r.changed() {
            if let Some(c) = app.project.clip_mut(id) {
                c.name = name;
            }
        }
        if r.lost_focus() {
            ui.close();
        }
    });
}

fn lane_menu(app: &mut App, ui: &mut Ui, track: Option<usize>, t: f64) {
    let t = t.max(0.0);
    if ui.add_enabled(!app.clipboard.is_empty(), egui::Button::new("Paste")).clicked() {
        app.checkpoint();
        let new = edit_ops::paste(&mut app.project, &app.clipboard, t);
        if !new.is_empty() {
            app.selection = new;
        }
        ui.close();
    }
    ui.separator();
    if item(ui, "Add title here") {
        app.checkpoint();
        if let Some(id) = edit_ops::add_text(&mut app.project, t) {
            app.selection = vec![id];
        }
    }
    if item(ui, "Add color matte") {
        app.checkpoint();
        if let Some(id) = edit_ops::add_matte(&mut app.project, [32, 32, 32, 255], t) {
            app.selection = vec![id];
        }
    }
    ui.separator();
    if item(ui, "Add video track") {
        app.checkpoint();
        edit_ops::add_track(&mut app.project, TrackKind::Video);
    }
    if item(ui, "Add audio track") {
        app.checkpoint();
        edit_ops::add_track(&mut app.project, TrackKind::Audio);
    }
    ui.separator();
    if ui.add_enabled(track.is_some(), egui::Button::new("Close gaps on this track")).clicked() {
        if let Some(i) = track {
            app.checkpoint();
            edit_ops::close_gaps(&mut app.project, i);
        }
        ui.close();
    }
}

fn track_menu(app: &mut App, ui: &mut Ui, ti: usize) -> bool {
    let mut name = app.project.tracks[ti].name.clone();
    let r = ui.text_edit_singleline(&mut name);
    if r.gained_focus() {
        app.checkpoint();
    }
    if r.changed() {
        app.project.tracks[ti].name = name;
    }
    if r.lost_focus() {
        ui.close();
    }
    ui.separator();
    if item(ui, "Add video track") {
        app.checkpoint();
        edit_ops::add_track(&mut app.project, TrackKind::Video);
    }
    if item(ui, "Add audio track") {
        app.checkpoint();
        edit_ops::add_track(&mut app.project, TrackKind::Audio);
    }
    ui.separator();
    if item(ui, "Delete track") {
        app.checkpoint();
        edit_ops::remove_track(&mut app.project, ti);
        return true;
    }
    false
}
