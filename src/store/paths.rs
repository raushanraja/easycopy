use crate::config::dirs::Directories;

// ================================================================
//  PATHS
// ================================================================
// Single source of truth for all data-file names. Each function
// borrows Directories so callers don't need to clone or move it.

pub fn history(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("index.json")
}

pub fn browser_actions(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("browser_actions.json")
}

pub fn apps_cache(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("apps_cache.json")
}

pub fn app_usage(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("app_usage.json")
}

pub fn daemon_socket(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("daemon.sock")
}

pub fn daemon_pid(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("daemon.pid")
}
