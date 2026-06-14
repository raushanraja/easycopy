// ── Domain modules ────────────────────────────────────────────────

pub mod clipboard;
pub mod launcher;
pub mod browser;
pub mod ui;
pub mod ipc;
pub mod config;

// ── Persistence layer ─────────────────────────────────────────────

pub mod store;

// ── Utilities (root-level, no domain grouping needed) ─────────────

pub mod hotkey;

// ── Backward-compatible re-exports ────────────────────────────────
// Callers can still use `crate::history::ClipItem`, `crate::browser_action::resolve`, etc.

pub use clipboard::{ClipCache, ClipItem, HistoryManager};
pub use launcher::DesktopApp;
pub use browser::action::BrowserAction;
