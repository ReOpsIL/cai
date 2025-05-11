use crate::configuration;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
}

pub async fn list_openrouter_models(
    api_key: &str,
) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let url = "https://openrouter.ai/api/v1/models";

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
pub async fn call_openrouter_api(
    api_key: &str,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = configuration::load_configuration()?;

    let client = Client::new();
    let endpoint = "https://openrouter.ai/api/v1/chat/completions";

    let body = json!({
        "model": &config.model,
        "messages": [{"role": "user", "content": prompt}]
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
