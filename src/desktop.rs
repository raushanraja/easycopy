use crate::parser::parse_desktop_file;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
//  PUBLIC API (domain logic only — persistence in store::desktop)
// ================================================================

/// Scan .desktop files (slow – full I/O scan).
pub fn load_desktop_apps() -> Vec<DesktopApp> {
    scan_desktop_files()
}

/// Full scan + cache update. Call from a background thread.
pub fn refresh_and_cache_apps(dirs: crate::dirs::Directories) -> Vec<DesktopApp> {
    let mut apps = scan_desktop_files();
    apply_app_usage(dirs.clone(), &mut apps);
    let _ = crate::store::desktop::save_apps_cache(dirs, &apps);
    apps
}

pub fn record_app_launch(dirs: crate::dirs::Directories, app: &DesktopApp) {
    crate::store::desktop::record_app_launch(dirs, app);
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

fn app_usage_key(app: &DesktopApp) -> String {
    format!("{}\n{}", app.name, app.exec)
}

fn apply_app_usage(dirs: crate::dirs::Directories, apps: &mut [DesktopApp]) {
    let usage = crate::store::desktop::load_app_usage(dirs);
    for app in apps {
        app.use_count = usage.get(&app_usage_key(app)).copied().unwrap_or(0);
    }
}
