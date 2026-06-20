use crate::config::config::{AiConfig, AiProvider};
use adk_rust::prelude::*;
use std::sync::Arc;

/// Build a ready-to-run adk-rust `LlmAgent` from config. Reads the provider's
/// API key from the environment. No network I/O at construction time.
/// `tools` are attached to the agent so the model can call them mid-turn.
pub fn build_agent(
    cfg: &AiConfig,
    tools: Vec<Arc<dyn Tool>>,
) -> std::result::Result<Arc<dyn Agent>, Box<dyn std::error::Error>> {
    let mut builder = LlmAgentBuilder::new("easycopy-chat").instruction(&cfg.system_prompt);

    builder = match cfg.provider {
        AiProvider::Gemini => {
            let key = std::env::var("GOOGLE_API_KEY")?;
            builder.model(Arc::new(GeminiModel::new(&key, &cfg.model)?))
        }
        AiProvider::OpenAI => {
            let key = std::env::var("OPENAI_API_KEY")?;
            builder.model(Arc::new(OpenAIClient::new(OpenAIConfig::new(
                key,
                cfg.model.clone(),
            ))?))
        }
        AiProvider::Anthropic => {
            let key = std::env::var("ANTHROPIC_API_KEY")?;
            builder.model(Arc::new(AnthropicClient::new(AnthropicConfig::new(
                key,
                cfg.model.clone(),
            ))?))
        }
        AiProvider::Ollama => builder.model(Arc::new(OllamaModel::new(OllamaConfig::with_host(
            &cfg.ollama_url,
            &cfg.model,
        ))?)),
    };

    if let Some(mt) = cfg.max_tokens {
        builder = builder.max_output_tokens(mt as i32);
    }
    if let Some(t) = cfg.temperature {
        builder = builder.temperature(t);
    }

    for tool in tools {
        builder = builder.tool(tool);
    }

    let agent: Arc<dyn Agent> = Arc::new(builder.build()?);
    Ok(agent)
}
