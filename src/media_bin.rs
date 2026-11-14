use crate::app::{App, DragPayload};
use crate::edit_ops;
use crate::icons::{self, Icon};
use crate::model::{Media, MediaId, MediaKind};
use crate::theme;
use eframe::egui::{self, text::LayoutJob, Color32, CornerRadius, FontId, Id, Rect, Sense, Stroke, StrokeKind};
use std::path::{Path, PathBuf};

const TILE_W: f32 = 96.0;
const ROW_H: f32 = 44.0;
const ROW_THUMB: egui::Vec2 = egui::vec2(64.0, 36.0);

enum Action {
    Select(MediaId),
    Add(MediaId),
    Remove(MediaId),
    Reveal(PathBuf),
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let grid_id = Id::unique("media_bin_grid");
    let search_id = Id::unique("media_bin_search");
    let mut grid = ctx.data(|d| d.get_temp::<bool>(grid_id)).unwrap_or(true);
    let mut query = ctx.data(|d| d.get_temp::<String>(search_id)).unwrap_or_default();

    ui.horizontal(|ui| {
        let w = ui.available_width() - 2.0 * 26.0 - 2.0 * ui.spacing().item_spacing.x;
        ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search media").desired_width(w.max(40.0)));
        if icons::button(ui, Icon::Import, "Import media").clicked() {
            app.import_dialog();
        }
        if icons::toggle(ui, Icon::Grip, grid, if grid { "Switch to list view" } else { "Switch to grid view" }).clicked() {
            grid = !grid;
        }
    });
    ui.add_space(theme::SP_1);

    let pending = app.importer.pending().to_vec();
    if app.project.media.is_empty() && pending.is_empty() {
        empty_state(app, ui);
    } else {
        let q = query.trim().to_lowercase();
        let missing = missing_files(app, &ctx);
        let mut actions = vec![];
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let items: Vec<usize> = (0..app.project.media.len())
                .filter(|&i| q.is_empty() || app.project.media[i].file_name().to_lowercase().contains(&q))
                .collect();
            if grid {
                let gap = ui.spacing().item_spacing.x;
                let cols = (((ui.available_width() + gap) / (TILE_W + gap)) as usize).max(1);
                let tile_w = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).floor();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.y = theme::SP_2;
                    for &i in &items {
                        let m = &app.project.media[i];
                        let sel = app.selected_media == Some(m.id);
                        let gone = missing.contains(&m.id);
                        let size = egui::vec2(tile_w, tile_w * 9.0 / 16.0 + 22.0);
                        tile(ui, m, &mut app.thumbs, sel, gone, size, &mut actions);
                    }
                    for p in &pending {
                        pending_tile(ui, p, tile_w);
                    }
                });
            } else {
                ui.spacing_mut().item_spacing.y = 2.0;
                for &i in &items {
                    let m = &app.project.media[i];
                    let sel = app.selected_media == Some(m.id);
                    let gone = missing.contains(&m.id);
                    row(ui, m, &mut app.thumbs, sel, gone, &mut actions);
                }
                for p in &pending {
                    pending_row(ui, p);
                }
            }
            if items.is_empty() && pending.is_empty() {
                ui.add_space(theme::SP_4);
                ui.vertical_centered(|ui| ui.label(theme::muted("No media matches your search")));
            }
        });
        for a in actions {
            apply(app, a);
        }
    }

    ctx.data_mut(|d| {
        d.insert_temp(grid_id, grid);
        d.insert_temp(search_id, query);
    });
}

fn missing_files(app: &App, ctx: &egui::Context) -> Vec<MediaId> {
    let id = Id::unique("media_bin_missing");
    let now = ctx.input(|i| i.time);
    if let Some((t, n, v)) = ctx.data(|d| d.get_temp::<(f64, usize, Vec<MediaId>)>(id)) {
        if now - t < 3.0 && n == app.project.media.len() {
            return v;
        }
    }
    let v: Vec<MediaId> = app.project.media.iter().filter(|m| !m.path.exists()).map(|m| m.id).collect();
    ctx.data_mut(|d| d.insert_temp(id, (now, app.project.media.len(), v.clone())));
    v
}

fn apply(app: &mut App, action: Action) {
    match action {
        Action::Select(id) => {
            app.selected_media = Some(id);
            app.selection.clear();
        }
        Action::Add(id) => {
            app.checkpoint();
            edit_ops::add_media(&mut app.project, id, None, app.playhead);
        }
        Action::Remove(id) => {
            app.checkpoint();
            edit_ops::remove_media(&mut app.project, id);
            if app.selected_media == Some(id) {
                app.selected_media = None;
            }
        }
        Action::Reveal(path) => reveal(&path),
    }
}

fn reveal(path: &Path) {
    if cfg!(windows) {
        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
    } else if let Some(dir) = path.parent() {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        let _ = std::process::Command::new(opener).arg(dir).spawn();
    }
}

fn empty_state(app: &mut App, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(theme::SP_5 * 2.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::hover());
        icons::paint(ui.painter(), rect, Icon::Import, theme::TEXT_MUTED);
        ui.add_space(theme::SP_2);
        ui.label(theme::strong("Import footage"));
        ui.label(theme::muted("Drop video, audio or images here"));
        ui.add_space(theme::SP_2);
        if theme::primary_button(ui, "Browse files").clicked() {
            app.import_dialog();
        }
    });
}

fn fit(rect: Rect, size: egui::Vec2) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return rect;
    }
    let s = (rect.width() / size.x).min(rect.height() / size.y);
    Rect::from_center_size(rect.center(), size * s)
}

fn paint_thumb(ui: &egui::Ui, rect: Rect, m: &Media, thumbs: &mut crate::thumbs::Thumbs, gone: bool) {
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(theme::R_INNER), theme::BG_RAISED);
    if m.kind == MediaKind::Audio {
        icons::paint(p, Rect::from_center_size(rect.center(), egui::vec2(24.0, 24.0)), Icon::Music, theme::TEXT_MUTED);
    } else if let Some(tex) = thumbs.poster(m) {
        let img = fit(rect, tex.size_vec2());
        p.image(tex.id(), img, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    }
    if m.kind != MediaKind::Image && m.duration > 0.0 {
        let g = p.layout_no_wrap(duration_text(m.duration), FontId::monospace(theme::SIZE_SMALL), theme::TEXT);
        let plate = Rect::from_min_size(rect.right_bottom() - g.size() - egui::vec2(8.0, 4.0), g.size() + egui::vec2(6.0, 2.0));
        p.rect_filled(plate, CornerRadius::same(2), Color32::from_black_alpha(190));
        p.galley(plate.min + egui::vec2(3.0, 1.0), g, theme::TEXT);
    }
    if gone {
        let w = Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(theme::ICON, theme::ICON));
        icons::paint(p, w, Icon::Warning, theme::DANGER);
    }
}

fn duration_text(d: f64) -> String {
    let s = d.round() as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

fn clipped(ui: &egui::Ui, text: &str, font: FontId, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::simple(text.to_owned(), font, color, width);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    ui.painter().layout_job(job)
}

fn item_ui(
    ui: &mut egui::Ui,
    m: &Media,
    size: egui::Vec2,
    gone: bool,
    actions: &mut Vec<Action>,
    paint: impl FnOnce(&egui::Ui, Rect, bool),
) {
    let id = Id::unique(("media_item", m.id));
    let dragging = ui.ctx().is_being_dragged(id);
    let resp = ui
        .dnd_drag_source(id, DragPayload::Media(m.id), |ui| {
            if dragging {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(140.0, 28.0), Sense::hover());
                let p = ui.painter();
                let r = CornerRadius::same(theme::R_INNER);
                p.rect_filled(rect, r, theme::BG_ACTIVE);
                p.rect_stroke(rect, r, Stroke::new(1.0, theme::ACCENT), StrokeKind::Inside);
                let icon = if m.kind == MediaKind::Audio { Icon::Music } else { Icon::Film };
                icons::paint(p, Rect::from_min_size(rect.min + egui::vec2(6.0, 6.0), egui::vec2(16.0, 16.0)), icon, theme::TEXT);
                let g = clipped(ui, &m.file_name(), FontId::proportional(theme::SIZE_BODY), theme::TEXT, rect.width() - 34.0);
                p.galley(egui::pos2(rect.min.x + 28.0, rect.center().y - g.size().y / 2.0), g, theme::TEXT);
            } else {
                let (rect, r) = ui.allocate_exact_size(size, Sense::click());
                paint(ui, rect, r.hovered());
            }
        })
        .response;
    if dragging {
        return;
    }
    if resp.clicked() {
        actions.push(Action::Select(m.id));
    }
    if resp.double_clicked() {
        actions.push(Action::Add(m.id));
    }
    let tip = if gone { format!("File not found\n{}", m.path.display()) } else { format!("{}\n{}", m.file_name(), m.path.display()) };
    resp.on_hover_text(tip).context_menu(|ui| {
        if ui.button("Add to timeline at playhead").clicked() {
            actions.push(Action::Add(m.id));
            ui.close();
        }
        if ui.button("Reveal in folder").clicked() {
            actions.push(Action::Reveal(m.path.clone()));
            ui.close();
        }
        ui.separator();
        if ui.button("Remove from project").clicked() {
            actions.push(Action::Remove(m.id));
            ui.close();
        }
    });
}

fn tile(
    ui: &mut egui::Ui,
    m: &Media,
    thumbs: &mut crate::thumbs::Thumbs,
    sel: bool,
    gone: bool,
    size: egui::Vec2,
    actions: &mut Vec<Action>,
) {
    item_ui(ui, m, size, gone, actions, |ui, rect, hovered| {
        let p = ui.painter();
        if hovered {
            p.rect_filled(rect, CornerRadius::same(theme::R_INNER), theme::BG_HOVER);
        }
        let th = Rect::from_min_size(rect.min, egui::vec2(rect.width(), rect.width() * 9.0 / 16.0));
        paint_thumb(ui, th, m, thumbs, gone);
        if sel {
            p.rect_stroke(th, CornerRadius::same(theme::R_INNER), Stroke::new(2.0, theme::ACCENT), StrokeKind::Inside);
        }
        let color = if sel { theme::TEXT } else { theme::TEXT_MUTED };
        let g = clipped(ui, &m.file_name(), FontId::proportional(theme::SIZE_SMALL), color, rect.width());
        p.galley(egui::pos2(rect.min.x, th.max.y + theme::SP_1), g, color);
    });
}

fn row(ui: &mut egui::Ui, m: &Media, thumbs: &mut crate::thumbs::Thumbs, sel: bool, gone: bool, actions: &mut Vec<Action>) {
    let size = egui::vec2(ui.available_width(), ROW_H);
    item_ui(ui, m, size, gone, actions, |ui, rect, hovered| {
        let p = ui.painter();
        let r = CornerRadius::same(theme::R_INNER);
        if hovered {
            p.rect_filled(rect, r, theme::BG_HOVER);
        }
        let th = Rect::from_min_size(rect.min + egui::vec2(theme::SP_1, (ROW_H - ROW_THUMB.y) / 2.0), ROW_THUMB);
        paint_thumb(ui, th, m, thumbs, gone);
        if sel {
            p.rect_stroke(rect, r, Stroke::new(1.5, theme::ACCENT), StrokeKind::Inside);
        }
        let dur = if m.kind == MediaKind::Image { String::new() } else { theme::timecode(m.duration, m.fps.max(1.0)) };
        let dg = p.layout_no_wrap(dur, FontId::monospace(theme::SIZE_MONO), theme::TEXT_MUTED);
        let right = rect.max.x - theme::SP_2;
        let dw = dg.size().x;
        p.galley(egui::pos2(right - dw, rect.center().y - dg.size().y / 2.0), dg, theme::TEXT_MUTED);
        let x = th.max.x + theme::SP_2;
        let w = (right - dw - theme::SP_2 - x).max(20.0);
        let name = clipped(ui, &m.file_name(), FontId::proportional(theme::SIZE_BODY), theme::TEXT, w);
        let meta = clipped(ui, &meta_text(m), FontId::proportional(theme::SIZE_SMALL), theme::TEXT_MUTED, w);
        let ny = name.size().y;
        let y = rect.center().y - (ny + meta.size().y) / 2.0;
        p.galley(egui::pos2(x, y), name, theme::TEXT);
        p.galley(egui::pos2(x, y + ny), meta, theme::TEXT_MUTED);
    });
}

fn meta_text(m: &Media) -> String {
    match m.kind {
        MediaKind::Audio => format!("{} Hz  {}", m.sample_rate, m.codec),
        MediaKind::Image => format!("{}x{}  {}", m.width, m.height, m.codec),
        MediaKind::Video => format!("{}x{}  {:.2} fps  {}", m.width, m.height, m.fps, m.codec),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn pending_tile(ui: &mut egui::Ui, path: &Path, w: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0 + 22.0), Sense::hover());
    let th = Rect::from_min_size(rect.min, egui::vec2(w, w * 9.0 / 16.0));
    ui.painter().rect_filled(th, CornerRadius::same(theme::R_INNER), theme::BG_RAISED);
    ui.put(Rect::from_center_size(th.center(), egui::vec2(20.0, 20.0)), egui::Spinner::new());
    let g = clipped(ui, &file_name(path), FontId::proportional(theme::SIZE_SMALL), theme::TEXT_MUTED, w);
    ui.painter().galley(egui::pos2(rect.min.x, th.max.y + theme::SP_1), g, theme::TEXT_MUTED);
}

fn pending_row(ui: &mut egui::Ui, path: &Path) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), Sense::hover());
    let th = Rect::from_min_size(rect.min + egui::vec2(theme::SP_1, (ROW_H - ROW_THUMB.y) / 2.0), ROW_THUMB);
    ui.painter().rect_filled(th, CornerRadius::same(theme::R_INNER), theme::BG_RAISED);
    ui.put(Rect::from_center_size(th.center(), egui::vec2(20.0, 20.0)), egui::Spinner::new());
    let x = th.max.x + theme::SP_2;
    let g = clipped(ui, &file_name(path), FontId::proportional(theme::SIZE_BODY), theme::TEXT_MUTED, rect.max.x - x - theme::SP_2);
    ui.painter().galley(egui::pos2(x, rect.center().y - g.size().y / 2.0), g, theme::TEXT_MUTED);
}
