use serde::{Deserialize, Serialize};

// ── Typed enums for closed-domain string fields ────────────────────
// These make invalid states unrepresentable at compile time and
// eliminate the runtime string-sanitization fallback logic.

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Nord,
    Catppuccin,
    Dracula,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FontPreset {
    Default,
    DejaVu,
    Liberation,
    Fira,
    JetBrains,
    Iosevka,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FontSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Normal,
    Bold,
}

// ── GeneralConfig ──────────────────────────────────────────────────
// Simple fields are public — callers read/write directly.
// Setters with side effects (theme reload, config save) remain as methods.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralConfig {
    pub max_text_items: usize,
    pub max_image_items: usize,
    pub hotkey: String,
    pub auto_paste: bool,
    pub poll_interval_ms: u64,
    pub popup_width: f32,
    pub popup_height: f32,
    pub preview_chars: usize,
    pub paste_delay_ms: u64,
    pub theme: Theme,
    pub hide_main_header: bool,
    pub hide_secondary_header: bool,
    pub hide_counts: bool,
    pub enable_theming: bool,
    pub enable_clipping: bool,
    pub close_on_focus_out: bool,
    pub keep_search_on_reopen: bool,
    pub debug_logging: bool,
    pub font_preset: FontPreset,
    pub font_size: FontSize,
    pub font_proportional_path: String,
    pub font_monospace_path: String,
    pub font_weight: FontWeight,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_text_items: 200,
            max_image_items: 50,
            hotkey: "Ctrl+Alt+V".to_string(),
            auto_paste: true,
            poll_interval_ms: 500,
            popup_width: 640.0,
            popup_height: 720.0,
            preview_chars: 220,
            paste_delay_ms: 120,
            theme: Theme::Dark,
            hide_main_header: false,
            hide_secondary_header: false,
            hide_counts: false,
            enable_theming: true,
            enable_clipping: true,
            close_on_focus_out: true,
            keep_search_on_reopen: true,
            debug_logging: false,
            font_preset: FontPreset::Default,
            font_size: FontSize::Medium,
            font_proportional_path: String::new(),
            font_monospace_path: String::new(),
            font_weight: FontWeight::Normal,
        }
    }
}

impl GeneralConfig {
    // ── Setters with side-effect logic ──────────────────────────────
    // These trigger theme reload + config save in popup.rs; kept as methods.

    pub fn set_theme(&mut self, t: Theme) {
        self.theme = t;
    }

    pub fn set_font_preset(&mut self, p: FontPreset) {
        self.font_preset = p;
    }

    pub fn set_font_size(&mut self, s: FontSize) {
        self.font_size = s;
    }

    pub fn set_font_weight(&mut self, w: FontWeight) {
        self.font_weight = w;
    }

    pub fn set_keep_search_on_reopen(&mut self, v: bool) {
        self.keep_search_on_reopen = !v;
    }

    // ── Validation ──────────────────────────────────────────────────
    // Numeric bounds enforced here; typed enums guarantee theme/font
    // values are always valid. Called after TOML load to clamp fields
    // that may have been hand-edited.

    pub fn sanitize(&mut self) {
        self.max_text_items = self.max_text_items.max(1);
        self.max_image_items = self.max_image_items.max(1);
        self.poll_interval_ms = self.poll_interval_ms.max(100);
        self.popup_width = self.popup_width.max(320.0);
        self.popup_height = self.popup_height.max(360.0);
        self.preview_chars = self.preview_chars.max(20);
        self.paste_delay_ms = self.paste_delay_ms.min(1_000);
    }
}

// ── FooterConfig ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FooterConfig {
    pub enable: bool,
    pub show_help: bool,
    pub show_clear: bool,
    pub show_settings: bool,
    pub show_theme: bool,
}

impl Default for FooterConfig {
    fn default() -> Self {
        Self {
            enable: true,
            show_help: true,
            show_clear: true,
            show_settings: true,
            show_theme: true,
        }
    }
}

// ── Config ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub footer: FooterConfig,
    #[serde(skip)]
    pub parse_error: Option<String>,
}

impl Config {
    /// Load config from the path derived from `dirs`.
    /// Directories is discovered once by the caller.
    pub fn load(dirs: &crate::config::dirs::Directories) -> Self {
        crate::store::config::load(dirs)
    }

    /// Save config atomically to the path derived from `dirs`.
    pub fn save(&self, dirs: &crate::config::dirs::Directories) -> std::io::Result<()> {
        crate::store::config::save(dirs, self)
    }

    pub fn sanitize(&mut self) {
        self.general.sanitize();
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.general.max_text_items, 200);
        assert_eq!(cfg.general.max_image_items, 50);
        assert_eq!(cfg.general.hotkey, "Ctrl+Alt+V");
        assert!(cfg.general.popup_width >= 320.0);
        assert_eq!(cfg.general.theme, Theme::Dark);
        assert_eq!(cfg.general.font_preset, FontPreset::Default);
        assert_eq!(cfg.general.font_size, FontSize::Medium);
        assert_eq!(cfg.general.font_weight, FontWeight::Normal);
    }

    #[test]
    fn old_config_missing_new_fields_gets_defaults() {
        let text = r#"
[general]
max_text_items = 10
max_image_items = 5
hotkey = "Ctrl+Shift+V"
auto_paste = false
poll_interval_ms = 250
"#;
        let mut cfg: Config = toml::from_str(text).unwrap();
        cfg.sanitize();
        assert_eq!(cfg.general.max_text_items, 10);
        assert_eq!(cfg.general.popup_width, 640.0);
        assert_eq!(cfg.general.preview_chars, 220);
        assert!(!cfg.general.hide_main_header);
        assert!(!cfg.general.hide_secondary_header);
        assert!(!cfg.general.hide_counts);
        assert!(cfg.general.enable_theming);
        assert!(cfg.general.enable_clipping);
        assert!(cfg.general.close_on_focus_out);
        assert!(cfg.general.keep_search_on_reopen);
        assert!(!cfg.general.debug_logging);
        assert_eq!(cfg.general.font_preset, FontPreset::Default);
        assert_eq!(cfg.general.font_size, FontSize::Medium);
        assert_eq!(cfg.general.font_proportional_path, "");
        assert_eq!(cfg.general.font_monospace_path, "");
        assert_eq!(cfg.general.font_weight, FontWeight::Normal);
        assert!(cfg.footer.enable);
        assert!(cfg.footer.show_help);
        assert!(cfg.footer.show_clear);
        assert!(cfg.footer.show_settings);
    }

    #[test]
    fn invalid_numbers_are_sanitized() {
        let mut cfg = Config::default();
        cfg.general.max_text_items = 0;
        cfg.general.poll_interval_ms = 1;
        cfg.general.popup_width = 1.0;
        cfg.general.popup_height = 1.0;
        cfg.general.preview_chars = 1;
        cfg.sanitize();
        assert_eq!(cfg.general.max_text_items, 1);
        assert_eq!(cfg.general.poll_interval_ms, 100);
        assert_eq!(cfg.general.popup_width, 320.0);
        assert_eq!(cfg.general.popup_height, 360.0);
        assert_eq!(cfg.general.preview_chars, 20);
    }

    #[test]
    fn new_themes_are_preserved() {
        for theme in &[Theme::Nord, Theme::Catppuccin, Theme::Dracula, Theme::Light, Theme::System] {
            let mut cfg = Config::default();
            cfg.general.set_theme(*theme);
            assert_eq!(cfg.general.theme, *theme);
        }
    }

    #[test]
    fn typed_enum_serializes_to_lowercase() {
        let cfg = Config {
            general: GeneralConfig {
                theme: Theme::Nord,
                font_preset: FontPreset::JetBrains,
                font_size: FontSize::Small,
                font_weight: FontWeight::Bold,
                ..GeneralConfig::default()
            },
            footer: FooterConfig::default(),
            parse_error: None,
        };
        let toml_str = toml::to_string(&cfg).unwrap();
        assert!(toml_str.contains("theme = \"nord\""));
        assert!(toml_str.contains("font_preset = \"jetbrains\""));
        assert!(toml_str.contains("font_size = \"small\""));
        assert!(toml_str.contains("font_weight = \"bold\""));
    }

    #[test]
    fn typed_enum_deserializes_from_lowercase() {
        let text = r#"
[general]
theme = "dracula"
font_preset = "fira"
font_size = "large"
font_weight = "bold"
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert_eq!(cfg.general.theme, Theme::Dracula);
        assert_eq!(cfg.general.font_preset, FontPreset::Fira);
        assert_eq!(cfg.general.font_size, FontSize::Large);
        assert_eq!(cfg.general.font_weight, FontWeight::Bold);
    }

    #[test]
    fn invalid_theme_string_defaults_to_dark() {
        let text = r#"
[general]
theme = "unknown"
"#;
        let cfg: Config = toml::from_str(text).unwrap_or_else(|_| Config::default());
        assert_eq!(cfg.general.theme, Theme::Dark);
    }

    #[test]
    fn font_preset_as_str_maps_correctly() {
        assert_eq!(FontPreset::Default.as_str(), "default");
        assert_eq!(FontPreset::JetBrains.as_str(), "jetbrains");
        assert_eq!(FontPreset::Iosevka.as_str(), "iosevka");
    }

    #[test]
    fn font_size_as_str_maps_correctly() {
        assert_eq!(FontSize::Small.as_str(), "small");
        assert_eq!(FontSize::Medium.as_str(), "medium");
        assert_eq!(FontSize::Large.as_str(), "large");
    }

    #[test]
    fn font_weight_as_str_maps_correctly() {
        assert_eq!(FontWeight::Normal.as_str(), "normal");
        assert_eq!(FontWeight::Bold.as_str(), "bold");
    }
}
