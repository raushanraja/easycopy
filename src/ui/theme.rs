use crate::config::{Config, FontPreset, FontSize, FontWeight, Theme};
use egui;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Global debug-logging flag – set once from config at startup.
static DEBUG_LOGGING: AtomicBool = AtomicBool::new(false);

/// Enable or disable verbose diagnostic logging (font resolution, etc.).
pub fn set_debug_logging(enabled: bool) {
    DEBUG_LOGGING.store(enabled, Ordering::Relaxed);
}

/// Check whether debug logging is enabled.
pub fn is_debug_logging() -> bool {
    DEBUG_LOGGING.load(Ordering::Relaxed)
}

/// Emit a diagnostic message only when debug logging is enabled.
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        if DEBUG_LOGGING.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    }};
}

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

    // Text colors
    pub text_color: egui::Color32,
    pub weak_text_color: egui::Color32,
}

/// Generate a theme palette constructor from inline field:value data.
/// Replaces the repetitive `pub fn name() -> Self { Self { ... } }` pattern.
macro_rules! theme_colors {
    ($name:ident { $($field:ident: $value:expr),* $(,)? }) => {
        pub fn $name() -> ThemeColors {
            ThemeColors { $($field: $value),* }
        }
    };
}

impl ThemeColors {
    theme_colors!(dark {
        window_bg: egui::Color32::from_rgb(11, 15, 25),
        panel_bg: egui::Color32::from_rgb(11, 15, 25),
        extreme_bg: egui::Color32::from_rgb(20, 26, 38),
        widget_inactive_bg: egui::Color32::from_rgb(20, 26, 38),
        widget_hovered_bg: egui::Color32::from_rgb(28, 35, 51),
        widget_active_bg: egui::Color32::from_rgb(51, 65, 85),
        widget_border: egui::Color32::from_rgb(33, 41, 54),
        selection_bg: egui::Color32::from_rgb(79, 70, 229),
        selection_stroke: egui::Color32::from_rgb(129, 140, 248),
        card_bg: egui::Color32::from_rgb(15, 20, 30),
        card_bg_hovered: egui::Color32::from_rgb(20, 26, 38),
        card_bg_selected: egui::Color32::from_rgb(30, 27, 75),
        card_stroke: egui::Color32::from_rgb(33, 41, 54),
        card_stroke_selected: egui::Color32::from_rgb(99, 102, 241),
        card_rounding: 12.0,
        selection_bar: egui::Color32::from_rgb(99, 102, 241),
        accent: egui::Color32::from_rgb(99, 102, 241),
        accent_light: egui::Color32::from_rgb(129, 140, 248),
        accent_dark: egui::Color32::from_rgb(79, 70, 229),
        badge_bg_selected: egui::Color32::from_rgb(20, 26, 38),
        badge_bg_normal: egui::Color32::from_rgb(20, 26, 38),
        badge_icon_color: egui::Color32::from_rgb(99, 102, 241),
        icon_color_badge_normal: egui::Color32::from_rgb(99, 102, 241),
        lightbox_overlay: egui::Color32::from_rgba_unmultiplied(11, 15, 25, 220),
        lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(30, 41, 59, 200),
        lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
        lightbox_icon_hovered: egui::Color32::WHITE,
        shortcut_color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
        text_color: egui::Color32::from_rgb(241, 245, 249),
        weak_text_color: egui::Color32::from_rgb(148, 163, 184),
    });

    theme_colors!(light {
        window_bg: egui::Color32::from_rgb(248, 250, 252),
        panel_bg: egui::Color32::from_rgb(248, 250, 252),
        extreme_bg: egui::Color32::from_rgb(241, 245, 249),
        widget_inactive_bg: egui::Color32::from_rgb(241, 245, 249),
        widget_hovered_bg: egui::Color32::from_rgb(226, 232, 240),
        widget_active_bg: egui::Color32::from_rgb(203, 213, 225),
        widget_border: egui::Color32::from_rgb(226, 232, 240),
        selection_bg: egui::Color32::from_rgb(79, 70, 229),
        selection_stroke: egui::Color32::from_rgb(99, 102, 241),
        card_bg: egui::Color32::from_rgb(255, 255, 255),
        card_bg_hovered: egui::Color32::from_rgb(241, 245, 249),
        card_bg_selected: egui::Color32::from_rgb(224, 231, 255),
        card_stroke: egui::Color32::from_rgb(226, 232, 240),
        card_stroke_selected: egui::Color32::from_rgb(99, 102, 241),
        card_rounding: 12.0,
        selection_bar: egui::Color32::from_rgb(99, 102, 241),
        accent: egui::Color32::from_rgb(99, 102, 241),
        accent_light: egui::Color32::from_rgb(129, 140, 248),
        accent_dark: egui::Color32::from_rgb(79, 70, 229),
        badge_bg_selected: egui::Color32::from_rgb(226, 232, 240),
        badge_bg_normal: egui::Color32::from_rgb(241, 245, 249),
        badge_icon_color: egui::Color32::from_rgb(99, 102, 241),
        icon_color_badge_normal: egui::Color32::from_rgb(99, 102, 241),
        lightbox_overlay: egui::Color32::from_rgba_unmultiplied(11, 15, 25, 220),
        lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(30, 41, 59, 200),
        lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
        lightbox_icon_hovered: egui::Color32::WHITE,
        shortcut_color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        text_color: egui::Color32::from_rgb(15, 23, 42),
        weak_text_color: egui::Color32::from_rgb(100, 116, 139),
    });

    theme_colors!(nord {
        window_bg: egui::Color32::from_rgb(46, 52, 64),
        panel_bg: egui::Color32::from_rgb(46, 52, 64),
        extreme_bg: egui::Color32::from_rgb(59, 66, 82),
        widget_inactive_bg: egui::Color32::from_rgb(59, 66, 82),
        widget_hovered_bg: egui::Color32::from_rgb(67, 76, 94),
        widget_active_bg: egui::Color32::from_rgb(76, 86, 106),
        widget_border: egui::Color32::from_rgb(67, 76, 94),
        selection_bg: egui::Color32::from_rgb(94, 129, 172),
        selection_stroke: egui::Color32::from_rgb(136, 192, 208),
        card_bg: egui::Color32::from_rgb(46, 52, 64),
        card_bg_hovered: egui::Color32::from_rgb(59, 66, 82),
        card_bg_selected: egui::Color32::from_rgb(67, 76, 94),
        card_stroke: egui::Color32::from_rgb(59, 66, 82),
        card_stroke_selected: egui::Color32::from_rgb(136, 192, 208),
        card_rounding: 12.0,
        selection_bar: egui::Color32::from_rgb(136, 192, 208),
        accent: egui::Color32::from_rgb(136, 192, 208),
        accent_light: egui::Color32::from_rgb(143, 188, 187),
        accent_dark: egui::Color32::from_rgb(129, 161, 193),
        badge_bg_selected: egui::Color32::from_rgb(59, 66, 82),
        badge_bg_normal: egui::Color32::from_rgb(59, 66, 82),
        badge_icon_color: egui::Color32::from_rgb(136, 192, 208),
        icon_color_badge_normal: egui::Color32::from_rgb(136, 192, 208),
        lightbox_overlay: egui::Color32::from_rgba_unmultiplied(36, 41, 51, 230),
        lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(46, 52, 64, 200),
        lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
        lightbox_icon_hovered: egui::Color32::WHITE,
        shortcut_color: egui::Color32::from_rgba_unmultiplied(216, 222, 233, 180),
        text_color: egui::Color32::from_rgb(236, 239, 244),
        weak_text_color: egui::Color32::from_rgb(162, 175, 195),
    });

    theme_colors!(catppuccin {
        window_bg: egui::Color32::from_rgb(30, 30, 46),
        panel_bg: egui::Color32::from_rgb(30, 30, 46),
        extreme_bg: egui::Color32::from_rgb(24, 24, 37),
        widget_inactive_bg: egui::Color32::from_rgb(24, 24, 37),
        widget_hovered_bg: egui::Color32::from_rgb(49, 50, 68),
        widget_active_bg: egui::Color32::from_rgb(67, 76, 94),
        widget_border: egui::Color32::from_rgb(49, 50, 68),
        selection_bg: egui::Color32::from_rgb(137, 180, 250),
        selection_stroke: egui::Color32::from_rgb(180, 190, 254),
        card_bg: egui::Color32::from_rgb(30, 30, 46),
        card_bg_hovered: egui::Color32::from_rgb(24, 24, 37),
        card_bg_selected: egui::Color32::from_rgb(49, 50, 68),
        card_stroke: egui::Color32::from_rgb(49, 50, 68),
        card_stroke_selected: egui::Color32::from_rgb(180, 190, 254),
        card_rounding: 12.0,
        selection_bar: egui::Color32::from_rgb(180, 190, 254),
        accent: egui::Color32::from_rgb(180, 190, 254),
        accent_light: egui::Color32::from_rgb(203, 166, 247),
        accent_dark: egui::Color32::from_rgb(116, 199, 236),
        badge_bg_selected: egui::Color32::from_rgb(24, 24, 37),
        badge_bg_normal: egui::Color32::from_rgb(24, 24, 37),
        badge_icon_color: egui::Color32::from_rgb(180, 190, 254),
        icon_color_badge_normal: egui::Color32::from_rgb(180, 190, 254),
        lightbox_overlay: egui::Color32::from_rgba_unmultiplied(17, 17, 27, 230),
        lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(30, 30, 46, 200),
        lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
        lightbox_icon_hovered: egui::Color32::WHITE,
        shortcut_color: egui::Color32::from_rgba_unmultiplied(205, 214, 244, 180),
        text_color: egui::Color32::from_rgb(205, 214, 244),
        weak_text_color: egui::Color32::from_rgb(166, 173, 200),
    });

    theme_colors!(dracula {
        window_bg: egui::Color32::from_rgb(40, 42, 54),
        panel_bg: egui::Color32::from_rgb(40, 42, 54),
        extreme_bg: egui::Color32::from_rgb(68, 71, 90),
        widget_inactive_bg: egui::Color32::from_rgb(68, 71, 90),
        widget_hovered_bg: egui::Color32::from_rgb(90, 93, 115),
        widget_active_bg: egui::Color32::from_rgb(98, 114, 164),
        widget_border: egui::Color32::from_rgb(98, 114, 164),
        selection_bg: egui::Color32::from_rgb(189, 147, 249),
        selection_stroke: egui::Color32::from_rgb(80, 250, 123),
        card_bg: egui::Color32::from_rgb(40, 42, 54),
        card_bg_hovered: egui::Color32::from_rgb(68, 71, 90),
        card_bg_selected: egui::Color32::from_rgb(50, 52, 67),
        card_stroke: egui::Color32::from_rgb(68, 71, 90),
        card_stroke_selected: egui::Color32::from_rgb(189, 147, 249),
        card_rounding: 12.0,
        selection_bar: egui::Color32::from_rgb(189, 147, 249),
        accent: egui::Color32::from_rgb(189, 147, 249),
        accent_light: egui::Color32::from_rgb(255, 121, 198),
        accent_dark: egui::Color32::from_rgb(139, 233, 253),
        badge_bg_selected: egui::Color32::from_rgb(68, 71, 90),
        badge_bg_normal: egui::Color32::from_rgb(68, 71, 90),
        badge_icon_color: egui::Color32::from_rgb(189, 147, 249),
        icon_color_badge_normal: egui::Color32::from_rgb(189, 147, 249),
        lightbox_overlay: egui::Color32::from_rgba_unmultiplied(20, 21, 28, 230),
        lightbox_control_bg: egui::Color32::from_rgba_unmultiplied(40, 42, 54, 200),
        lightbox_close_btn_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        lightbox_icon: egui::Color32::from_rgb(200, 200, 200),
        lightbox_icon_hovered: egui::Color32::WHITE,
        shortcut_color: egui::Color32::from_rgba_unmultiplied(248, 248, 242, 180),
        text_color: egui::Color32::from_rgb(248, 248, 242),
        weak_text_color: egui::Color32::from_rgb(160, 172, 206),
    });

    pub fn from_config(config: &Config) -> Option<Self> {
        if !config.general.enable_theming {
            return None;
        }
        Some(config.general.theme.resolve())
    }
}

// ── egui theming ─────────────────────────────────────────────────

/// Font size values for each preset.
fn font_size_values(size: FontSize) -> (f32, f32, f32, f32, f32) {
    match size {
        FontSize::Small => (19.0, 13.0, 11.5, 10.0, 12.0),
        FontSize::Large => (25.0, 19.0, 17.5, 16.0, 18.0),
        FontSize::Medium => (22.0, 16.0, 14.5, 13.0, 15.0),
    }
}

/// Find a font file by name in standard Linux font directories.
/// Searches up to 3 levels deep to handle nested structures like
/// `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`.
fn find_font_file(filename: &str) -> Option<std::path::PathBuf> {
    let mut dirs = vec![
        std::path::PathBuf::from("/usr/share/fonts"),
        std::path::PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".fonts"));
        dirs.push(home.join(".local/share/fonts"));
    }

    for base in &dirs {
        // Direct hit
        let direct = base.join(filename);
        if direct.exists() {
            return Some(direct);
        }
        // Walk up to 3 levels
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let l1 = entry.path();
                if !l1.is_dir() {
                    continue;
                }
                let candidate = l1.join(filename);
                if candidate.exists() {
                    return Some(candidate);
                }
                if let Ok(sub) = std::fs::read_dir(&l1) {
                    for sub_entry in sub.flatten() {
                        let l2 = sub_entry.path();
                        if !l2.is_dir() {
                            continue;
                        }
                        let candidate = l2.join(filename);
                        if candidate.exists() {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Generate both `.ttf` and `.otf` variants of a base filename stem.
fn with_ext(stem: &str) -> Vec<String> {
    vec![format!("{}.ttf", stem), format!("{}.otf", stem)]
}

/// Fallback: query fontconfig for a font file path by family name.
/// This handles Nerd Font, variable font, and other naming schemes that
/// our file-scanner doesn't know about.
///
/// Uses `fc-list` first to verify the family actually exists (otherwise
/// `fc-match` silently falls back to a default, giving us a wrong file).
fn resolve_via_fc_match(pattern: &str) -> Option<std::path::PathBuf> {
    // Step 1: does this family actually exist?
    let list = std::process::Command::new("fc-list")
        .arg(pattern)
        .output()
        .ok()?;
    if !list.status.success() || list.stdout.is_empty() {
        return None;
    }

    // Step 2: get the file path
    let output = std::process::Command::new("fc-match")
        .args(["--format", "%{file}", pattern])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            let p = std::path::PathBuf::from(&path);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// Try each name in order and return the first one found.
/// For each name, both `.ttf` and `.otf` are attempted automatically.
/// If file scanning fails, falls back to `fc-match` for fontconfig resolution.
fn try_find(names: &[String]) -> Option<String> {
    // Phase 1: scan known directories
    for name in names {
        let stem = name
            .strip_suffix(".ttf")
            .or_else(|| name.strip_suffix(".otf"))
            .unwrap_or(name);
        for candidate in with_ext(stem) {
            if let Some(path) = find_font_file(&candidate) {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    // Phase 2: ask fontconfig (handles Nerd Font, etc.)
    for name in names {
        let stem = name
            .strip_suffix(".ttf")
            .or_else(|| name.strip_suffix(".otf"))
            .unwrap_or(name);
        if let Some(path) = resolve_via_fc_match(stem) {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

/// Resolve font file paths for a given preset name and weight.
/// Returns (proportional_path, monospace_path).
/// Tries weight-specific variants first, then falls back to common names.
/// Both `.ttf` and `.otf` extensions are tried automatically.
fn resolve_font_paths(preset: &str, weight: &str) -> (Option<String>, Option<String>) {
    let w = if weight == "bold" { "Bold" } else { "Regular" };

    match preset {
        "dejavu" => {
            let prop = try_find(&[
                format!("DejaVuSans-{}", w),
                "DejaVuSans".into(),
                "DejaVu Sans".into(), // fc-match family name
            ]);
            let mono = try_find(&[
                format!("DejaVuSansMono-{}", w),
                "DejaVuSansMono".into(),
                "DejaVu Sans Mono".into(), // fc-match family name
            ]);
            (prop, mono)
        }
        "liberation" => {
            let prop = try_find(&[
                format!("LiberationSans-{}", w),
                "LiberationSans-Regular".into(),
                "LiberationSans".into(),
                "Liberation Sans".into(), // fc-match family name
            ]);
            let mono = try_find(&[
                format!("LiberationMono-{}", w),
                "LiberationMono-Regular".into(),
                "LiberationMono".into(),
                "Liberation Mono".into(), // fc-match family name
            ]);
            (prop, mono)
        }
        "fira" => {
            let prop_attempt = try_find(&[
                format!("FiraSans-{}", w),
                "FiraSans-Regular".into(),
                "FiraSans".into(),
                "Fira Sans".into(), // fc-match family name
            ]);
            let mono = try_find(&[
                format!("FiraCode-{}", w),
                "FiraCode-Regular".into(),
                "FiraCode".into(),
                "FiraCode-VF".into(),
                "Fira Code".into(),          // fc-match family name
                "FiraCode Nerd Font".into(), // Nerd Font variant
                "FiraCode Nerd Font Mono".into(),
            ]);
            let prop = prop_attempt.or_else(|| mono.clone());
            (prop, mono)
        }
        "jetbrains" => {
            let mono = try_find(&[
                format!("JetBrainsMono-{}", w),
                "JetBrainsMono-Regular".into(),
                "JetBrainsMono".into(),
                "JetBrainsMono-VF".into(),
                "JetBrains Mono".into(),            // fc-match family name
                "JetBrainsMono Nerd Font".into(),   // Nerd Font variant
                "JetBrainsMonoNL Nerd Font".into(), // NL = No Ligatures
            ]);
            (mono.clone(), mono)
        }
        "iosevka" => {
            let prop = try_find(&[
                format!("IosevkaNerdFontPropo-{}", w),
                "IosevkaNerdFontPropo-Regular".into(),
                "IosevkaNerdFont-Regular".into(),
                "Iosevka-Regular".into(),
                "Iosevka".into(),
                "Iosevka Nerd Font Propo".into(),
                "Iosevka NFP".into(),
                "Iosevka Nerd Font".into(),
                "Iosevka NF".into(),
                "Iosevka".into(),
            ]);
            let mono = try_find(&[
                format!("IosevkaNerdFontMono-{}", w),
                "IosevkaNerdFontMono-Regular".into(),
                "IosevkaMono-Regular".into(),
                "IosevkaMono".into(),
                "Iosevka Nerd Font Mono".into(),
                "Iosevka NFM".into(),
            ]);
            (prop, mono)
        }
        _ => (None, None),
    }
}

// ── Typed enum helpers ─────────────────────────────────────────────
// These live here (in ui/theme) rather than config/ to avoid a cyclic
// dependency: ui/theme defines ThemeColors, and Theme::resolve() returns it.

impl Theme {
    /// Resolve to the concrete palette for this theme.
    /// `System` falls back to light mode here; actual OS detection
    /// happens in `apply_theme_and_fonts` via egui::Visuals::light()/dark().
    pub fn resolve(self) -> ThemeColors {
        match self {
            Theme::Light => ThemeColors::light(),
            Theme::Nord => ThemeColors::nord(),
            Theme::Catppuccin => ThemeColors::catppuccin(),
            Theme::Dracula => ThemeColors::dracula(),
            Theme::System => ThemeColors::light(),
            _ => ThemeColors::dark(),
        }
    }

    /// Returns `true` for non-dark themes (used for visuals toggle).
    pub fn is_light(self) -> bool {
        matches!(self, Theme::Light | Theme::System)
    }
}

impl FontPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            FontPreset::Default => "default",
            FontPreset::DejaVu => "dejavu",
            FontPreset::Liberation => "liberation",
            FontPreset::Fira => "fira",
            FontPreset::JetBrains => "jetbrains",
            FontPreset::Iosevka => "iosevka",
        }
    }
}

impl FontSize {
    pub fn as_str(self) -> &'static str {
        match self {
            FontSize::Small => "small",
            FontSize::Medium => "medium",
            FontSize::Large => "large",
        }
    }
}

impl FontWeight {
    pub fn as_str(self) -> &'static str {
        match self {
            FontWeight::Normal => "normal",
            FontWeight::Bold => "bold",
        }
    }
}

/// Cache for font availability — scanned once, reused forever.
static FONT_AVAILABILITY: OnceLock<HashMap<FontPreset, bool>> = OnceLock::new();

/// Check whether a font preset has at least one font file available on this system.
/// Uses "Regular" weight for the probe since we only need to know if the font exists.
/// Results are cached after the first call.
pub fn is_font_preset_available(preset: FontPreset) -> bool {
    let map = FONT_AVAILABILITY.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(FontPreset::Default, true);
        for &p in &[FontPreset::DejaVu, FontPreset::Liberation, FontPreset::Fira, FontPreset::JetBrains, FontPreset::Iosevka] {
            let (prop, mono) = resolve_font_paths(p.as_str(), "normal");
            m.insert(p, prop.is_some() || mono.is_some());
        }
        m
    });
    map.get(&preset).copied().unwrap_or(true)
}

/// Log font diagnostic info (only when debug logging is enabled).
fn log_font_diag(preset: &str, prop: &Option<String>, mono: &Option<String>) {
    match (prop, mono) {
        (Some(p), Some(m)) => {
            debug_log!(
                "[fonts] {} → proportional: {} | monospace: {}",
                preset,
                p,
                m
            )
        }
        (Some(p), None) => debug_log!("[fonts] {} → proportional: {} (no monospace)", preset, p),
        (None, Some(m)) => debug_log!("[fonts] {} → monospace: {} (no proportional)", preset, m),
        (None, None) => debug_log!("[fonts] {} → no font files found on this system", preset),
    }
}

/// Load custom font files into the egui context based on the config.
pub fn load_custom_fonts(ctx: &egui::Context, config: &Config) {
    let preset = config.general.font_preset.as_str();
    if preset == "default"
        && config.general.font_proportional_path.is_empty()
        && config.general.font_monospace_path.is_empty()
    {
        return;
    }

    let weight = config.general.font_weight.as_str();

    // Use explicitly configured paths, or fall back to auto-detected ones
    let (auto_prop, auto_mono) = resolve_font_paths(preset, weight);

    let prop_path = if !config.general.font_proportional_path.is_empty() {
        Some(std::path::PathBuf::from(
            &config.general.font_proportional_path,
        ))
    } else {
        auto_prop.map(std::path::PathBuf::from)
    };

    let mono_path = if !config.general.font_monospace_path.is_empty() {
        Some(std::path::PathBuf::from(
            &config.general.font_monospace_path,
        ))
    } else {
        auto_mono.map(std::path::PathBuf::from)
    };

    log_font_diag(
        preset,
        &prop_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        &mono_path.as_ref().map(|p| p.to_string_lossy().to_string()),
    );

    if prop_path.is_none() && mono_path.is_none() {
        return;
    }

    let mut fonts = egui::FontDefinitions::default();

    if let Some(path) = &prop_path {
        if let Ok(data) = std::fs::read(path) {
            let name = format!("custom_prop_{}", preset);
            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(data));
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, name);
        }
    }

    if let Some(path) = &mono_path {
        if let Ok(data) = std::fs::read(path) {
            let name = format!("custom_mono_{}", preset);
            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(data));
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, name);
        }
    }

    ctx.set_fonts(fonts);
}

/// Apply the configured theme, fonts, and font sizes to the egui context.
pub fn apply_theme_and_fonts(ctx: &egui::Context, config: &Config) {
    // --- Theme visuals ---
    if config.general.enable_theming {
        let theme = config.general.theme;
        let colors = theme.resolve();
        let mut visuals = if theme.is_light() {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };

        visuals.window_fill = colors.window_bg;
        visuals.panel_fill = colors.panel_bg;
        visuals.extreme_bg_color = colors.extreme_bg;
        visuals.widgets.noninteractive.bg_fill = colors.widget_inactive_bg;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors.widget_border);
        visuals.widgets.inactive.bg_fill = colors.widget_inactive_bg;
        visuals.widgets.hovered.bg_fill = colors.widget_hovered_bg;
        visuals.widgets.active.bg_fill = colors.widget_active_bg;
        visuals.selection.bg_fill = colors.selection_bg;
        visuals.selection.stroke = egui::Stroke::new(1.0, colors.selection_stroke);

        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, colors.text_color);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors.text_color);
        let fg_hover = if theme.is_light() {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg_hover);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, fg_hover);

        visuals.window_rounding = egui::Rounding::same(16.0);
        ctx.set_visuals(visuals);
    }

    // --- Custom fonts ---
    load_custom_fonts(ctx, config);

    // --- Font sizes ---
    let (h_size, b_size, btn_size, s_size, m_size) =
        font_size_values(config.general.font_size);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(0.0);
    style.visuals.window_rounding = egui::Rounding::same(16.0);

    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(h_size));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(b_size));
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(btn_size),
    );
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(s_size));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(m_size));

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

pub fn paint_app_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Application window icon: a rounded rectangle with a title bar
    let inner = rect.shrink2(egui::vec2(1.5, 1.5));
    if inner.width() < 4.0 || inner.height() < 4.0 {
        return;
    }

    // Main window body (rectangle with small rounding)
    painter.rect(
        inner,
        egui::Rounding::same(2.5),
        egui::Color32::TRANSPARENT,
        stroke,
    );

    // Title bar line (across the top portion)
    let title_y = inner.top() + inner.height() * 0.35;
    painter.line_segment(
        [egui::pos2(inner.left() + 2.5, title_y), egui::pos2(inner.right() - 2.5, title_y)],
        stroke,
    );

    // Title bar dot (close button)
    let dot_center = egui::pos2(inner.left() + 4.5, inner.top() + 3.5);
    painter.circle_filled(dot_center, 1.8, color);

    // Content area small line (like a content preview)
    let content_y1 = title_y + inner.height() * 0.2;
    let content_y2 = title_y + inner.height() * 0.45;
    painter.line_segment(
    [
        egui::pos2(inner.left() + 4.0, content_y1),
        egui::pos2(inner.right() - 4.0, content_y1),
    ],
        egui::Stroke::new(1.0, color),
    );
    painter.line_segment(
    [
        egui::pos2(inner.left() + 4.0, content_y2),
        egui::pos2(inner.right() - 4.0, content_y2),
    ],
        egui::Stroke::new(1.0, color),
    );
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

pub fn paint_palette_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.5, color);

    // Draw artist palette outline shape
    let center = rect.center();
    let r = rect.width() * 0.4;
    painter.circle_stroke(center, r, stroke);

    // Small circular thumb hole at bottom right
    let hole_center = center + egui::vec2(r * 0.45, r * 0.45);
    painter.circle_stroke(hole_center, r * 0.15, stroke);

    // Draw three small paint spots of different colors
    let dot_r = r * 0.15;

    // Spot 1: Red
    let spot1 = center + egui::vec2(-r * 0.4, -r * 0.3);
    painter.circle_filled(spot1, dot_r, egui::Color32::from_rgb(239, 68, 68));

    // Spot 2: Green
    let spot2 = center + egui::vec2(r * 0.15, -r * 0.45);
    painter.circle_filled(spot2, dot_r, egui::Color32::from_rgb(34, 197, 94));

    // Spot 3: Blue
    let spot3 = center + egui::vec2(-r * 0.45, r * 0.25);
    painter.circle_filled(spot3, dot_r, egui::Color32::from_rgb(59, 130, 246));
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
        theme.map_or_else(|| ui.visuals().extreme_bg_color, |t| t.badge_bg_selected)
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
        "application" => paint_app_icon(ui, icon_rect, icon_color),
        _ => {}
    }
}
