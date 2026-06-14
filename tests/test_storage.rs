use easycopy::clipboard::history::ClipItem;
use easycopy::config::dirs::Directories;
use easycopy::store::history;
use easycopy::store::ImageStore;
use std::collections::VecDeque;

#[test]
fn index_json_roundtrip_through_storage_helpers() {
    let dirs = Directories::discover();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.json");

    let mut items = VecDeque::new();
    items.push_back(ClipItem::Text {
        content: "hello".into(),
        timestamp: 1234567890,
        use_count: 0,
    });
    items.push_back(ClipItem::Image {
        width: 640,
        height: 480,
        timestamp: 1234567891,
        filename: "img_test.png".into(),
        data: None,
        use_count: 0,
    });

    history::save_history_to_path(&dirs, &path, &items).unwrap();
    let loaded = history::load_history_from_path(&dirs, &path).unwrap();
    assert_eq!(loaded.len(), 2);

    match &loaded[0] {
        ClipItem::Text { content, timestamp, .. } => {
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
        use_count: 0,
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(!json.contains("data"));
}

#[test]
fn image_png_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let store = ImageStore::new_from_dir(dir.path().to_path_buf());
    let w = 4u32;
    let h = 4u32;
    let data = vec![255u8, 0, 0, 255].repeat((w * h) as usize);

    let filename = store.save(&data, w, h).unwrap();
    let (loaded_w, loaded_h, loaded) = store.load(&filename).unwrap();

    assert_eq!((loaded_w, loaded_h), (w, h));
    assert_eq!(loaded.len(), (w * h * 4) as usize);
}

#[test]
fn orphan_cleanup_removes_unused_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = ImageStore::new_from_dir(dir.path().to_path_buf());
    std::fs::write(dir.path().join("used.png"), b"used").unwrap();
    std::fs::write(dir.path().join("unused.png"), b"unused").unwrap();

    let mut items = VecDeque::new();
    items.push_back(ClipItem::Image {
        width: 1,
        height: 1,
        timestamp: 1,
        filename: "used.png".into(),
        data: None,
        use_count: 0,
    });

    let removed = store.cleanup_orphaned(&items).unwrap();
    assert_eq!(removed, 1);
    assert!(dir.path().join("used.png").exists());
    assert!(!dir.path().join("unused.png").exists());
}

#[test]
fn image_thumbnail_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let store = ImageStore::new_from_dir(dir.path().to_path_buf());
    let w = 10u32;
    let h = 10u32;
    let data = vec![0u8; (w * h * 4) as usize];

    // 1. Saving creates the main file and the thumbnail
    let filename = store.save(&data, w, h).unwrap();
    let filepath = dir.path().join(&filename);
    let thumb_filepath = dir.path().join(format!("thumb_{}", filename));

    assert!(filepath.exists());
    assert!(thumb_filepath.exists());

    // 2. Deleting deletes both
    store.delete(&filename);
    assert!(!filepath.exists());
    assert!(!thumb_filepath.exists());
}
