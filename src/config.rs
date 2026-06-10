use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_max_text_items() -> usize { 200 }
fn default_max_image_items() -> usize { 50 }
fn default_hotkey() -> String { "Ctrl+Alt+V".to_string() }
fn default_auto_paste() -> bool { true }
fn default_poll_interval_ms() -> u64 { 500 }
fn default_popup_width() -> f32 { 520.0 }
fn default_popup_height() -> f32 { 620.0 }
fn default_preview_chars() -> usize { 180 }
fn default_paste_delay_ms() -> u64 { 120 }
fn default_theme() -> String { "dark".to_string() }

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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self { general: GeneralConfig::default() }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clipit")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clipit")
    }

    pub fn images_dir() -> PathBuf {
        Self::data_dir().join("images")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            Self::load_from_path(&path).unwrap_or_default()
        } else {
            let cfg = Self::default();
            let _ = cfg.save();
            cfg
        }
    }

    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut cfg = toml::from_str::<Self>(&text).unwrap_or_default();
        cfg.sanitize();
        Ok(cfg)
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
            "light" | "system" => theme,
            _ => "dark".to_string(),
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
        assert_eq!(cfg.general.popup_width, 520.0);
        assert_eq!(cfg.general.preview_chars, 180);
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
}
