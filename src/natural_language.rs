use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;

/// Natural language parser for converting user input to structured commands
#[derive(Debug)]
pub struct NaturalLanguageParser {
    intent_classifier: IntentClassifier,
    entity_extractor: EntityExtractor,
    command_mapper: CommandMapper,
    pattern_cache: HashMap<String, Regex>,
}

/// Classification of user intents
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserIntent {
    // Prompt management
    SearchPrompts { query: Option<String> },
    ListPrompts { category: Option<String> },
    ShowPrompt { name: String },
    CreatePrompt { name: String, content: Option<String> },
    
    // Workflow management
    StartWorkflow { description: String },
    ContinueWorkflow { workflow_id: Option<String> },
    ShowWorkflowStatus,
    
    // Chat and interaction
    StartChat,
    AskQuestion { question: String },
    ExecuteTasks,
    
    // MCP operations
    ListMCPServers,
    StartMCPServer { server_name: String },
    ListMCPTools { server_name: Option<String> },
    CallMCPTool { server_name: String, tool_name: String, args: Option<String> },
    
    // File operations
    ReadFile { path: String },
    WriteFile { path: String, content: Option<String> },
    ListFiles { directory: Option<String> },
    
    // System operations
    ShowHelp { topic: Option<String> },
    ShowStatus,
    Configure { setting: Option<String>, value: Option<String> },
    
    // Unknown intent
    Unknown { original_input: String },
}

/// Extracted entities from natural language input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntities {
    pub file_paths: Vec<String>,
    pub prompt_names: Vec<String>,
    pub workflow_ids: Vec<String>,
    pub server_names: Vec<String>,
    pub tool_names: Vec<String>,
    pub quoted_strings: Vec<String>,
    pub numbers: Vec<f64>,
    pub keywords: Vec<String>,
}

/// Structured command intent with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIntent {
    pub intent: UserIntent,
    pub confidence: f64,
    pub entities: ExtractedEntities,
    pub suggested_command: String,
    pub alternative_interpretations: Vec<UserIntent>,
}

/// Intent classification system
#[derive(Debug)]
pub struct IntentClassifier {
    patterns: Vec<IntentPattern>,
}

/// Pattern for matching user intents
#[derive(Debug, Clone)]
pub struct IntentPattern {
    pub intent: UserIntent,
    pub patterns: Vec<String>,
    pub keywords: Vec<String>,
    pub confidence_base: f64,
}

/// Entity extraction system
#[derive(Debug)]
pub struct EntityExtractor {
    patterns: HashMap<String, Regex>,
}

/// Command mapping system
#[derive(Debug)]
pub struct CommandMapper {
    intent_to_command: HashMap<String, String>,
}

impl NaturalLanguageParser {
    /// Create a new natural language parser
    pub fn new() -> Result<Self> {
        Ok(Self {
            intent_classifier: IntentClassifier::new()?,
            entity_extractor: EntityExtractor::new()?,
            command_mapper: CommandMapper::new(),
            pattern_cache: HashMap::new(),
        })
    }

    /// Parse natural language input into structured command intent
    pub fn parse_natural_command(&self, input: &str) -> Result<CommandIntent> {
        let cleaned_input = self.preprocess_input(input);
        
        // Extract entities first
        let entities = self.entity_extractor.extract_entities(&cleaned_input)?;
        
        // Classify intent
        let (intent, confidence) = self.intent_classifier.classify_intent(&cleaned_input, &entities)?;
        
        // Generate suggested command
        let suggested_command = self.command_mapper.map_intent_to_command(&intent, &entities)?;
        
        // Find alternative interpretations
        let alternatives = self.intent_classifier.get_alternative_intents(&cleaned_input, &entities)?;
        
        Ok(CommandIntent {
            intent,
            confidence,
            entities,
            suggested_command,
            alternative_interpretations: alternatives,
        })
    }

    /// Preprocess input text
    fn preprocess_input(&self, input: &str) -> String {
        input
            .trim()
            .to_lowercase()
            // Normalize whitespace
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get suggestions for incomplete or unclear input
    pub fn get_suggestions(&self, input: &str) -> Result<Vec<String>> {
        let cleaned_input = self.preprocess_input(input);
        
        if cleaned_input.is_empty() {
            return Ok(vec![
                "list prompts".to_string(),
                "start chat".to_string(),
                "show status".to_string(),
                "help".to_string(),
            ]);
        }
        
        // Find partial matches
        let mut suggestions = Vec::new();
        
        // Check for partial command matches
        for pattern in &self.intent_classifier.patterns {
            for pattern_text in &pattern.patterns {
                if pattern_text.contains(&cleaned_input) || cleaned_input.contains(pattern_text) {
                    suggestions.push(pattern_text.clone());
                }
            }
        }
        
        // Add keyword-based suggestions
        for pattern in &self.intent_classifier.patterns {
            for keyword in &pattern.keywords {
                if keyword.contains(&cleaned_input) {
                    suggestions.extend(pattern.patterns.clone());
                }
            }
        }
        
        // Remove duplicates and sort by relevance
        suggestions.sort();
        suggestions.dedup();
        
        // Limit to top 5 suggestions
        suggestions.truncate(5);
        
        Ok(suggestions)
    }
}

impl IntentClassifier {
    /// Create a new intent classifier with predefined patterns
    fn new() -> Result<Self> {
        let patterns = vec![
            // Prompt management patterns
            IntentPattern {
                intent: UserIntent::SearchPrompts { query: None },
                patterns: vec![
                    "search prompts".to_string(),
                    "find prompts".to_string(),
                    "look for prompts".to_string(),
                    "search for".to_string(),
                    "find".to_string(),
                ],
                keywords: vec!["search".to_string(), "find".to_string(), "prompts".to_string()],
                confidence_base: 0.8,
            },
            IntentPattern {
                intent: UserIntent::ListPrompts { category: None },
                patterns: vec![
                    "list prompts".to_string(),
                    "show prompts".to_string(),
                    "show all prompts".to_string(),
                    "list all prompts".to_string(),
                    "prompts".to_string(),
                ],
                keywords: vec!["list".to_string(), "show".to_string(), "prompts".to_string()],
                confidence_base: 0.9,
            },
            IntentPattern {
                intent: UserIntent::ShowPrompt { name: String::new() },
                patterns: vec![
                    "show prompt".to_string(),
                    "display prompt".to_string(),
                    "get prompt".to_string(),
                    "view prompt".to_string(),
                ],
                keywords: vec!["show".to_string(), "display".to_string(), "prompt".to_string()],
                confidence_base: 0.8,
            },

            // Workflow management patterns
            IntentPattern {
                intent: UserIntent::StartWorkflow { description: String::new() },
                patterns: vec![
                    "start workflow".to_string(),
                    "begin workflow".to_string(),
                    "create workflow".to_string(),
                    "new workflow".to_string(),
                    "workflow start".to_string(),
                ],
                keywords: vec!["start".to_string(), "workflow".to_string(), "begin".to_string()],
                confidence_base: 0.9,
            },
            IntentPattern {
                intent: UserIntent::ContinueWorkflow { workflow_id: None },
                patterns: vec![
                    "continue workflow".to_string(),
                    "resume workflow".to_string(),
                    "workflow continue".to_string(),
                    "keep going".to_string(),
                ],
                keywords: vec!["continue".to_string(), "resume".to_string(), "workflow".to_string()],
                confidence_base: 0.8,
            },
            IntentPattern {
                intent: UserIntent::ShowWorkflowStatus,
                patterns: vec![
                    "workflow status".to_string(),
                    "show workflow".to_string(),
                    "workflow info".to_string(),
                    "check workflow".to_string(),
                ],
                keywords: vec!["workflow".to_string(), "status".to_string(), "info".to_string()],
                confidence_base: 0.8,
            },

            // Chat patterns
            IntentPattern {
                intent: UserIntent::StartChat,
                patterns: vec![
                    "start chat".to_string(),
                    "begin chat".to_string(),
                    "chat".to_string(),
                    "talk".to_string(),
                    "interactive".to_string(),
                ],
                keywords: vec!["chat".to_string(), "talk".to_string(), "interactive".to_string()],
                confidence_base: 0.9,
            },
            IntentPattern {
                intent: UserIntent::ExecuteTasks,
                patterns: vec![
                    "execute".to_string(),
                    "run tasks".to_string(),
                    "execute tasks".to_string(),
                    "run".to_string(),
                    "go".to_string(),
                ],
                keywords: vec!["execute".to_string(), "run".to_string(), "tasks".to_string()],
                confidence_base: 0.8,
            },

            // MCP patterns
            IntentPattern {
                intent: UserIntent::ListMCPServers,
                patterns: vec![
                    "list mcp servers".to_string(),
                    "mcp servers".to_string(),
                    "show mcp".to_string(),
                    "mcp list".to_string(),
                ],
                keywords: vec!["mcp".to_string(), "servers".to_string(), "list".to_string()],
                confidence_base: 0.9,
            },
            IntentPattern {
                intent: UserIntent::ListMCPTools { server_name: None },
                patterns: vec![
                    "mcp tools".to_string(),
                    "list tools".to_string(),
                    "show tools".to_string(),
                    "tools".to_string(),
                ],
                keywords: vec!["mcp".to_string(), "tools".to_string(), "list".to_string()],
                confidence_base: 0.8,
            },

            // File operation patterns
            IntentPattern {
                intent: UserIntent::ListFiles { directory: None },
                patterns: vec![
                    "list files".to_string(),
                    "show files".to_string(),
                    "ls".to_string(),
                    "dir".to_string(),
                    "files".to_string(),
                ],
                keywords: vec!["list".to_string(), "files".to_string(), "directory".to_string()],
                confidence_base: 0.8,
            },

            // System patterns
            IntentPattern {
                intent: UserIntent::ShowHelp { topic: None },
                patterns: vec![
                    "help".to_string(),
                    "how to".to_string(),
                    "usage".to_string(),
                    "commands".to_string(),
                    "what can you do".to_string(),
                ],
                keywords: vec!["help".to_string(), "usage".to_string(), "commands".to_string()],
                confidence_base: 0.9,
            },
            IntentPattern {
                intent: UserIntent::ShowStatus,
                patterns: vec![
                    "status".to_string(),
                    "info".to_string(),
                    "information".to_string(),
                    "state".to_string(),
                    "what's happening".to_string(),
                ],
                keywords: vec!["status".to_string(), "info".to_string(), "state".to_string()],
                confidence_base: 0.8,
            },
        ];

        Ok(Self { patterns })
    }

    /// Classify user intent from input text and entities
    fn classify_intent(&self, input: &str, entities: &ExtractedEntities) -> Result<(UserIntent, f64)> {
        let mut best_match: Option<(UserIntent, f64)> = None;
        
        for pattern in &self.patterns {
            let confidence = self.calculate_confidence(input, pattern, entities);
            
            if confidence > 0.5 {
                if let Some((_, best_confidence)) = &best_match {
                    if confidence > *best_confidence {
                        let enhanced_intent = self.enhance_intent_with_entities(&pattern.intent, entities);
                        best_match = Some((enhanced_intent, confidence));
                    }
                } else {
                    let enhanced_intent = self.enhance_intent_with_entities(&pattern.intent, entities);
                    best_match = Some((enhanced_intent, confidence));
                }
            }
        }
        
        match best_match {
            Some((intent, confidence)) => Ok((intent, confidence)),
            None => Ok((
                UserIntent::Unknown {
                    original_input: input.to_string(),
                },
                0.0,
            )),
        }
    }

    /// Calculate confidence score for a pattern match
    fn calculate_confidence(&self, input: &str, pattern: &IntentPattern, entities: &ExtractedEntities) -> f64 {
        let mut confidence = 0.0;
        
        // Check pattern matches
        for pattern_text in &pattern.patterns {
            if input.contains(pattern_text) {
                confidence += pattern.confidence_base;
                break;
            }
        }
        
        // Check keyword matches
        let keyword_matches = pattern.keywords.iter()
            .filter(|keyword| input.contains(*keyword))
            .count();
        
        if keyword_matches > 0 {
            confidence += 0.1 * keyword_matches as f64;
        }
        
        // Boost confidence based on relevant entities
        confidence += self.calculate_entity_boost(&pattern.intent, entities);
        
        // Normalize confidence to 0-1 range
        confidence.min(1.0)
    }

    /// Calculate confidence boost based on extracted entities
    fn calculate_entity_boost(&self, intent: &UserIntent, entities: &ExtractedEntities) -> f64 {
        match intent {
            UserIntent::ShowPrompt { .. } if !entities.prompt_names.is_empty() => 0.3,
            UserIntent::StartWorkflow { .. } if !entities.quoted_strings.is_empty() => 0.2,
            UserIntent::ContinueWorkflow { .. } if !entities.workflow_ids.is_empty() => 0.3,
            UserIntent::CallMCPTool { .. } if !entities.server_names.is_empty() && !entities.tool_names.is_empty() => 0.4,
            UserIntent::ReadFile { .. } | UserIntent::WriteFile { .. } if !entities.file_paths.is_empty() => 0.3,
            _ => 0.0,
        }
    }

    /// Enhance intent with extracted entities
    fn enhance_intent_with_entities(&self, intent: &UserIntent, entities: &ExtractedEntities) -> UserIntent {
        match intent {
            UserIntent::SearchPrompts { .. } => {
                let query = entities.quoted_strings.first().cloned()
                    .or_else(|| entities.keywords.first().cloned());
                UserIntent::SearchPrompts { query }
            }
            UserIntent::ShowPrompt { .. } => {
                let name = entities.prompt_names.first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                UserIntent::ShowPrompt { name }
            }
            UserIntent::StartWorkflow { .. } => {
                let description = entities.quoted_strings.first()
                    .cloned()
                    .unwrap_or_else(|| "New workflow".to_string());
                UserIntent::StartWorkflow { description }
            }
            UserIntent::ContinueWorkflow { .. } => {
                let workflow_id = entities.workflow_ids.first().cloned();
                UserIntent::ContinueWorkflow { workflow_id }
            }
            UserIntent::ReadFile { .. } => {
                let path = entities.file_paths.first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                UserIntent::ReadFile { path }
            }
            UserIntent::WriteFile { .. } => {
                let path = entities.file_paths.first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let content = entities.quoted_strings.first().cloned();
                UserIntent::WriteFile { path, content }
            }
            UserIntent::CallMCPTool { .. } => {
                let server_name = entities.server_names.first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let tool_name = entities.tool_names.first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let args = entities.quoted_strings.first().cloned();
                UserIntent::CallMCPTool { server_name, tool_name, args }
            }
            UserIntent::ShowHelp { .. } => {
                let topic = entities.keywords.first().cloned();
                UserIntent::ShowHelp { topic }
            }
            _ => intent.clone(),
        }
    }

    /// Get alternative intent interpretations
    fn get_alternative_intents(&self, input: &str, entities: &ExtractedEntities) -> Result<Vec<UserIntent>> {
        let mut alternatives = Vec::new();
        
        for pattern in &self.patterns {
            let confidence = self.calculate_confidence(input, pattern, entities);
            if confidence > 0.3 {
                let enhanced_intent = self.enhance_intent_with_entities(&pattern.intent, entities);
                alternatives.push(enhanced_intent);
            }
        }
        
        // Remove duplicates and sort by confidence
        alternatives.sort_by(|a, b| {
            let conf_a = self.calculate_confidence(input, 
                &self.patterns.iter().find(|p| std::mem::discriminant(&p.intent) == std::mem::discriminant(a)).unwrap(), 
                entities);
            let conf_b = self.calculate_confidence(input, 
                &self.patterns.iter().find(|p| std::mem::discriminant(&p.intent) == std::mem::discriminant(b)).unwrap(), 
                entities);
            conf_b.partial_cmp(&conf_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        alternatives.truncate(3); // Limit to top 3 alternatives
        Ok(alternatives)
    }
}

impl EntityExtractor {
    /// Create a new entity extractor with predefined patterns
    fn new() -> Result<Self> {
        let mut patterns = HashMap::new();
        
        // File path patterns
        patterns.insert(
            "file_paths".to_string(),
            Regex::new(r#"(?:^|\s)([a-zA-Z]:[\\\/](?:[^\\\/:\*\?"<>\|]+[\\\/])*[^\\\/:\*\?"<>\|]*|\/(?:[^\/\0]+\/)*[^\/\0]*|\.\/[^\s]+|\.\.\/[^\s]+|\w+\.\w+)"#)?,
        );
        
        // Quoted strings
        patterns.insert(
            "quoted_strings".to_string(),
            Regex::new(r#""([^"]+)"|'([^']+)'"#)?,
        );
        
        // Numbers
        patterns.insert(
            "numbers".to_string(),
            Regex::new(r"\b\d+(?:\.\d+)?\b")?,
        );
        
        // Workflow IDs (UUID-like patterns)
        patterns.insert(
            "workflow_ids".to_string(),
            Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")?,
        );
        
        Ok(Self { patterns })
    }

    /// Extract entities from input text
    fn extract_entities(&self, input: &str) -> Result<ExtractedEntities> {
        let mut entities = ExtractedEntities {
            file_paths: Vec::new(),
            prompt_names: Vec::new(),
            workflow_ids: Vec::new(),
            server_names: Vec::new(),
            tool_names: Vec::new(),
            quoted_strings: Vec::new(),
            numbers: Vec::new(),
            keywords: Vec::new(),
        };

        // Extract file paths
        if let Some(regex) = self.patterns.get("file_paths") {
            for cap in regex.captures_iter(input) {
                if let Some(path) = cap.get(1) {
                    entities.file_paths.push(path.as_str().to_string());
                }
            }
        }

        // Extract quoted strings
        if let Some(regex) = self.patterns.get("quoted_strings") {
            for cap in regex.captures_iter(input) {
                if let Some(quoted) = cap.get(1).or_else(|| cap.get(2)) {
                    entities.quoted_strings.push(quoted.as_str().to_string());
                }
            }
        }

        // Extract numbers
        if let Some(regex) = self.patterns.get("numbers") {
            for cap in regex.captures_iter(input) {
                if let Ok(num) = cap.get(0).unwrap().as_str().parse::<f64>() {
                    entities.numbers.push(num);
                }
            }
        }

        // Extract workflow IDs
        if let Some(regex) = self.patterns.get("workflow_ids") {
            for cap in regex.captures_iter(input) {
                entities.workflow_ids.push(cap.get(0).unwrap().as_str().to_string());
            }
        }

        // Extract keywords (simple word extraction)
        let words: Vec<String> = input
            .split_whitespace()
            .filter(|word| word.len() > 2)
            .map(|word| word.to_lowercase())
            .collect();
        entities.keywords = words;

        // Extract server names and tool names based on context
        self.extract_mcp_entities(input, &mut entities);
        self.extract_prompt_names(input, &mut entities);

        Ok(entities)
    }

    /// Extract MCP-related entities
    fn extract_mcp_entities(&self, input: &str, entities: &mut ExtractedEntities) {
        // Look for common MCP server names
        let mcp_servers = ["filesystem", "database", "web", "git", "system"];
        for server in &mcp_servers {
            if input.contains(server) {
                entities.server_names.push(server.to_string());
            }
        }

        // Look for common tool names
        let common_tools = ["read_file", "write_file", "list_files", "execute", "query"];
        for tool in &common_tools {
            if input.contains(tool) {
                entities.tool_names.push(tool.to_string());
            }
        }
    }

    /// Extract prompt names from context
    fn extract_prompt_names(&self, input: &str, entities: &mut ExtractedEntities) {
        // Look for words that might be prompt names (after "prompt" keyword)
        let words = input.split_whitespace().collect::<Vec<_>>();
        for (i, word) in words.iter().enumerate() {
            if word.to_lowercase() == "prompt" && i + 1 < words.len() {
                entities.prompt_names.push(words[i + 1].to_string());
            }
        }
    }
}

impl CommandMapper {
    /// Create a new command mapper
    fn new() -> Self {
        Self {
            intent_to_command: HashMap::new(),
        }
    }

    /// Map intent to structured command
    fn map_intent_to_command(&self, intent: &UserIntent, entities: &ExtractedEntities) -> Result<String> {
        let command = match intent {
            UserIntent::SearchPrompts { query } => {
                match query {
                    Some(q) => format!("cai search \"{}\"", q),
                    None => "cai search".to_string(),
                }
            }
            UserIntent::ListPrompts { category } => {
                match category {
                    Some(cat) => format!("cai list --category {}", cat),
                    None => "cai list".to_string(),
                }
            }
            UserIntent::ShowPrompt { name } => {
                format!("cai show {}", name)
            }
            UserIntent::StartWorkflow { description } => {
                format!("cai workflow start \"{}\"", description)
            }
            UserIntent::ContinueWorkflow { workflow_id } => {
                match workflow_id {
                    Some(id) => format!("cai workflow continue {}", id),
                    None => "cai workflow continue".to_string(),
                }
            }
            UserIntent::ShowWorkflowStatus => {
                "cai workflow status".to_string()
            }
            UserIntent::StartChat => {
                "cai chat".to_string()
            }
            UserIntent::ExecuteTasks => {
                "@execute".to_string()
            }
            UserIntent::ListMCPServers => {
                "cai mcp list".to_string()
            }
            UserIntent::ListMCPTools { server_name } => {
                match server_name {
                    Some(name) => format!("cai mcp tools {}", name),
                    None => "cai mcp tools".to_string(),
                }
            }
            UserIntent::CallMCPTool { server_name, tool_name, args } => {
                match args {
                    Some(a) => format!("cai mcp call {} {} --args '{}'", server_name, tool_name, a),
                    None => format!("cai mcp call {} {}", server_name, tool_name),
                }
            }
            UserIntent::ReadFile { path } => {
                format!("cai mcp call filesystem read_file --args '{{\"path\":\"{}\"}}'", path)
            }
            UserIntent::WriteFile { path, content } => {
                match content {
                    Some(c) => format!("cai mcp call filesystem write_file --args '{{\"path\":\"{}\",\"content\":\"{}\"}}'", path, c),
                    None => format!("cai mcp call filesystem write_file --args '{{\"path\":\"{}\"}}'", path),
                }
            }
            UserIntent::ListFiles { directory } => {
                match directory {
                    Some(dir) => format!("cai mcp call filesystem list_files --args '{{\"path\":\"{}\"}}'", dir),
                    None => "cai mcp call filesystem list_files".to_string(),
                }
            }
            UserIntent::ShowHelp { topic } => {
                match topic {
                    Some(t) => format!("cai help {}", t),
                    None => "cai help".to_string(),
                }
            }
            UserIntent::ShowStatus => {
                "@status".to_string()
            }
            UserIntent::Configure { setting, value } => {
                match (setting, value) {
                    (Some(s), Some(v)) => format!("cai config set {} {}", s, v),
                    (Some(s), None) => format!("cai config get {}", s),
                    _ => "cai config".to_string(),
                }
            }
            UserIntent::AskQuestion { question } => {
                question.clone()
            }
            UserIntent::CreatePrompt { name, content } => {
                match content {
                    Some(c) => format!("cai create-prompt {} \"{}\"", name, c),
                    None => format!("cai create-prompt {}", name),
                }
            }
            UserIntent::StartMCPServer { server_name } => {
                format!("cai mcp start {}", server_name)
            }
            UserIntent::Unknown { original_input } => {
                return Err(anyhow!("Could not understand: {}", original_input));
            }
        };

        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_classification() {
        let parser = NaturalLanguageParser::new().unwrap();
        
        let result = parser.parse_natural_command("list all prompts").unwrap();
        assert!(matches!(result.intent, UserIntent::ListPrompts { .. }));
        assert!(result.confidence > 0.5);
        
        let result = parser.parse_natural_command("start a new workflow for building an API").unwrap();
        assert!(matches!(result.intent, UserIntent::StartWorkflow { .. }));
        
        let result = parser.parse_natural_command("show me the status").unwrap();
        assert!(matches!(result.intent, UserIntent::ShowStatus));
    }

    #[test]
    fn test_entity_extraction() {
        let parser = NaturalLanguageParser::new().unwrap();
        
        let result = parser.parse_natural_command("read file /path/to/file.txt").unwrap();
        assert!(!result.entities.file_paths.is_empty());
        assert_eq!(result.entities.file_paths[0], "/path/to/file.txt");
        
        let result = parser.parse_natural_command("start workflow \"Build REST API\"").unwrap();
        assert!(!result.entities.quoted_strings.is_empty());
        assert_eq!(result.entities.quoted_strings[0], "Build REST API");
    }

    #[test]
    fn test_command_mapping() {
        let parser = NaturalLanguageParser::new().unwrap();
        
        let result = parser.parse_natural_command("list prompts").unwrap();
        assert_eq!(result.suggested_command, "cai list");
        
        let result = parser.parse_natural_command("start workflow \"Test project\"").unwrap();
        assert_eq!(result.suggested_command, "cai workflow start \"Test project\"");
    }

    #[test]
    fn test_suggestions() {
        let parser = NaturalLanguageParser::new().unwrap();
        
        let suggestions = parser.get_suggestions("list").unwrap();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("prompts")));
        
        let suggestions = parser.get_suggestions("work").unwrap();
        assert!(suggestions.iter().any(|s| s.contains("workflow")));
    }
}