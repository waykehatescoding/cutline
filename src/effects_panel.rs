use crate::app::{App, DragPayload, ToastKind};
use crate::edit_ops;
use crate::icons::{self, Icon};
use crate::model::{Effect, EffectKind, TransitionKind};
use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, Sense};

const ROW_H: f32 = 28.0;

const MATTES: [(&str, [u8; 4]); 4] =
    [("Black", [0, 0, 0, 255]), ("White", [255, 255, 255, 255]), ("Gray", [128, 128, 128, 255]), ("Green screen", [0, 177, 64, 255])];

enum Glyph {
    Icon(Icon),
    Swatch(Color32),
}

fn effect_hint(kind: &EffectKind) -> &'static str {
    match kind {
        EffectKind::Blur { region: None, .. } => "Softens the whole frame.",
        EffectKind::Blur { .. } => "Blurs only a rectangle you place on the frame. Use it to hide faces or plates.",
        EffectKind::Pixelate { region: None, .. } => "Turns the whole frame into large blocks.",
        EffectKind::Pixelate { .. } => "Pixelates only a rectangle you place on the frame. Use it to censor details.",
        EffectKind::ColorAdjust { .. } => "Adjusts brightness, contrast, saturation and gamma.",
        EffectKind::WhiteBalance { .. } => "Shifts temperature and tint to neutralize color casts.",
        EffectKind::HueShift { .. } => "Rotates every color around the color wheel.",
        EffectKind::Grayscale => "Removes all color from the clip.",
        EffectKind::Sepia { .. } => "Warm brown monochrome tone for an aged look.",
        EffectKind::Invert => "Flips every color to its opposite.",
        EffectKind::Vignette { .. } => "Darkens the edges to pull focus to the center.",
        EffectKind::Sharpen { .. } => "Boosts edge detail for a crisper image.",
        EffectKind::Noise { .. } => "Adds animated grain for a filmic texture.",
        EffectKind::ChromaKey { .. } => "Makes a chosen color transparent, such as a green screen.",
    }
}

fn transition_hint(kind: TransitionKind) -> &'static str {
    match kind {
        TransitionKind::Crossfade => "Blends the previous clip into this one.",
        TransitionKind::FadeThroughBlack => "Fades the previous clip to black, then up into this one.",
        TransitionKind::WipeLeft => "A hard edge sweeps from right to left, revealing this clip.",
        TransitionKind::WipeRight => "A hard edge sweeps from left to right, revealing this clip.",
        TransitionKind::SlideLeft => "This clip pushes the previous one out to the left.",
        TransitionKind::SlideRight => "This clip pushes the previous one out to the right.",
        TransitionKind::ZoomIn => "Zooms into the previous clip while this one takes over.",
    }
}

fn row(ui: &mut egui::Ui, glyph: &Glyph, name: &str, hint: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), Sense::click_and_drag());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(theme::R_INNER), theme::BG_HOVER);
    }
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + theme::SP_2, rect.center().y - theme::ICON / 2.0),
        egui::Vec2::splat(theme::ICON),
    );
    match glyph {
        Glyph::Icon(i) => icons::paint(ui.painter(), icon_rect, *i, theme::TEXT_MUTED),
        Glyph::Swatch(c) => {
            let r = icon_rect.shrink(2.0);
            ui.painter().rect_filled(r, CornerRadius::same(2), *c);
            ui.painter().rect_stroke(r, CornerRadius::same(2), egui::Stroke::new(1.0, theme::BORDER_STRONG), egui::StrokeKind::Inside);
        }
    }
    ui.painter().text(
        egui::pos2(icon_rect.right() + theme::SP_2, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(theme::SIZE_BODY),
        theme::TEXT,
    );
    resp.on_hover_text(hint)
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(theme::SP_3);
    ui.label(theme::heading(title));
    ui.add_space(theme::SP_1);
}

fn applied(app: &mut App, what: &str) {
    app.notify(format!("Applied {what}"), ToastKind::Success);
}

fn apply_effect(app: &mut App, kind: &EffectKind) {
    let Some(id) = app.primary_clip() else {
        return app.notify("Select a clip first", ToastKind::Info);
    };
    app.checkpoint();
    if let Some(clip) = app.project.clip_mut(id) {
        clip.effects.push(Effect { enabled: true, kind: kind.clone() });
    }
    applied(app, kind.name());
}

fn apply_transition(app: &mut App, kind: TransitionKind) {
    let Some(id) = app.primary_clip() else {
        return app.notify("Select a clip first", ToastKind::Info);
    };
    app.checkpoint();
    edit_ops::set_transition(&mut app.project, id, Some(kind), 0.5);
    applied(app, kind.name());
}

fn apply_title(app: &mut App) {
    app.checkpoint();
    let at = app.playhead;
    match edit_ops::add_text(&mut app.project, at) {
        Some(id) => {
            app.selection = vec![id];
            applied(app, "title");
        }
        None => app.notify("No room for a title here", ToastKind::Error),
    }
}

fn apply_matte(app: &mut App, name: &str, color: [u8; 4]) {
    app.checkpoint();
    let at = app.playhead;
    match edit_ops::add_matte(&mut app.project, color, at) {
        Some(id) => {
            app.selection = vec![id];
            applied(app, &format!("{} matte", name.to_lowercase()));
        }
        None => app.notify("No room for a matte here", ToastKind::Error),
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let id = egui::Id::unique("effects_search");
    let mut query: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.add(
        egui::TextEdit::singleline(&mut query)
            .hint_text("Search effects")
            .desired_width(f32::INFINITY)
            .margin(egui::vec2(theme::SP_2, theme::SP_1)),
    );
    let q = query.trim().to_lowercase();
    ui.data_mut(|d| d.insert_temp(id, query));
    let hit = |name: &str| q.is_empty() || name.to_lowercase().contains(&q);

    let mut found = false;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;

        let effects: Vec<EffectKind> = EffectKind::catalog().into_iter().filter(|k| hit(k.name())).collect();
        if !effects.is_empty() {
            found = true;
            section(ui, "Effects");
            for kind in effects {
                let r = row(ui, &Glyph::Icon(Icon::Effect), kind.name(), effect_hint(&kind));
                r.dnd_set_drag_payload(DragPayload::Effect(kind.clone()));
                if r.double_clicked() {
                    apply_effect(app, &kind);
                }
            }
        }

        let transitions: Vec<TransitionKind> = TransitionKind::ALL.into_iter().filter(|k| hit(k.name())).collect();
        if !transitions.is_empty() {
            found = true;
            section(ui, "Transitions");
            for kind in transitions {
                let r = row(ui, &Glyph::Icon(Icon::Transition), kind.name(), transition_hint(kind));
                r.dnd_set_drag_payload(DragPayload::Transition(kind));
                if r.double_clicked() {
                    apply_transition(app, kind);
                }
            }
        }

        if hit("Title") {
            found = true;
            section(ui, "Titles");
            let r = row(ui, &Glyph::Icon(Icon::Text), "Title", "Adds an editable text clip at the playhead.");
            r.dnd_set_drag_payload(DragPayload::Title);
            if r.double_clicked() {
                apply_title(app);
            }
        }

        let mattes: Vec<_> = MATTES.iter().filter(|(n, _)| hit(n)).collect();
        if !mattes.is_empty() {
            found = true;
            section(ui, "Mattes");
            for (name, c) in mattes {
                let swatch = Color32::from_rgb(c[0], c[1], c[2]);
                let r = row(ui, &Glyph::Swatch(swatch), name, "Adds a solid color clip at the playhead.");
                r.dnd_set_drag_payload(DragPayload::Matte(*c));
                if r.double_clicked() {
                    apply_matte(app, name, *c);
                }
            }
        }

        if !found {
            ui.add_space(theme::SP_3);
            ui.label(theme::muted("No matches"));
        }
    });
}
