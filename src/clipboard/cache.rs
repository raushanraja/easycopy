use crate::clipboard::history::ClipItem;
use std::collections::HashMap;
use std::path::Path;

// ================================================================
//  CLIP CACHE
// ================================================================
// Owns the pre-computed per-clip display data that PopupApp needs
// for search, preview rendering, and file-size display. Extracted
// from PopupApp so the cache invariants (all vectors same length,
// consistent with clips list) are encapsulated in one place.

#[derive(Debug, Clone, Default)]
pub struct ClipCache {
    char_counts: Vec<usize>,
    previews: Vec<String>,
    search: Vec<String>,
    file_sizes: HashMap<String, u64>,
}

impl ClipCache {
    /// Build all caches from a list of clips. Call from a background thread.
    pub fn build_from(clips: &[ClipItem], preview_chars: usize, images_dir: &Path) -> Self {
        let mut char_counts = Vec::with_capacity(clips.len());
        let mut previews = Vec::with_capacity(clips.len());
        let mut search = Vec::with_capacity(clips.len());
        let mut file_sizes = HashMap::new();

        for item in clips {
            match item {
                ClipItem::Text { content, .. } => {
                    char_counts.push(content.chars().count());
                    previews.push(preview_text(content, preview_chars));
                    search.push(content.to_lowercase());
                }
                ClipItem::Image {
                    width,
                    height,
                    filename,
                    ..
                } => {
                    char_counts.push(0);
                    previews.push(String::new());
                    search.push(
                        format!("{}\u{00d7}{} {}x{} {}", width, height, width, height, filename)
                            .to_lowercase(),
                    );
                    if !filename.is_empty() && !file_sizes.contains_key(filename) {
                        if let Ok(meta) = std::fs::metadata(images_dir.join(filename)) {
                            file_sizes.insert(filename.clone(), meta.len());
                        }
                    }
                }
            }
        }

        Self {
            char_counts,
            previews,
            search,
            file_sizes,
        }
    }

    /// Rebuild caches from the current clips list (for incremental updates).
    pub fn rebuild_from(&mut self, clips: &[ClipItem], preview_chars: usize, images_dir: &Path) {
        *self = Self::build_from(clips, preview_chars, images_dir);
    }

    pub fn len(&self) -> usize {
        self.char_counts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.char_counts.is_empty()
    }

    pub fn clear(&mut self) {
        self.char_counts.clear();
        self.previews.clear();
        self.search.clear();
        self.file_sizes.clear();
    }

    // ── per-item accessors ────────────────────────────────────────

    /// Preview text for a clip index, or None if index is out of range.
    pub fn get_preview(&self, idx: usize) -> Option<&str> {
        self.previews.get(idx).map(|s| s.as_str())
    }

    /// Character count for a clip index, or None if out of range.
    pub fn get_char_count(&self, idx: usize) -> Option<usize> {
        self.char_counts.get(idx).copied()
    }

    /// Search text for a clip index. Returns "" for out-of-range.
    pub fn search_text(&self, idx: usize) -> &str {
        self.search.get(idx).map(|s| s.as_str()).unwrap_or("")
    }

    /// Cached file size for a filename, if known.
    pub fn file_size(&self, filename: &str) -> Option<u64> {
        self.file_sizes.get(filename).copied()
    }

    /// Whether the clip at idx matches the query.
    pub fn matches_query(&self, idx: usize, query: &str) -> bool {
        self.search
            .get(idx)
            .map(|s| s.contains(query))
            .unwrap_or(false)
    }
}

// ================================================================
//  INTERNAL HELPERS
// ================================================================

pub fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(max_chars).collect::<String>();
    if normalized.chars().count() > max_chars {
        preview.push('\u{2026}');
    }
    preview
}

// ================================================================
//  TESTS
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn text_item(content: &str) -> ClipItem {
        ClipItem::Text {
            content: content.into(),
            timestamp: 0,
            use_count: 0,
        }
    }

    fn image_item(filename: &str) -> ClipItem {
        ClipItem::Image {
            width: 100,
            height: 200,
            timestamp: 0,
            filename: filename.into(),
            data: None,
            use_count: 0,
        }
    }

    #[test]
    fn build_from_computes_all_caches() {
        let clips = vec![text_item("hello world"), image_item("img.png")];
        let dir = std::path::Path::new("/tmp");
        let cache = ClipCache::build_from(&clips, 10, dir);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get_char_count(0), Some(11));
        assert_eq!(cache.get_char_count(1), Some(0));
        assert!(cache.get_preview(0).unwrap().contains("hello"));
        assert!(cache.search_text(1).contains("100"));
    }

    #[test]
    fn clear_empties_all_fields() {
        let mut cache = ClipCache::default();
        cache.char_counts.push(1);
        cache.previews.push("x".into());
        cache.search.push("y".into());
        cache.file_sizes.insert("f".into(), 42);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.file_size("f"), None);
    }

    #[test]
    fn matches_query_checks_search_field() {
        let clips = vec![text_item("hello world")];
        let dir = std::path::Path::new("/tmp");
        let cache = ClipCache::build_from(&clips, 10, dir);
        assert!(cache.matches_query(0, "hello"));
        assert!(!cache.matches_query(0, "goodbye"));
    }

    #[test]
    fn file_size_lookup() {
        let mut cache = ClipCache::default();
        cache.file_sizes.insert("a.png".into(), 1024);
        assert_eq!(cache.file_size("a.png"), Some(1024));
        assert_eq!(cache.file_size("b.png"), None);
    }

    #[test]
    fn rebuild_from_replaces_all_data() {
        let clips1 = vec![text_item("first")];
        let clips2 = vec![text_item("second"), text_item("third")];
        let dir = std::path::Path::new("/tmp");
        let mut cache = ClipCache::build_from(&clips1, 10, dir);
        cache.rebuild_from(&clips2, 5, dir);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get_char_count(0), Some(6)); // "second"
    }

    #[test]
    fn preview_text_truncates_long_text() {
        let result = preview_text("hello world foo bar", 5);
        assert!(result.contains("…"));
        assert!(result.starts_with("hello"));
    }

    #[test]
    fn preview_text_collapses_whitespace() {
        let result = preview_text("hello   world\t\tfoo", 20);
        assert_eq!(result, "hello world foo");
    }
}
