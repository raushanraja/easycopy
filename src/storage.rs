use crate::config::Config;
use crate::history::ClipItem;
use std::collections::{HashSet, VecDeque};
use std::io::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserAction {
    pub query: String,
    pub url: String,
    pub description: String,
    pub use_count: usize,
}

#[derive(serde::Deserialize)]
struct BrowserActionIndex {
    actions: Vec<BrowserAction>,
}

#[derive(serde::Serialize)]
struct BrowserActionIndexRef<'a> {
    actions: &'a [BrowserAction],
}

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

// ── browser actions persistence ──────────────────────────────────

pub fn browser_actions_path() -> PathBuf {
    Config::data_dir().join("browser_actions.json")
}

pub fn save_browser_actions(actions: &[BrowserAction]) -> Result<()> {
    let path = browser_actions_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let file = std::fs::File::create(&tmp)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, &BrowserActionIndexRef { actions })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::rename(&tmp, path)?; // atomic on same filesystem
    Ok(())
}

pub fn load_browser_actions() -> Vec<BrowserAction> {
    let path = browser_actions_path();
    if !path.exists() {
        return Vec::new();
    }
    let json = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    match serde_json::from_str::<BrowserActionIndex>(&json) {
        Ok(idx) => idx.actions,
        Err(_) => Vec::new(),
    }
}

// ── image files ────────────────────────────────────────────────────

fn save_thumbnail_to_dir(dir: &Path, filename: &str, img: &image::RgbaImage) -> Result<()> {
    let dyn_img = image::DynamicImage::ImageRgba8(img.clone());
    let thumb = dyn_img.resize(52, 52, image::imageops::FilterType::Triangle);
    let thumb_path = dir.join(format!("thumb_{}", filename));
    thumb
        .save(&thumb_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(())
}

pub fn save_image(data: &[u8], w: u32, h: u32) -> Result<String> {
    save_image_to_dir(&Config::images_dir(), data, w, h)
}

pub fn save_image_to_dir(dir: &Path, data: &[u8], w: u32, h: u32) -> Result<String> {
    std::fs::create_dir_all(dir)?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let hash = simple_hash(data) & 0xFFF;
    let filename = format!("img_{}_{:03x}.png", ts, hash);
    let filepath = dir.join(&filename);
    let img = image::RgbaImage::from_raw(w, h, data.to_vec()).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "bad RGBA image data")
    })?;
    img.save(&filepath)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Save thumbnail too
    let _ = save_thumbnail_to_dir(dir, &filename, &img);

    Ok(filename)
}

/// Like `save_image` but takes ownership of the data buffer, avoiding an
/// extra copy when the caller already owns the `Vec<u8>`.
pub fn save_image_owned(data: Vec<u8>, w: u32, h: u32) -> Result<String> {
    save_image_owned_to_dir(&Config::images_dir(), data, w, h)
}

pub fn save_image_owned_to_dir(dir: &Path, data: Vec<u8>, w: u32, h: u32) -> Result<String> {
    std::fs::create_dir_all(dir)?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let hash = simple_hash(&data) & 0xFFF;
    let filename = format!("img_{}_{:03x}.png", ts, hash);
    let filepath = dir.join(&filename);
    let img = image::RgbaImage::from_raw(w, h, data).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "bad RGBA image data")
    })?;
    img.save(&filepath)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Save thumbnail too
    let _ = save_thumbnail_to_dir(dir, &filename, &img);

    Ok(filename)
}

pub fn load_image(filename: &str) -> Result<(u32, u32, Vec<u8>)> {
    load_image_from_dir(&Config::images_dir(), filename)
}

pub fn load_image_from_dir(dir: &Path, filename: &str) -> Result<(u32, u32, Vec<u8>)> {
    let path = dir.join(filename);
    let img = image::open(&path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}

pub fn delete_image_file(filename: &str) {
    let _ = std::fs::remove_file(Config::images_dir().join(filename));
    let _ = std::fs::remove_file(Config::images_dir().join(format!("thumb_{}", filename)));
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
    let _ = cleanup_orphaned_in_dir(&Config::images_dir(), items);
}

pub fn cleanup_orphaned_in_dir(dir: &Path, items: &VecDeque<ClipItem>) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let known: HashSet<&str> = items
        .iter()
        .filter_map(|i| match i {
            ClipItem::Image { filename, .. } if !filename.is_empty() => Some(filename.as_str()),
            _ => None,
        })
        .collect();

    let mut removed = 0usize;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let base_name = name.strip_prefix("thumb_").unwrap_or(name);
        if !known.contains(base_name) {
            std::fs::remove_file(path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn simple_hash(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
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
