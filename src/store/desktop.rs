use crate::launcher::DesktopApp;
use crate::config::dirs::Directories;
use crate::store::atomic::AtomicWriter;
use crate::store::paths;
use std::collections::HashMap;
use std::fs;
use std::io::Result;

// ================================================================
//  DESKTOP (storage)
// ================================================================
// File I/O for desktop app cache and usage stats.
// Domain logic (scanning .desktop files) stays in desktop.rs.

// ── apps cache ─────────────────────────────────────────────────────

pub fn save_apps_cache(dirs: &Directories, apps: &[DesktopApp]) -> Result<()> {
    let path = paths::apps_cache(dirs);
    let data = serde_json::to_vec(apps).map_err(|e| std::io::Error::other(e))?;
    AtomicWriter::write(&path, &data)
}

pub fn load_apps_cache(dirs: &Directories) -> Option<Vec<DesktopApp>> {
    let path = paths::apps_cache(dirs);
    if !path.exists() {
        return None;
    }
    let json = fs::read_to_string(path).ok()?;
    let mut apps: Vec<DesktopApp> = serde_json::from_str(&json).ok()?;
    apply_app_usage(dirs, &mut apps);
    Some(apps)
}

// ── app usage ──────────────────────────────────────────────────────

pub fn load_app_usage(dirs: &Directories) -> HashMap<String, u64> {
    let path = paths::app_usage(dirs);
    if !path.exists() {
        return HashMap::new();
    }
    let Ok(json) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub fn save_app_usage(dirs: &Directories, usage: &HashMap<String, u64>) -> Result<()> {
    let path = paths::app_usage(dirs);
    let data = serde_json::to_vec(usage).map_err(|e| std::io::Error::other(e))?;
    AtomicWriter::write(&path, &data)
}

pub fn record_app_launch(dirs: &Directories, app: &DesktopApp) {
    let mut usage = load_app_usage(dirs);
    let key = app.usage_key();
    let count = usage.get(&key).copied().unwrap_or(app.use_count);
    usage.insert(key, count.saturating_add(1));
    let _ = save_app_usage(dirs, &usage);
}

// ── internal helpers ───────────────────────────────────────────────

fn apply_app_usage(dirs: &Directories, apps: &mut [DesktopApp]) {
    let usage = load_app_usage(dirs);
    for app in apps {
        app.use_count = usage.get(&app.usage_key()).copied().unwrap_or(0);
    }
}
