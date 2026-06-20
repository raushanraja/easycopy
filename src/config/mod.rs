//! Configuration and XDG directory discovery.
//!
//! Domain: user settings (GeneralConfig, FooterConfig), validation,
//! and XDG filesystem path resolution.

pub mod config;
pub mod dirs;

pub use config::{Config, FontPreset, FontSize, FontWeight, FooterConfig, GeneralConfig, Theme};
pub use dirs::Directories;
