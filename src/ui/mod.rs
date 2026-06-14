//! Popup UI and theming.
//!
//! Domain: the eframe/egui popup window, theme system,
//! font loading, and programmatic icon painting.

pub mod popup;
pub mod theme;

pub use theme::{
    apply_theme_and_fonts, is_font_preset_available, load_custom_fonts, ThemeColors,
};
