use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClipItem {
    #[serde(rename = "text")]
    Text {
        content: String,
        timestamp: u64,
    },
    #[serde(rename = "image")]
    Image {
        width: u32,
        height: u32,
        timestamp: u64,
        filename: String,
        #[serde(skip)]
        data: Option<Vec<u8>>,
    },
}

impl ClipItem {
    pub fn timestamp(&self) -> u64 {
        match self {
            ClipItem::Text { timestamp, .. } | ClipItem::Image { timestamp, .. } => *timestamp,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, ClipItem::Text { .. })
    }

    pub fn is_image(&self) -> bool {
        matches!(self, ClipItem::Image { .. })
    }
}

pub struct HistoryManager {
    items: VecDeque<ClipItem>,
    max_text: usize,
    max_image: usize,
}

impl HistoryManager {
    pub fn new(max_text: usize, max_image: usize) -> Self {
        Self {
            items: VecDeque::new(),
            max_text,
            max_image,
        }
    }

    /// Add an item to the front of history.  Returns `true` when the
    /// internal state actually changed (caller should persist to disk).
    pub fn add(&mut self, item: ClipItem) -> bool {
        // Skip if it matches the current front item
        if let Some(front) = self.items.front() {
            match (&item, front) {
                (
                    ClipItem::Text { content: a, .. },
                    ClipItem::Text { content: b, .. },
                ) if a == b => return false,
                (
                    ClipItem::Image { filename: a, .. },
                    ClipItem::Image { filename: b, .. },
                ) if !a.is_empty() && a == b => return false,
                _ => {}
            }
        }

        // Remove older duplicate (move-to-top behaviour for text)
        let dup_pos = match &item {
            ClipItem::Text { content: new_c, .. } => self.items.iter().position(|existing| {
                matches!(existing, ClipItem::Text { content, .. } if content == new_c)
            }),
            ClipItem::Image { filename: new_f, .. } if !new_f.is_empty() => {
                self.items.iter().position(|existing| {
                    matches!(existing, ClipItem::Image { filename, .. } if filename == new_f)
                })
            }
            _ => None,
        };
        if let Some(pos) = dup_pos {
            self.items.remove(pos);
        }

        self.items.push_front(item);
        self.enforce_limits();
        true
    }

    fn enforce_limits(&mut self) {
        // O(n) retain instead of O(n²) remove-in-reverse.
        let max_text = self.max_text;
        let max_image = self.max_image;
        let (mut tc, mut ic) = (0usize, 0usize);
        self.items.retain(|item| match item {
            ClipItem::Text { .. } => {
                tc += 1;
                tc <= max_text
            }
            ClipItem::Image { .. } => {
                ic += 1;
                ic <= max_image
            }
        });
    }

    pub fn remove(&mut self, index: usize) -> Option<ClipItem> {
        self.items.remove(index)
    }

    pub fn search(&self, query: &str) -> Vec<(usize, &ClipItem)> {
        let q = query.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| match item {
                ClipItem::Text { content, .. } => content.to_lowercase().contains(&q),
                ClipItem::Image { width, height, .. } => {
                    if q.is_empty() {
                        return true;
                    }
                    format!("{}×{} {}x{}", width, height, width, height)
                        .to_lowercase()
                        .contains(&q)
                }
            })
            .collect()
    }

    pub fn items(&self) -> &VecDeque<ClipItem> {
        &self.items
    }

    pub fn set_items(&mut self, items: VecDeque<ClipItem>) {
        self.items = items;
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str, ts: u64) -> ClipItem {
        ClipItem::Text {
            content: s.into(),
            timestamp: ts,
        }
    }

    #[test]
    fn test_add_and_len() {
        let mut hm = HistoryManager::new(200, 50);
        assert!(hm.is_empty());
        hm.add(text("a", 1));
        hm.add(text("b", 2));
        assert_eq!(hm.len(), 2);
    }

    #[test]
    fn test_dedup_front() {
        let mut hm = HistoryManager::new(200, 50);
        assert!(hm.add(text("a", 1)));
        assert!(!hm.add(text("a", 2))); // identical front → false
        assert_eq!(hm.len(), 1);
    }

    #[test]
    fn test_move_to_top() {
        let mut hm = HistoryManager::new(200, 50);
        hm.add(text("a", 1));
        hm.add(text("b", 2));
        hm.add(text("c", 3));
        // re-adding "a" should move it to front
        assert!(hm.add(text("a", 4)));
        assert_eq!(hm.len(), 3);
        // front should now be "a"
        match hm.items().front().unwrap() {
            ClipItem::Text { content, .. } => assert_eq!(content, "a"),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn test_enforce_limits() {
        let mut hm = HistoryManager::new(2, 50);
        hm.add(text("a", 1));
        hm.add(text("b", 2));
        hm.add(text("c", 3));
        assert_eq!(hm.len(), 2);
        // "a" should have been evicted
        let has_a = hm.items().iter().any(|i| match i {
            ClipItem::Text { content, .. } => content == "a",
            _ => false,
        });
        assert!(!has_a);
    }

    #[test]
    fn test_search() {
        let mut hm = HistoryManager::new(200, 50);
        hm.add(text("hello world", 1));
        hm.add(text("foo bar", 2));
        hm.add(text("Hello Again", 3));
        let results = hm.search("hello");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut hm = HistoryManager::new(200, 50);
        hm.add(text("a", 1));
        hm.add(text("b", 2));
        let removed = hm.remove(0);
        assert!(removed.is_some());
        assert_eq!(hm.len(), 1);
    }
}
