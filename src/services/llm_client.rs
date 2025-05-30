use crate::app::config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
}

pub struct LLMClient {
    client: Client,
    api_key: String,
}

impl LLMClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| "OPENROUTER_API_KEY environment variable not set")?;
        
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub async fn list_models(&self) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
        let url = "https://openrouter.ai/api/v1/models";
        
        let resp = self.client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let models: Vec<Model> = serde_json::from_value(json["data"].clone())?;

        Ok(models)
    }

    pub async fn chat_completion(
        &self, 
        prompt: &str, 
        config: &Config
    ) -> Result<String, Box<dyn std::error::Error>> {
        let endpoint = "https://openrouter.ai/api/v1/chat/completions";

        let body = json!({
            "model": &config.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let resp = self.client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
}