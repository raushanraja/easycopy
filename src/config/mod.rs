//! Configuration and XDG directory discovery.
//!
//! Domain: user settings (GeneralConfig, FooterConfig), validation,
//! and XDG filesystem path resolution.

pub mod config;
pub mod dirs;

pub use config::{Config, FooterConfig, GeneralConfig, FontPreset, FontSize, FontWeight, Theme};
pub use dirs::Directories;
