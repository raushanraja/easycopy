// Smoke test: ask the model about weather, verify it calls get_weather.
//   cargo run --example weather_smoke
// Requires: ollama serve + ollama pull llama3.2:3b
use easycopy::ai::{session, worker};
use easycopy::config::config::{AiConfig, AiProvider};
use std::sync::mpsc;
use std::sync::Arc;

fn main() {
    let cfg = AiConfig {
        enable: true,
        provider: AiProvider::Ollama,
        model: "llama3.2:3b".into(),
        system_prompt: "You are a helpful assistant. Use the get_weather tool when asked about weather. Reply concisely.".into(),
        stream: true,
        max_tokens: Some(256),
        temperature: Some(0.1),
        ..Default::default()
    };

    let dirs = easycopy::config::dirs::Directories::discover();
    // Use a fresh temp DB to avoid stale-session collisions.
    let db_path = std::env::temp_dir().join("easycopy_weather_smoke.db");
    let _ = std::fs::remove_file(&db_path);
    let session_id = session::new_session_id();

    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(tokio::sync::Notify::new());
    let prompt = "What is the current weather in London?";

    println!("[smoke] prompt: {prompt:?}");
    let handle = worker::spawn_turn(cfg, db_path, session_id, prompt.into(), tx, cancel);

    while let Ok(ev) = rx.recv() {
        match ev {
            worker::ChatEvent::Delta(s) => print!("{s}"),
            worker::ChatEvent::Done => {
                println!("\n[smoke] DONE");
                break;
            }
            worker::ChatEvent::Error(e) => {
                println!("\n[smoke] ERROR: {e}");
                break;
            }
        }
    }
    let _ = handle.join();
}
