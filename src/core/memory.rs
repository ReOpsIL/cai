use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref MEMORY: Mutex<HashMap<String, Prompt>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum PromptType {
    #[default]
    QUESTION,
    ANSWER,
    ALIAS,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Prompt {
    pub id: String,
    pub date: DateTime<Utc>,
    pub value: String,
    pub ptype: PromptType,
}

impl Prompt {
    pub fn new(value: String, ptype: PromptType) -> Self {
        let prompt = Prompt {
            id: uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("")
                .to_string(),
            date: Utc::now(),
            value,
            ptype,
        };
        let mut memory = get_memory().lock().unwrap();
        memory.insert(prompt.id.clone(), prompt.clone());
        prompt
    }
}

pub fn get_memory() -> &'static Mutex<HashMap<String, Prompt>> {
    &MEMORY
}

#[derive(Clone)]
pub struct MemoryService;

impl MemoryService {
    pub fn new() -> Self {
        Self
    }

    pub fn add_prompt(&self, value: String, ptype: PromptType) -> Prompt {
        Prompt::new(value, ptype)
    }

    pub fn get_prompt(&self, id: &str) -> Option<Prompt> {
        let memory = get_memory().lock().unwrap();
        memory.get(id).cloned()
    }

    pub fn remove_prompt(&self, id: &str) -> bool {
        let mut memory = get_memory().lock().unwrap();
        memory.remove(id).is_some()
    }

    pub fn clear_memory(&self) {
        let mut memory = get_memory().lock().unwrap();
        memory.clear();
    }

    pub fn get_all_prompts(&self) -> Vec<Prompt> {
        let memory = get_memory().lock().unwrap();
        memory.values().cloned().collect()
    }

    pub fn get_prompts_by_type(&self, ptype: PromptType) -> Vec<Prompt> {
        let memory = get_memory().lock().unwrap();
        memory.values().filter(|p| p.ptype == ptype).cloned().collect()
    }
}