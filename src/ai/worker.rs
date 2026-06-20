use crate::ai::client::build_agent;
use crate::ai::session::build_session_service;
use crate::config::config::AiConfig;
use adk_rust::futures::StreamExt;
use adk_rust::prelude::*;
use adk_rust::runner::Runner;
use adk_rust::session::CreateRequest;
use adk_rust::{SessionId, UserId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Notify;

const APP: &str = "easycopy";
const USER: &str = "easycopy-user";

/// Messages the worker sends back to the egui main thread.
pub enum ChatEvent {
    Delta(String),
    Done,
    Error(String),
}

/// Spawn a worker thread that runs one chat turn and streams text deltas back.
/// `cancel` is signalled (`.notify_one()`) to drop the stream mid-flight (Esc).
pub fn spawn_turn(
    cfg: AiConfig,
    db_path: PathBuf,
    session_id: String,
    prompt: String,
    tx: mpsc::Sender<ChatEvent>,
    cancel: Arc<Notify>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(ChatEvent::Error(format!("runtime: {e}")));
                return;
            }
        };
        rt.block_on(async move {
            if let Err(e) = run_turn(&cfg, &db_path, &session_id, &prompt, &tx, &cancel).await {
                let _ = tx.send(ChatEvent::Error(e.to_string()));
            }
        });
    })
}

async fn run_turn(
    cfg: &AiConfig,
    db_path: &Path,
    session_id: &str,
    prompt: &str,
    tx: &mpsc::Sender<ChatEvent>,
    cancel: &Notify,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let agent = build_agent(cfg, vec![crate::ai::tools::weather::build_weather_tool()])?;
    // FIXME: adk-session 1.0.0 SQLite backend has a duplicate-event-id bug
    // during tool-call loops (UNIQUE constraint failed on events.id). Using
    // in-memory sessions until the bug is fixed. Conversation history will
    // not persist across popup opens.
    let sessions = crate::ai::session::build_inmemory_session_service();

    // Ensure the session exists (idempotent — ignore "already exists" errors).
    let _ = sessions
        .create(CreateRequest {
            app_name: APP.into(),
            user_id: USER.into(),
            session_id: Some(session_id.into()),
            state: HashMap::new(),
        })
        .await;

    let runner = Runner::builder()
        .app_name(APP)
        .agent(agent)
        .session_service(sessions.clone())
        .build()?;

    let mut stream = runner
        .run(
            UserId::new_unchecked(USER),
            SessionId::new_unchecked(session_id),
            Content::new("user").with_text(prompt),
        )
        .await?;

    let mut full_response = String::new();
    let mut normal_completion = false;

    loop {
        tokio::select! {
            _ = cancel.notified() => break,
            maybe_ev = stream.next() => {
                let Some(ev) = maybe_ev else { break };
                match ev {
                    Ok(event) => {
                        eprintln!("[worker-debug] event: id={}, author={}, is_final={}, content_parts={}",
                            event.id,
                            event.author,
                            event.is_final_response(),
                            event.content().map(|c| c.parts.len()).unwrap_or(0),
                        );
                        if let Some(content) = event.content() {
                            for (i, part) in content.parts.iter().enumerate() {
                                eprintln!("[worker-debug]   part[{}]: {:?}", i, part);
                                if let Some(text) = part.text() {
                                    if !text.is_empty() {
                                        full_response.push_str(text);
                                        let _ = tx.send(ChatEvent::Delta(text.to_string()));
                                    }
                                }
                            }
                        }
                        if event.is_final_response() {
                            normal_completion = true;
                            let _ = tx.send(ChatEvent::Done);
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(ChatEvent::Error(e.to_string()));
                        break;
                    }
                }
            }
        }
    }

    if normal_completion && !full_response.is_empty() {
        let mut ev_assistant = Event::new(crate::ai::session::new_session_id());
        ev_assistant.author = "assistant".to_string();
        ev_assistant.llm_response.content = Some(Content::new("model").with_text(&full_response));
        let _ = sessions.append_event(session_id, ev_assistant).await;
    }

    Ok(())
}
