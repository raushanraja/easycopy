use dirs;
use std::path::PathBuf;

// ================================================================
//  DIRECTORIES
// ================================================================
// Owns XDG filesystem path discovery. Extracted from Config so that
// modules can resolve paths without depending on the settings type.

#[derive(Debug, Clone)]
pub struct Directories {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub images_dir: PathBuf,
    pub config_path: PathBuf,
}

impl Directories {
    /// Discover XDG directories for this application.
    pub fn discover() -> Self {
        let config_dir = Self::config_dir();
        let data_dir = Self::data_dir();
        Self {
            config_dir: config_dir.clone(),
            data_dir: data_dir.clone(),
            images_dir: data_dir.join("images"),
            config_path: config_dir.join("config.toml"),
        }
    }

    /// XDG config directory (~/.config/easycopy).
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("easycopy")
    }

    /// XDG data directory (~/.local/share/easycopy).
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("easycopy")
    }

    /// Images subdirectory within the data directory.
    pub fn images_dir() -> PathBuf {
        Self::data_dir().join("images")
    }

    /// Path to the TOML config file.
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }
}
