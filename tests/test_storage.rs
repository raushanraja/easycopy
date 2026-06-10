use clipit_rs::history::ClipItem;
use clipit_rs::storage;
use std::collections::VecDeque;

#[test]
fn index_json_roundtrip_through_storage_helpers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.json");

    let mut items = VecDeque::new();
    items.push_back(ClipItem::Text {
        content: "hello".into(),
        timestamp: 1234567890,
    });
    items.push_back(ClipItem::Image {
        width: 640,
        height: 480,
        timestamp: 1234567891,
        filename: "img_test.png".into(),
        data: None,
    });

    storage::save_history_to_path(&path, &items).unwrap();
    let loaded = storage::load_history_from_path(&path).unwrap();
    assert_eq!(loaded.len(), 2);

    match &loaded[0] {
        ClipItem::Text { content, timestamp } => {
            assert_eq!(content, "hello");
            assert_eq!(*timestamp, 1234567890);
        }
        _ => panic!("expected Text"),
    }

    match &loaded[1] {
        ClipItem::Image {
            width,
            height,
            filename,
            ..
        } => {
            assert_eq!(*width, 640);
            assert_eq!(*height, 480);
            assert_eq!(filename, "img_test.png");
        }
        _ => panic!("expected Image"),
    }
}

#[test]
fn image_data_not_in_json() {
    let item = ClipItem::Image {
        width: 10,
        height: 10,
        timestamp: 100,
        filename: "test.png".into(),
        data: Some(vec![0u8; 400]),
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(!json.contains("data"));
}

#[test]
fn image_png_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let w = 4u32;
    let h = 4u32;
    let data = vec![255u8, 0, 0, 255].repeat((w * h) as usize);

    let filename = storage::save_image_to_dir(dir.path(), &data, w, h).unwrap();
    let (loaded_w, loaded_h, loaded) = storage::load_image_from_dir(dir.path(), &filename).unwrap();

    assert_eq!((loaded_w, loaded_h), (w, h));
    assert_eq!(loaded.len(), (w * h * 4) as usize);
}

#[test]
fn orphan_cleanup_removes_unused_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("used.png"), b"used").unwrap();
    std::fs::write(dir.path().join("unused.png"), b"unused").unwrap();

    let mut items = VecDeque::new();
    items.push_back(ClipItem::Image {
        width: 1,
        height: 1,
        timestamp: 1,
        filename: "used.png".into(),
        data: None,
    });

    let removed = storage::cleanup_orphaned_in_dir(dir.path(), &items).unwrap();
    assert_eq!(removed, 1);
    assert!(dir.path().join("used.png").exists());
    assert!(!dir.path().join("unused.png").exists());
}
