use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
fn default_theme() -> String {
    "dark".to_string()
}
fn default_hide_main_header() -> bool {
    false
}
fn default_hide_secondary_header() -> bool {
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
fn default_font_preset() -> String {
    "default".to_string()
}
fn default_font_size() -> String {
    "medium".to_string()
}
fn default_font_proportional_path() -> String {
    String::new()
}
fn default_font_monospace_path() -> String {
    String::new()
}
fn default_font_weight() -> String {
    "normal".to_string()
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
    /// "dark", "light", or "system".
    pub theme: String,
    pub hide_main_header: bool,
    pub hide_secondary_header: bool,
    pub enable_theming: bool,
    pub enable_clipping: bool,
    pub close_on_focus_out: bool,
    /// Font preset: "default", "dejavu", "liberation", "fira", "jetbrains"
    pub font_preset: String,
    /// Font size: "small", "medium", "large"
    pub font_size: String,
    /// Custom proportional font file path (TTF/OTF)
    pub font_proportional_path: String,
    /// Custom monospace font file path (TTF/OTF)
    pub font_monospace_path: String,
    /// Font weight: "normal" or "bold"
    pub font_weight: String,
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
            enable_theming: default_enable_theming(),
            enable_clipping: default_enable_clipping(),
            close_on_focus_out: default_close_on_focus_out(),
            font_preset: default_font_preset(),
            font_size: default_font_size(),
            font_proportional_path: default_font_proportional_path(),
            font_monospace_path: default_font_monospace_path(),
            font_weight: default_font_weight(),
        }
    }
}

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
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("easycopy")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("easycopy")
    }

    pub fn images_dir() -> PathBuf {
        Self::data_dir().join("images")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("easycopy.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match Self::load_from_path(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    let mut cfg = Self::default();
                    cfg.parse_error = Some(format!("Failed to read config file: {}", e));
                    cfg
                }
            }
        } else {
            let cfg = Self::default();
            let _ = cfg.save();
            cfg
        }
    }

    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        match toml::from_str::<Self>(&text) {
            Ok(mut cfg) => {
                cfg.sanitize();
                Ok(cfg)
            }
            Err(e) => {
                let err_msg = e.to_string();
                eprintln!(
                    "Warning: Failed to parse config file: {}. Using default settings.",
                    err_msg
                );
                let mut cfg = Self::default();
                cfg.parse_error = Some(err_msg);
                Ok(cfg)
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to_path(&Self::config_path())
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, text)
    }

    pub fn sanitize(&mut self) {
        if self.general.max_text_items == 0 {
            self.general.max_text_items = default_max_text_items();
        }
        if self.general.max_image_items == 0 {
            self.general.max_image_items = default_max_image_items();
        }
        if self.general.poll_interval_ms < 100 {
            self.general.poll_interval_ms = 100;
        }
        if self.general.popup_width < 320.0 {
            self.general.popup_width = 320.0;
        }
        if self.general.popup_height < 360.0 {
            self.general.popup_height = 360.0;
        }
        if self.general.preview_chars < 20 {
            self.general.preview_chars = 20;
        }
        if self.general.paste_delay_ms > 1_000 {
            self.general.paste_delay_ms = 1_000;
        }
        let theme = self.general.theme.to_lowercase();
        self.general.theme = match theme.as_str() {
            "light" | "nord" | "catppuccin" | "dracula" | "system" => theme,
            _ => "dark".to_string(),
        };

        let font_preset = self.general.font_preset.to_lowercase();
        self.general.font_preset = match font_preset.as_str() {
            "dejavu" | "liberation" | "fira" | "jetbrains" => font_preset,
            _ => "default".to_string(),
        };

        let font_size = self.general.font_size.to_lowercase();
        self.general.font_size = match font_size.as_str() {
            "small" | "large" => font_size,
            _ => "medium".to_string(),
        };

        let font_weight = self.general.font_weight.to_lowercase();
        self.general.font_weight = match font_weight.as_str() {
            "bold" => font_weight,
            _ => "normal".to_string(),
        };
    }
}

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
        assert!(cfg.general.popup_height >= 360.0);
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
        assert_eq!(cfg.general.hide_main_header, false);
        assert_eq!(cfg.general.hide_secondary_header, false);
        assert_eq!(cfg.general.enable_theming, true);
        assert_eq!(cfg.general.enable_clipping, true);
        assert_eq!(cfg.general.close_on_focus_out, true);
        assert_eq!(cfg.general.font_preset, "default");
        assert_eq!(cfg.general.font_size, "medium");
        assert_eq!(cfg.general.font_proportional_path, "");
        assert_eq!(cfg.general.font_monospace_path, "");
        assert_eq!(cfg.general.font_weight, "normal");
        assert_eq!(cfg.footer.enable, true);
        assert_eq!(cfg.footer.show_help, true);
        assert_eq!(cfg.footer.show_clear, true);
        assert_eq!(cfg.footer.show_settings, true);
    }

    #[test]
    fn invalid_numbers_are_sanitized() {
        let mut cfg = Config::default();
        cfg.general.max_text_items = 0;
        cfg.general.poll_interval_ms = 1;
        cfg.general.popup_width = 1.0;
        cfg.general.popup_height = 1.0;
        cfg.general.preview_chars = 1;
        cfg.general.theme = "unknown".into();
        cfg.sanitize();
        assert_eq!(cfg.general.max_text_items, 200);
        assert_eq!(cfg.general.poll_interval_ms, 100);
        assert_eq!(cfg.general.popup_width, 320.0);
        assert_eq!(cfg.general.popup_height, 360.0);
        assert_eq!(cfg.general.preview_chars, 20);
        assert_eq!(cfg.general.theme, "dark");
    }

    #[test]
    fn new_themes_are_preserved() {
        for theme in &["nord", "catppuccin", "dracula", "light", "system"] {
            let mut cfg = Config::default();
            cfg.general.theme = theme.to_string();
            cfg.sanitize();
            assert_eq!(cfg.general.theme, theme.to_string());
        }
    }
}
