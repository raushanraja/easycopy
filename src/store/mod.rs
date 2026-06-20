use crate::browser::action::BrowserAction;
use crate::clipboard::history::ClipItem;
use crate::config::dirs::Directories;
use crate::config::Config;
use crate::launcher::DesktopApp;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::Result;

pub mod atomic;
pub mod browser_actions;
pub mod config;
pub mod desktop;
pub mod history;
pub mod images;
pub mod paths;
pub use atomic::AtomicWriter;
pub use images::ImageStore;
pub use paths::{
    app_usage, apps_cache, browser_actions as paths_browser_actions, daemon_pid, daemon_socket,
    history as paths_history,
};

// ================================================================
//  STORE
// ================================================================
// Single entry point for all persistence. Owns Directories internally
// so callers never pass dirs around — they call methods on &Store.

#[derive(Debug, Clone, Default)]
pub struct Store {
    dirs: Directories,
}

impl Store {
    pub fn new(dirs: Directories) -> Self {
        Self { dirs }
    }

    // ── history ───────────────────────────────────────────────────

    pub fn load_history(&self) -> VecDeque<ClipItem> {
        history::load_history(&self.dirs)
    }

    pub fn save_history(&self, items: &VecDeque<ClipItem>) -> Result<()> {
        history::save_history(&self.dirs, items)
    }

    // ── images ────────────────────────────────────────────────────

    pub fn images(&self) -> ImageStore {
        ImageStore::new(&self.dirs)
    }

    // ── config ────────────────────────────────────────────────────

    pub fn load_config(&self) -> Config {
        config::load(&self.dirs)
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        config::save(&self.dirs, config)
    }

    // ── desktop ───────────────────────────────────────────────────

    pub fn load_apps_cache(&self) -> Option<Vec<DesktopApp>> {
        desktop::load_apps_cache(&self.dirs)
    }

    pub fn save_apps_cache(&self, apps: &[DesktopApp]) -> Result<()> {
        desktop::save_apps_cache(&self.dirs, apps)
    }

    pub fn load_app_usage(&self) -> HashMap<String, u64> {
        desktop::load_app_usage(&self.dirs)
    }

    pub fn save_app_usage(&self, usage: &HashMap<String, u64>) -> Result<()> {
        desktop::save_app_usage(&self.dirs, usage)
    }

    pub fn record_app_launch(&self, app: &DesktopApp) {
        crate::launcher::desktop::record_app_launch(&self.dirs, app)
    }

    /// Full scan + cache update. Call from a background thread.
    pub fn refresh_and_cache_apps(&self) -> Vec<DesktopApp> {
        crate::launcher::desktop::refresh_and_cache_apps(&self.dirs)
    }

    // ── browser actions ───────────────────────────────────────────

    pub fn load_browser_actions(&self) -> Vec<BrowserAction> {
        browser_actions::load(&self.dirs)
    }

    pub fn save_browser_actions(&self, actions: &[BrowserAction]) -> Result<()> {
        browser_actions::save(&self.dirs, actions)
    }

    // ── paths (for ipc, opener) ───────────────────────────────────

    pub fn history_path(&self) -> std::path::PathBuf {
        paths::history(&self.dirs)
    }

    pub fn images_dir(&self) -> &std::path::Path {
        &self.dirs.images_dir
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.dirs.data_dir
    }
}
