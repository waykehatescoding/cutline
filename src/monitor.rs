use crate::app::App;
use crate::compositor::TextOverlay;
use crate::icons::{self, Icon};
use crate::model::*;
use crate::theme;
use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Align, Color32, ColorImage, CornerRadius, CursorIcon, FontFamily, FontId, Id, Layout, Painter, Pos2,
    Rect, Response, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, UiBuilder, Vec2,
};
use std::sync::Arc;

const BAR_H: f32 = 44.0;
const HANDLE: f32 = 9.0;
const ZOOMS: [&str; 3] = ["Fit", "50%", "100%"];
const CORNERS: [(bool, bool); 4] = [(true, true), (false, true), (true, false), (false, false)];

#[derive(Clone, Default)]
struct State {
    crop: bool,
    guides: bool,
    zoom: usize,
    tex: Option<TextureHandle>,
    shown: Option<Arc<Frame>>,
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let id = ui.scope_id().with("monitor");
    let mut st: State = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    let full = ui.available_rect_before_wrap();
    ui.allocate_rect(full, Sense::hover());
    let bar = Rect::from_min_max(pos2(full.min.x, full.max.y - BAR_H), full.max);
    let area = Rect::from_min_max(full.min, pos2(full.max.x, bar.min.y - theme::SP_1));
    transport(app, ui, bar, &mut st);
    video(app, ui, area, &mut st);
    ui.data_mut(|d| d.insert_temp(id, st));
}

pub fn screen(app: &mut App, ui: &mut egui::Ui, area: Rect) {
    let id = ui.scope_id().with("monitor");
    let mut st: State = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    video(app, ui, area, &mut st);
    ui.data_mut(|d| d.insert_temp(id, st));
}

fn even(v: f32) -> u32 {
    (v as u32).max(2) & !1
}

fn notice(ui: &mut egui::Ui, area: Rect, title: egui::RichText, line: &str, action: bool, app: &mut App) {
    ui.scope_builder(UiBuilder::new().id_salt("notice").max_rect(area).layout(Layout::top_down(Align::Center)), |ui| {
        ui.add_space((area.height() - if action { 96.0 } else { 64.0 }).max(0.0) / 2.0);
        ui.label(title);
        ui.add(egui::Label::new(theme::muted(line)));
        if action {
            ui.add_space(theme::SP_2);
            if theme::primary_button(ui, "Import media").clicked() {
                app.import_dialog();
            }
        }
    });
}

fn video(app: &mut App, ui: &mut egui::Ui, area: Rect, st: &mut State) {
    let vp = ui.painter_at(area);
    vp.rect_filled(area, CornerRadius::same(theme::R_INNER), theme::BG_INPUT);
    if !app.ffmpeg_ok {
        let title = theme::heading("ffmpeg is required").color(theme::DANGER);
        notice(
            ui,
            area,
            title,
            "Install ffmpeg and add it to PATH, or place ffmpeg.exe and ffprobe.exe next to cutline.exe, then restart.",
            false,
            app,
        );
        return;
    }
    if app.project.tracks.iter().all(|t| t.clips.is_empty()) {
        notice(ui, area, theme::heading("Start by importing footage"), "Drop files anywhere or browse your computer", true, app);
        return;
    }

    let ppp = ui.ctx().pixels_per_point();
    let (sw, sh) = (app.project.settings.width as f32, app.project.settings.height as f32);
    let avail = area.shrink(theme::SP_2);
    let fit = (avail.width() / sw).min(avail.height() / sh).max(0.01);
    let k = [fit, 0.5 / ppp, 1.0 / ppp][st.zoom.min(2)];
    let rect = Rect::from_center_size(area.center(), vec2(sw, sh) * k);
    let (rw, rh) = (rect.width() * ppp, rect.height() * ppp);
    let cap = if app.playing { (1280.0, 720.0) } else { (sw, sh) };
    let f = (cap.0 / rw).min(cap.1 / rh).min(1.0);
    let (w, h) = (even(rw * f), even(rh * f));

    let comp = app.compositor.render(&app.project, app.playhead, &mut app.decoder, w, h, app.playing);
    if !st.shown.as_ref().is_some_and(|s| Arc::ptr_eq(s, &comp.frame)) {
        let img = ColorImage::from_rgba_unmultiplied([comp.frame.w as usize, comp.frame.h as usize], &comp.frame.rgba);
        match &mut st.tex {
            Some(tex) => tex.set(img, TextureOptions::LINEAR),
            None => st.tex = Some(ui.ctx().load_texture("monitor", img, TextureOptions::LINEAR)),
        }
        st.shown = Some(comp.frame.clone());
    }
    if let Some(tex) = &st.tex {
        vp.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }

    let text_rects = draw_texts(ui, &vp, rect, sh, &comp.texts);

    if st.guides {
        for s in [0.9, 0.8] {
            vp.rect_stroke(
                Rect::from_center_size(rect.center(), rect.size() * s),
                0.0,
                Stroke::new(1.0, theme::TEXT.gamma_multiply(0.45)),
                StrokeKind::Inside,
            );
        }
    }

    overlay(app, ui, &vp, rect, st.crop, &text_rects);

    if !comp.complete {
        let spot = Rect::from_min_size(pos2(area.max.x - 16.0 - theme::SP_3, area.min.y + theme::SP_3), Vec2::splat(16.0));
        ui.put(spot, egui::Spinner::new().size(16.0));
    }
}

fn tint(c: [u8; 4], opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (c[3] as f32 * opacity.clamp(0.0, 1.0)) as u8)
}

fn draw_texts(ui: &egui::Ui, vp: &Painter, rect: Rect, project_h: f32, texts: &[TextOverlay]) -> Vec<(ClipId, Rect)> {
    let k = rect.height() / project_h;
    let mut out = Vec::with_capacity(texts.len());
    for t in texts {
        let s = &t.style;
        let family = if s.bold { theme::semibold() } else { FontFamily::Proportional };
        let mut job =
            LayoutJob::simple(s.text.clone(), FontId::new((s.size * k).max(1.0), family), tint(s.color, t.opacity), f32::INFINITY);
        job.halign = match s.align {
            TextAlign::Left => Align::LEFT,
            TextAlign::Center => Align::Center,
            TextAlign::Right => Align::RIGHT,
        };
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let center = rect.min + vec2(t.transform.x * rect.width(), t.transform.y * rect.height());
        let origin = center - galley.rect.center().to_vec2();
        let bounds = galley.rect.translate(origin.to_vec2()).expand(theme::SP_1);
        if let Some(bg) = s.background {
            vp.rect_filled(bounds, CornerRadius::same(theme::R_INNER), tint(bg, t.opacity));
        }
        vp.galley(origin, galley, Color32::WHITE);
        out.push((t.clip, bounds));
    }
    out
}

fn layer_rect(project: &Project, clip: &Clip, frame: Rect) -> Rect {
    let (ow, oh) = (project.settings.width as f32, project.settings.height as f32);
    let (lw, lh) = match clip.source {
        ClipSource::Media(id) => {
            project.media(id).filter(|m| m.width > 0 && m.height > 0).map(|m| (m.width as f32, m.height as f32)).unwrap_or((ow, oh))
        }
        _ => (ow, oh),
    };
    let fit = (ow / lw).min(oh / lh);
    let size = vec2(lw * fit / ow * frame.width(), lh * fit / oh * frame.height()) * clip.transform.scale;
    let center = frame.min + vec2(clip.transform.x * frame.width(), clip.transform.y * frame.height());
    Rect::from_center_size(center, size)
}

fn region_mut(clip: &mut Clip) -> Option<&mut Rect01> {
    clip.effects.iter_mut().filter(|e| e.enabled).find_map(|e| match &mut e.kind {
        EffectKind::Blur { region, .. } | EffectKind::Pixelate { region, .. } => region.as_mut(),
        _ => None,
    })
}

fn edit(app: &mut App, id: ClipId, f: impl FnOnce(&mut Clip)) {
    if let Some(c) = app.project.clip_mut(id) {
        f(c);
    }
}

fn handle(ui: &mut egui::Ui, vp: &Painter, id: Id, at: Pos2, cursor: CursorIcon) -> Response {
    let resp = ui.interact(Rect::from_center_size(at, Vec2::splat(HANDLE + 6.0)), id, Sense::drag()).on_hover_cursor(cursor);
    let fill = if resp.hovered() || resp.dragged() { theme::ACCENT_HOVER } else { theme::ACCENT };
    let r = Rect::from_center_size(at, Vec2::splat(HANDLE));
    vp.rect_filled(r, 1.0, fill);
    vp.rect_stroke(r, 1.0, Stroke::new(1.0, theme::BG_APP), StrokeKind::Outside);
    resp
}

fn corners(r: Rect) -> [Pos2; 4] {
    [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()]
}

fn corner_cursor(i: usize) -> CursorIcon {
    if i == 0 || i == 3 {
        CursorIcon::ResizeNwSe
    } else {
        CursorIcon::ResizeNeSw
    }
}

fn overlay(app: &mut App, ui: &mut egui::Ui, vp: &Painter, rect: Rect, crop_mode: bool, text_rects: &[(ClipId, Rect)]) {
    let Some(id) = app.primary_clip() else { return };
    let Some((ti, ci)) = app.project.clip(id) else { return };
    let track = &app.project.tracks[ti];
    let clip = &track.clips[ci];
    if track.kind != TrackKind::Video || track.hidden || !clip.contains(app.playhead) {
        return;
    }
    let mut clip = clip.clone();
    let is_text = matches!(clip.source, ClipSource::Text(_));
    let layer = if is_text {
        match text_rects.iter().find(|(c, _)| *c == id) {
            Some((_, r)) => *r,
            None => return,
        }
    } else {
        layer_rect(&app.project, &clip, rect)
    };
    let c = &clip.crop;
    let vis = if is_text {
        layer
    } else {
        Rect::from_min_max(
            layer.min + vec2(c.left * layer.width(), c.top * layer.height()),
            layer.max - vec2(c.right * layer.width(), c.bottom * layer.height()),
        )
    };
    let base = ui.scope_id().with(("layer", id));
    let accent = Stroke::new(1.0, theme::ACCENT);
    vp.rect_stroke(vis, 0.0, accent, StrokeKind::Outside);

    if !crop_mode || is_text {
        let body = ui.interact(vis, base.with("body"), Sense::drag()).on_hover_cursor(CursorIcon::Move);
        if body.drag_started() {
            app.checkpoint();
        }
        if body.dragged() {
            let d = body.drag_delta();
            edit(app, id, |c| {
                c.transform.x += d.x / rect.width();
                c.transform.y += d.y / rect.height();
            });
        }
    }

    let region = region_mut(&mut clip).map(|r| *r);
    if let Some(reg) = region {
        let rr = Rect::from_min_size(
            rect.min + vec2(reg.x * rect.width(), reg.y * rect.height()),
            vec2(reg.w * rect.width(), reg.h * rect.height()),
        );
        let mut path = corners(rr).to_vec();
        path.swap(2, 3);
        path.push(rr.left_top());
        vp.extend(Shape::dashed_line(&path, Stroke::new(1.5, theme::INFO), 5.0, 4.0));
        let body = ui.interact(rr, base.with("region"), Sense::drag()).on_hover_cursor(CursorIcon::Grab);
        if body.drag_started() {
            app.checkpoint();
        }
        if body.dragged() {
            let d = body.drag_delta() / rect.size();
            edit(app, id, |c| {
                if let Some(r) = region_mut(c) {
                    r.x = (r.x + d.x).clamp(0.0, 1.0 - r.w);
                    r.y = (r.y + d.y).clamp(0.0, 1.0 - r.h);
                }
            });
        }
        for (i, (left, top)) in CORNERS.into_iter().enumerate() {
            let resp = handle(ui, vp, base.with(("region", i)), corners(rr)[i], corner_cursor(i));
            if resp.drag_started() {
                app.checkpoint();
            }
            if resp.dragged() {
                let d = resp.drag_delta() / rect.size();
                edit(app, id, |c| {
                    let Some(r) = region_mut(c) else { return };
                    let (mut x0, mut y0, mut x1, mut y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
                    if left {
                        x0 = (x0 + d.x).clamp(0.0, x1 - 0.02)
                    } else {
                        x1 = (x1 + d.x).clamp(x0 + 0.02, 1.0)
                    }
                    if top {
                        y0 = (y0 + d.y).clamp(0.0, y1 - 0.02)
                    } else {
                        y1 = (y1 + d.y).clamp(y0 + 0.02, 1.0)
                    }
                    *r = Rect01 { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
                });
            }
        }
    }

    if is_text {
        return;
    }
    if crop_mode {
        let edges = [
            (pos2(vis.left(), vis.center().y), CursorIcon::ResizeHorizontal),
            (pos2(vis.right(), vis.center().y), CursorIcon::ResizeHorizontal),
            (pos2(vis.center().x, vis.top()), CursorIcon::ResizeVertical),
            (pos2(vis.center().x, vis.bottom()), CursorIcon::ResizeVertical),
        ];
        for (i, (at, cursor)) in edges.into_iter().enumerate() {
            let resp = handle(ui, vp, base.with(("crop", i)), at, cursor);
            if resp.drag_started() {
                app.checkpoint();
            }
            if resp.dragged() {
                let d = resp.drag_delta() / layer.size();
                edit(app, id, |c| {
                    let cr = &mut c.crop;
                    match i {
                        0 => cr.left = (cr.left + d.x).clamp(0.0, 0.95 - cr.right),
                        1 => cr.right = (cr.right - d.x).clamp(0.0, 0.95 - cr.left),
                        2 => cr.top = (cr.top + d.y).clamp(0.0, 0.95 - cr.bottom),
                        _ => cr.bottom = (cr.bottom - d.y).clamp(0.0, 0.95 - cr.top),
                    }
                });
            }
        }
    } else {
        let center = layer.center();
        for (i, at) in corners(vis).into_iter().enumerate() {
            let resp = handle(ui, vp, base.with(("scale", i)), at, corner_cursor(i));
            if resp.drag_started() {
                app.checkpoint();
            }
            if let (true, Some(p)) = (resp.dragged(), resp.interact_pointer_pos()) {
                let prev = p - resp.drag_delta();
                let ratio = (p - center).length() / (prev - center).length().max(1.0);
                edit(app, id, |c| c.transform.scale = (c.transform.scale * ratio).clamp(0.05, 10.0));
            }
        }
    }
}

fn step(app: &mut App, frames: f64) {
    let fps = app.project.settings.fps.max(1.0);
    app.seek(((app.playhead * fps).round() + frames) / fps);
}

fn set_in(app: &mut App) {
    app.checkpoint();
    let end = app.project.range.map(|r| r.1).filter(|e| *e > app.playhead).unwrap_or(app.project.duration());
    app.project.range = Some((app.playhead, end));
}

fn set_out(app: &mut App) {
    app.checkpoint();
    let start = app.project.range.map(|r| r.0).filter(|s| *s < app.playhead).unwrap_or(0.0);
    app.project.range = Some((start, app.playhead));
}

pub fn play_button(app: &mut App, ui: &mut egui::Ui) {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    let fill = if resp.hovered() { theme::ACCENT_HOVER } else { theme::ACCENT };
    ui.painter().circle_filled(r.center(), 17.0, fill);
    let (icon, tip, nudge) = if app.playing { (Icon::Pause, "Pause", 0.0) } else { (Icon::Play, "Play", 1.0) };
    icons::paint(ui.painter(), Rect::from_center_size(r.center() + vec2(nudge, 0.0), Vec2::splat(theme::ICON)), icon, theme::ON_ACCENT);
    if resp.on_hover_text(tip).clicked() {
        app.toggle_play();
    }
}

fn transport(app: &mut App, ui: &mut egui::Ui, bar: Rect, st: &mut State) {
    let mid = Rect::from_center_size(bar.center(), vec2(240.0, bar.height()));
    ui.scope_builder(UiBuilder::new().id_salt("transport").max_rect(mid).layout(Layout::left_to_right(Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = theme::SP_1;
        if icons::button(ui, Icon::SkipStart, "Go to start").clicked() {
            app.seek(app.project.range.map(|r| r.0).unwrap_or(0.0));
        }
        if icons::button(ui, Icon::StepBack, "Previous frame").clicked() {
            step(app, -1.0);
        }
        play_button(app, ui);
        if icons::button(ui, Icon::StepForward, "Next frame").clicked() {
            step(app, 1.0);
        }
        if icons::button(ui, Icon::SkipEnd, "Go to end").clicked() {
            app.seek(app.end_time());
        }
        if icons::toggle(ui, Icon::Loop, app.loop_playback, "Loop").clicked() {
            app.loop_playback = !app.loop_playback;
        }
    });

    let left = Rect::from_min_max(pos2(bar.min.x + theme::SP_2, bar.min.y), pos2(mid.min.x - theme::SP_2, bar.max.y));
    ui.scope_builder(UiBuilder::new().id_salt("readout").max_rect(left).layout(Layout::left_to_right(Align::Center)), |ui| {
        let fps = app.project.settings.fps;
        ui.label(theme::mono(theme::timecode(app.playhead, fps)).color(theme::TEXT));
        if left.width() > 220.0 {
            ui.label(theme::mono(theme::timecode(app.project.duration(), fps)).color(theme::TEXT_MUTED));
        }
    });

    let right = Rect::from_min_max(pos2(mid.max.x + theme::SP_2, bar.min.y), pos2(bar.max.x - theme::SP_2, bar.max.y));
    ui.scope_builder(UiBuilder::new().id_salt("tools").max_rect(right).layout(Layout::right_to_left(Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = theme::SP_1;
        egui::ComboBox::from_id_salt("zoom").width(60.0).selected_text(ZOOMS[st.zoom.min(2)]).show_ui(ui, |ui| {
            for (i, name) in ZOOMS.iter().enumerate() {
                ui.selectable_value(&mut st.zoom, i, *name);
            }
        });
        if icons::toggle(ui, Icon::Fit, st.guides, "Safe area guides").clicked() {
            st.guides = !st.guides;
        }
        if icons::toggle(ui, Icon::Crop, st.crop, "Crop mode").clicked() {
            st.crop = !st.crop;
        }
        if icons::button(ui, Icon::OutPoint, "Set out point").clicked() {
            set_out(app);
        }
        if icons::button(ui, Icon::InPoint, "Set in point").clicked() {
            set_in(app);
        }
    });
}
