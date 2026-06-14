use crate::config::Config;
use crate::config::dirs::Directories;
use crate::store::atomic::AtomicWriter;
use std::io::Result;
use std::path::Path;

// ================================================================
//  CONFIG
// ================================================================
// File I/O for the TOML config. Config itself (data struct + sanitize)
// lives in config.rs — this module only touches the filesystem.

// ── load ────────────────────────────────────────────────────────────

/// Load config from the path derived from `dirs`.
pub fn load(dirs: &Directories) -> Config {
    let path = dirs.config_path.clone();
    if path.exists() {
        match load_from_path(dirs, &path) {
            Ok(cfg) => cfg,
            Err(e) => {
                let mut cfg = Config::default();
                cfg.parse_error = Some(format!("Failed to read config file: {}", e));
                cfg
            }
        }
    } else {
        let cfg = Config::default();
        let _ = save(dirs, &cfg);
        cfg
    }
}

pub fn load_from_path(_dirs: &Directories, path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)?;
    match toml::from_str::<Config>(&text) {
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
            let mut cfg = Config::default();
            cfg.parse_error = Some(err_msg);
            Ok(cfg)
        }
    }
}

// ── save ────────────────────────────────────────────────────────────

/// Save config atomically to the path derived from `dirs`.
pub fn save(dirs: &Directories, config: &Config) -> Result<()> {
    let path = dirs.config_path.clone();
    save_to_path(dirs, &path, config)
}

pub fn save_to_path(_dirs: &Directories, path: &Path, config: &Config) -> Result<()> {
    let text = toml::to_string_pretty(config).unwrap_or_default();
    AtomicWriter::write(path, text.as_bytes())
}
