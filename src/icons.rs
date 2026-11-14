use eframe::egui::{self, Color32, Painter, Rect, Response, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Stop,
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
    Export,
    Plus,
    Minus,
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
    Marker,
    InPoint,
    OutPoint,
    ZoomIn,
    ZoomOut,
    Fit,
    Folder,
    Film,
    Music,
    Image,
    Search,
    Close,
    ChevronDown,
    ChevronRight,
    Settings,
    Grip,
    Check,
    Warning,
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let _ = (icon, color);
    painter.rect_stroke(rect.shrink(3.0), 1.0, (1.0, color), egui::StrokeKind::Inside);
}

pub fn button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    toggle(ui, icon, false, tooltip)
}

pub fn toggle(ui: &mut Ui, icon: Icon, on: bool, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    paint(ui.painter(), rect, icon, if on { crate::theme::ACCENT } else { crate::theme::TEXT_MUTED });
    resp.on_hover_text(tooltip)
}
