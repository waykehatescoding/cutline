use crate::app::App;
use crate::icons::{self, Icon};
use crate::{commands, monitor, theme};
use eframe::egui::{self, pos2, vec2, Align, Color32, FontId, Key, Layout, Margin, Modifiers, Rect, Sense, UiBuilder, Vec2, ViewportCommand};

const BAR_H: f32 = 64.0;
const SKIP: f64 = 10.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
    keys(app, &ctx, fullscreen);
    if !fullscreen {
        egui::Panel::top("player_title")
            .exact_size(commands::TITLE_H)
            .frame(egui::Frame::new().fill(theme::BG_APP))
            .show(ui, |ui| title(app, ui));
    }
    egui::Panel::bottom("player_controls")
        .exact_size(BAR_H)
        .frame(egui::Frame::new().fill(theme::BG_PANEL).inner_margin(Margin::symmetric(theme::SP_3 as i8, 0)))
        .show(ui, |ui| controls(app, ui, fullscreen));
    egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::BLACK)).show(ui, |ui| screen(app, ui, fullscreen));
}

fn clock(t: f64) -> String {
    let s = t.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

fn apply_volume(app: &mut App) {
    app.audio.set_volume(if app.muted { 0.0 } else { app.volume * app.volume });
}

fn keys(app: &mut App, ctx: &egui::Context, fullscreen: bool) {
    let pressed = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
    if pressed(Key::Space) {
        app.toggle_play();
    }
    if pressed(Key::ArrowLeft) {
        app.seek(app.playhead - SKIP);
    }
    if pressed(Key::ArrowRight) {
        app.seek(app.playhead + SKIP);
    }
    if pressed(Key::M) {
        app.muted = !app.muted;
        apply_volume(app);
    }
    if fullscreen && pressed(Key::Escape) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
    }
}

fn title(app: &App, ui: &mut egui::Ui) {
    let bar = ui.max_rect();
    let maximized = commands::title_drag(ui);
    let name = app.original.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let galley = ui.painter().layout_no_wrap(format!("{name} - Cutline"), FontId::proportional(theme::SIZE_BODY), theme::TEXT_MUTED);
    ui.painter().galley(bar.center() - galley.size() / 2.0, galley, theme::TEXT_MUTED);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        commands::caption_buttons(ui, maximized);
    });
}

fn screen(app: &mut App, ui: &mut egui::Ui, fullscreen: bool) {
    let area = ui.max_rect();
    if app.project.duration() <= 0.0 {
        ui.put(Rect::from_center_size(area.center(), Vec2::splat(24.0)), egui::Spinner::new().size(24.0));
        return;
    }
    monitor::screen(app, ui, area);
    let resp = ui.interact(area, ui.scope_id().with("screen"), Sense::click());
    if resp.double_clicked() {
        app.toggle_play();
        ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(!fullscreen));
    } else if resp.clicked() {
        app.toggle_play();
    }
}

fn track(ui: &egui::Ui, rect: Rect, frac: f32, hot: bool) {
    let h = if hot { 5.0 } else { 3.0 };
    let bar = Rect::from_center_size(rect.center(), vec2(rect.width(), h));
    let x = bar.left() + bar.width() * frac.clamp(0.0, 1.0);
    let p = ui.painter();
    p.rect_filled(bar, h / 2.0, theme::BG_ACTIVE);
    p.rect_filled(Rect::from_min_max(bar.min, pos2(x, bar.max.y)), h / 2.0, theme::ACCENT);
    if hot {
        p.circle_filled(pos2(x, bar.center().y), 6.0, theme::ACCENT_HOVER);
    }
}

fn seek_bar(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let resp = ui.interact(rect, ui.scope_id().with("seek"), Sense::click_and_drag());
    let dur = app.project.duration();
    let at = |x: f32| ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * dur;
    let resume = resp.id.with("resume");
    if resp.drag_started() {
        let was = app.playing;
        ui.data_mut(|d| d.insert_temp(resume, was));
        app.stop();
    }
    if let Some(p) = resp.interact_pointer_pos() {
        if resp.dragged() {
            app.playhead = at(p.x);
        } else if resp.clicked() {
            app.seek(at(p.x));
        }
    }
    if resp.drag_stopped() && ui.data(|d| d.get_temp(resume)).unwrap_or(false) {
        app.play();
    }
    let frac = if dur > 0.0 { (app.playhead / dur) as f32 } else { 0.0 };
    track(ui, rect, frac, resp.hovered() || resp.dragged());
    resp.widget_info(|| egui::WidgetInfo::slider(true, app.playhead, "Seek"));
    if let Some(p) = resp.hover_pos() {
        resp.on_hover_text_at_pointer(clock(at(p.x)));
    }
}

fn volume_bar(app: &mut App, ui: &mut egui::Ui) {
    let (rect, resp) = ui.allocate_exact_size(vec2(90.0, 28.0), Sense::click_and_drag());
    if let (true, Some(p)) = (resp.clicked() || resp.dragged(), resp.interact_pointer_pos()) {
        app.volume = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        app.muted = false;
        apply_volume(app);
    }
    track(ui, rect, if app.muted { 0.0 } else { app.volume }, resp.hovered() || resp.dragged());
    resp.widget_info(|| egui::WidgetInfo::slider(true, app.volume as f64, "Volume"));
    resp.on_hover_text(format!("Volume {}%", (app.volume * 100.0).round()));
}

fn controls(app: &mut App, ui: &mut egui::Ui, fullscreen: bool) {
    let full = ui.max_rect();
    let seek = Rect::from_min_size(full.min + vec2(0.0, theme::SP_1), vec2(full.width(), 14.0));
    seek_bar(app, ui, seek);
    let row = Rect::from_min_max(pos2(full.min.x, seek.max.y), full.max);

    let mid = Rect::from_center_size(row.center(), vec2(5.0 * 28.0 + 34.0 + 5.0 * theme::SP_1, row.height()));
    ui.scope_builder(UiBuilder::new().id_salt("player_mid").max_rect(mid).layout(Layout::left_to_right(Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = theme::SP_1;
        if icons::toggle(ui, Icon::Loop, app.loop_playback, "Repeat").clicked() {
            app.loop_playback = !app.loop_playback;
        }
        if icons::button(ui, Icon::Stop, "Stop").clicked() {
            app.stop();
            app.seek(0.0);
        }
        if icons::button(ui, Icon::StepBack, "Back 10 seconds").clicked() {
            app.seek(app.playhead - SKIP);
        }
        monitor::play_button(app, ui);
        if icons::button(ui, Icon::StepForward, "Forward 10 seconds").clicked() {
            app.seek(app.playhead + SKIP);
        }
    });

    let left = Rect::from_min_max(row.min, pos2(mid.min.x - theme::SP_2, row.max.y));
    ui.scope_builder(UiBuilder::new().id_salt("player_time").max_rect(left).layout(Layout::left_to_right(Align::Center)), |ui| {
        ui.label(theme::mono(format!("{} / {}", clock(app.playhead), clock(app.project.duration()))).color(theme::TEXT_MUTED));
    });

    let right = Rect::from_min_max(pos2(mid.max.x + theme::SP_2, row.min.y), row.max);
    ui.scope_builder(UiBuilder::new().id_salt("player_right").max_rect(right).layout(Layout::right_to_left(Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = theme::SP_1;
        if icons::button(ui, Icon::Edit, "Open in editor").clicked() {
            app.player = false;
            if fullscreen {
                ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(false));
            }
        }
        let (icon, tip) = if fullscreen { (Icon::ExitFullscreen, "Exit full screen") } else { (Icon::Fullscreen, "Full screen") };
        if icons::button(ui, icon, tip).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Fullscreen(!fullscreen));
        }
        ui.add_space(theme::SP_2);
        volume_bar(app, ui);
        let silent = app.muted || app.volume == 0.0;
        let (icon, tip) = if silent { (Icon::SpeakerOff, "Unmute") } else { (Icon::Speaker, "Mute") };
        if icons::button(ui, icon, tip).clicked() {
            if app.volume == 0.0 {
                app.volume = 1.0;
                app.muted = false;
            } else {
                app.muted = !app.muted;
            }
            apply_volume(app);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_clock() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(75.9), "1:15");
        assert_eq!(clock(3725.0), "1:02:05");
    }
}
