use crate::config::Config;
use crate::history::ClipItem;
use crate::image_store::ImageStore;
use std::collections::VecDeque;
use std::io::Result;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct Index {
    items: VecDeque<ClipItem>,
}

#[derive(serde::Serialize)]
struct IndexRef<'a> {
    items: &'a VecDeque<ClipItem>,
}

// ── index persistence ──────────────────────────────────────────────

pub fn history_path() -> PathBuf {
    Config::data_dir().join("index.json")
}

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

// ── image files (delegates to ImageStore) ──────────────────────────

use std::sync::OnceLock;

fn image_store() -> &'static ImageStore {
    static STORE: OnceLock<ImageStore> = OnceLock::new();
    STORE.get_or_init(|| ImageStore::from_config())
}

pub fn save_image(data: &[u8], w: u32, h: u32) -> Result<String> {
    image_store().save(data, w, h)
}

pub fn save_image_to_dir(dir: &Path, data: &[u8], w: u32, h: u32) -> Result<String> {
    let store = ImageStore::new(dir.to_path_buf());
    store.save_to_dir(dir, data, w, h)
}

pub fn save_image_owned(data: Vec<u8>, w: u32, h: u32) -> Result<String> {
    image_store().save_owned(data, w, h)
}

pub fn save_image_owned_to_dir(
    dir: &Path,
    data: Vec<u8>,
    w: u32,
    h: u32,
) -> Result<String> {
    let store = ImageStore::new(dir.to_path_buf());
    store.save_owned_to_dir(dir, data, w, h)
}

pub fn load_image(filename: &str) -> Result<(u32, u32, Vec<u8>)> {
    image_store().load(filename)
}

pub fn load_image_from_dir(dir: &Path, filename: &str) -> Result<(u32, u32, Vec<u8>)> {
    let store = ImageStore::new(dir.to_path_buf());
    store.load_from_dir(dir, filename)
}

pub fn delete_image_file(filename: &str) {
    image_store().delete(filename);
}

pub fn delete_image_file_in_dir(dir: &Path, filename: &str) -> Result<()> {
    let path = dir.join(filename);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let thumb_path = dir.join(format!("thumb_{}", filename));
    if thumb_path.exists() {
        std::fs::remove_file(thumb_path)?;
    }
    Ok(())
}

pub fn cleanup_orphaned(items: &VecDeque<ClipItem>) {
    let _ = image_store().cleanup_orphaned(items);
}

pub fn cleanup_orphaned_in_dir(dir: &Path, items: &VecDeque<ClipItem>) -> Result<usize> {
    let store = ImageStore::new(dir.to_path_buf());
    store.cleanup_orphaned_in_dir(dir, items)
}

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

    #[test]
    fn image_file_roundtrip_to_specific_dir() {
        let dir = tempfile::tempdir().unwrap();
        let data = vec![255u8, 0, 0, 255].repeat(4); // 2x2 red RGBA
        let filename = save_image_to_dir(dir.path(), &data, 2, 2).unwrap();
        let (w, h, loaded) = load_image_from_dir(dir.path(), &filename).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(loaded.len(), data.len());
    }

    #[test]
    fn invalid_rgba_buffer_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let result = save_image_to_dir(dir.path(), &[1, 2, 3], 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_removes_only_unknown_images() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("keep.png"), b"keep").unwrap();
        std::fs::write(dir.path().join("delete.png"), b"delete").unwrap();

        let mut items = VecDeque::new();
        items.push_back(image("keep.png", 1));
        let removed = cleanup_orphaned_in_dir(dir.path(), &items).unwrap();

        assert_eq!(removed, 1);
        assert!(dir.path().join("keep.png").exists());
        assert!(!dir.path().join("delete.png").exists());
    }
}
