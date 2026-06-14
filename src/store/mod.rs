// ================================================================
//  STORE
// ================================================================
// Unified persistence module. Owns all file I/O for the application.
// Directories are discovered once by the caller and passed in — no
// module independently resolves paths.
//
// Sub-modules:
//   paths     — filename registry (all data-file names in one place)
//   atomic    — shared atomic-write utility
//   history   — index.json (clip history)
//   images    — PNG images + thumbnails
//   config    — config.toml
//   browser_actions — browser_actions.json
//   desktop   — apps_cache.json, app_usage.json

pub mod atomic;
pub mod browser_actions;
pub mod config;
pub mod desktop;
pub mod history;
pub mod images;
pub mod paths;

// Re-export commonly used types and functions for convenience.
pub use atomic::AtomicWriter;
pub use paths::{
    app_usage, apps_cache, browser_actions, daemon_pid, daemon_socket, history,
};
