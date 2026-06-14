use crate::dirs::Directories;
use std::path::PathBuf;

// ================================================================
//  PATHS
// ================================================================
// Single source of truth for all data-file names within the
// application's data directory. Each function takes a Directories
// instance so paths are built from the same source — no module
// independently guesses a filename.

pub fn history(dirs: Directories) -> PathBuf {
    dirs.data_dir.join("index.json")
}

pub fn browser_actions(dirs: Directories) -> PathBuf {
    dirs.data_dir.join("browser_actions.json")
}

pub fn apps_cache(dirs: Directories) -> PathBuf {
    dirs.data_dir.join("apps_cache.json")
}

pub fn app_usage(dirs: Directories) -> PathBuf {
    dirs.data_dir.join("app_usage.json")
}

pub fn daemon_socket(dirs: Directories) -> PathBuf {
    dirs.data_dir.join("daemon.sock")
}

pub fn daemon_pid(dirs: Directories) -> PathBuf {
    dirs.data_dir.join("daemon.pid")
}
