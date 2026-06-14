use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Result;
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::config::Config;
use crate::opener;

// ── Type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    pub query: String,
    pub url: String,
    pub description: String,
    pub use_count: usize,
}

// ── Shortcut table (single source of truth) ─────────────────────────

static SHORTCUTS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    [
        ("google", "https://www.google.com"),
        ("gmail", "https://mail.google.com"),
        ("x", "https://x.com"),
        ("twitter", "https://x.com"),
        ("twitch", "https://www.twitch.tv"),
        ("alibaba", "https://www.alibaba.com"),
        ("amazon", "https://www.amazon.in"),
        ("github", "https://github.com"),
        ("youtube", "https://www.youtube.com"),
        ("reddit", "https://www.reddit.com"),
    ]
    .into_iter()
    .collect()
});

// ── Query mode ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryMode {
    Normal,
    AppsOnly,
    Browser,
}

pub fn filter_query(query: &str) -> (QueryMode, String) {
    let trimmed = query.trim();
    if let Some(app_query) = trimmed.strip_prefix('/') {
        (QueryMode::AppsOnly, app_query.trim().to_lowercase())
    } else if let Some(browser_query) = trimmed.strip_prefix(':') {
        (QueryMode::Browser, browser_query.trim().to_string())
    } else {
        (QueryMode::Normal, trimmed.to_lowercase())
    }
}

// ── Resolution ──────────────────────────────────────────────────────

/// Resolve a raw query string to a fully-formed BrowserAction.
/// Returns None for empty queries.
pub fn resolve(query: &str) -> Option<BrowserAction> {
    let trimmed = query.trim();
    let text = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
    if text.is_empty() {
        return None;
    }

    let lower = text.to_lowercase();

    // 1. Shortcut lookup
    if let Some(&url) = SHORTCUTS.get(lower.as_str()) {
        return Some(BrowserAction {
            query: text.to_string(),
            url: url.to_string(),
            description: format!("Open {}", url),
            use_count: 0,
        });
    }

    // 2. Numeric → localhost
    if text.chars().all(|c| c.is_ascii_digit()) {
        let url = format!("http://localhost:{}", text);
        return Some(BrowserAction {
            query: text.to_string(),
            url: url.clone(),
            description: format!("Open {}", url),
            use_count: 0,
        });
    }

    // 3. Domain with dots (no spaces)
    if text.contains('.') && !text.contains(' ') {
        let url = if text.starts_with("http://") || text.starts_with("https://") {
            text.to_string()
        } else {
            format!("https://{}", text)
        };
        return Some(BrowserAction {
            query: text.to_string(),
            url: url.clone(),
            description: format!("Open {}", url),
            use_count: 0,
        });
    }

    // 4. Fallback → Google search
    let encoded = percent_encode(text);
    let url = format!("https://www.google.com/search?q={}", encoded);
    Some(BrowserAction {
        query: text.to_string(),
        url: url.clone(),
        description: format!("Search Google for {}", text),
        use_count: 0,
    })
}

/// Search saved actions by query text, returning indices sorted by
/// use_count descending. The query is the raw user input (with or without
/// `:` prefix).
pub fn search(actions: &[BrowserAction], query: &str) -> Vec<usize> {
    let trimmed = query.trim();
    let text = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
    if text.is_empty() {
        return Vec::new();
    }
    let q = text.to_lowercase();

    let mut matches: Vec<(usize, &BrowserAction)> = actions
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            let haystack = format!("{} {} {}", a.query, a.url, a.description).to_lowercase();
            haystack.contains(&q)
        })
        .collect();

    matches.sort_by(|(_, a), (_, b)| b.use_count.cmp(&a.use_count));

    matches.into_iter().map(|(i, _)| i).collect()
}

// ── Execution ───────────────────────────────────────────────────────

/// Open a URL in the user's browser.
pub fn open_url(url: &str) -> Result<()> {
    opener::open_url(url)
}

// ── Persistence ─────────────────────────────────────────────────────

fn data_dir() -> PathBuf {
    Config::data_dir()
}

pub fn save(actions: &[BrowserAction]) -> Result<()> {
    let path = data_dir().join("browser_actions.json");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("json.tmp");
    let file = fs::File::create(&tmp)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, &actions)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load() -> Vec<BrowserAction> {
    let path = data_dir().join("browser_actions.json");
    if !path.exists() {
        return Vec::new();
    }
    let json = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    match serde_json::from_str(&json) {
        Ok(actions) => actions,
        Err(_) => Vec::new(),
    }
}

// ── Utilities ───────────────────────────────────────────────────────

pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_table_has_no_duplicates() {
        let mut keys = Vec::new();
        for key in SHORTCUTS.keys() {
            keys.push(*key);
        }
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(keys.len(), sorted.len(), "duplicate shortcut keys found");
    }

    #[test]
    fn resolve_shortcut_google() {
        let action = resolve(":google").unwrap();
        assert_eq!(action.url, "https://www.google.com");
        assert_eq!(action.description, "Open https://www.google.com");
    }

    #[test]
    fn resolve_shortcut_case_insensitive() {
        let action = resolve(":GOOGLE").unwrap();
        assert_eq!(action.url, "https://www.google.com");
    }

    #[test]
    fn resolve_numeric_localhost() {
        let action = resolve(":3000").unwrap();
        assert_eq!(action.url, "http://localhost:3000");
    }

    #[test]
    fn resolve_domain_with_dots() {
        let action = resolve("example.com").unwrap();
        assert_eq!(action.url, "https://example.com");
    }

    #[test]
    fn resolve_full_url_passthrough() {
        let action = resolve("https://rust-lang.org").unwrap();
        assert_eq!(action.url, "https://rust-lang.org");
    }

    #[test]
    fn resolve_fallback_google_search() {
        let action = resolve(":hello world").unwrap();
        assert!(action.url.contains("google.com/search"));
        assert_eq!(action.description, "Search Google for hello world");
    }

    #[test]
    fn resolve_empty_returns_none() {
        assert!(resolve(":").is_none());
        assert!(resolve("").is_none());
        assert!(resolve("   ").is_none());
    }

    #[test]
    fn search_sorts_by_use_count() {
        let actions = vec![
            BrowserAction { query: "a".into(), url: "http://a.com".into(), description: "A".into(), use_count: 1 },
            BrowserAction { query: "b".into(), url: "http://b.com".into(), description: "B".into(), use_count: 5 },
            BrowserAction { query: "c".into(), url: "http://c.com".into(), description: "C".into(), use_count: 3 },
        ];
        let indices = search(&actions, "http");
        // "http" matches all three (all URLs contain it); sorted by use_count desc
        assert_eq!(indices.len(), 3);
        assert_eq!(actions[indices[0]].query, "b"); // use_count=5
        assert_eq!(actions[indices[1]].query, "c"); // use_count=3
        assert_eq!(actions[indices[2]].query, "a"); // use_count=1
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let actions = vec![
            BrowserAction { query: "a".into(), url: "http://a".into(), description: "A".into(), use_count: 0 },
        ];
        assert!(search(&actions, "").is_empty());
        assert!(search(&actions, ":").is_empty());
    }

    #[test]
    fn filter_query_modes() {
        assert_eq!(filter_query("/firefox"), (QueryMode::AppsOnly, "firefox".into()));
        assert_eq!(filter_query(" / terminal "), (QueryMode::AppsOnly, "terminal".into()));
        assert_eq!(filter_query(":google"), (QueryMode::Browser, "google".into()));
        assert_eq!(filter_query(" : hello "), (QueryMode::Browser, "hello".into()));
        assert_eq!(filter_query("hello"), (QueryMode::Normal, "hello".into()));
        assert_eq!(filter_query("  "), (QueryMode::Normal, "".into()));
    }

    #[test]
    fn percent_encode_basic() {
        assert_eq!(percent_encode("hello world"), "hello+world");
        assert_eq!(percent_encode("a-b_c.d"), "a-b_c.d");
        assert_eq!(percent_encode("foo&bar"), "foo%26bar");
    }
}
