use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use url::Url;
use strsim::levenshtein;
use crate::logger::{log_debug, log_info, log_warn, log_error, ops};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub title: String,
    pub content: String,
    #[serde(default = "default_score")]
    pub score: u32,
    #[serde(default = "default_id")]
    pub id: String,
}

fn default_score() -> u32 {
    0
}

fn default_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Prompt {
    /// Get the resolved content, fetching from URL if needed
    pub async fn get_resolved_content(&self) -> Result<String> {
        if self.is_url_reference() {
            self.fetch_content_from_url().await
        } else {
            Ok(self.content.clone())
        }
    }

    /// Check if content is a URL reference
    pub fn is_url_reference(&self) -> bool {
        self.content.starts_with("http://") 
            || self.content.starts_with("https://") 
            || self.content.starts_with("file://")
    }

    /// Fetch content from URL (file:// or http(s)://)
    async fn fetch_content_from_url(&self) -> Result<String> {
        let fetch_start = Instant::now();
        log_debug!("url", "🔗 Fetching content from: {}", self.content);
        
        let result = if self.content.starts_with("file://") {
            // Handle file:// URLs specially
            let file_path = &self.content[7..]; // Remove "file://" prefix
            let full_path = if file_path.starts_with("/") {
                file_path.to_string()
            } else {
                // Relative path - resolve from current working directory
                std::env::current_dir()
                    .with_context(|| "Failed to get current directory")?
                    .join(file_path)
                    .to_string_lossy()
                    .to_string()
            };
            
            log_debug!("url", "📁 Reading local file: {}", full_path);
            ops::file_operation("READ", &full_path, true);
            fs::read_to_string(&full_path)
                .with_context(|| format!("Failed to read file: {}", full_path))
        } else {
            let url = Url::parse(&self.content)
                .with_context(|| format!("Invalid URL: {}", self.content))?;

            match url.scheme() {
                "http" | "https" => {
                    log_debug!("url", "🌐 Making HTTP request to: {}", self.content);
                    ops::network_operation("GET", &self.content, None);
                    
                    let client = reqwest::Client::new();
                    let response = client.get(&self.content)
                        .send()
                        .await
                        .with_context(|| format!("Failed to fetch URL: {}", self.content))?;
                    
                    let status_code = response.status().as_u16();
                    ops::network_operation("GET", &self.content, Some(status_code));
                    
                    if !response.status().is_success() {
                        let error_msg = format!("HTTP error {}: {}", response.status(), self.content);
                        ops::error_with_context("HTTP_FETCH", &error_msg, None);
                        return Err(anyhow::anyhow!(error_msg));
                    }

                    response.text()
                        .await
                        .with_context(|| format!("Failed to read response body from: {}", self.content))
                }
                _ => {
                    let error_msg = format!("Unsupported URL scheme: {}", url.scheme());
                    ops::error_with_context("URL_SCHEME", &error_msg, Some(&self.content));
                    Err(anyhow::anyhow!(error_msg))
                }
            }
        };

        let fetch_duration = fetch_start.elapsed().as_millis() as u64;
        ops::performance("URL_FETCH", fetch_duration);

        match &result {
            Ok(content) => {
                log_info!("url", "✅ Successfully fetched content ({} chars) from: {}", 
                    content.len(), self.content);
            }
            Err(e) => {
                ops::error_with_context("URL_FETCH", &e.to_string(), Some(&self.content));
            }
        }

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub name: String,
    pub prompts: Vec<Prompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptFile {
    pub name: String,
    pub description: String,
    pub subjects: Vec<Subject>,
}

#[derive(Debug, Clone)]
pub struct PromptData {
    pub file_name: String,
    pub file_path: String,
    pub prompt_file: PromptFile,
}

#[derive(Debug, Clone)]
pub struct PromptManager {
    pub prompts: Vec<PromptData>,
}

impl PromptManager {
    pub fn new() -> Self {
        Self {
            prompts: Vec::new(),
        }
    }

    pub fn load_from_directory<P: AsRef<Path>>(dir_path: P) -> Result<Self> {
        let load_start = Instant::now();
        let dir_path_str = dir_path.as_ref().to_string_lossy();
        log_info!("loader", "📂 Loading prompts from directory: {}", dir_path_str);
        
        let mut manager = Self::new();
        let mut file_count = 0;
        let mut prompt_count = 0;
        
        for entry in WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml"))
        {
            let file_path = entry.path();
            let file_name = file_path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            log_debug!("loader", "📄 Processing file: {}", file_path.display());
            ops::file_operation("LOAD", &file_path.to_string_lossy(), true);

            let read_start = Instant::now();
            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
            let read_duration = read_start.elapsed().as_millis() as u64;
            ops::performance("FILE_READ", read_duration);

            let parse_start = Instant::now();
            let prompt_file: PromptFile = serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML file: {}", file_path.display()))?;
            let parse_duration = parse_start.elapsed().as_millis() as u64;
            ops::performance("YAML_PARSE", parse_duration);

            let file_prompt_count: usize = prompt_file.subjects.iter()
                .map(|s| s.prompts.len()).sum();
            prompt_count += file_prompt_count;
            file_count += 1;

            log_info!("loader", "✅ Loaded '{}': {} subjects, {} prompts", 
                file_name, prompt_file.subjects.len(), file_prompt_count);

            manager.prompts.push(PromptData {
                file_name,
                file_path: file_path.to_string_lossy().to_string(),
                prompt_file,
            });
        }

        let load_duration = load_start.elapsed().as_millis() as u64;
        ops::performance("DIRECTORY_LOAD", load_duration);
        log_info!("loader", "🎉 Successfully loaded {} files with {} total prompts in {}ms", 
            file_count, prompt_count, load_duration);

        Ok(manager)
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for prompt_data in &self.prompts {
            // Search in file name
            if prompt_data.file_name.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    file_name: prompt_data.file_name.clone(),
                    file_path: prompt_data.file_path.clone(),
                    match_type: MatchType::FileName,
                    subject_name: None,
                    prompt_title: None,
                    prompt_content: None,
                });
            }

            // Search in file description
            if prompt_data.prompt_file.description.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    file_name: prompt_data.file_name.clone(),
                    file_path: prompt_data.file_path.clone(),
                    match_type: MatchType::FileDescription,
                    subject_name: None,
                    prompt_title: None,
                    prompt_content: Some(prompt_data.prompt_file.description.clone()),
                });
            }

            // Search in subjects and prompts
            for subject in &prompt_data.prompt_file.subjects {
                // Search in subject name
                if subject.name.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        file_name: prompt_data.file_name.clone(),
                        file_path: prompt_data.file_path.clone(),
                        match_type: MatchType::SubjectName,
                        subject_name: Some(subject.name.clone()),
                        prompt_title: None,
                        prompt_content: None,
                    });
                }

                // Search in prompts
                for prompt in &subject.prompts {
                    if prompt.title.to_lowercase().contains(&query_lower) {
                        results.push(SearchResult {
                            file_name: prompt_data.file_name.clone(),
                            file_path: prompt_data.file_path.clone(),
                            match_type: MatchType::PromptTitle,
                            subject_name: Some(subject.name.clone()),
                            prompt_title: Some(prompt.title.clone()),
                            prompt_content: Some(prompt.content.clone()),
                        });
                    }

                    // Search in direct content or try to search in resolved content
                    let content_to_search = if prompt.is_url_reference() {
                        // For URL references, search in the URL itself first
                        if prompt.content.to_lowercase().contains(&query_lower) {
                            results.push(SearchResult {
                                file_name: prompt_data.file_name.clone(),
                                file_path: prompt_data.file_path.clone(),
                                match_type: MatchType::PromptContent,
                                subject_name: Some(subject.name.clone()),
                                prompt_title: Some(prompt.title.clone()),
                                prompt_content: Some(prompt.content.clone()),
                            });
                        }
                        
                        // Skip URL content search in sync context to avoid async issues
                        // TODO: Consider making search async in the future for URL content
                        None
                    } else {
                        Some(prompt.content.clone())
                    };

                    if let Some(content) = content_to_search {
                        if content.to_lowercase().contains(&query_lower) && !prompt.is_url_reference() {
                            results.push(SearchResult {
                                file_name: prompt_data.file_name.clone(),
                                file_path: prompt_data.file_path.clone(),
                                match_type: MatchType::PromptContent,
                                subject_name: Some(subject.name.clone()),
                                prompt_title: Some(prompt.title.clone()),
                                prompt_content: Some(content),
                            });
                        } else if content.to_lowercase().contains(&query_lower) && prompt.is_url_reference() {
                            // Match found in resolved content from URL
                            results.push(SearchResult {
                                file_name: prompt_data.file_name.clone(),
                                file_path: prompt_data.file_path.clone(),
                                match_type: MatchType::PromptContent,
                                subject_name: Some(subject.name.clone()),
                                prompt_title: Some(prompt.title.clone()),
                                prompt_content: Some(format!("🔗 {} (resolved content matched)", prompt.content)),
                            });
                        }
                    }
                }
            }
        }

        results
    }

    pub fn list_all(&self) -> Vec<&PromptData> {
        self.prompts.iter().collect()
    }

    pub fn get_by_file_name(&self, file_name: &str) -> Option<&PromptData> {
        self.prompts.iter().find(|p| p.file_name == file_name)
    }

    pub async fn find_similar_prompts(&self, task_content: &str, threshold: f32) -> Vec<SimilarPrompt> {
        let mut similar_prompts = Vec::new();
        
        for prompt_data in &self.prompts {
            for subject in &prompt_data.prompt_file.subjects {
                for prompt in &subject.prompts {
                    // Use simple string similarity for now
                    let resolved_content = prompt.get_resolved_content().await.unwrap_or(prompt.content.clone());
                    let similarity = calculate_text_similarity(task_content, &resolved_content);
                    
                    if similarity >= threshold {
                        similar_prompts.push(SimilarPrompt {
                            prompt: prompt.clone(),
                            subject_name: subject.name.clone(),
                            file_name: prompt_data.file_name.clone(),
                            similarity_score: similarity,
                        });
                    }
                }
            }
        }
        
        // Sort by similarity score (highest first)
        similar_prompts.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        similar_prompts
    }

    pub fn add_prompt_to_subject(&mut self, file_name: &str, subject_name: &str, new_prompt: Prompt) -> Result<()> {
        for prompt_data in &mut self.prompts {
            if prompt_data.file_name == file_name {
                for subject in &mut prompt_data.prompt_file.subjects {
                    if subject.name == subject_name {
                        subject.prompts.push(new_prompt);
                        let file_path = prompt_data.file_path.clone();
                        let prompt_file = prompt_data.prompt_file.clone();
                        return self.save_prompt_file(&file_path, &prompt_file);
                    }
                }
                // Subject not found, create it
                let new_subject = Subject {
                    name: subject_name.to_string(),
                    prompts: vec![new_prompt],
                };
                prompt_data.prompt_file.subjects.push(new_subject);
                let file_path = prompt_data.file_path.clone();
                let prompt_file = prompt_data.prompt_file.clone();
                return self.save_prompt_file(&file_path, &prompt_file);
            }
        }
        Err(anyhow::anyhow!("File '{}' not found", file_name))
    }

    pub fn update_prompt(&mut self, file_name: &str, subject_name: &str, prompt_id: &str, updated_content: String) -> Result<()> {
        for prompt_data in &mut self.prompts {
            if prompt_data.file_name == file_name {
                for subject in &mut prompt_data.prompt_file.subjects {
                    if subject.name == subject_name {
                        for prompt in &mut subject.prompts {
                            if prompt.id == prompt_id {
                                prompt.content = updated_content;
                                let file_path = prompt_data.file_path.clone();
                                let prompt_file = prompt_data.prompt_file.clone();
                                return self.save_prompt_file(&file_path, &prompt_file);
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Prompt not found"))
    }

    pub fn increment_prompt_score(&mut self, file_name: &str, subject_name: &str, prompt_id: &str) -> Result<()> {
        for prompt_data in &mut self.prompts {
            if prompt_data.file_name == file_name {
                for subject in &mut prompt_data.prompt_file.subjects {
                    if subject.name == subject_name {
                        for prompt in &mut subject.prompts {
                            if prompt.id == prompt_id {
                                prompt.score += 1;
                                let file_path = prompt_data.file_path.clone();
                                let prompt_file = prompt_data.prompt_file.clone();
                                return self.save_prompt_file(&file_path, &prompt_file);
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Prompt not found"))
    }

    fn save_prompt_file(&self, file_path: &str, prompt_file: &PromptFile) -> Result<()> {
        let yaml_content = serde_yaml::to_string(prompt_file)
            .context("Failed to serialize prompt file to YAML")?;
        
        fs::write(file_path, yaml_content)
            .with_context(|| format!("Failed to write file: {}", file_path))
    }

    pub fn get_or_create_ai_generated_file(&mut self) -> Result<String> {
        let ai_file_name = "ai_generated";
        
        // Check if file exists
        if self.get_by_file_name(ai_file_name).is_some() {
            return Ok(ai_file_name.to_string());
        }

        // Determine the prompts directory from existing files
        let prompts_dir = if let Some(existing_data) = self.prompts.first() {
            let existing_path = Path::new(&existing_data.file_path);
            existing_path.parent().unwrap_or(Path::new("."))
        } else {
            Path::new("prompts")
        };

        // Create new AI-generated prompts file
        let file_path = prompts_dir.join("ai_generated.yaml");
        let prompt_file = PromptFile {
            name: "AI Generated".to_string(),
            description: "Prompts automatically generated through chat interactions".to_string(),
            subjects: Vec::new(),
        };

        self.save_prompt_file(&file_path.to_string_lossy(), &prompt_file)?;
        
        // Add to manager
        let prompt_data = PromptData {
            file_name: ai_file_name.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            prompt_file,
        };
        self.prompts.push(prompt_data);

        Ok(ai_file_name.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_name: String,
    pub file_path: String,
    pub match_type: MatchType,
    pub subject_name: Option<String>,
    pub prompt_title: Option<String>,
    pub prompt_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SimilarPrompt {
    pub prompt: Prompt,
    pub subject_name: String,
    pub file_name: String,
    pub similarity_score: f32,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    FileName,
    FileDescription,
    SubjectName,
    PromptTitle,
    PromptContent,
}

impl std::fmt::Display for MatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            MatchType::FileName => "file name",
            MatchType::FileDescription => "file description",
            MatchType::SubjectName => "subject name",
            MatchType::PromptTitle => "prompt title",
            MatchType::PromptContent => "prompt content",
        };
        write!(f, "{}", name)
    }
}

/// Calculate text similarity using a word-based approach with Levenshtein distance
pub fn calculate_text_similarity(text1: &str, text2: &str) -> f32 {
    let calc_start = Instant::now();
    
    // Normalize texts to lowercase and split into words
    let text1_lower = text1.to_lowercase();
    let text2_lower = text2.to_lowercase();
    let words1: Vec<&str> = text1_lower.split_whitespace().collect();
    let words2: Vec<&str> = text2_lower.split_whitespace().collect();
    
    log_debug!("similarity", "🔍 Calculating similarity: {} words vs {} words", 
        words1.len(), words2.len());
    
    if words1.is_empty() && words2.is_empty() {
        ops::similarity_calculation(text1, text2, 1.0);
        return 1.0;
    }
    
    if words1.is_empty() || words2.is_empty() {
        ops::similarity_calculation(text1, text2, 0.0);
        return 0.0;
    }
    
    // Calculate exact word overlap similarity
    let common_words: Vec<_> = words1.iter()
        .filter(|word| words2.contains(word))
        .collect();
    
    let word_overlap_score = (common_words.len() * 2) as f32 / (words1.len() + words2.len()) as f32;
    log_debug!("similarity", "📊 Word overlap: {} common words, score: {:.3}", 
        common_words.len(), word_overlap_score);
    
    // Calculate semantic similarity for related words (weighted less than exact matches)
    let semantic_matches = count_semantic_matches(&words1, &words2);
    let semantic_score = (semantic_matches as f32 * 1.6) / (words1.len() + words2.len()) as f32;
    log_debug!("similarity", "🧠 Semantic matches: {}, score: {:.3}", 
        semantic_matches, semantic_score);
    
    // Calculate character-level Levenshtein similarity as a secondary metric
    let distance = levenshtein(text1, text2);
    let max_len = text1.len().max(text2.len());
    let char_similarity = if max_len == 0 {
        1.0
    } else {
        1.0 - (distance as f32 / max_len as f32)
    };
    log_debug!("similarity", "🔤 Character similarity: distance={}, score: {:.3}", 
        distance, char_similarity);
    
    // Combine all metrics (balance word overlap, semantic similarity, and character similarity)
    let word_based_score = word_overlap_score.max(semantic_score);
    
    // If there are no word matches at all, rely entirely on character similarity
    // If there are some word matches, balance between word and character similarity
    let final_score = if word_based_score == 0.0 {
        log_debug!("similarity", "🎯 No word matches, using character similarity: {:.3}", char_similarity);
        char_similarity
    } else if word_based_score < 0.1 {
        let score = (word_based_score * 0.2) + (char_similarity * 0.8);
        log_debug!("similarity", "🎯 Low word matches, weighted: {:.3}", score);
        score
    } else {
        let score = (word_based_score * 0.5) + (char_similarity * 0.5);
        log_debug!("similarity", "🎯 Good word matches, balanced: {:.3}", score);
        score
    };

    let calc_duration = calc_start.elapsed().as_micros() as u64;
    ops::performance("SIMILARITY_CALC", calc_duration / 1000); // Convert to ms
    ops::similarity_calculation(text1, text2, final_score);

    final_score
}

/// Count semantic matches between word lists
fn count_semantic_matches(words1: &[&str], words2: &[&str]) -> usize {
    let mut matches = 0;
    
    for word1 in words1 {
        for word2 in words2 {
            if are_semantically_similar(word1, word2) {
                matches += 1;
                break; // Only count one match per word1
            }
        }
    }
    
    matches
}

/// Check if two words are semantically similar
fn are_semantically_similar(word1: &str, word2: &str) -> bool {
    // Check if words are exactly the same
    if word1 == word2 {
        return true;
    }
    
    // Check common semantic equivalencies for optimization/performance domain
    let synonyms = [
        ("optimize", "optimization"),
        ("improve", "improved"),
        ("better", "improved"),
        ("analyze", "analysis"),
        ("implement", "create"),
        ("application", "app"),
    ];
    
    for (syn1, syn2) in &synonyms {
        if (word1 == *syn1 && word2 == *syn2) || (word1 == *syn2 && word2 == *syn1) {
            return true;
        }
    }
    
    // Check if one word contains the other (for word variants)
    if word1.len() > 3 && word2.len() > 3 {
        if word1.contains(word2) || word2.contains(word1) {
            return true;
        }
    }
    
    false
}