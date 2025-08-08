use crate::configuration;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
}

pub async fn list_openrouter_models() -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let url = "https://openrouter.ai/api/v1/models";
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    let client = Client::new();
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    let models: Vec<Model> = serde_json::from_value(json["data"].clone())?;

    Ok(models)
}

#[allow(dead_code)]
pub async fn call_openrouter_api(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = configuration::get_effective_config()?;
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    let client = Client::new();
    let endpoint = "https://openrouter.ai/api/v1/chat/completions";

    let mut messages = vec![json!({
        "role": "user",
        "content": prompt
    })];

    // Add system prompt if configured
    if let Some(system_prompt) = &config.llm.system_prompt {
        messages.insert(0, json!({
            "role": "system",
            "content": system_prompt
        }));
    }

    let body = json!({
        "model": &config.llm.model,
        "messages": messages,
        "temperature": config.llm.temperature,
        "max_tokens": config.llm.max_tokens,
        "top_p": config.llm.top_p
    });

    let resp = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://github.com/smol-ai/OpenRouter")
        .header("X-Custom-Metadata", "Rust Chat App")
        .json(&body)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    // Extract the response message
    let response_content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No response");

    Ok(response_content.to_string())
}
