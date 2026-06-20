// ── Domain modules ────────────────────────────────────────────────

pub mod ai;
pub mod browser;
pub mod clipboard;
pub mod config;
pub mod ipc;
pub mod launcher;
pub mod ui;

// ── Persistence layer ─────────────────────────────────────────────

pub mod store;

// ── Utilities (root-level, no domain grouping needed) ─────────────

pub mod hotkey;

// ── Backward-compatible re-exports ────────────────────────────────
// Callers can still use `crate::history::ClipItem`, `crate::browser_action::resolve`, etc.

pub use browser::action::BrowserAction;
pub use clipboard::{ClipCache, ClipItem, HistoryManager};
pub use launcher::DesktopApp;
