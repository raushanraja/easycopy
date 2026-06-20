use crate::store::atomic::AtomicWriter;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChatState {
    pub current_session_id: Option<String>,
}

impl ChatState {
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let st: Self = serde_json::from_str(&text).unwrap_or_default();
        Ok(st)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        AtomicWriter::write(path, text.as_bytes())
    }
}
