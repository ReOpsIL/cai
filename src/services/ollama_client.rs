use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::options::GenerationOptions;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    static ref OLLAMA_CLIENT: Mutex<Option<OllamaClient>> = Mutex::new(None);
}

pub struct OllamaClient {
    client: Ollama,
    model: String,
}

impl OllamaClient {
    pub async fn new(host: Option<String>, model: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let host = host.unwrap_or_else(|| "http://localhost".to_string());
        let port = 11434; // Default Ollama port
        let model = model.unwrap_or_else(|| "codellama:7b-instruct".to_string());

        let client = Ollama::new(host, port);

        Ok(Self {
            client,
            model,
        })
    }

    pub async fn get_completion(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let request = GenerationRequest::new(self.model.clone(), prompt.to_string())
            .options(GenerationOptions::default());

        let response = self.client.generate(request).await?;

        Ok(response.response)
    }

    pub async fn get_global_client() -> Result<&'static Mutex<Option<OllamaClient>>, Box<dyn std::error::Error>> {
        let client_guard = OLLAMA_CLIENT.lock().await;

        if client_guard.is_none() {
            drop(client_guard);

            let client = OllamaClient::new(None, None).await?;
            let mut client_guard = OLLAMA_CLIENT.lock().await;
            *client_guard = Some(client);
        }

        Ok(&OLLAMA_CLIENT)
    }
}
