use crate::dirs::Directories;
use crate::history::ClipItem;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::io::Result;
use std::path::{Path, PathBuf};

// ================================================================
//  IMAGE STORE
// ================================================================
// Owns the images directory and all image lifecycle operations.
// Extracted from storage.rs to decouple image I/O from Config paths.

#[derive(Clone)]
pub struct ImageStore {
    dir: PathBuf,
}

impl ImageStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Create an ImageStore from Directories (for convenience in non-test code).
    /// Test code should use `ImageStore::new(temp_dir)` directly.
    pub fn from_dirs() -> Self {
        let dir = Directories::images_dir();
        Self::new(dir)
    }

    /// Create an ImageStore from Config (legacy — prefer from_dirs).
    pub fn from_config() -> Self {
        Self::from_dirs()
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Save RGBA image data, returns the generated filename.
    pub fn save(&self, data: &[u8], w: u32, h: u32) -> Result<String> {
        self.save_to_dir(&self.dir, data, w, h)
    }

    /// Save RGBA image data taking ownership of the buffer.
    pub fn save_owned(&self, data: Vec<u8>, w: u32, h: u32) -> Result<String> {
        self.save_owned_to_dir(&self.dir, data, w, h)
    }

    /// Load a saved image, returns (width, height, rgba_bytes).
    pub fn load(&self, filename: &str) -> Result<(u32, u32, Vec<u8>)> {
        self.load_from_dir(&self.dir, filename)
    }

    /// Delete an image file and its thumbnail.
    pub fn delete(&self, filename: &str) {
        let _ = std::fs::remove_file(self.dir.join(filename));
        let _ = std::fs::remove_file(self.dir.join(format!("thumb_{}", filename)));
    }

    /// Remove image files not referenced by any clip item.
    pub fn cleanup_orphaned(&self, items: &VecDeque<ClipItem>) -> Result<usize> {
        self.cleanup_orphaned_in_dir(&self.dir, items)
    }

    /// Thumbnail path for a given filename.
    pub fn thumbnail_path(&self, filename: &str) -> PathBuf {
        self.dir.join(format!("thumb_{}", filename))
    }

    /// Full path for a given filename.
    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.dir.join(filename)
    }

    // ── internal helpers (operate on any dir) ─────────────────────

    pub(crate) fn save_to_dir(&self, dir: &Path, data: &[u8], w: u32, h: u32) -> Result<String> {
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

        let _ = save_thumbnail_to_dir(dir, &filename, &img);
        Ok(filename)
    }

    pub(crate) fn save_owned_to_dir(
        &self,
        dir: &Path,
        data: Vec<u8>,
        w: u32,
        h: u32,
    ) -> Result<String> {
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

        let _ = save_thumbnail_to_dir(dir, &filename, &img);
        Ok(filename)
    }

    pub(crate) fn load_from_dir(&self, dir: &Path, filename: &str) -> Result<(u32, u32, Vec<u8>)> {
        let path = dir.join(filename);
        let img = image::open(&path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Ok((w, h, rgba.into_raw()))
    }

    pub(crate) fn cleanup_orphaned_in_dir(
        &self,
        dir: &Path,
        items: &VecDeque<ClipItem>,
    ) -> Result<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let known: HashSet<&str> = items
            .iter()
            .filter_map(|i| match i {
                ClipItem::Image { filename, .. } if !filename.is_empty() => {
                    Some(filename.as_str())
                }
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
}

// ================================================================
//  INTERNAL HELPERS
// ================================================================

fn save_thumbnail_to_dir(dir: &Path, filename: &str, img: &image::RgbaImage) -> Result<()> {
    let dyn_img = image::DynamicImage::ImageRgba8(img.clone());
    let thumb = dyn_img.resize(52, 52, image::imageops::FilterType::Triangle);
    let thumb_path = dir.join(format!("thumb_{}", filename));
    thumb
        .save(&thumb_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(())
}

fn simple_hash(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}

// ================================================================
//  TESTS
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

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
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        let data = vec![255u8, 0, 0, 255].repeat(4); // 2x2 red RGBA
        let filename = store.save(&data, 2, 2).unwrap();
        let (w, h, loaded) = store.load(&filename).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(loaded.len(), data.len());
    }

    #[test]
    fn save_owned_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        let data = vec![0u8, 255, 0, 255].repeat(4); // 2x2 green RGBA
        let filename = store.save_owned(data.clone(), 2, 2).unwrap();
        let (w, h, loaded) = store.load(&filename).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(loaded, data);
    }

    #[test]
    fn save_creates_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        let data = vec![255u8, 0, 0, 255].repeat(4);
        let filename = store.save(&data, 2, 2).unwrap();
        let thumb = store.thumbnail_path(&filename);
        assert!(thumb.exists());
    }

    #[test]
    fn delete_removes_image_and_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        let data = vec![255u8, 0, 0, 255].repeat(4);
        let filename = store.save(&data, 2, 2).unwrap();
        assert!(store.file_path(&filename).exists());
        assert!(store.thumbnail_path(&filename).exists());
        store.delete(&filename);
        assert!(!store.file_path(&filename).exists());
        assert!(!store.thumbnail_path(&filename).exists());
    }

    #[test]
    fn invalid_rgba_buffer_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        let result = store.save(&[1, 2, 3], 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_removes_only_unknown_images() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        std::fs::write(dir.path().join("keep.png"), b"keep").unwrap();
        std::fs::write(dir.path().join("delete.png"), b"delete").unwrap();

        let mut items = VecDeque::new();
        items.push_back(image("keep.png", 1));
        let removed = store.cleanup_orphaned(&items).unwrap();

        assert_eq!(removed, 1);
        assert!(dir.path().join("keep.png").exists());
        assert!(!dir.path().join("delete.png").exists());
    }

    #[test]
    fn dir_returns_correct_path() {
        let dir = tempfile::tempdir().unwrap();
        let store = ImageStore::new(dir.path().to_path_buf());
        assert_eq!(store.dir(), dir.path());
    }
}
