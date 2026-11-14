use crate::theme;
use eframe::egui::{
    self, epaint::PathShape, vec2, Color32, CornerRadius, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, WidgetInfo,
};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    SkipStart,
    SkipEnd,
    StepBack,
    StepForward,
    Loop,
    Select,
    Razor,
    Slip,
    Magnet,
    Ripple,
    Undo,
    Redo,
    Import,
    Plus,
    Trash,
    Eye,
    EyeOff,
    Speaker,
    SpeakerOff,
    Lock,
    Unlock,
    Text,
    Effect,
    Transition,
    Crop,
    InPoint,
    OutPoint,
    ZoomIn,
    ZoomOut,
    Fit,
    Film,
    Music,
    Close,
    Minimize,
    Maximize,
    Restore,
    ChevronDown,
    Grip,
    Check,
    Warning,
}

struct Pen<'a> {
    painter: &'a Painter,
    origin: Pos2,
    scale: f32,
    color: Color32,
    stroke: Stroke,
}

impl Pen<'_> {
    fn pt(&self, p: (f32, f32)) -> Pos2 {
        self.origin + vec2(p.0, p.1) * self.scale
    }

    fn pts(&self, points: &[(f32, f32)]) -> Vec<Pos2> {
        points.iter().map(|&p| self.pt(p)).collect()
    }

    fn line(&self, points: &[(f32, f32)]) {
        self.painter.add(PathShape::line(self.pts(points), self.stroke));
    }

    fn closed(&self, points: &[(f32, f32)]) {
        self.painter.add(PathShape::closed_line(self.pts(points), self.stroke));
    }

    fn fill(&self, points: &[(f32, f32)]) {
        self.painter.add(Shape::convex_polygon(self.pts(points), self.color, Stroke::NONE));
    }

    fn circle(&self, c: (f32, f32), r: f32) {
        self.painter.circle_stroke(self.pt(c), r * self.scale, self.stroke);
    }

    fn dot(&self, c: (f32, f32), r: f32) {
        self.painter.circle_filled(self.pt(c), r * self.scale, self.color);
    }

    fn rect(&self, min: (f32, f32), max: (f32, f32), radius: u8) {
        let rect = Rect::from_min_max(self.pt(min), self.pt(max));
        self.painter.rect_stroke(rect, CornerRadius::same(radius), self.stroke, StrokeKind::Middle);
    }

    fn block(&self, min: (f32, f32), max: (f32, f32), radius: u8) {
        let rect = Rect::from_min_max(self.pt(min), self.pt(max));
        self.painter.rect_filled(rect, CornerRadius::same(radius), self.color);
    }
}

fn arc(c: (f32, f32), r: f32, a0: f32, a1: f32) -> Vec<(f32, f32)> {
    (0..=12).map(|i| a0 + (a1 - a0) * i as f32 / 12.0).map(|a| (c.0 + r * a.cos(), c.1 + r * a.sin())).collect()
}

fn chain(parts: &[&[(f32, f32)]]) -> Vec<(f32, f32)> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

fn flip(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    points.iter().map(|&(x, y)| (16.0 - x, y)).collect()
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let scale = rect.width().min(rect.height()) / 16.0;
    let pen = Pen {
        painter,
        origin: rect.center() - vec2(8.0, 8.0) * scale,
        scale,
        color,
        stroke: Stroke::new(theme::ICON_STROKE * scale, color),
    };
    match icon {
        Icon::Play => pen.fill(&[(5.0, 3.0), (13.0, 8.0), (5.0, 13.0)]),
        Icon::Pause => {
            pen.block((4.0, 3.0), (6.5, 13.0), 1);
            pen.block((9.5, 3.0), (12.0, 13.0), 1);
        }
        Icon::SkipStart => {
            pen.line(&[(3.5, 3.0), (3.5, 13.0)]);
            pen.closed(&[(13.0, 3.5), (6.0, 8.0), (13.0, 12.5)]);
        }
        Icon::SkipEnd => {
            pen.line(&[(12.5, 3.0), (12.5, 13.0)]);
            pen.closed(&[(3.0, 3.5), (10.0, 8.0), (3.0, 12.5)]);
        }
        Icon::StepBack => {
            pen.line(&[(3.5, 3.0), (3.5, 13.0)]);
            pen.line(&[(11.5, 3.5), (6.5, 8.0), (11.5, 12.5)]);
        }
        Icon::StepForward => {
            pen.line(&[(12.5, 3.0), (12.5, 13.0)]);
            pen.line(&[(4.5, 3.5), (9.5, 8.0), (4.5, 12.5)]);
        }
        Icon::Loop => {
            pen.line(&[(11.0, 2.5), (13.5, 5.0), (11.0, 7.5)]);
            pen.line(&[(2.5, 8.0), (2.5, 7.0), (4.5, 5.0), (13.5, 5.0)]);
            pen.line(&[(5.0, 8.5), (2.5, 11.0), (5.0, 13.5)]);
            pen.line(&[(13.5, 8.0), (13.5, 9.0), (11.5, 11.0), (2.5, 11.0)]);
        }
        Icon::Select => pen.closed(&[(4.0, 2.5), (4.0, 12.5), (6.8, 10.0), (9.0, 14.0), (10.8, 13.1), (8.7, 9.2), (12.5, 9.0)]),
        Icon::Razor => {
            pen.circle((4.0, 4.0), 2.0);
            pen.circle((4.0, 12.0), 2.0);
            pen.line(&[(5.6, 5.4), (13.5, 12.5)]);
            pen.line(&[(5.6, 10.6), (13.5, 3.5)]);
        }
        Icon::Slip => {
            pen.line(&[(4.5, 3.0), (2.5, 3.0), (2.5, 13.0), (4.5, 13.0)]);
            pen.line(&[(11.5, 3.0), (13.5, 3.0), (13.5, 13.0), (11.5, 13.0)]);
            pen.line(&[(5.5, 8.0), (10.5, 8.0)]);
            pen.line(&[(7.0, 6.3), (5.5, 8.0), (7.0, 9.7)]);
            pen.line(&[(9.0, 6.3), (10.5, 8.0), (9.0, 9.7)]);
        }
        Icon::Magnet => {
            pen.line(&chain(&[&[(2.5, 2.5), (2.5, 7.0)], &arc((8.0, 7.0), 5.5, PI, 0.0), &[(13.5, 2.5)]]));
            pen.line(&chain(&[&[(5.5, 2.5), (5.5, 7.0)], &arc((8.0, 7.0), 2.5, PI, 0.0), &[(10.5, 2.5)]]));
            pen.line(&[(2.5, 2.5), (5.5, 2.5)]);
            pen.line(&[(10.5, 2.5), (13.5, 2.5)]);
            pen.line(&[(2.5, 5.0), (5.5, 5.0)]);
            pen.line(&[(10.5, 5.0), (13.5, 5.0)]);
        }
        Icon::Ripple => {
            pen.line(&[(2.0, 8.0), (7.5, 8.0)]);
            pen.line(&[(5.3, 5.8), (7.5, 8.0), (5.3, 10.2)]);
            pen.rect((9.5, 3.5), (13.5, 12.5), 1);
        }
        Icon::Undo | Icon::Redo => {
            let path = chain(&[&[(3.0, 6.0), (9.5, 6.0)], &arc((9.5, 9.5), 3.5, -PI / 2.0, PI / 2.0), &[(5.0, 13.0)]]);
            let head = [(6.0, 3.0), (3.0, 6.0), (6.0, 9.0)];
            if icon == Icon::Undo {
                pen.line(&path);
                pen.line(&head);
            } else {
                pen.line(&flip(&path));
                pen.line(&flip(&head));
            }
        }
        Icon::Import => {
            pen.line(&[(2.5, 10.0), (2.5, 13.0), (13.5, 13.0), (13.5, 10.0)]);
            pen.line(&[(8.0, 2.5), (8.0, 10.0)]);
            pen.line(&[(5.0, 7.2), (8.0, 10.2), (11.0, 7.2)]);
        }
        Icon::Plus => {
            pen.line(&[(8.0, 3.0), (8.0, 13.0)]);
            pen.line(&[(3.0, 8.0), (13.0, 8.0)]);
        }
        Icon::Trash => {
            pen.line(&[(2.5, 4.0), (13.5, 4.0)]);
            pen.line(&[(6.0, 4.0), (6.0, 2.5), (10.0, 2.5), (10.0, 4.0)]);
            pen.line(&[(4.0, 4.0), (4.8, 13.5), (11.2, 13.5), (12.0, 4.0)]);
            pen.line(&[(6.8, 7.0), (6.8, 11.0)]);
            pen.line(&[(9.2, 7.0), (9.2, 11.0)]);
        }
        Icon::Eye | Icon::EyeOff => {
            let lid = |sign: f32| -> Vec<(f32, f32)> {
                (0..=16).map(|i| -1.0 + 2.0 * i as f32 / 16.0).map(|u| (8.0 + 6.5 * u, 8.0 + sign * 4.2 * (1.0 - u * u))).collect()
            };
            pen.closed(&chain(&[&lid(-1.0), &lid(1.0)[1..16].iter().rev().copied().collect::<Vec<_>>()]));
            pen.circle((8.0, 8.0), 2.2);
            if icon == Icon::EyeOff {
                pen.line(&[(2.5, 13.5), (13.5, 2.5)]);
            }
        }
        Icon::Speaker | Icon::SpeakerOff => {
            pen.closed(&[(2.5, 6.0), (5.5, 6.0), (9.0, 3.0), (9.0, 13.0), (5.5, 10.0), (2.5, 10.0)]);
            if icon == Icon::Speaker {
                pen.line(&arc((9.0, 8.0), 3.2, -PI / 4.0, PI / 4.0));
                pen.line(&arc((9.0, 8.0), 5.8, -PI / 4.0, PI / 4.0));
            } else {
                pen.line(&[(11.5, 6.0), (14.5, 10.0)]);
                pen.line(&[(14.5, 6.0), (11.5, 10.0)]);
            }
        }
        Icon::Lock | Icon::Unlock => {
            pen.rect((3.0, 7.5), (13.0, 14.0), 2);
            if icon == Icon::Lock {
                pen.line(&chain(&[&[(5.0, 7.5), (5.0, 5.0)], &arc((8.0, 5.0), 3.0, PI, TAU), &[(11.0, 7.5)]]));
            } else {
                pen.line(&chain(&[&[(5.0, 7.5), (5.0, 5.0)], &arc((8.0, 5.0), 3.0, PI, TAU * 0.92)]));
            }
            pen.line(&[(8.0, 10.0), (8.0, 11.5)]);
        }
        Icon::Text => {
            pen.line(&[(3.0, 3.5), (13.0, 3.5)]);
            pen.line(&[(8.0, 3.5), (8.0, 13.0)]);
            pen.line(&[(6.0, 13.0), (10.0, 13.0)]);
        }
        Icon::Effect => {
            pen.closed(&[(6.5, 3.5), (7.8, 7.7), (12.0, 9.0), (7.8, 10.3), (6.5, 14.5), (5.2, 10.3), (1.0, 9.0), (5.2, 7.7)]);
            pen.line(&[(13.0, 1.5), (13.0, 4.5)]);
            pen.line(&[(11.5, 3.0), (14.5, 3.0)]);
        }
        Icon::Transition => {
            pen.closed(&[(2.5, 4.0), (8.0, 8.0), (2.5, 12.0)]);
            pen.closed(&[(13.5, 4.0), (8.0, 8.0), (13.5, 12.0)]);
        }
        Icon::Crop => {
            pen.line(&[(4.5, 1.5), (4.5, 11.5), (14.5, 11.5)]);
            pen.line(&[(11.5, 14.5), (11.5, 4.5), (1.5, 4.5)]);
        }
        Icon::InPoint | Icon::OutPoint => {
            let mut rects = [((3.5, 2.5), (5.5, 13.5)), ((5.5, 2.5), (10.5, 4.5)), ((5.5, 11.5), (10.5, 13.5))];
            if icon == Icon::OutPoint {
                rects = rects.map(|((x0, y0), (x1, y1))| ((16.0 - x1, y0), (16.0 - x0, y1)));
            }
            for (min, max) in rects {
                pen.block(min, max, 0);
            }
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            pen.circle((7.0, 7.0), 4.5);
            pen.line(&[(10.3, 10.3), (13.8, 13.8)]);
            pen.line(&[(5.0, 7.0), (9.0, 7.0)]);
            if icon == Icon::ZoomIn {
                pen.line(&[(7.0, 5.0), (7.0, 9.0)]);
            }
        }
        Icon::Fit => {
            pen.line(&[(2.5, 6.0), (2.5, 2.5), (6.0, 2.5)]);
            pen.line(&[(10.0, 2.5), (13.5, 2.5), (13.5, 6.0)]);
            pen.line(&[(13.5, 10.0), (13.5, 13.5), (10.0, 13.5)]);
            pen.line(&[(6.0, 13.5), (2.5, 13.5), (2.5, 10.0)]);
        }
        Icon::Film => {
            pen.rect((2.5, 2.5), (13.5, 13.5), 2);
            pen.line(&[(5.5, 2.5), (5.5, 13.5)]);
            pen.line(&[(10.5, 2.5), (10.5, 13.5)]);
            for y in [5.5, 8.0, 10.5] {
                pen.line(&[(2.5, y), (5.5, y)]);
                pen.line(&[(10.5, y), (13.5, y)]);
            }
        }
        Icon::Music => {
            pen.circle((4.5, 11.5), 2.0);
            pen.circle((11.5, 10.5), 2.0);
            pen.line(&[(6.5, 11.5), (6.5, 3.5), (13.5, 2.5), (13.5, 10.5)]);
            pen.line(&[(6.5, 6.0), (13.5, 5.0)]);
        }
        Icon::Close => {
            pen.line(&[(3.5, 3.5), (12.5, 12.5)]);
            pen.line(&[(12.5, 3.5), (3.5, 12.5)]);
        }
        Icon::Minimize => pen.line(&[(3.0, 8.0), (13.0, 8.0)]),
        Icon::Maximize => pen.rect((3.0, 3.0), (13.0, 13.0), 1),
        Icon::Restore => {
            pen.rect((3.0, 5.5), (10.5, 13.0), 1);
            pen.line(&[(5.5, 5.5), (5.5, 3.0), (13.0, 3.0), (13.0, 10.5), (10.5, 10.5)]);
        }
        Icon::ChevronDown => pen.line(&[(4.0, 6.0), (8.0, 10.0), (12.0, 6.0)]),
        Icon::Grip => {
            for x in [5.5, 10.5] {
                for y in [3.5, 8.0, 12.5] {
                    pen.dot((x, y), 1.0);
                }
            }
        }
        Icon::Check => pen.line(&[(3.0, 8.5), (6.5, 12.0), (13.0, 4.5)]),
        Icon::Warning => {
            pen.closed(&[(8.0, 2.0), (14.5, 13.5), (1.5, 13.5)]);
            pen.line(&[(8.0, 6.0), (8.0, 9.5)]);
            pen.dot((8.0, 11.7), 0.8);
        }
    }
}

fn widget(ui: &mut Ui, icon: Icon, on: bool, enabled: bool, tooltip: &str) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), sense);
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, enabled, tooltip));
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(theme::R_INNER);
        let down = enabled && resp.is_pointer_button_down_on();
        let fill = if down || on {
            Some(theme::BG_ACTIVE)
        } else if enabled && resp.hovered() {
            Some(theme::BG_HOVER)
        } else {
            None
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, radius, fill);
        }
        let color = if !enabled {
            theme::TEXT_FAINT
        } else if on {
            theme::ACCENT
        } else if resp.hovered() || down {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        };
        paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(theme::ICON, theme::ICON)), icon, color);
        if resp.has_focus() {
            ui.painter().rect_stroke(rect, radius, Stroke::new(1.0, theme::ACCENT), StrokeKind::Inside);
        }
    }
    resp.on_hover_text(tooltip)
}

pub fn button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    widget(ui, icon, false, true, tooltip)
}

pub fn toggle(ui: &mut Ui, icon: Icon, on: bool, tooltip: &str) -> Response {
    widget(ui, icon, on, true, tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Icon; 42] = [
        Icon::Play,
        Icon::Pause,
        Icon::SkipStart,
        Icon::SkipEnd,
        Icon::StepBack,
        Icon::StepForward,
        Icon::Loop,
        Icon::Select,
        Icon::Razor,
        Icon::Slip,
        Icon::Magnet,
        Icon::Ripple,
        Icon::Undo,
        Icon::Redo,
        Icon::Import,
        Icon::Plus,
        Icon::Trash,
        Icon::Eye,
        Icon::EyeOff,
        Icon::Speaker,
        Icon::SpeakerOff,
        Icon::Lock,
        Icon::Unlock,
        Icon::Text,
        Icon::Effect,
        Icon::Transition,
        Icon::Crop,
        Icon::InPoint,
        Icon::OutPoint,
        Icon::ZoomIn,
        Icon::ZoomOut,
        Icon::Fit,
        Icon::Film,
        Icon::Music,
        Icon::Close,
        Icon::Minimize,
        Icon::Maximize,
        Icon::Restore,
        Icon::ChevronDown,
        Icon::Grip,
        Icon::Check,
        Icon::Warning,
    ];

    #[test]
    fn paints_and_tessellates_every_icon() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            for icon in ALL {
                button(ui, icon, "icon");
                toggle(ui, icon, true, "icon");
            }
        });
        out.textures_delta.clear();
        assert!(!ctx.tessellate(out.shapes, 1.0).is_empty());
    }
}
