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
    let mut c = AiConfig::default();
    c.provider = AiProvider::OpenAI;
    let s = toml::to_string_pretty(&c).unwrap();
    assert!(
        s.contains("provider = \"openai\""),
        "expected lowercase openai, got:\n{s}"
    );
}
