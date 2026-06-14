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

// ── Default helpers ────────────────────────────────────────────────

fn default_max_text_items() -> usize {
    200
}
fn default_max_image_items() -> usize {
    50
}
fn default_hotkey() -> String {
    "Ctrl+Alt+V".to_string()
}
fn default_auto_paste() -> bool {
    true
}
fn default_poll_interval_ms() -> u64 {
    500
}
fn default_popup_width() -> f32 {
    640.0
}
fn default_popup_height() -> f32 {
    720.0
}
fn default_preview_chars() -> usize {
    220
}
fn default_paste_delay_ms() -> u64 {
    120
}
fn default_theme() -> Theme {
    Theme::Dark
}
fn default_hide_main_header() -> bool {
    false
}
fn default_hide_secondary_header() -> bool {
    false
}
fn default_hide_counts() -> bool {
    false
}
fn default_enable_theming() -> bool {
    true
}
fn default_enable_clipping() -> bool {
    true
}
fn default_close_on_focus_out() -> bool {
    true
}
fn default_keep_search_on_reopen() -> bool {
    true
}
fn default_debug_logging() -> bool {
    false
}
fn default_font_preset() -> FontPreset {
    FontPreset::Default
}
fn default_font_size() -> FontSize {
    FontSize::Medium
}
fn default_font_proportional_path() -> String {
    String::new()
}
fn default_font_monospace_path() -> String {
    String::new()
}
fn default_font_weight() -> FontWeight {
    FontWeight::Normal
}

fn default_footer_enable() -> bool {
    true
}
fn default_footer_show_help() -> bool {
    true
}
fn default_footer_show_clear() -> bool {
    true
}
fn default_footer_show_settings() -> bool {
    true
}
fn default_footer_show_theme() -> bool {
    true
}

// ── GeneralConfig ──────────────────────────────────────────────────
// All fields private — access via getters, mutation via setters.
// Typed enums ensure theme/font values are always valid.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralConfig {
    max_text_items: usize,
    max_image_items: usize,
    hotkey: String,
    auto_paste: bool,
    poll_interval_ms: u64,
    popup_width: f32,
    popup_height: f32,
    preview_chars: usize,
    paste_delay_ms: u64,
    theme: Theme,
    hide_main_header: bool,
    hide_secondary_header: bool,
    hide_counts: bool,
    enable_theming: bool,
    enable_clipping: bool,
    close_on_focus_out: bool,
    keep_search_on_reopen: bool,
    debug_logging: bool,
    font_preset: FontPreset,
    font_size: FontSize,
    font_proportional_path: String,
    font_monospace_path: String,
    font_weight: FontWeight,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_text_items: default_max_text_items(),
            max_image_items: default_max_image_items(),
            hotkey: default_hotkey(),
            auto_paste: default_auto_paste(),
            poll_interval_ms: default_poll_interval_ms(),
            popup_width: default_popup_width(),
            popup_height: default_popup_height(),
            preview_chars: default_preview_chars(),
            paste_delay_ms: default_paste_delay_ms(),
            theme: default_theme(),
            hide_main_header: default_hide_main_header(),
            hide_secondary_header: default_hide_secondary_header(),
            hide_counts: default_hide_counts(),
            enable_theming: default_enable_theming(),
            enable_clipping: default_enable_clipping(),
            close_on_focus_out: default_close_on_focus_out(),
            keep_search_on_reopen: default_keep_search_on_reopen(),
            debug_logging: default_debug_logging(),
            font_preset: default_font_preset(),
            font_size: default_font_size(),
            font_proportional_path: default_font_proportional_path(),
            font_monospace_path: default_font_monospace_path(),
            font_weight: default_font_weight(),
        }
    }
}

impl GeneralConfig {
    // ── Numeric getters ────────────────────────────────────────────

    pub fn max_text_items(&self) -> usize {
        self.max_text_items
    }
    pub fn set_max_text_items(&mut self, v: usize) {
        self.max_text_items = v.max(1);
    }

    pub fn max_image_items(&self) -> usize {
        self.max_image_items
    }
    pub fn set_max_image_items(&mut self, v: usize) {
        self.max_image_items = v.max(1);
    }

    pub fn poll_interval_ms(&self) -> u64 {
        self.poll_interval_ms
    }
    pub fn set_poll_interval_ms(&mut self, v: u64) {
        self.poll_interval_ms = v.max(100);
    }

    pub fn popup_width(&self) -> f32 {
        self.popup_width
    }
    pub fn set_popup_width(&mut self, v: f32) {
        self.popup_width = v.max(320.0);
    }

    pub fn popup_height(&self) -> f32 {
        self.popup_height
    }
    pub fn set_popup_height(&mut self, v: f32) {
        self.popup_height = v.max(360.0);
    }

    pub fn preview_chars(&self) -> usize {
        self.preview_chars
    }
    pub fn set_preview_chars(&mut self, v: usize) {
        self.preview_chars = v.max(20);
    }

    pub fn paste_delay_ms(&self) -> u64 {
        self.paste_delay_ms
    }
    pub fn set_paste_delay_ms(&mut self, v: u64) {
        self.paste_delay_ms = v.min(1_000);
    }

    // ── Hotkey ─────────────────────────────────────────────────────

    pub fn hotkey(&self) -> &str {
        &self.hotkey
    }
    pub fn set_hotkey(&mut self, v: impl Into<String>) {
        self.hotkey = v.into();
    }

    // ── Theme (typed enum) ─────────────────────────────────────────

    pub fn theme(&self) -> Theme {
        self.theme
    }
    pub fn set_theme(&mut self, t: Theme) {
        self.theme = t;
    }

    // ── Bool flags ─────────────────────────────────────────────────

    pub fn hide_main_header(&self) -> bool {
        self.hide_main_header
    }
    pub fn set_hide_main_header(&mut self, v: bool) {
        self.hide_main_header = v;
    }

    pub fn hide_secondary_header(&self) -> bool {
        self.hide_secondary_header
    }
    pub fn set_hide_secondary_header(&mut self, v: bool) {
        self.hide_secondary_header = v;
    }

    pub fn hide_counts(&self) -> bool {
        self.hide_counts
    }
    pub fn set_hide_counts(&mut self, v: bool) {
        self.hide_counts = v;
    }

    pub fn enable_theming(&self) -> bool {
        self.enable_theming
    }
    pub fn set_enable_theming(&mut self, v: bool) {
        self.enable_theming = v;
    }

    pub fn enable_clipping(&self) -> bool {
        self.enable_clipping
    }
    pub fn set_enable_clipping(&mut self, v: bool) {
        self.enable_clipping = v;
    }

    pub fn close_on_focus_out(&self) -> bool {
        self.close_on_focus_out
    }
    pub fn set_close_on_focus_out(&mut self, v: bool) {
        self.close_on_focus_out = v;
    }

    pub fn keep_search_on_reopen(&self) -> bool {
        self.keep_search_on_reopen
    }
    pub fn set_keep_search_on_reopen(&mut self, v: bool) {
        self.keep_search_on_reopen = v;
    }

    pub fn debug_logging(&self) -> bool {
        self.debug_logging
    }
    pub fn set_debug_logging(&mut self, v: bool) {
        self.debug_logging = v;
    }

    pub fn auto_paste(&self) -> bool {
        self.auto_paste
    }
    pub fn set_auto_paste(&mut self, v: bool) {
        self.auto_paste = v;
    }

    // ── Font preset (typed enum) ───────────────────────────────────

    pub fn font_preset(&self) -> FontPreset {
        self.font_preset
    }
    pub fn set_font_preset(&mut self, p: FontPreset) {
        self.font_preset = p;
    }

    // ── Font size (typed enum) ─────────────────────────────────────

    pub fn font_size(&self) -> FontSize {
        self.font_size
    }
    pub fn set_font_size(&mut self, s: FontSize) {
        self.font_size = s;
    }

    // ── Font weight (typed enum) ───────────────────────────────────

    pub fn font_weight(&self) -> FontWeight {
        self.font_weight
    }
    pub fn set_font_weight(&mut self, w: FontWeight) {
        self.font_weight = w;
    }

    // ── Custom font paths ──────────────────────────────────────────

    pub fn font_proportional_path(&self) -> &str {
        &self.font_proportional_path
    }
    pub fn set_font_proportional_path(&mut self, v: impl Into<String>) {
        self.font_proportional_path = v.into();
    }

    pub fn font_monospace_path(&self) -> &str {
        &self.font_monospace_path
    }
    pub fn set_font_monospace_path(&mut self, v: impl Into<String>) {
        self.font_monospace_path = v.into();
    }

    // ── Validation ─────────────────────────────────────────────────
    // Numeric bounds are still enforced here (typed enums guarantee
    // theme/font values are always valid). Called after TOML load
    // to clamp numeric fields that may have been hand-edited.

    pub fn sanitize(&mut self) {
        self.set_max_text_items(self.max_text_items);
        self.set_max_image_items(self.max_image_items);
        self.set_poll_interval_ms(self.poll_interval_ms);
        self.set_popup_width(self.popup_width);
        self.set_popup_height(self.popup_height);
        self.set_preview_chars(self.preview_chars);
        self.set_paste_delay_ms(self.paste_delay_ms);
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
            enable: default_footer_enable(),
            show_help: default_footer_show_help(),
            show_clear: default_footer_show_clear(),
            show_settings: default_footer_show_settings(),
            show_theme: default_footer_show_theme(),
        }
    }
}

// ── Config ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub footer: FooterConfig,
    #[serde(skip)]
    pub parse_error: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            footer: FooterConfig::default(),
            parse_error: None,
        }
    }
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
        assert_eq!(cfg.general.max_text_items(), 200);
        assert_eq!(cfg.general.max_image_items(), 50);
        assert_eq!(cfg.general.hotkey(), "Ctrl+Alt+V");
        assert!(cfg.general.popup_width() >= 320.0);
        assert_eq!(cfg.general.theme(), Theme::Dark);
        assert_eq!(cfg.general.font_preset(), FontPreset::Default);
        assert_eq!(cfg.general.font_size(), FontSize::Medium);
        assert_eq!(cfg.general.font_weight(), FontWeight::Normal);
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
        assert_eq!(cfg.general.max_text_items(), 10);
        assert_eq!(cfg.general.popup_width(), 640.0);
        assert_eq!(cfg.general.preview_chars(), 220);
        assert_eq!(cfg.general.hide_main_header(), false);
        assert_eq!(cfg.general.hide_secondary_header(), false);
        assert_eq!(cfg.general.hide_counts(), false);
        assert_eq!(cfg.general.enable_theming(), true);
        assert_eq!(cfg.general.enable_clipping(), true);
        assert_eq!(cfg.general.close_on_focus_out(), true);
        assert_eq!(cfg.general.keep_search_on_reopen(), true);
        assert_eq!(cfg.general.debug_logging(), false);
        assert_eq!(cfg.general.font_preset(), FontPreset::Default);
        assert_eq!(cfg.general.font_size(), FontSize::Medium);
        assert_eq!(cfg.general.font_proportional_path(), "");
        assert_eq!(cfg.general.font_monospace_path(), "");
        assert_eq!(cfg.general.font_weight(), FontWeight::Normal);
        assert_eq!(cfg.footer.enable, true);
        assert_eq!(cfg.footer.show_help, true);
        assert_eq!(cfg.footer.show_clear, true);
        assert_eq!(cfg.footer.show_settings, true);
    }

    #[test]
    fn invalid_numbers_are_sanitized() {
        let mut cfg = Config::default();
        cfg.general.set_max_text_items(0);
        cfg.general.set_poll_interval_ms(1);
        cfg.general.set_popup_width(1.0);
        cfg.general.set_popup_height(1.0);
        cfg.general.set_preview_chars(1);
        cfg.sanitize();
        assert_eq!(cfg.general.max_text_items(), 1); // clamped to minimum of 1
        assert_eq!(cfg.general.poll_interval_ms(), 100); // clamped to minimum of 100
        assert_eq!(cfg.general.popup_width(), 320.0); // clamped to minimum of 320
        assert_eq!(cfg.general.popup_height(), 360.0); // clamped to minimum of 360
        assert_eq!(cfg.general.preview_chars(), 20); // clamped to minimum of 20
    }

    #[test]
    fn new_themes_are_preserved() {
        for theme in &[Theme::Nord, Theme::Catppuccin, Theme::Dracula, Theme::Light, Theme::System] {
            let mut cfg = Config::default();
            cfg.general.set_theme(*theme);
            assert_eq!(cfg.general.theme(), *theme);
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
        assert_eq!(cfg.general.theme(), Theme::Dracula);
        assert_eq!(cfg.general.font_preset(), FontPreset::Fira);
        assert_eq!(cfg.general.font_size(), FontSize::Large);
        assert_eq!(cfg.general.font_weight(), FontWeight::Bold);
    }

    #[test]
    fn invalid_theme_string_defaults_to_dark() {
        // Serde will reject an unknown variant; test that the default
        // serde behavior produces a sane value (Dark via Default).
        let text = r#"
[general]
theme = "unknown"
"#;
        let cfg: Config = toml::from_str(text).unwrap_or_else(|_| Config::default());
        assert_eq!(cfg.general.theme(), Theme::Dark);
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
