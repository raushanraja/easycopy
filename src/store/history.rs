use crate::dirs::Directories;
use crate::history::ClipItem;
use crate::store::paths::history;
use std::collections::VecDeque;
use std::io::Result;
use std::path::Path;

// ================================================================
//  HISTORY
// ================================================================
// Persists the clip history index to index.json.

#[derive(serde::Deserialize)]
struct Index {
    items: VecDeque<ClipItem>,
}

#[derive(serde::Serialize)]
struct IndexRef<'a> {
    items: &'a VecDeque<ClipItem>,
}

// ── save ────────────────────────────────────────────────────────────

pub fn save_history(dirs: Directories, items: &VecDeque<ClipItem>) -> Result<()> {
    save_history_to_path(dirs.clone(), &history(dirs), items)
}

pub fn save_history_to_path(
    _dirs: Directories,
    path: &Path,
    items: &VecDeque<ClipItem>,
) -> Result<()> {
    let data = serde_json::to_vec(&IndexRef { items })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    crate::store::AtomicWriter::write(path, &data)
}

// ── load ────────────────────────────────────────────────────────────

pub fn load_history(dirs: Directories) -> VecDeque<ClipItem> {
    load_history_from_path(dirs.clone(), &history(dirs)).unwrap_or_default()
}

pub fn load_history_from_path(
    _dirs: Directories,
    path: &Path,
) -> Result<VecDeque<ClipItem>> {
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
    fn history_roundtrip() {
        let dirs = Directories::discover();
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("index.json");
        let mut items = VecDeque::new();
        items.push_back(text("hello", 1));
        items.push_back(image("a.png", 2));

        save_history_to_path(dirs.clone(), &path, &items).unwrap();
        let loaded = load_history_from_path(dirs, &path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(matches!(&loaded[0], ClipItem::Text { content, .. } if content == "hello"));
    }

    #[test]
    fn missing_file_returns_empty() {
        let dirs = Directories::discover();
        let tmp = tempfile::tempdir().unwrap();
        let loaded = load_history_from_path(dirs, &tmp.path().join("missing.json")).unwrap();
        assert!(loaded.is_empty());
    }
}
