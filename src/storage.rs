// ================================================================
//  STORAGE (facade)
// ================================================================
// Re-exports from history_storage for backward compatibility.
// Image operations should use ImageStore directly.

pub use crate::history_storage::{
    history_path, load_history, load_history_from_path, save_history, save_history_to_path,
};
