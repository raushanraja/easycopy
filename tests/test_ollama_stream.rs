use adk_rust::prelude::*;
use adk_rust::futures::StreamExt;

#[tokio::test]
async fn test_ollama_raw_generate() {
    let model = OllamaModel::new(OllamaConfig::with_host("http://localhost:11434", "llama3.2:3b")).unwrap();
    
    // Simulate the second turn after a tool response
    let contents = vec![
        Content::new("user").with_text("what is the weather in London in a bullet list format?"),
        Content {
            role: "model".to_string(),
            parts: vec![
                Part::FunctionCall {
                    name: "get_weather".to_string(),
                    args: serde_json::json!({"city": "London"}),
                    id: None,
                    thought_signature: None,
                }
            ],
        },
        Content {
            role: "tool".to_string(),
            parts: vec![
                Part::FunctionResponse {
                    function_response: adk_rust::FunctionResponseData::new(
                        "get_weather",
                        serde_json::json!({
                            "city": "London",
                            "country": "United Kingdom",
                            "temperature_c": 25.2,
                            "humidity": 53,
                            "wind_kmh": 14.4,
                            "condition": "Partly cloudy"
                        })
                    ),
                    id: None,
                }
            ],
        },
    ];

    let request = adk_rust::LlmRequest {
        model: "llama3.2:3b".to_string(),
        contents,
        tools: std::collections::HashMap::new(),
        config: None,
        previous_response_id: None,
    };

    let mut stream = model.generate_content(request, true).await.unwrap();

    println!("--- RAW GENERATE START ---");
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(response) => {
                if let Some(content) = response.content {
                    for part in &content.parts {
                        println!("Part: {:?}", part);
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
    println!("--- RAW GENERATE END ---");
}
