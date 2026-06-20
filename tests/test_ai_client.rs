use easycopy::ai::client::build_agent;
use easycopy::config::config::{AiConfig, AiProvider};

#[test]
fn build_agent_missing_cloud_key_is_err() {
    let mut cfg = AiConfig::default();
    cfg.provider = AiProvider::OpenAI;
    cfg.model = "gpt-4o-mini".into();
    // Ensure no key in env for this test.
    std::env::remove_var("OPENAI_API_KEY");
    let res = build_agent(&cfg, vec![]);
    assert!(res.is_err(), "expected Err when OPENAI_API_KEY is unset");
}

#[test]
fn build_agent_ollama_needs_no_key() {
    let mut cfg = AiConfig::default();
    cfg.provider = AiProvider::Ollama;
    cfg.model = "llama3.2:3b".into();
    cfg.ollama_url = "http://localhost:11434".into();
    let res = build_agent(&cfg, vec![]);
    assert!(res.is_ok(), "Ollama needs no API key: {:?}", res.err());
}
