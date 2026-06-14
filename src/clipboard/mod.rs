//! Clipboard monitoring and history management.
//!
//! Domain: reading the system clipboard, tracking selection events,
//! and managing the in-memory history of copied items.

pub mod cache;
pub mod history;
pub mod monitor;
pub mod x11;

pub use cache::ClipCache;
pub use history::{ClipItem, HistoryManager};
pub use monitor::ClipboardMonitor;
pub use x11::{SelectionEvent, X11Watcher};
