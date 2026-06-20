# AI Chat Feature — Design

**Date:** 2026-06-20
**Status:** Accepted
**SDK:** [adk-rust](https://github.com/zavora-ai/adk-rust) v1.1.0 (Apache-2.0)

## Goal

Add an AI chat mode to the easycopy popup. When a `/` app-launcher search
returns no results, the popup switches into a chat panel where the user
converses with an AI. Conversation history persists across popup opens.

## Locked decisions

| Decision | Choice |
|---|---|
| SDK | adk-rust v1.1.0 |
| MSRV | Bump to rustc 1.94; add `rust-version = "1.94"` to `Cargo.toml` |
| Providers | Gemini + OpenAI + Anthropic + Ollama (4) |
| Cargo features | Keep defaults (minimal = Gemini + agent runtime + sessions) + `["openai","anthropic","ollama"]` |
| Persistence | adk-session SQLite |
| Session UX | Explicit "New chat" / "Continue" controls in the popup |
| Trigger | Auto-switch to chat when `/` search has no results |
| Streaming | Live token render (SSE default), Esc cancels in-flight stream |
| API keys | Env vars only, never in `config.toml` |

## Verified adk-rust APIs (v1.1.0)

Confirmed against docs.rs + repo source/examples:

- **Streaming entry point:** `Runner::run(user_id, session_id, content) -> Pin<Box<dyn Stream<Item = Result<Event, AdkError>> + Send>>` (`adk_runner/runner.rs:227`). Event type is `Event`. Iterate with `futures::StreamExt::next` (re-exported by adk-rust).
- **No `Launcher` needed:** `Runner::builder().app_name(..).agent(Arc<dyn Agent>).session_service(Arc<dyn SessionService>).build()` → `runner.run(..)`. Verified in `examples/ollama_qwen`.
- **Send/Sync:** `Agent: Send + Sync`, `Event`/`Part`/`String` all `Send`. Building the agent inside a `std::thread::spawn` worker and sending `String` deltas over `std::sync::mpsc` is sound.
- **Tokio runtime:** Use `Runtime::new()` (multi_thread). `current_thread` is unverified.
- **Ollama endpoint:** `OllamaConfig::with_host("http://host:11434", "llama3.2")` (pub `host` field).
- **Generation config:** `LlmAgentBuilder::max_output_tokens(i32)`, `.temperature(f32)`, `.top_p()`, `.top_k()`, or `.generate_content_config(GenerateContentConfig { .. })`.
- **Stream granularity:** Default `StreamingMode::SSE` yields token-level deltas; `LlmResponse.partial: bool` marks chunks. Ollama confirmed per-chunk text deltas in source; Gemini strongly implied (default provider, uniform path).

## Cargo.toml changes

```toml
[package]
# ...
rust-version = "1.94"          # NEW — adk-rust requires 1.94+ (edition 2024, per-crate)

[dependencies]
# ... existing deps unchanged ...
adk-rust = { version = "1.1.0", features = ["openai", "anthropic", "ollama"] }  # defaults keep minimal = Gemini + core + sessions
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
# futures::StreamExt is re-exported by adk-rust — no extra dep
```

Release profile stays LTO + strip + opt-level 2.

## AiConfig

Extend `Config` (`src/config/config.rs:174`, already `#[serde(default)]`) with a new `ai: AiConfig` field. `#[serde(default)]` keeps existing configs valid.

```rust
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
    pub ollama_url: String,            // default "http://localhost:11434"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    Gemini,
    OpenAI,
    Anthropic,
    Ollama,
}
```

### Provider → adk-rust client mapping

| `AiProvider` | adk-rust constructor | Env var |
|---|---|---|
| Gemini | `GeminiModel::new(&k, &model)` | `GOOGLE_API_KEY` |
| OpenAI | `OpenAIClient::new(OpenAIConfig::new(k, &model))` | `OPENAI_API_KEY` |
| Anthropic | `AnthropicClient::new(AnthropicConfig::new(k, &model))` | `ANTHROPIC_API_KEY` |
| Ollama | `OllamaModel::new(OllamaConfig::with_host(&cfg.ollama_url, &model))` | none |

### Example config.toml

```toml
[ai]
enable = true
provider = "ollama"
model = "llama3.2"
system_prompt = "You are a concise assistant inside a clipboard manager."
stream = true
max_tokens = 512
temperature = 0.3
ollama_url = "http://localhost:11434"
```

## Architecture — popup-local worker + adk Runner stream

The egui loop is synchronous, but the popup already does async I/O via the
spawn + `std::sync::mpsc` + drain-in-`update` pattern (`src/ui/popup.rs:247-347`
spawn, `:1993-2058` drain, `ctx.request_repaint()` at `:2031/:2045/:2056`). The
AI call reuses this exact pattern — adk-rust lives entirely inside a worker
thread; only `String` deltas cross to the main thread.

- On entering chat mode: spawn one worker thread holding a long-lived
  `tokio::runtime::Runtime::new()` (multi_thread) for the popup's lifetime.
- Build once and cache on `PopupApp`: `LlmAgentBuilder::new("easycopy-chat")`
  `.instruction(&cfg.system_prompt)` `.model(Arc::new(client))`
  `.max_output_tokens(cfg.max_tokens)` `.temperature(cfg.temperature)` `.build()`
  → `Arc<dyn Agent>`; then
  `Runner::builder().app_name("easycopy").agent(agent).session_service(Arc::new(sqlite_svc)).build()`.
- Per user send:
  `let stream = runner.run(user_id, session_id, Content::new("user").with_text(prompt)).await?;`
  then `while let Some(ev) = stream.next().await { for part in ev.content().parts { if let Part::Text { .. } = part { tx.send(text.clone()) } } }`;
  emit a Done sentinel at `turn_complete`.
- Main thread `update`: drain `rx` into `self.ai_buffer`, call `ctx.request_repaint()` (as at `popup.rs:2030-2031`).
- Esc cancels: a `tokio::sync::CancellationToken` drops the stream.

No changes to the daemon or the existing Unix-socket IPC.

## Trigger flow

In `draw_body` (`src/ui/popup.rs:873`), the empty-state branch is at
`:895-908` — when `self.filtered.is_empty()`, it currently shows a browser
preview or `draw_empty_state("No matches", ..)`. Insert the chat trigger
**before** the `"No matches"` fallback:

> When `mode == AppsOnly && self.filtered.is_empty() && query.len() > 1`
> (i.e. `/foo` with no app matches) → render the chat panel instead of
> `draw_empty_state`.

The chat panel reuses the search `TextEdit` (`draw_search`, `popup.rs:723`) as
the prompt input; Enter sends; the live answer renders below. Esc or
backspace-to-`/` returns to app-search mode. If `ai.enable == false`, keep the
current empty state (no chat).

## Session management (SQLite, explicit controls)

- `SqliteSessionService` backed by `~/.local/share/easycopy/chat.db` (reuse
  `store/` paths).
- Stable `user_id = "easycopy-user"`. Each conversation = one `session_id` (UUID).
- Popup UI: **New chat** button (generates a fresh `session_id`) and **Continue**
  (resumes the last). The current `session_id` is persisted to
  `chat_state.json` so "Continue" works across popup closes. adk-session stores
  turn history, so resuming a `session_id` feeds prior turns to the model
  automatically.
- A "Copy last answer" action copies the latest assistant message to the
  clipboard (via `arboard`, already a dep). AI messages do **not** enter
  clipboard history automatically (would pollute it).

## Out of scope (v2)

- adk-rust `#[tool]` / MCP function-calling (e.g. a "copy answer to clipboard"
  tool, "open URL" tool).
- adk-memory long-term memory / RAG.
- adk-realtime voice.

## Deferred to implementation

- **Binary size:** measure `cargo build --release` after integration — 4
  providers + tokio + reqwest/rustls is non-trivial; LTO+strip helps but won't
  eliminate it.
- **Runtime lifecycle:** long-lived-per-popup-open (recommended) vs per-query.
- **Cancel/timeout:** Esc cancels the in-flight stream; add a timeout config.
