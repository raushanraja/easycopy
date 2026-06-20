# AI Chat Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an AI chat mode to the easycopy popup that activates when a `/` app-search returns no results, using adk-rust v1.1.0, with SQLite-persisted conversation history.

**Architecture:** The AI call runs in a spawned worker thread inside the popup process (reusing the existing `std::thread` + `std::sync::mpsc` + drain-in-`update` pattern at `src/ui/popup.rs:247-347`/`:1993-2058`). A long-lived `tokio::runtime::Runtime` lives in the worker; adk-rust's `Runner::run(..)` returns an `EventStream` of `Event`s that are drained into `String` token deltas and sent over mpsc to the egui main thread, which renders them live via `ctx.request_repaint()`. No daemon or IPC changes. Conversation state persists via adk-session's SQLite backend at `~/.local/share/easycopy/chat.db`; the active `session_id` persists in `chat_state.json`.

**Tech Stack:** Rust 1.94+ (edition 2021 binary, adk-rust is edition 2024 — per-crate, compatible), adk-rust v1.1.0 (features: default `minimal` + `openai` + `anthropic` + `ollama`), tokio 1, egui/eframe 0.28, serde/toml.

**Design doc:** `docs/plans/2026-06-20-ai-chat-design.md` (read it first).

---

## Import-path caveat (read before Task 1)

adk-rust re-exports model clients and builder via `adk_rust::prelude::*` (`GeminiModel`, `OpenAIClient`, `OpenAIConfig`, `AnthropicClient`, `AnthropicConfig`, `OllamaModel`, `OllamaConfig`, `LlmAgentBuilder`, `Agent`). Runner/session/futures are at `adk_rust::runner::Runner`, `adk_rust::session::{SessionService, InMemorySessionService, CreateRequest}`, `adk_rust::futures::StreamExt`.

**Unverified:** whether `adk_core::{Content, Part, UserId, SessionId}` and the SQLite session type are reachable from a single `adk-rust` dep, or require direct deps on `adk-core` / `adk-model` / `adk-session` (same 1.x version). **Task 1 resolves this** by compiling a tracer. If `use adk_core::..` fails, either add the sub-crates as direct deps or find the `adk_rust::` re-export. Do not proceed past Task 1 until the tracer compiles.

---

### Task 1: Bump MSRV + add deps + tracer compile

**Files:**
- Modify: `Cargo.toml`
- Create: `src/ai/mod.rs`
- Modify: `src/lib.rs` (add `pub mod ai;` under "Domain modules")

**Step 1: Edit `Cargo.toml`**

Add under `[package]` (after `readme = "README.md"`, line 7):
```toml
rust-version = "1.94"
```
Add under `[dependencies]` (after `libc = "0.2.186"`, line 22):
```toml
adk-rust = { version = "1.1.0", features = ["openai", "anthropic", "ollama"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
```

**Step 2: Add the module declaration in `src/lib.rs`**

Insert after `pub mod config;` (line 8):
```rust
pub mod ai;
```

**Step 3: Create `src/ai/mod.rs` with a tracer that compiles but does no network I/O**

```rust
use adk_rust::prelude::*;
use std::sync::Arc;

/// Build an Ollama model client from config — no network I/O at construction.
/// Tracer: exists only to confirm adk-rust imports compile in this crate.
pub fn tracer_ollama_model(model: &str, host: &str) -> anyhow::Result<Arc<OllamaModel>> {
    let cfg = OllamaConfig::with_host(host, model);
    Ok(Arc::new(OllamaModel::new(cfg)?))
}
```

If `anyhow` is not a dep, return `Result<Arc<OllamaModel>, Box<dyn std::error::Error>>` instead. If `OllamaConfig::with_host` / `OllamaModel` are not reachable via the prelude, try `use adk_model::ollama::{OllamaConfig, OllamaModel};` — and if that crate isn't reachable, add `adk-model = "1"` to `[dependencies]` and retry. Record whichever import path works for use in later tasks.

**Step 4: Verify it builds**

Run: `cargo build`
Expected: compiles with no errors. Warnings about an unused function are fine.

If it fails on `adk_rust::futures` or edition/MSRV, fix the toolchain (`rustup update stable` or install 1.94+) and re-run. Do not commit a broken build.

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/ai/mod.rs
git commit -m "build: add adk-rust + tokio deps, bump MSRV to 1.94, ai module tracer"
```

---

### Task 2: AiConfig + AiProvider structs

**Files:**
- Modify: `src/config/config.rs` (add structs before `Config` at line 172; add `ai` field to `Config` at line 174)
- Test: `tests/test_ai_config.rs` (create)

**Step 1: Write the failing test `tests/test_ai_config.rs`**

```rust
use easycopy::config::config::{AiConfig, AiProvider, Config};

#[test]
fn ai_config_default_is_disabled_gemini() {
    let c = AiConfig::default();
    assert!(!c.enable);
    assert_eq!(c.provider, AiProvider::Gemini);
    assert!(c.stream);
    assert_eq!(c.ollama_url, "http://localhost:11434");
}

#[test]
fn ai_config_toml_roundtrip() {
    let toml = r#"
[ai]
enable = true
provider = "ollama"
model = "llama3.2"
system_prompt = "be brief"
stream = true
max_tokens = 512
temperature = 0.3
ollama_url = "http://host:11434"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.ai.provider, AiProvider::Ollama);
    assert_eq!(cfg.ai.model, "llama3.2");
    assert_eq!(cfg.ai.max_tokens, Some(512));
    // round-trip back
    let s = toml::to_string_pretty(&cfg).unwrap();
    let cfg2: Config = toml::from_str(&s).unwrap();
    assert_eq!(cfg.ai, cfg2.ai);
}

#[test]
fn ai_provider_serializes_lowercase() {
    let p = toml::to_string(&AiProvider::OpenAI).unwrap();
    assert!(p.trim().contains("openai"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_ai_config`
Expected: FAIL — `AiConfig`/`AiProvider` not found, `ai` field missing on `Config`.

**Step 3: Implement in `src/config/config.rs`**

Insert before the `// ── Config ───` comment (line 170):
```rust
// ── AI ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    Gemini,
    OpenAI,
    Anthropic,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AiConfig {
    pub enable: bool,
    pub provider: AiProvider,
    pub model: String,
    pub system_prompt: String,
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub ollama_url: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enable: false,
            provider: AiProvider::Gemini,
            model: String::new(),
            system_prompt: String::from(
                "You are a concise assistant inside a clipboard manager.",
            ),
            stream: true,
            max_tokens: None,
            temperature: None,
            ollama_url: String::from("http://localhost:11434"),
        }
    }
}
```
Add the field to `Config` (line 174 block), after `pub footer: FooterConfig,`:
```rust
    pub ai: AiConfig,
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_ai_config`
Expected: PASS (3 tests).

**Step 5: Commit**

```bash
git add src/config/config.rs tests/test_ai_config.rs
git commit -m "feat: add AiConfig + AiProvider to Config with TOML round-trip"
```

---

### Task 3: AiConfig::sanitize (per-provider default model)

**Files:**
- Modify: `src/config/config.rs` (add `AiConfig::sanitize`; call it from `Config::sanitize`)
- Test: `tests/test_ai_config.rs` (append)

**Step 1: Append failing tests**

```rust
#[test]
fn sanitize_fills_default_model_per_provider() {
    let mut c = AiConfig::default();
    c.provider = AiProvider::Ollama;
    c.sanitize();
    assert_eq!(c.model, "llama3.2");

    let mut c = AiConfig::default();
    c.provider = AiProvider::Gemini;
    c.sanitize();
    assert_eq!(c.model, "gemini-2.5-flash");

    let mut c = AiConfig::default();
    c.provider = AiProvider::OpenAI;
    c.sanitize();
    assert_eq!(c.model, "gpt-4o-mini");

    let mut c = AiConfig::default();
    c.provider = AiProvider::Anthropic;
    c.sanitize();
    assert_eq!(c.model, "claude-sonnet-4-6");
}

#[test]
fn sanitize_keeps_user_model() {
    let mut c = AiConfig::default();
    c.provider = AiProvider::Ollama;
    c.model = "my-custom-model".into();
    c.sanitize();
    assert_eq!(c.model, "my-custom-model");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_ai_config`
Expected: FAIL — no method `sanitize` on `AiConfig`.

**Step 3: Implement**

Add to `AiConfig` impl (create `impl AiConfig { .. }` if none):
```rust
impl AiConfig {
    pub fn sanitize(&mut self) {
        if self.model.is_empty() {
            self.model = match self.provider {
                AiProvider::Gemini => "gemini-2.5-flash".into(),
                AiProvider::OpenAI => "gpt-4o-mini".into(),
                AiProvider::Anthropic => "claude-sonnet-4-6".into(),
                AiProvider::Ollama => "llama3.2".into(),
            };
        }
        if self.ollama_url.is_empty() {
            self.ollama_url = "http://localhost:11434".into();
        }
    }
}
```
Find `Config::sanitize` (grep `fn sanitize` in `src/config/config.rs`) and add `self.ai.sanitize();` inside it. If `Config::sanitize` does not exist yet, add:
```rust
impl Config {
    pub fn sanitize(&mut self) {
        self.general.sanitize();
        self.footer.sanitize();
        self.ai.sanitize();
    }
}
```
(match the existing pattern for `general`/`footer`; reuse their `sanitize` calls if already present).

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_ai_config`
Expected: PASS (all).

**Step 5: Commit**

```bash
git add src/config/config.rs tests/test_ai_config.rs
git commit -m "feat: AiConfig::sanitize fills per-provider default model"
```

---

### Task 4: AI agent builder (provider → adk-rust client)

**Files:**
- Create: `src/ai/client.rs`
- Modify: `src/ai/mod.rs` (add `pub mod client;`, remove tracer from Task 1 or leave it)
- Test: `tests/test_ai_client.rs` (create)

**Step 1: Write the failing test `tests/test_ai_client.rs`**

```rust
use easycopy::ai::client::build_agent;
use easycopy::config::config::{AiConfig, AiProvider};

#[test]
fn build_agent_missing_cloud_key_is_err() {
    let mut cfg = AiConfig::default();
    cfg.provider = AiProvider::OpenAI;
    cfg.model = "gpt-4o-mini".into();
    // Ensure no key in env for this test.
    std::env::remove_var("OPENAI_API_KEY");
    let res = build_agent(&cfg);
    assert!(res.is_err(), "expected Err when OPENAI_API_KEY is unset");
}

#[test]
fn build_agent_ollama_needs_no_key() {
    let mut cfg = AiConfig::default();
    cfg.provider = AiProvider::Ollama;
    cfg.model = "llama3.2".into();
    cfg.ollama_url = "http://localhost:11434".into();
    let res = build_agent(&cfg);
    assert!(res.is_ok(), "Ollama needs no API key: {:?}", res.err());
}
```
Note: the Ollama test asserts construction only (no network). If `build_agent` for Ollama still fails locally because no ollama is running, that's a construction-vs-connection issue — confirm `OllamaModel::new` does not connect; if it does, mark this test `#[ignore]` and note it.

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_ai_client`
Expected: FAIL — `build_agent` not found.

**Step 3: Implement `src/ai/client.rs`**

```rust
use crate::config::config::{AiConfig, AiProvider};
use adk_rust::prelude::*;
use std::sync::Arc;

/// Build a ready-to-run adk-rust `LlmAgent` from config. Reads the provider's
/// API key from the environment. No network I/O at construction time.
pub fn build_agent(cfg: &AiConfig) -> Result<Arc<dyn Agent>, Box<dyn std::error::Error>> {
    let model: Arc<dyn Llm> = match cfg.provider {
        AiProvider::Gemini => {
            let key = std::env::var("GOOGLE_API_KEY")?;
            Arc::new(GeminiModel::new(&key, &cfg.model)?)
        }
        AiProvider::OpenAI => {
            let key = std::env::var("OPENAI_API_KEY")?;
            Arc::new(OpenAIClient::new(OpenAIConfig::new(key, &cfg.model))?)
        }
        AiProvider::Anthropic => {
            let key = std::env::var("ANTHROPIC_API_KEY")?;
            Arc::new(AnthropicClient::new(AnthropicConfig::new(key, &cfg.model))?)
        }
        AiProvider::Ollama => {
            Arc::new(OllamaModel::new(OllamaConfig::with_host(&cfg.ollama_url, &cfg.model))?)
        }
    };

    let mut builder = LlmAgentBuilder::new("easycopy-chat")
        .instruction(&cfg.system_prompt)
        .model(model);
    if let Some(mt) = cfg.max_tokens {
        builder = builder.max_output_tokens(mt as i32);
    }
    if let Some(t) = cfg.temperature {
        builder = builder.temperature(t);
    }
    let agent: Arc<dyn Agent> = Arc::new(builder.build()?);
    Ok(agent)
}
```
**Import caveat:** `Arc<dyn Llm>` — confirm the model trait name. The verified skeleton passed `Arc::new(OllamaModel::new(..)?)` straight to `.model()`; if `.model()` accepts a concrete `Arc<OllamaModel>` only (not a unified trait), you'll need to build the agent inside each match arm instead of returning a `model` first. Prefer that fallback: move `LlmAgentBuilder::new(..).instruction(..).model(Arc::new(<client>))` into each arm and return `Arc<dyn Agent>` at the end of each. Adjust until `cargo build` passes; record the working shape.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_ai_client`
Expected: PASS (2 tests; Ollama one passes without a running server since `OllamaModel::new` only stores config).

**Step 5: Commit**

```bash
git add src/ai/client.rs src/ai/mod.rs tests/test_ai_client.rs
git commit -m "feat: ai::client::build_agent maps provider to adk-rust LlmAgent"
```

---

### Task 5: Paths for chat.db + chat_state.json

**Files:**
- Modify: `src/store/paths.rs` (append two fns)
- Test: `tests/test_storage.rs` (append) or a new `tests/test_ai_paths.rs`

**Step 1: Write the failing test `tests/test_ai_paths.rs`**

```rust
use easycopy::config::dirs::Directories;
use easycopy::store::paths;

#[test]
fn chat_paths_under_data_dir() {
    let dirs = Directories::discover();
    assert_eq!(paths::chat_db(&dirs), dirs.data_dir.join("chat.db"));
    assert_eq!(paths::chat_state(&dirs), dirs.data_dir.join("chat_state.json"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_ai_paths`
Expected: FAIL — `chat_db`/`chat_state` not found.

**Step 3: Implement in `src/store/paths.rs`** (append after `daemon_pid`):

```rust
pub fn chat_db(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("chat.db")
}

pub fn chat_state(dirs: &Directories) -> std::path::PathBuf {
    dirs.data_dir.join("chat_state.json")
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_ai_paths`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/store/paths.rs tests/test_ai_paths.rs
git commit -m "feat: add chat_db + chat_state data paths"
```

---

### Task 6: Chat session state persistence (chat_state.json)

**Files:**
- Create: `src/ai/session.rs`
- Modify: `src/ai/mod.rs` (add `pub mod session;`)
- Test: `tests/test_ai_session.rs` (create)

**Step 1: Write the failing test `tests/test_ai_session.rs`**

```rust
use easycopy::ai::session::ChatState;
use std::fs;

#[test]
fn chat_state_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chat_state.json");

    let st = ChatState { current_session_id: Some("abc-123".into()) };
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_ai_session`
Expected: FAIL — `ChatState` not found.

**Step 3: Implement `src/ai/session.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChatState {
    pub current_session_id: Option<String>,
}

impl ChatState {
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let st: Self = serde_json::from_str(&text).unwrap_or_default();
        Ok(st)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, text)
    }
}
```
(Use `crate::store::atomic::AtomicWriter::write` instead of `std::fs::write` to match the repo's atomic-write convention — check `src/store/atomic.rs` for the API; it takes `(path, &[u8])`.)

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_ai_session`
Expected: PASS (2 tests).

**Step 5: Commit**

```bash
git add src/ai/session.rs src/ai/mod.rs tests/test_ai_session.rs
git commit -m "feat: ai::session::ChatState persists current session_id"
```

---

### Task 7: SQLite SessionService + Runner setup

**Files:**
- Modify: `src/ai/session.rs` (add `build_session_service` + `new_session_id`)
- Test: `tests/test_ai_session.rs` (append)

**Step 1: Confirm the SQLite session type from adk-rust**

Run: `cargo doc --package adk-rust --open` (or grep `adk-rust` in `~/.cargo/registry/src` for `SessionService` impls / `Sqlite`). Look in the `adk_session` re-export for a SQLite backend type name (likely `SqliteSessionService` or similar). Record the exact constructor signature (does it take a path? a connection string?).

If no SQLite backend is reachable from the `adk-rust` dep alone, add `adk-session = "1"` to `[dependencies]` and re-confirm. If SQLite genuinely isn't available, fall back to `InMemorySessionService` for v1 and open an issue to restore the SQLite decision from the design — but first exhaust `adk_session::` and `adk_rust::session::` namespaces.

**Step 2: Append a failing test**

```rust
use easycopy::ai::session::new_session_id;

#[test]
fn new_session_id_is_nonempty_unique() {
    let a = new_session_id();
    let b = new_session_id();
    assert!(!a.is_empty());
    assert_ne!(a, b);
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test --test test_ai_session`
Expected: FAIL — `new_session_id` not found.

**Step 4: Implement in `src/ai/session.rs`**

```rust
use adk_rust::session::SessionService; // adjust if re-export path differs
use std::sync::Arc;

/// UUID-ish session id. Keep simple — use timestamp + counter or `uuid` crate if already a dep.
pub fn new_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let c = CTR.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", nanos, c)
}

/// Build the SQLite-backed session service at `chat.db`.
/// Adjust the constructor to the confirmed API from Step 1.
pub fn build_session_service(
    db_path: &std::path::Path,
) -> Result<Arc<dyn SessionService>, Box<dyn std::error::Error>> {
    // EXAMPLE — replace with the confirmed SQLite type:
    // let svc = SqliteSessionService::open(db_path)?;
    // Ok(Arc::new(svc))
    todo!("replace with confirmed SQLite SessionService constructor from Step 1")
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test --test test_ai_session`
Expected: `new_session_id_is_nonempty_unique` PASS. (`build_session_service` is exercised manually in Task 8; leave the `todo!` only if you defer SQLite to Task 8, else implement it here.)

**Step 6: Commit**

```bash
git add src/ai/session.rs tests/test_ai_session.rs Cargo.toml Cargo.lock
git commit -m "feat: ai::session SQLite SessionService + session id generator"
```

---

### Task 8: AI worker thread + streaming drain

**Files:**
- Create: `src/ai/worker.rs`
- Modify: `src/ai/mod.rs` (add `pub mod worker;`)

This task has no automated test (adk-rust has no mock model; real calls need a key + network). Verify by build + a manual one-shot bin or log.

**Step 1: Implement `src/ai/worker.rs`**

```rust
use crate::ai::client::build_agent;
use crate::ai::session::build_session_service;
use crate::config::config::AiConfig;
use adk_rust::futures::StreamExt;
use adk_rust::prelude::*; // Agent, Content, Part, UserId, SessionId (confirm prelude has these)
use adk_rust::runner::Runner;
use adk_rust::session::{CreateRequest, SessionService};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use tokio::sync::CancellationToken;

const APP: &str = "easycopy";
const USER: &str = "easycopy-user";

/// Messages the worker sends back to the egui main thread.
pub enum ChatEvent {
    Delta(String),
    Done,
    Error(String),
}

/// Spawn a worker thread that runs one turn and streams deltas back.
/// `cancel` drops the stream mid-flight.
pub fn spawn_turn(
    cfg: AiConfig,
    db_path: std::path::PathBuf,
    session_id: String,
    prompt: String,
    tx: mpsc::Sender<ChatEvent>,
    cancel: CancellationToken,
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
    cancel: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error>> {
    let agent = build_agent(cfg)?;
    let sessions = build_session_service(db_path)?;
    // Ensure the session exists (create is idempotent if it already exists).
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
        .session_service(sessions)
        .build()?;

    let mut stream = runner
        .run(
            UserId::new(USER)?,
            SessionId::new(session_id)?,
            Content::new("user").with_text(prompt),
        )
        .await?;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            maybe_ev = stream.next() => {
                let Some(ev) = maybe_ev else { break };
                match ev {
                    Ok(event) => {
                        if let Some(content) = event.content() {
                            for part in &content.parts {
                                if let Part::Text { text } = part {
                                    if !text.is_empty() {
                                        let _ = tx.send(ChatEvent::Delta(text.clone()));
                                    }
                                }
                            }
                        }
                        if event.is_final_response() {
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
    Ok(())
}
```
**Caveats to resolve by compiling:** `Content`, `Part`, `UserId`, `SessionId`, `event.content()`, `event.is_final_response()` import paths (see plan header). `CreateRequest` field names and `state` type. `Runner::builder()` required methods (typestate). Fix imports until `cargo build` passes.

**Step 2: Verify it builds**

Run: `cargo build`
Expected: compiles. (Unused-function warnings are fine.)

**Step 3: Manual smoke check (optional but recommended)**

Add a temporary `examples/chat_smoke.rs` that calls `spawn_turn` with an Ollama config and prints deltas, run with `cargo run --example chat_smoke` (requires `ollama serve` + `ollama pull llama3.2`). Delete the example after confirming. Skip if no local Ollama.

**Step 4: Commit**

```bash
git add src/ai/worker.rs src/ai/mod.rs
git commit -m "feat: ai::worker spawns tokio runtime + drains adk EventStream to mpsc"
```
(if you added and removed an example, don't commit it)

---

### Task 9: PopupApp chat state + channels

**Files:**
- Modify: `src/ui/popup.rs` (extend `PopupApp` fields near line 145-183; extend the constructor in `new`/creation near line 247-347)

**Step 1: Add chat state fields to `PopupApp`** (near the other `rx`/`tx` fields around line 169-182):

```rust
// ── AI chat ──
chat_messages: Vec<ChatMessage>,           // rendered history (user + assistant turns)
ai_buffer: String,                         // accumulating assistant reply
chat_rx: std::sync::mpsc::Receiver<ChatEvent>,
chat_tx: std::sync::mpsc::ChatTx,          // see note below — use the Sender type
chat_cancel: Option<tokio::sync::CancellationToken>,
chat_session_id: Option<String>,
chat_active: bool,                         // are we in chat mode right now?
```
Define a small `ChatMessage` enum in `popup.rs` (or `src/ai/mod.rs`):
```rust
pub enum ChatMessage {
    User(String),
    Assistant(String),
}
```
**Note on mpsc direction:** the worker owns the `Sender<ChatEvent>` and the popup owns the `Receiver<ChatEvent>`. The popup does NOT send prompts over mpsc — it spawns a new `spawn_turn` per user message (passing a fresh `Sender`). So `chat_tx` may be unnecessary; instead store the `chat_rx` + a `cfg.ai` clone + the `db_path`. Revise fields to what's actually needed: `chat_rx`, `chat_cancel`, `chat_session_id`, `chat_messages`, `ai_buffer`, `chat_active`, and a cached `Arc<dyn Agent>` is NOT needed (worker builds its own per turn — simpler; optimize later).

So the minimal set:
```rust
chat_messages: Vec<ChatMessage>,
ai_buffer: String,
chat_rx: Option<std::sync::mpsc::Receiver<ChatEvent>>,
chat_cancel: Option<tokio::sync::CancellationToken>,
chat_session_id: Option<String>,
chat_active: bool,
```

**Step 2: Initialize in the `PopupApp` constructor** (where the other channels are created, around line 247-347):

```rust
chat_messages: Vec::new(),
ai_buffer: String::new(),
chat_rx: None,
chat_cancel: None,
chat_session_id: ChatState::load_from_path(&paths::chat_state(&dirs)).ok()
    .and_then(|s| s.current_session_id),
chat_active: false,
```
Import `crate::ai::session::ChatState` and `crate::store::paths` at the top of `popup.rs`. `dirs` is already available in the constructor (it's used for other paths).

**Step 3: Verify it builds**

Run: `cargo build`
Expected: compiles. (Field may be unused → warnings ok for now.)

**Step 4: Commit**

```bash
git add src/ui/popup.rs
git commit -m "feat: add AI chat state fields to PopupApp, load chat_state on startup"
```

---

### Task 10: Drain chat events in `update`

**Files:**
- Modify: `src/ui/popup.rs` (`update` method, after the existing drain blocks at line 1993-2058)

**Step 1: Add a drain block in `update`** (after the app-loading block ending ~line 2058, before the `close_on_focus_out` block at line 2060):

```rust
// ── AI chat: drain streamed deltas ──
if let Some(rx) = self.chat_rx.as_ref() {
    let mut got = false;
    while let Ok(ev) = rx.try_recv() {
        got = true;
        match ev {
            ChatEvent::Delta(s) => self.ai_buffer.push_str(&s),
            ChatEvent::Done => {
                if !self.ai_buffer.is_empty() {
                    self.chat_messages
                        .push(ChatMessage::Assistant(std::mem::take(&mut self.ai_buffer)));
                }
                self.chat_cancel = None;
            }
            ChatEvent::Error(e) => {
                self.chat_messages
                    .push(ChatMessage::Assistant(format!("[error: {e}]")));
                self.ai_buffer.clear();
                self.chat_cancel = None;
            }
        }
    }
    if got {
        ctx.request_repaint();
    }
}
```
**Borrow check:** `self.chat_rx.as_ref()` borrows `self` immutably while the loop body borrows `self` mutably — this won't compile as written. Fix by taking the receiver out: `if let Some(rx) = self.chat_rx.as_ref()` → instead drain via a temporary take, e.g. `let mut got = false; if let Some(rx) = &self.chat_rx { while let Ok(ev) = rx.try_recv() { ... } }` and collect events into a local `Vec<ChatEvent>` first, then apply them to `&mut self` after the loop. Implement that two-phase drain to satisfy the borrow checker.

**Step 2: Verify it builds**

Run: `cargo build`
Expected: compiles.

**Step 3: Commit**

```bash
git add src/ui/popup.rs
git commit -m "feat: drain AI chat stream events in PopupApp::update"
```

---

### Task 11: Trigger flow in `draw_body` (chat panel on no `/` results)

**Files:**
- Modify: `src/ui/popup.rs` (`draw_body` empty-state branch at line 895-908; `draw_search` at line 723)

**Step 1: Insert the chat trigger** in `draw_body` (line 895 block). The current code:
```rust
if self.filtered.is_empty() {
    if let Some(preview) = self.browser_preview.as_ref() { /* ... */ return; }
    draw_empty_state(ui, "No matches", "Try a shorter search term.", weak_color);
    return;
}
```
Change to: when `ai.enable && mode == AppsOnly && self.filtered.is_empty() && query_after_slash.len() > 1`, call `self.draw_chat_panel(ui)` and `return;` before the `"No matches"` fallback. You need the current query and mode — check how `apply_filter`/`filter_query` expose them (grep `AppsOnly` and the query string in `popup.rs`). Likely `self.query` (the raw TextEdit string) and a mode helper.

```rust
if self.filtered.is_empty() {
    if let Some(preview) = self.browser_preview.as_ref() { /* keep existing */ return; }

    // AI chat fallback: "/foo" with no app matches → chat with "foo" as prompt.
    if self.config.ai.enable && self.is_apps_only() && self.query.trim_start_matches('/').len() > 1 {
        self.draw_chat_panel(ui);
        return;
    }

    draw_empty_state(ui, "No matches", "Try a shorter search term.", weak_color);
    return;
}
```
Add helper `fn is_apps_only(&self) -> bool` reusing the existing mode detection (don't duplicate the logic — call the same `filter_query`/mode function the rest of the file uses).

**Step 2: Implement `draw_chat_panel`** (new method on `PopupApp`):

```rust
fn draw_chat_panel(&mut self, ui: &mut egui::Ui) {
    // Mark chat active so Esc / backspace handling knows to exit chat on "/" revert.
    self.chat_active = true;

    egui::ScrollArea::vertical()
        .id_source("easycopy_chat")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;

            // New chat / Continue controls
            ui.horizontal(|ui| {
                if ui.button("New chat").clicked() {
                    let id = crate::ai::session::new_session_id();
                    self.chat_session_id = Some(id.clone());
                    self.chat_messages.clear();
                    self.ai_buffer.clear();
                    self.persist_chat_state();
                }
                if ui.button("Continue").clicked() {
                    // keep current chat_session_id; messages will load from session history
                    // (for v1, we rely on adk-session history on the model side; the local
                    //  Vec<ChatMessage> only holds this popup-open's visible turns)
                }
            });
            ui.add_space(6.0);

            // Render message history
            for m in &self.chat_messages {
                match m {
                    ChatMessage::User(t) => ui.label(format!("> {t}")),
                    ChatMessage::Assistant(t) => ui.label(t),
                };
            }
            // Live streaming buffer
            if !self.ai_buffer.is_empty() {
                ui.label(&self.ai_buffer);
            }

            // Copy last answer
            if let Some(ChatMessage::Assistant(t)) = self.chat_messages.last() {
                if ui.button("Copy last answer").clicked() {
                    let _ = crate::clipboard::set_text(t.clone()); // use the existing clipboard write helper
                }
            }
        });
}
```
**Verify the clipboard-write helper name** by grepping `src/clipboard/` for the function that writes text (don't invent). Use the real one.

**Step 3: Wire Enter in `draw_search`** (line 723 area). When `chat_active` and the user presses Enter, send the current query (minus the leading `/`) as a prompt:

```rust
// inside draw_search key handling, on Enter when chat_active:
if self.chat_active && enter_pressed {
    let prompt = self.query.trim_start_matches('/').trim().to_string();
    if !prompt.is_empty() {
        self.chat_messages.push(ChatMessage::User(prompt.clone()));
        self.ai_buffer.clear();
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = tokio::sync::CancellationToken::new();
        self.chat_rx = Some(rx);
        self.chat_cancel = Some(cancel.clone());
        crate::ai::worker::spawn_turn(
            self.config.ai.clone(),
            paths::chat_db(&self.dirs),         // confirm PopupApp holds `dirs` or a path
            self.chat_session_id.clone().unwrap_or_else(crate::ai::session::new_session_id),
            prompt,
            tx,
            cancel,
        );
        // remember session id
        if self.chat_session_id.is_none() {
            self.chat_session_id = Some(crate::ai::session::new_session_id());
        }
        self.persist_chat_state();
    }
}
```
Add `fn persist_chat_state(&self)` that writes `ChatState { current_session_id: self.chat_session_id.clone() }` to `paths::chat_state(&self.dirs)`. If `PopupApp` doesn't already hold `dirs`/a data-dir path, store the `chat_db`/`chat_state` paths as fields set in the constructor (Task 9).

**Step 4: Esc / backspace exit.** In the existing Esc/key handling (grep for `Esc`/backspace in `popup.rs`), when `chat_active` and the query is back to just `/` (or empty), set `self.chat_active = false` so the next frame reverts to app-search empty state. Cancel any in-flight stream: `if let Some(c) = self.chat_cancel.take() { c.cancel(); }`.

**Step 5: Verify it builds**

Run: `cargo build`
Expected: compiles.

**Step 6: Manual verification**

Run the popup (the repo's run command — check `BUILD.md`/`README.md` for the daemon+popup launch). With `ai.enable=true` and an Ollama server running: type `/xyz` (no app match) → chat panel appears → type a message + Enter → tokens stream in → "Copy last answer" works → "New chat" clears → Esc exits. If no Ollama, set `provider = "openai"` + `OPENAI_API_KEY` and repeat with a network connection.

**Step 7: Commit**

```bash
git add src/ui/popup.rs
git commit -m "feat: AI chat panel triggers on no-results / search, streams replies"
```

---

### Task 12: Config default + docs + release build + size check

**Files:**
- Modify: `README.md` (add an "AI chat" section: config snippet + env vars + Ollama note)
- Modify: `BUILD.md` if it documents deps/MSRV (note rustc 1.94 requirement)

**Step 1: Confirm default config carries `[ai]`**

Run: `cargo test --test test_ai_config`
Then start the app once and inspect the generated `~/.config/easycopy/config.toml` — it should contain an `[ai]` table with `enable = false`. If not, ensure `Config::default()` + `toml::to_string_pretty` emits it (it will, since `ai` is a non-skipped field).

**Step 2: Document in `README.md`**

Add a section covering: the `[ai]` config table, the 4 providers + their env vars (`GOOGLE_API_KEY`/`OPENAI_API_KEY`/`ANTHROPIC_API_KEY`/none for Ollama), the `ollama_url`, the auto-switch-on-no-`/`-results behavior, New chat / Continue, Copy last answer, and the MSRV (rustc 1.94).

**Step 3: Release build + size measurement**

Run:
```bash
cargo build --release
ls -lh target/release/easycopy
```
Record the binary size before (git stash the Cargo change, build, note size, unstash) and after, and put the after-size + the delta in the README AI section or a `docs/notes/ai-binary-size.md` line. This satisfies the design's "deferred to implementation: binary-size measurement."

**Step 4: Run the full check suite**

Run: `cargo test` and `./run_checks.sh` (if present) and `cargo clippy -- -D warnings` (if the repo uses clippy — check `run_checks.sh`).

**Step 5: Commit**

```bash
git add README.md BUILD.md
git commit -m "docs: document AI chat feature, MSRV bump, and binary-size impact"
```

---

## Out of scope for this plan (v2)

- adk-rust `#[tool]` / MCP function-calling (copy-to-clipboard / open-URL tools)
- adk-memory / RAG
- adk-realtime voice
- Cargo feature-gating the AI deps (so users without AI get a smaller binary) — consider later if binary size is unacceptable
- Settings UI entry (v1 is config-file-only)
