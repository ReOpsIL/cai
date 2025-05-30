use crate::core::memory::{MemoryService, Prompt, PromptType};
use crate::core::command_processor::CommandProcessor;
use crate::services::llm_client::LLMClient;
use crate::app::config::Config;
use std::sync::Arc;

#[derive(Clone)]
pub struct ChatService {
    memory: MemoryService,
    command_processor: CommandProcessor,
    llm_client: Arc<LLMClient>,
}

impl ChatService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            memory: MemoryService::new(),
            command_processor: CommandProcessor::new(),
            llm_client: Arc::new(LLMClient::new()?),
        })
    }

    pub fn process_input(&self, input: &str) -> (String, bool) {
        self.command_processor.check_embedded_commands(input)
    }

    pub fn add_question(&self, content: String) -> Prompt {
        self.memory.add_prompt(content, PromptType::QUESTION)
    }

    pub fn add_answer(&self, content: String) -> Prompt {
        self.memory.add_prompt(content, PromptType::ANSWER)
    }

    pub async fn get_llm_response(&self, prompt: &str, config: &Config) -> Result<String, Box<dyn std::error::Error>> {
        self.llm_client.chat_completion(prompt, config).await
    }

    pub fn get_memory_service(&self) -> &MemoryService {
        &self.memory
    }
}