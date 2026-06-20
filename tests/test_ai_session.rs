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

#[tokio::test]
async fn test_load_history_async() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("chat.db");
    let svc = easycopy::ai::session::build_session_service(&db)
        .await
        .unwrap();

    let session_id = "test-session-123";
    let _session = svc
        .create(adk_rust::session::CreateRequest {
            app_name: "easycopy".to_string(),
            user_id: "easycopy-user".to_string(),
            session_id: Some(session_id.to_string()),
            state: std::collections::HashMap::new(),
        })
        .await
        .unwrap();

    // Append user event
    let mut ev_user = adk_rust::Event::new("inv-1");
    ev_user.author = "easycopy-user".to_string();
    ev_user.llm_response.content = Some(adk_rust::Content::new("user").with_text("hello AI"));
    svc.append_event(session_id, ev_user).await.unwrap();

    // Append assistant event
    let mut ev_assistant = adk_rust::Event::new("inv-2");
    ev_assistant.author = "assistant".to_string();
    ev_assistant.llm_response.content =
        Some(adk_rust::Content::new("model").with_text("hello user"));
    svc.append_event(session_id, ev_assistant).await.unwrap();

    // Load history
    let msgs = easycopy::ai::session::load_history_async(&db, session_id)
        .await
        .unwrap();
    assert_eq!(msgs.len(), 2);
    match &msgs[0] {
        easycopy::ai::ChatMessage::User(t) => assert_eq!(t, "hello AI"),
        _ => panic!("Expected user message"),
    }
    match &msgs[1] {
        easycopy::ai::ChatMessage::Assistant(t) => assert_eq!(t, "hello user"),
        _ => panic!("Expected assistant message"),
    }
}
