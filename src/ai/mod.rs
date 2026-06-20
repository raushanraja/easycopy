use adk_rust::prelude::*;
use std::sync::Arc;

/// Build an Ollama model client from config — no network I/O at construction.
/// Tracer: exists only to confirm adk-rust imports compile in this crate.
#[allow(dead_code)]
pub fn tracer_ollama_model(
    model: &str,
    host: &str,
) -> std::result::Result<Arc<OllamaModel>, Box<dyn std::error::Error>> {
    let cfg = OllamaConfig::with_host(host, model);
    Ok(Arc::new(OllamaModel::new(cfg)?))
}
