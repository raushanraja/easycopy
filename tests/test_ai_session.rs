use easycopy::ai::session::ChatState;

#[test]
fn chat_state_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chat_state.json");

    let st = ChatState {
        current_session_id: Some("abc-123".into()),
    };
    st.save_to_path(&path).unwrap();
    assert!(path.exists());

    let loaded = ChatState::load_from_path(&path).unwrap();
    assert_eq!(loaded.current_session_id.as_deref(), Some("abc-123"));
}

#[test]
fn chat_state_load_missing_is_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.json");
    let loaded = ChatState::load_from_path(&path).unwrap();
    assert!(loaded.current_session_id.is_none());
}

#[test]
fn new_session_id_is_nonempty_unique() {
    let a = easycopy::ai::session::new_session_id();
    let b = easycopy::ai::session::new_session_id();
    assert!(!a.is_empty());
    assert_ne!(a, b);
}

#[tokio::test]
async fn build_session_service_connects_and_migrates() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("chat.db");
    let svc = easycopy::ai::session::build_session_service(&db).await;
    assert!(
        svc.is_ok(),
        "sqlite session service failed: {:?}",
        svc.err()
    );
    assert!(db.exists(), "db file should be created by connect+migrate");
}
