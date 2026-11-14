use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, RichText, Shadow, Stroke, TextStyle,
};
use std::sync::Arc;

pub const BG_APP: Color32 = Color32::from_rgb(0x0d, 0x0e, 0x10);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x16, 0x17, 0x1a);
pub const BG_RAISED: Color32 = Color32::from_rgb(0x1e, 0x20, 0x24);
pub const BG_HOVER: Color32 = Color32::from_rgb(0x27, 0x2a, 0x2f);
pub const BG_ACTIVE: Color32 = Color32::from_rgb(0x31, 0x35, 0x3b);
pub const BG_INPUT: Color32 = Color32::from_rgb(0x11, 0x12, 0x14);
pub const BORDER: Color32 = Color32::from_rgb(0x2a, 0x2d, 0x33);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x3b, 0x3f, 0x47);

pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xea, 0xed);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xa3, 0xa9, 0xb3);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x70, 0x76, 0x80);

pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0xae, 0x1f);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xff, 0xc1, 0x52);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x5c, 0x40, 0x0f);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x1a, 0x12, 0x00);

pub const DANGER: Color32 = Color32::from_rgb(0xf0, 0x5a, 0x5e);
pub const SUCCESS: Color32 = Color32::from_rgb(0x45, 0xc0, 0x6b);
pub const INFO: Color32 = Color32::from_rgb(0x6a, 0xa8, 0xff);

pub const CLIP_VIDEO: Color32 = Color32::from_rgb(0x3f, 0x5a, 0xa8);
pub const CLIP_AUDIO: Color32 = Color32::from_rgb(0x2b, 0x86, 0x74);
pub const CLIP_TEXT: Color32 = Color32::from_rgb(0x84, 0x58, 0xbd);
pub const CLIP_MATTE: Color32 = Color32::from_rgb(0x5d, 0x63, 0x6d);

pub const CLIP_LABELS: [Color32; 8] = [
    CLIP_VIDEO,
    Color32::from_rgb(0xb8, 0x4a, 0x4f),
    Color32::from_rgb(0xc0, 0x7a, 0x2a),
    Color32::from_rgb(0xa9, 0x9a, 0x2c),
    Color32::from_rgb(0x3d, 0x93, 0x52),
    Color32::from_rgb(0x2f, 0x8b, 0x9c),
    CLIP_TEXT,
    Color32::from_rgb(0xb0, 0x4f, 0x8f),
];

pub const SP_1: f32 = 4.0;
pub const SP_2: f32 = 8.0;
pub const SP_3: f32 = 12.0;
pub const SP_4: f32 = 16.0;
pub const SP_5: f32 = 24.0;

pub const R_OUTER: u8 = 8;
pub const PAD: u8 = 4;
pub const R_INNER: u8 = R_OUTER - PAD;

pub const SIZE_BODY: f32 = 13.0;
pub const SIZE_SMALL: f32 = 11.5;
pub const SIZE_HEADING: f32 = 14.0;
pub const SIZE_MONO: f32 = 12.0;
pub const ICON: f32 = 16.0;
pub const ICON_STROKE: f32 = 1.5;

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn mono(text: impl Into<String>) -> RichText {
    RichText::new(text).family(FontFamily::Monospace).size(SIZE_MONO)
}

pub fn strong(text: impl Into<String>) -> RichText {
    RichText::new(text).family(semibold()).color(TEXT)
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(TEXT_MUTED)
}

pub fn small(text: impl Into<String>) -> RichText {
    RichText::new(text).size(SIZE_SMALL).color(TEXT_MUTED)
}

pub fn heading(text: impl Into<String>) -> RichText {
    RichText::new(text).family(semibold()).size(SIZE_HEADING).color(TEXT)
}

pub fn timecode(t: f64, fps: f64) -> String {
    let fps = fps.max(1.0);
    let total = (t.max(0.0) * fps).round() as u64;
    let f = total % fps.round() as u64;
    let s = total / fps.round() as u64;
    format!("{:02}:{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60, f)
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let label = RichText::new(text).family(semibold()).color(ON_ACCENT);
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        w.inactive.weak_bg_fill = ACCENT;
        w.inactive.bg_fill = ACCENT;
        w.hovered.weak_bg_fill = ACCENT_HOVER;
        w.hovered.bg_fill = ACCENT_HOVER;
        w.active.weak_bg_fill = ACCENT_HOVER;
        w.active.bg_fill = ACCENT_HOVER;
        for s in [&mut w.inactive, &mut w.hovered, &mut w.active] {
            s.bg_stroke = Stroke::NONE;
        }
        ui.spacing_mut().button_padding = egui::vec2(SP_3 + 2.0, SP_1);
        ui.add(egui::Button::new(label).min_size(egui::vec2(0.0, 28.0)))
    })
    .inner
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new().fill(BG_PANEL).inner_margin(Margin::same(PAD as i8)).corner_radius(CornerRadius::same(R_OUTER))
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert("inter".into(), Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf"))));
    fonts.font_data.insert("inter-semibold".into(), Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Inter-SemiBold.ttf"))));
    fonts.font_data.insert("mono".into(), Arc::new(FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf"))));
    fonts.families.insert(FontFamily::Proportional, vec!["inter".into(), "mono".into()]);
    fonts.families.insert(semibold(), vec!["inter-semibold".into(), "inter".into()]);
    fonts.families.insert(FontFamily::Monospace, vec!["mono".into(), "inter".into()]);
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Dark);

    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(SIZE_SMALL)),
            (TextStyle::Body, FontId::proportional(SIZE_BODY)),
            (TextStyle::Button, FontId::proportional(SIZE_BODY)),
            (TextStyle::Heading, FontId::new(SIZE_HEADING, semibold())),
            (TextStyle::Monospace, FontId::monospace(SIZE_MONO)),
        ]
        .into();

        let s = &mut style.spacing;
        s.item_spacing = egui::vec2(SP_2, SP_1 + 2.0);
        s.button_padding = egui::vec2(SP_3 - 2.0, SP_1);
        s.interact_size = egui::vec2(28.0, 24.0);
        s.menu_margin = Margin::same(PAD as i8);
        s.window_margin = Margin::same(SP_4 as i8);
        s.slider_width = 140.0;
        s.combo_width = 140.0;
        s.indent = SP_4;
        s.icon_width = 12.0;
        s.icon_width_inner = 6.0;
        s.icon_spacing = SP_2;

        style.interaction.tooltip_delay = 0.35;
        style.animation_time = 0.12;

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = None;
        v.panel_fill = BG_APP;
        v.window_fill = BG_RAISED;
        v.window_stroke = Stroke::new(1.0, BORDER_STRONG);
        v.window_corner_radius = CornerRadius::same(R_OUTER + 2);
        v.menu_corner_radius = CornerRadius::same(R_OUTER);
        v.window_shadow = Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(140) };
        v.popup_shadow = Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::from_black_alpha(120) };
        v.extreme_bg_color = BG_INPUT;
        v.faint_bg_color = BG_RAISED;
        v.code_bg_color = BG_INPUT;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = ACCENT;
        v.error_fg_color = DANGER;
        v.selection.bg_fill = ACCENT_DIM;
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.slider_trailing_fill = true;
        v.striped = false;

        let r = CornerRadius::same(R_INNER);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = BG_PANEL;
        w.noninteractive.weak_bg_fill = BG_PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
        w.noninteractive.corner_radius = r;

        w.inactive.bg_fill = BG_ACTIVE;
        w.inactive.weak_bg_fill = BG_RAISED;
        w.inactive.bg_stroke = Stroke::NONE;
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        w.inactive.corner_radius = r;

        w.hovered.bg_fill = BORDER_STRONG;
        w.hovered.weak_bg_fill = BG_HOVER;
        w.hovered.bg_stroke = Stroke::NONE;
        w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
        w.hovered.corner_radius = r;
        w.hovered.expansion = 0.0;

        w.active.bg_fill = ACCENT;
        w.active.weak_bg_fill = BG_ACTIVE;
        w.active.bg_stroke = Stroke::NONE;
        w.active.fg_stroke = Stroke::new(1.0, TEXT);
        w.active.corner_radius = r;
        w.active.expansion = 0.0;

        w.open.bg_fill = BG_ACTIVE;
        w.open.weak_bg_fill = BG_HOVER;
        w.open.bg_stroke = Stroke::NONE;
        w.open.fg_stroke = Stroke::new(1.0, TEXT);
        w.open.corner_radius = r;
    });
}
