use crate::history::ClipItem;
use arboard::Clipboard;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct ClipboardMonitor {
    last_text: Option<String>,
    last_image_hash: Option<u64>,
    clipboard: Option<Clipboard>,
}

impl ClipboardMonitor {
    pub fn new() -> Self {
        Self {
            last_text: None,
            last_image_hash: None,
            clipboard: Clipboard::new().ok(),
        }
    }

    /// Poll the system clipboard.  Returns `Some(ClipItem)` when new
    /// content is detected.  For images the raw RGBA bytes are
    /// temporarily stored in the `data` field; the caller is
    /// responsible for saving them to disk and clearing that field
    /// before adding the item to history.
    pub fn poll(&mut self) -> Option<ClipItem> {
        let clipboard = match &mut self.clipboard {
            Some(cb) => cb,
            None => {
                self.clipboard = Clipboard::new().ok();
                return None;
            }
        };

        // ── try text ──────────────────────────────────────────────
        // A text miss can simply mean the clipboard currently contains
        // an image, so do not treat every get_text error as fatal.
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                let is_new = match &self.last_text {
                    Some(prev) => prev != &text,
                    None => true,
                };
                if is_new {
                    let item = ClipItem::Text {
                        content: text.clone(),
                        timestamp: now_ts(),
                    };
                    self.last_text = Some(text);
                    self.last_image_hash = None;
                    return Some(item);
                }
                return None; // unchanged text
            }
        }

        // ── try image ─────────────────────────────────────────────
        match clipboard.get_image() {
            Ok(img) => {
                let hash = byte_hash(&img.bytes);
                let is_new = match self.last_image_hash {
                    Some(prev) => prev != hash,
                    None => true,
                };
                if is_new {
                    let item = ClipItem::Image {
                        width: img.width as u32,
                        height: img.height as u32,
                        timestamp: now_ts(),
                        filename: String::new(),
                        data: Some(img.bytes.into_owned()),
                    };
                    self.last_image_hash = Some(hash);
                    self.last_text = None;
                    return Some(item);
                }
            }
            Err(_) => { /* no image or error */ }
        }

        None
    }
}

fn byte_hash(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

pub fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
