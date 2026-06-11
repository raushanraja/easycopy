use crate::config::Config;
use egui;

// ── Theme Colors ─────────────────────────────────────────────────

/// A set of custom colors for a named visual theme (dark, light, etc.).
/// When [`Config::enable_theming`] is true, these override egui's
/// default widget colours to give the app a distinctive look.
pub struct ThemeColors {
    // Core backgrounds
    pub window_bg: egui::Color32,
    pub panel_bg: egui::Color32,
    pub extreme_bg: egui::Color32,
    pub widget_inactive_bg: egui::Color32,
    pub widget_hovered_bg: egui::Color32,
    pub widget_active_bg: egui::Color32,
    pub widget_border: egui::Color32,

    // Selection
    pub selection_bg: egui::Color32,
    pub selection_stroke: egui::Color32,

    // Card / row
    pub card_bg: egui::Color32,
    pub card_bg_hovered: egui::Color32,
    pub card_bg_selected: egui::Color32,
    pub card_stroke: egui::Color32,
    pub card_stroke_selected: egui::Color32,
    pub card_rounding: f32,
    pub selection_bar: egui::Color32,

    // Accent
    pub accent: egui::Color32,
    pub accent_light: egui::Color32,
    pub accent_dark: egui::Color32,

    // Badge & icons
    pub badge_bg_selected: egui::Color32,
    pub badge_bg_normal: egui::Color32,
    pub badge_icon_color: egui::Color32,
    pub icon_color_badge_normal: egui::Color32,

    // Lightbox
    pub lightbox_overlay: egui::Color32,
    pub lightbox_control_bg: egui::Color32,
    pub lightbox_close_btn_bg: egui::Color32,
    pub lightbox_icon: egui::Color32,
    pub lightbox_icon_hovered: egui::Color32,

    // Misc
    pub shortcut_color: egui::Color32,
}

impl ThemeColors {
    pub fn dark() -> Self {
        Self {
            window_bg: egui::Color32::from_rgb(11, 15, 25), // Slate 950
            panel_bg: egui::Color32::from_rgb(11, 15, 25),
            extreme_bg: egui::Color32::from_rgb(20, 26, 38), // Slate 900
            widget_inactive_bg: egui::Color32::from_rgb(20, 26, 38),
            widget_hovered_bg: egui::Color32::from_rgb(28, 35, 51),
            widget_active_bg: egui::Color32::from_rgb(51, 65, 85),
            widget_border: egui::Color32::from_rgb(33, 41, 54), // Slate 800
            selection_bg: egui::Color32::from_rgb(79, 70, 229), // Indigo 600
            selection_stroke: egui::Color32::from_rgb(129, 140, 248), // Indigo 400
            card_bg: egui::Color32::from_rgb(15, 20, 30),       // Slate 950 variant
            card_bg_hovered: egui::Color32::from_rgb(20, 26, 38), // Slate 900
            card_bg_selected: egui::Color32::from_rgb(30, 27, 75), // Indigo 950
            card_stroke: egui::Color32::from_rgb(33, 41, 54),   // Slate 800
            card_stroke_selected: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            card_rounding: 12.0,
            selection_bar: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            accent: egui::Color32::from_rgb(99, 102, 241),        // Indigo 500
            accent_light: egui::Color32::from_rgb(129, 140, 248), // Indigo 400
            accent_dark: egui::Color32::from_rgb(79, 70, 229),    // Indigo 600
            badge_bg_selected: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25),
            badge_bg_normal: egui::Color32::from_rgb(20, 26, 38), // Slate 900
            badge_icon_color: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            icon_color_badge_normal: egui::Color32::from_rgb(99, 102, 241),
            lightbox_overlay: egui::Color32::from_rgba_unmultiplied(11, 15, 25, 220),
            lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(30, 41, 59, 200), // Slate 800
            lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
            lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
            lightbox_icon_hovered: egui::Color32::WHITE,
            shortcut_color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
        }
    }

    pub fn light() -> Self {
        Self {
            window_bg: egui::Color32::from_rgb(248, 250, 252), // Slate 50
            panel_bg: egui::Color32::from_rgb(248, 250, 252),
            extreme_bg: egui::Color32::from_rgb(241, 245, 249), // Slate 100
            widget_inactive_bg: egui::Color32::from_rgb(241, 245, 249),
            widget_hovered_bg: egui::Color32::from_rgb(226, 232, 240),
            widget_active_bg: egui::Color32::from_rgb(203, 213, 225),
            widget_border: egui::Color32::from_rgb(226, 232, 240), // Slate 200
            selection_bg: egui::Color32::from_rgb(79, 70, 229),    // Indigo 600
            selection_stroke: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            card_bg: egui::Color32::from_rgb(255, 255, 255),       // White
            card_bg_hovered: egui::Color32::from_rgb(241, 245, 249), // Slate 100
            card_bg_selected: egui::Color32::from_rgb(224, 231, 255), // Indigo 100
            card_stroke: egui::Color32::from_rgb(226, 232, 240),   // Slate 200
            card_stroke_selected: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            card_rounding: 12.0,
            selection_bar: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            accent: egui::Color32::from_rgb(99, 102, 241),        // Indigo 500
            accent_light: egui::Color32::from_rgb(129, 140, 248), // Indigo 400
            accent_dark: egui::Color32::from_rgb(79, 70, 229),    // Indigo 600
            badge_bg_selected: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 15),
            badge_bg_normal: egui::Color32::from_rgb(241, 245, 249), // Slate 100
            badge_icon_color: egui::Color32::from_rgb(99, 102, 241), // Indigo 500
            icon_color_badge_normal: egui::Color32::from_rgb(99, 102, 241),
            lightbox_overlay: egui::Color32::from_rgba_unmultiplied(11, 15, 25, 220),
            lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(30, 41, 59, 200),
            lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
            lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
            lightbox_icon_hovered: egui::Color32::WHITE,
            shortcut_color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        }
    }

    /// Return the theme palette based on config, or `None` if theming is disabled.
    pub fn from_config(config: &Config) -> Option<Self> {
        if !config.general.enable_theming {
            return None;
        }
        Some(match config.general.theme.as_str() {
            "light" => Self::light(),
            _ => Self::dark(),
        })
    }
}

// ── egui theming ─────────────────────────────────────────────────

/// Apply the configured theme (dark / light / system) and font sizes to
/// the egui context.
pub fn apply_theme_and_fonts(ctx: &egui::Context, theme: &str, enable_theming: bool) {
    if !enable_theming {
        return;
    }

    let mut visuals = if theme == "light" {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };

    if theme == "light" {
        visuals.window_fill = egui::Color32::from_rgb(248, 250, 252); // Slate 50
        visuals.panel_fill = egui::Color32::from_rgb(248, 250, 252);
        visuals.extreme_bg_color = egui::Color32::from_rgb(241, 245, 249); // Slate 100
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(241, 245, 249);
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)); // Slate 200
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(241, 245, 249);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(226, 232, 240);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(203, 213, 225);
        visuals.selection.bg_fill = egui::Color32::from_rgb(79, 70, 229); // Indigo 600
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(99, 102, 241));
    } else {
        visuals.window_fill = egui::Color32::from_rgb(11, 15, 25); // Slate 950
        visuals.panel_fill = egui::Color32::from_rgb(11, 15, 25);
        visuals.extreme_bg_color = egui::Color32::from_rgb(20, 26, 38); // Slate 900
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 26, 38);
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(33, 41, 54)); // Slate 800
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(20, 26, 38);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(28, 35, 51);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(51, 65, 85);
        visuals.selection.bg_fill = egui::Color32::from_rgb(79, 70, 229); // Indigo 600
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(129, 140, 248));
        // Indigo 400
    }

    visuals.window_rounding = egui::Rounding::same(16.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(0.0);
    style.visuals.window_rounding = egui::Rounding::same(16.0);

    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.5));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(15.0));

    ctx.set_style(style);
}

// ── custom vector icons drawn programmatically ───────────────────────

pub fn paint_search_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.8, color);

    // Search circle
    let radius = (rect.width() * 0.32).min(rect.height() * 0.32);
    let center = rect.center() - egui::vec2(1.5, 1.5);
    painter.circle_stroke(center, radius, stroke);

    // Handle line
    let start = center + egui::vec2(radius * 0.707, radius * 0.707);
    let end = rect.right_bottom() - egui::vec2(1.5, 1.5);
    painter.line_segment([start, end], stroke);
}

pub fn paint_close_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.8, color);
    painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
}

pub fn paint_text_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Document boundary
    painter.rect_stroke(rect, egui::Rounding::same(1.5), stroke);

    // Document text lines
    let line_y1 = rect.top() + rect.height() * 0.3;
    let line_y2 = rect.top() + rect.height() * 0.55;
    let line_y3 = rect.top() + rect.height() * 0.8;

    painter.line_segment(
        [
            egui::pos2(rect.left() + 3.0, line_y1),
            egui::pos2(rect.right() - 3.0, line_y1),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 3.0, line_y2),
            egui::pos2(rect.right() - 3.0, line_y2),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 3.0, line_y3),
            egui::pos2(rect.right() - 6.0, line_y3),
        ],
        stroke,
    );
}

pub fn paint_image_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Image border frame
    painter.rect_stroke(rect, egui::Rounding::same(1.5), stroke);

    // Sun
    let sun_center = rect.left_top() + egui::vec2(rect.width() * 0.3, rect.height() * 0.3);
    painter.circle_stroke(
        sun_center,
        rect.width() * 0.1,
        egui::Stroke::new(1.2, color),
    );

    // Mountains
    let p1 = egui::pos2(rect.left() + 2.0, rect.bottom() - 2.0);
    let p2 = egui::pos2(
        rect.left() + rect.width() * 0.4,
        rect.top() + rect.height() * 0.45,
    );
    let p3 = egui::pos2(rect.left() + rect.width() * 0.6, rect.bottom() - 4.0);
    let p4 = egui::pos2(
        rect.left() + rect.width() * 0.8,
        rect.top() + rect.height() * 0.55,
    );
    let p5 = egui::pos2(rect.right() - 2.0, rect.bottom() - 2.0);

    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p4], stroke);
    painter.line_segment([p4, p5], stroke);
}

pub fn paint_trash_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Lid line
    painter.line_segment(
        [
            egui::pos2(rect.left() - 1.0, rect.top() + rect.height() * 0.2),
            egui::pos2(rect.right() + 1.0, rect.top() + rect.height() * 0.2),
        ],
        stroke,
    );

    // Lid handle on top
    let handle_w = rect.width() * 0.3;
    let handle_h = rect.height() * 0.15;
    let handle_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + handle_h / 2.0),
        egui::vec2(handle_w, handle_h),
    );
    painter.rect_stroke(handle_rect, egui::Rounding::same(0.5), stroke);

    // Trash body
    let bin_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, rect.top() + rect.height() * 0.25),
        egui::pos2(rect.right() - 2.0, rect.bottom()),
    );
    painter.rect_stroke(bin_rect, egui::Rounding::same(1.0), stroke);

    // Ribs
    painter.line_segment(
        [
            egui::pos2(rect.center().x - 1.5, rect.top() + rect.height() * 0.4),
            egui::pos2(rect.center().x - 1.5, rect.bottom() - 2.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.center().x + 1.5, rect.top() + rect.height() * 0.4),
            egui::pos2(rect.center().x + 1.5, rect.bottom() - 2.0),
        ],
        stroke,
    );
}

pub fn paint_settings_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let r_outer = rect.width() * 0.35;
    let r_inner = rect.width() * 0.15;
    let tooth_len = rect.width() * 0.12;
    let stroke = egui::Stroke::new(1.5, color);

    // Draw center hole
    painter.circle_stroke(center, r_inner, stroke);

    // Draw outer base ring
    painter.circle_stroke(center, r_outer, stroke);

    // Draw 8 teeth around the ring
    let num_teeth = 8;
    for i in 0..num_teeth {
        let angle = (i as f32) * (2.0 * std::f32::consts::PI / (num_teeth as f32));
        let cos = angle.cos();
        let sin = angle.sin();

        // Tooth base position on outer ring
        let p_base = egui::pos2(center.x + r_outer * cos, center.y + r_outer * sin);
        // Tooth tip position
        let p_tip = egui::pos2(
            center.x + (r_outer + tooth_len) * cos,
            center.y + (r_outer + tooth_len) * sin,
        );

        painter.line_segment([p_base, p_tip], stroke);
    }
}

pub fn paint_open_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let s = rect.width() / 12.0;
    let stroke = egui::Stroke::new(1.2 * s, color);

    let x = rect.left();
    let y = rect.top();

    // 1. Draw the box/bracket shape with rounded-diagonal corners
    // Top segment of the box
    painter.line_segment(
        [
            egui::pos2(x + 5.1 * s, y + 2.4 * s),
            egui::pos2(x + 3.2 * s, y + 2.4 * s),
        ],
        stroke,
    );
    // Top-left corner
    painter.line_segment(
        [
            egui::pos2(x + 3.2 * s, y + 2.4 * s),
            egui::pos2(x + 2.4 * s, y + 3.2 * s),
        ],
        stroke,
    );
    // Left edge
    painter.line_segment(
        [
            egui::pos2(x + 2.4 * s, y + 3.2 * s),
            egui::pos2(x + 2.4 * s, y + 8.8 * s),
        ],
        stroke,
    );
    // Bottom-left corner
    painter.line_segment(
        [
            egui::pos2(x + 2.4 * s, y + 8.8 * s),
            egui::pos2(x + 3.2 * s, y + 9.6 * s),
        ],
        stroke,
    );
    // Bottom edge
    painter.line_segment(
        [
            egui::pos2(x + 3.2 * s, y + 9.6 * s),
            egui::pos2(x + 8.8 * s, y + 9.6 * s),
        ],
        stroke,
    );
    // Bottom-right corner
    painter.line_segment(
        [
            egui::pos2(x + 8.8 * s, y + 9.6 * s),
            egui::pos2(x + 9.6 * s, y + 8.8 * s),
        ],
        stroke,
    );
    // Right edge
    painter.line_segment(
        [
            egui::pos2(x + 9.6 * s, y + 8.8 * s),
            egui::pos2(x + 9.6 * s, y + 6.9 * s),
        ],
        stroke,
    );

    // 2. Draw the diagonal arrow line
    painter.line_segment(
        [
            egui::pos2(x + 5.8 * s, y + 5.8 * s),
            egui::pos2(x + 9.6 * s, y + 2.4 * s),
        ],
        stroke,
    );

    // 3. Draw the arrowhead L-shape (top-right)
    // Horizontal segment of arrowhead
    painter.line_segment(
        [
            egui::pos2(x + 7.2 * s, y + 2.4 * s),
            egui::pos2(x + 9.6 * s, y + 2.4 * s),
        ],
        stroke,
    );
    // Vertical segment of arrowhead
    painter.line_segment(
        [
            egui::pos2(x + 9.6 * s, y + 2.4 * s),
            egui::pos2(x + 9.6 * s, y + 4.8 * s),
        ],
        stroke,
    );
}

/// Draw a circular icon badge (used as a visual prefix for clip rows).
pub fn draw_icon_badge(
    ui: &mut egui::Ui,
    icon_type: &str,
    is_selected: bool,
    theme: Option<&ThemeColors>,
) {
    let size = egui::vec2(36.0, 36.0);
    let (rect, _) = ui.allocate_exact_size(
        size,
        egui::Sense {
            click: false,
            drag: false,
            focusable: false,
        },
    );

    let bg_color = if is_selected {
        theme.map_or_else(
            || {
                if ui.visuals().dark_mode {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                } else {
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 15)
                }
            },
            |t| t.badge_bg_selected,
        )
    } else {
        theme.map_or_else(
            || ui.visuals().widgets.noninteractive.bg_fill,
            |t| t.badge_bg_normal,
        )
    };

    let icon_color = if is_selected {
        theme.map_or_else(|| ui.visuals().text_color(), |t| t.badge_icon_color)
    } else {
        theme.map_or_else(|| ui.visuals().text_color(), |t| t.icon_color_badge_normal)
    };

    ui.painter().circle_filled(rect.center(), 18.0, bg_color);

    // Draw the actual icon centered inside the badge (16x16 size)
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0));
    match icon_type {
        "text" => paint_text_icon(ui, icon_rect, icon_color),
        "image" => paint_image_icon(ui, icon_rect, icon_color),
        _ => {}
    }
}
