use crate::config::Directories;
use crate::store::desktop;
use serde::{Deserialize, Serialize};

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

impl DesktopApp {
    /// Unique key for usage stats (name + exec combination).
    pub fn usage_key(&self) -> String {
        format!("{}\n{}", self.name, self.exec)
    }
}

// ================================================================
//  PUBLIC API (domain logic only — persistence in store::desktop)
// ================================================================

/// Scan .desktop files (slow – full I/O scan).
pub fn load_desktop_apps() -> Vec<DesktopApp> {
    scan_desktop_files()
}

/// Full scan + cache update. Call from a background thread.
pub fn refresh_and_cache_apps(dirs: &Directories) -> Vec<DesktopApp> {
    let mut apps = scan_desktop_files();
    apply_app_usage(dirs, &mut apps);
    let _ = desktop::save_apps_cache(dirs, &apps);
    apps
}

pub fn record_app_launch(dirs: &Directories, app: &DesktopApp) {
    desktop::record_app_launch(dirs, app);
}

// ================================================================
//  SCAN .desktop FILES
// ================================================================

fn desktop_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let local = std::path::Path::new(&home).join(".local/share/applications");
        if local.exists() {
            dirs.push(local);
        }
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        let local = std::path::Path::new(&data_home).join("applications");
        if local.exists() {
            dirs.push(local);
        }
    }
    if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for p in data_dirs.split(':') {
            let p = std::path::Path::new(p).join("applications");
            if p.exists() {
                dirs.push(p);
            }
        }
    }
    let fallback = std::path::Path::new("/usr/share/applications");
    if fallback.exists() && !dirs.contains(&fallback.to_path_buf()) {
        dirs.push(fallback.to_path_buf());
    }
    dirs
}

fn scan_desktop_files() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in desktop_dirs() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.into_iter().flatten() {
            let path = entry.path();
            if !path.extension().map(|e| e == "desktop").unwrap_or(false) {
                continue;
            }
            if let Some(app) = crate::launcher::parser::parse_desktop_file(&path) {
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

fn apply_app_usage(dirs: &Directories, apps: &mut [DesktopApp]) {
    let usage = desktop::load_app_usage(dirs);
    for app in apps {
        app.use_count = usage.get(&app.usage_key()).copied().unwrap_or(0);
    }
}
