use crate::dirs::Directories;
use crate::history::ClipItem;
use std::collections::VecDeque;
use std::io::Result;
use std::path::{Path, PathBuf};

// ================================================================
//  HISTORY STORAGE
// ================================================================
// Owns the index.json persistence layer for the clip history.
// Extracted from storage.rs so the history serialization concern
// is isolated from image file management.

#[derive(serde::Deserialize)]
struct Index {
    items: VecDeque<ClipItem>,
}

#[derive(serde::Serialize)]
struct IndexRef<'a> {
    items: &'a VecDeque<ClipItem>,
}

// ── path ────────────────────────────────────────────────────────────

pub fn history_path() -> PathBuf {
    Directories::data_dir().join("index.json")
}

// ── save ────────────────────────────────────────────────────────────

pub fn save_history(items: &VecDeque<ClipItem>) -> Result<()> {
    save_history_to_path(&history_path(), items)
}

pub fn save_history_to_path(path: &Path, items: &VecDeque<ClipItem>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let file = std::fs::File::create(&tmp)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, &IndexRef { items })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::rename(&tmp, path)?; // atomic on same filesystem
    Ok(())
}

// ── load ────────────────────────────────────────────────────────────

pub fn load_history() -> VecDeque<ClipItem> {
    load_history_from_path(&history_path()).unwrap_or_default()
}

pub fn load_history_from_path(path: &Path) -> Result<VecDeque<ClipItem>> {
    if !path.exists() {
        return Ok(VecDeque::new());
    }
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str::<Index>(&json)
        .map(|idx| idx.items)
        .unwrap_or_default())
}

// ================================================================
//  TESTS
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str, ts: u64) -> ClipItem {
        ClipItem::Text {
            content: s.into(),
            timestamp: ts,
            use_count: 0,
        }
    }

    fn image(filename: &str, ts: u64) -> ClipItem {
        ClipItem::Image {
            width: 2,
            height: 2,
            timestamp: ts,
            filename: filename.into(),
            use_count: 0,
            data: None,
        }
    }

    #[test]
    fn history_roundtrip_to_specific_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        let mut items = VecDeque::new();
        items.push_back(text("hello", 1));
        items.push_back(image("a.png", 2));

        save_history_to_path(&path, &items).unwrap();
        let loaded = load_history_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(matches!(&loaded[0], ClipItem::Text { content, .. } if content == "hello"));
    }

    #[test]
    fn missing_history_file_returns_empty_history() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_history_from_path(&dir.path().join("missing.json")).unwrap();
        assert!(loaded.is_empty());
    }
}
