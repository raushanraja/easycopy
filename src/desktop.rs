use crate::config::Config;
use crate::parser::parse_desktop_file;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

// ================================================================
//  DESKTOP APP
// ================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopApp {
    pub name: String,
    pub comment: String,
    pub exec: String,
    pub icon_path: Option<String>,
    #[serde(default)]
    pub use_count: u64,
}

// ================================================================
//  PUBLIC API
// ================================================================

/// Scan .desktop files (slow – full I/O scan).
pub fn load_desktop_apps() -> Vec<DesktopApp> {
    scan_desktop_files()
}

/// Path to the cached desktop apps JSON file.
fn apps_cache_path() -> PathBuf {
    Config::data_dir().join("apps_cache.json")
}

fn app_usage_path() -> PathBuf {
    Config::data_dir().join("app_usage.json")
}

fn app_usage_key(app: &DesktopApp) -> String {
    format!("{}\n{}", app.name, app.exec)
}

fn load_app_usage() -> HashMap<String, u64> {
    let path = app_usage_path();
    if !path.exists() {
        return HashMap::new();
    }
    let Ok(json) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

fn save_app_usage(usage: &HashMap<String, u64>) -> std::io::Result<()> {
    let path = app_usage_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let file = std::fs::File::create(&tmp)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, usage)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn apply_app_usage(apps: &mut [DesktopApp]) {
    let usage = load_app_usage();
    for app in apps {
        app.use_count = usage.get(&app_usage_key(app)).copied().unwrap_or(0);
    }
}

pub fn record_app_launch(app: &DesktopApp) {
    let mut usage = load_app_usage();
    let key = app_usage_key(app);
    let count = usage.get(&key).copied().unwrap_or(app.use_count);
    usage.insert(key, count.saturating_add(1));
    let _ = save_app_usage(&usage);
}

/// Save a list of apps to the cache file (written by the daemon).
pub fn save_apps_cache(apps: &[DesktopApp]) -> std::io::Result<()> {
    let path = apps_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let file = std::fs::File::create(&tmp)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, apps)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Load apps from the cache file (fast – single JSON read).
/// Returns `None` when no cache exists yet.
pub fn load_apps_cache() -> Option<Vec<DesktopApp>> {
    let path = apps_cache_path();
    if !path.exists() {
        return None;
    }
    let json = std::fs::read_to_string(path).ok()?;
    let mut apps: Vec<DesktopApp> = serde_json::from_str(&json).ok()?;
    apply_app_usage(&mut apps);
    Some(apps)
}

/// Full scan + cache update. Call from a background thread.
pub fn refresh_and_cache_apps() -> Vec<DesktopApp> {
    let mut apps = scan_desktop_files();
    apply_app_usage(&mut apps);
    let _ = save_apps_cache(&apps);
    apps
}

// ================================================================
//  SCAN .desktop FILES
// ================================================================

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let local = Path::new(&home).join(".local/share/applications");
        if local.exists() {
            dirs.push(local);
        }
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        let local = Path::new(&data_home).join("applications");
        if local.exists() {
            dirs.push(local);
        }
    }
    if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for p in data_dirs.split(':') {
            let p = Path::new(p).join("applications");
            if p.exists() {
                dirs.push(p);
            }
        }
    }
    let fallback = Path::new("/usr/share/applications");
    if fallback.exists() && !dirs.contains(&fallback.to_path_buf()) {
        dirs.push(fallback.to_path_buf());
    }
    dirs
}

fn scan_desktop_files() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    for dir in desktop_dirs() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.into_iter().flatten() {
            let path = entry.path();
            if !path.extension().map(|e| e == "desktop").unwrap_or(false) {
                continue;
            }
            if let Some(app) = parse_desktop_file(&path) {
                let key = app.name.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key);
                apps.push(app);
            }
        }
    }
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}
