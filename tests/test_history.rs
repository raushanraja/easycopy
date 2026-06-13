use clipit_rs::history::{ClipItem, HistoryManager};

fn text(s: &str, ts: u64) -> ClipItem {
    ClipItem::Text {
        content: s.into(),
        timestamp: ts,
        use_count: 0,
    }
}

fn img(filename: &str, ts: u64) -> ClipItem {
    ClipItem::Image {
        width: 100,
        height: 100,
        timestamp: ts,
        filename: filename.into(),
        data: None,
        use_count: 0,
    }
}

#[test]
fn add_increases_len() {
    let mut hm = HistoryManager::new(200, 50);
    assert!(hm.add(text("a", 1)));
    assert!(hm.add(text("b", 2)));
    assert_eq!(hm.len(), 2);
}

#[test]
fn duplicate_front_is_noop() {
    let mut hm = HistoryManager::new(200, 50);
    assert!(hm.add(text("x", 1)));
    assert!(!hm.add(text("x", 2)));
    assert_eq!(hm.len(), 1);
}

#[test]
fn duplicate_moves_to_front() {
    let mut hm = HistoryManager::new(200, 50);
    hm.add(text("a", 1));
    hm.add(text("b", 2));
    hm.add(text("c", 3));
    assert!(hm.add(text("a", 4)));
    assert_eq!(hm.len(), 3);
    match hm.items().front().unwrap() {
        ClipItem::Text { content, timestamp, .. } => {
            assert_eq!(content, "a");
            assert_eq!(*timestamp, 4);
        }
        _ => panic!("expected text at front"),
    }
}

#[test]
fn text_limits_keep_newest_items() {
    let mut hm = HistoryManager::new(3, 50);
    for i in 0..10 {
        hm.add(text(&format!("item{i}"), i));
    }
    assert_eq!(hm.len(), 3);
    let values: Vec<String> = hm
        .items()
        .iter()
        .filter_map(|item| match item {
            ClipItem::Text { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec!["item9", "item8", "item7"]);
}

#[test]
fn image_limits_keep_newest_items() {
    let mut hm = HistoryManager::new(200, 3);
    for i in 0..10 {
        hm.add(img(&format!("img{i}.png"), i));
    }
    assert_eq!(hm.len(), 3);
    let values: Vec<String> = hm
        .items()
        .iter()
        .filter_map(|item| match item {
            ClipItem::Image { filename, .. } => Some(filename.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec!["img9.png", "img8.png", "img7.png"]);
}

#[test]
fn mixed_limits_are_enforced_independently() {
    let mut hm = HistoryManager::new(2, 2);
    hm.add(text("t1", 1));
    hm.add(img("i1.png", 2));
    hm.add(text("t2", 3));
    hm.add(img("i2.png", 4));
    hm.add(text("t3", 5));
    hm.add(img("i3.png", 6));

    assert_eq!(hm.items().iter().filter(|i| i.is_text()).count(), 2);
    assert_eq!(hm.items().iter().filter(|i| i.is_image()).count(), 2);
    assert!(!hm
        .items()
        .iter()
        .any(|i| matches!(i, ClipItem::Text { content, .. } if content == "t1")));
    assert!(!hm
        .items()
        .iter()
        .any(|i| matches!(i, ClipItem::Image { filename, .. } if filename == "i1.png")));
}

#[test]
fn search_text_case_insensitive() {
    let mut hm = HistoryManager::new(200, 50);
    hm.add(text("Hello World", 1));
    hm.add(text("foo bar", 2));
    hm.add(text("HELLO again", 3));
    let results = hm.search("hello");
    assert_eq!(results.len(), 2);
}

#[test]
fn search_images_by_dimensions() {
    let mut hm = HistoryManager::new(200, 50);
    hm.add(img("a.png", 1));
    hm.add(text("text", 2));
    let results = hm.search("100");
    assert_eq!(results.len(), 1);
    assert!(results[0].1.is_image());
}

#[test]
fn remove_item() {
    let mut hm = HistoryManager::new(200, 50);
    hm.add(text("a", 1));
    hm.add(text("b", 2));
    let removed = hm.remove(0);
    assert!(removed.is_some());
    assert_eq!(hm.len(), 1);
}

#[test]
fn clear_empties_all() {
    let mut hm = HistoryManager::new(200, 50);
    hm.add(text("a", 1));
    hm.add(text("b", 2));
    hm.add(img("c.png", 3));
    hm.clear();
    assert!(hm.is_empty());
}
