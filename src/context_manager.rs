use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Advanced context management system for optimizing LLM interactions
#[derive(Debug)]
pub struct ContextManager {
    /// Maximum context window size in tokens
    context_window: usize,
    /// Context compression threshold (0.0 - 1.0)
    compression_threshold: f32,
    /// Context history entries
    context_history: Arc<RwLock<VecDeque<ContextEntry>>>,
    /// Relevance scoring system
    relevance_scorer: RelevanceScorer,
    /// Context optimization strategies
    optimization_strategies: Vec<OptimizationStrategy>,
    /// Session context data
    session_contexts: Arc<RwLock<HashMap<String, SessionContext>>>,
    /// Global context statistics
    stats: Arc<RwLock<ContextStatistics>>,
    /// Context template system
    template_system: ContextTemplateSystem,
}

/// Individual context entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    /// Unique identifier
    pub id: String,
    /// Entry type
    pub entry_type: ContextEntryType,
    /// Content of the entry
    pub content: String,
    /// Timestamp when created
    pub timestamp: SystemTime,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f32,
    /// Token count estimate
    pub token_count: usize,
    /// Priority level
    pub priority: Priority,
    /// Associated tags
    pub tags: Vec<String>,
    /// Session ID
    pub session_id: String,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Last accessed time
    pub last_accessed: SystemTime,
    /// Access count
    pub access_count: u32,
}

/// Types of context entries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextEntryType {
    /// System prompt or instruction
    SystemPrompt,
    /// User input/query
    UserInput,
    /// Assistant response
    AssistantResponse,
    /// Task execution result
    TaskResult,
    /// File content
    FileContent,
    /// Documentation
    Documentation,
    /// Error information
    ErrorInfo,
    /// Workflow state
    WorkflowState,
    /// Tool execution
    ToolExecution,
    /// Configuration
    Configuration,
    /// Memory/Learning
    Memory,
    /// Custom type
    Custom(String),
}

/// Priority levels for context entries
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Critical = 4,
    High = 3,
    Medium = 2,
    Low = 1,
    Minimal = 0,
}

/// Session-specific context data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Session ID
    pub session_id: String,
    /// Current task context
    pub current_task: Option<TaskContext>,
    /// Active workflow context
    pub workflow_context: Option<WorkflowContext>,
    /// Session variables
    pub variables: HashMap<String, String>,
    /// Conversation summary
    pub conversation_summary: Option<String>,
    /// Session goals
    pub goals: Vec<String>,
    /// Context preferences
    pub preferences: ContextPreferences,
    /// Session statistics
    pub stats: SessionStats,
}

/// Task-specific context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Task ID
    pub task_id: String,
    /// Task description
    pub description: String,
    /// Required context types
    pub required_context: Vec<ContextEntryType>,
    /// Task-specific variables
    pub variables: HashMap<String, String>,
    /// Progress indicators
    pub progress: TaskProgress,
}

/// Workflow context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Workflow ID
    pub workflow_id: String,
    /// Workflow title
    pub title: String,
    /// Current step
    pub current_step: String,
    /// Step history
    pub step_history: Vec<WorkflowStep>,
    /// Workflow variables
    pub variables: HashMap<String, String>,
}

/// Workflow step information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name
    pub name: String,
    /// Step result
    pub result: Option<String>,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Context used
    pub context_used: Vec<String>,
}

/// Task progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Completed steps
    pub completed_steps: Vec<String>,
    /// Current step
    pub current_step: Option<String>,
    /// Remaining steps
    pub remaining_steps: Vec<String>,
    /// Progress percentage
    pub percentage: f32,
}

/// Context preferences for sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPreferences {
    /// Preferred context window usage (0.0 - 1.0)
    pub max_window_usage: f32,
    /// Include file content by default
    pub include_file_content: bool,
    /// Include error history
    pub include_error_history: bool,
    /// Include tool execution history
    pub include_tool_history: bool,
    /// Context compression preference
    pub compression_preference: CompressionPreference,
}

/// Context compression preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionPreference {
    Aggressive,
    Balanced,
    Conservative,
    None,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total context entries
    pub total_entries: usize,
    /// Average relevance score
    pub avg_relevance: f32,
    /// Context compression events
    pub compression_events: u32,
    /// Token usage over time
    pub token_usage_history: Vec<(SystemTime, usize)>,
}

/// Relevance scoring system
#[derive(Debug)]
pub struct RelevanceScorer {
    /// Scoring algorithms
    algorithms: Vec<Box<dyn ScoringAlgorithm>>,
    /// Algorithm weights
    weights: HashMap<String, f32>,
    /// Learning system
    learning_system: Option<RelevanceLearningSystem>,
}

/// Scoring algorithm trait
pub trait ScoringAlgorithm: Send + Sync {
    /// Name of the algorithm
    fn name(&self) -> &str;
    
    /// Calculate relevance score
    fn score(&self, entry: &ContextEntry, context: &ContextScoringContext) -> f32;
    
    /// Update algorithm based on feedback
    fn update(&mut self, feedback: &ScoringFeedback) -> Result<()>;
}

/// Context for scoring algorithms
#[derive(Debug, Clone)]
pub struct ContextScoringContext {
    /// Current task
    pub current_task: Option<String>,
    /// Recent entries
    pub recent_entries: Vec<ContextEntry>,
    /// Session goals
    pub session_goals: Vec<String>,
    /// Keywords in current query
    pub query_keywords: Vec<String>,
    /// Time-based factors
    pub time_factors: TimeFactors,
}

/// Time-based scoring factors
#[derive(Debug, Clone)]
pub struct TimeFactors {
    /// How recent the entry is (0.0 - 1.0)
    pub recency: f32,
    /// Time since last access
    pub staleness: f32,
    /// Frequency of access
    pub frequency: f32,
}

/// Feedback for scoring algorithm improvement
#[derive(Debug, Clone)]
pub struct ScoringFeedback {
    /// Entry that was scored
    pub entry_id: String,
    /// Predicted score
    pub predicted_score: f32,
    /// Actual relevance (0.0 - 1.0)
    pub actual_relevance: f32,
    /// Feedback type
    pub feedback_type: FeedbackType,
}

/// Types of feedback
#[derive(Debug, Clone)]
pub enum FeedbackType {
    /// Entry was used in successful task
    Used,
    /// Entry was ignored despite high score
    Ignored,
    /// Entry was manually selected despite low score
    ManuallySelected,
    /// Entry contributed to error
    Harmful,
}

/// Relevance learning system
#[derive(Debug)]
pub struct RelevanceLearningSystem {
    /// Training data
    training_data: Vec<ScoringFeedback>,
    /// Model parameters
    parameters: HashMap<String, f32>,
    /// Learning rate
    learning_rate: f32,
}

/// Context optimization strategies
#[derive(Debug)]
pub enum OptimizationStrategy {
    /// Remove oldest entries
    LeastRecentlyUsed,
    /// Remove lowest scoring entries
    LowestRelevance,
    /// Compress similar entries
    SimilarityCompression,
    /// Remove redundant information
    RedundancyElimination,
    /// Summarize long entries
    Summarization,
    /// Priority-based removal
    PriorityBased,
}

/// Context template system
#[derive(Debug)]
pub struct ContextTemplateSystem {
    /// Available templates
    templates: HashMap<String, ContextTemplate>,
    /// Template usage statistics
    usage_stats: HashMap<String, u32>,
}

/// Context template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTemplate {
    /// Template name
    pub name: String,
    /// Template content
    pub template: String,
    /// Required variables
    pub required_variables: Vec<String>,
    /// Optional variables
    pub optional_variables: Vec<String>,
    /// Template priority
    pub priority: Priority,
    /// Usage count
    pub usage_count: u32,
}

/// Context statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStatistics {
    /// Total entries managed
    pub total_entries: usize,
    /// Average entry relevance
    pub average_relevance: f32,
    /// Compression events
    pub compression_events: u32,
    /// Token efficiency (useful tokens / total tokens)
    pub token_efficiency: f32,
    /// Context hit rate
    pub context_hit_rate: f32,
    /// Average context window usage
    pub avg_window_usage: f32,
}

/// Optimized context result
#[derive(Debug, Clone)]
pub struct OptimizedContext {
    /// Selected context entries
    pub entries: Vec<ContextEntry>,
    /// Total token count
    pub total_tokens: usize,
    /// Context window utilization
    pub window_utilization: f32,
    /// Optimization metadata
    pub optimization_info: OptimizationInfo,
}

/// Optimization information
#[derive(Debug, Clone)]
pub struct OptimizationInfo {
    /// Strategies applied
    pub strategies_applied: Vec<String>,
    /// Entries removed
    pub entries_removed: usize,
    /// Entries compressed
    pub entries_compressed: usize,
    /// Original token count
    pub original_tokens: usize,
    /// Final token count
    pub final_tokens: usize,
    /// Optimization duration
    pub optimization_duration: std::time::Duration,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(context_window: usize, compression_threshold: f32) -> Self {
        let relevance_scorer = RelevanceScorer::new();
        let optimization_strategies = vec![
            OptimizationStrategy::PriorityBased,
            OptimizationStrategy::LowestRelevance,
            OptimizationStrategy::LeastRecentlyUsed,
            OptimizationStrategy::RedundancyElimination,
        ];

        Self {
            context_window,
            compression_threshold,
            context_history: Arc::new(RwLock::new(VecDeque::new())),
            relevance_scorer,
            optimization_strategies,
            session_contexts: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ContextStatistics::default())),
            template_system: ContextTemplateSystem::new(),
        }
    }

    /// Add a new context entry
    pub async fn add_context_entry(
        &self,
        entry_type: ContextEntryType,
        content: String,
        session_id: String,
        priority: Priority,
        tags: Vec<String>,
    ) -> Result<String> {
        let entry_id = Uuid::new_v4().to_string();
        let token_count = self.estimate_token_count(&content);
        let now = SystemTime::now();

        let entry = ContextEntry {
            id: entry_id.clone(),
            entry_type,
            content,
            timestamp: now,
            relevance_score: 0.0, // Will be calculated later
            token_count,
            priority,
            tags,
            session_id: session_id.clone(),
            metadata: HashMap::new(),
            last_accessed: now,
            access_count: 0,
        };

        // Add to context history
        {
            let mut history = self.context_history.write().await;
            history.push_back(entry.clone());
        }

        // Update session context
        self.update_session_context(&session_id, &entry).await?;

        // Update statistics
        self.update_statistics().await;

        // Trigger optimization if needed
        if self.should_optimize().await? {
            self.optimize_context(&session_id).await?;
        }

        Ok(entry_id)
    }

    /// Get optimized context for a specific task
    pub async fn get_optimized_context(
        &self,
        session_id: &str,
        current_task: &str,
        max_tokens: Option<usize>,
    ) -> Result<OptimizedContext> {
        let start_time = std::time::Instant::now();
        let target_tokens = max_tokens.unwrap_or(self.context_window);

        // Get all relevant context entries
        let all_entries = self.get_session_entries(session_id).await;
        
        // Calculate relevance scores
        let scored_entries = self.score_entries_for_task(&all_entries, current_task, session_id).await?;
        
        // Apply optimization strategies
        let optimized_entries = self.apply_optimization_strategies(&scored_entries, target_tokens).await?;
        
        let total_tokens: usize = optimized_entries.iter().map(|e| e.token_count).sum();
        let window_utilization = total_tokens as f32 / target_tokens as f32;

        let optimization_info = OptimizationInfo {
            strategies_applied: self.optimization_strategies.iter()
                .map(|s| format!("{:?}", s))
                .collect(),
            entries_removed: all_entries.len() - optimized_entries.len(),
            entries_compressed: 0, // TODO: Track compression
            original_tokens: all_entries.iter().map(|e| e.token_count).sum(),
            final_tokens: total_tokens,
            optimization_duration: start_time.elapsed(),
        };

        Ok(OptimizedContext {
            entries: optimized_entries,
            total_tokens,
            window_utilization,
            optimization_info,
        })
    }

    /// Create context from template
    pub async fn create_context_from_template(
        &self,
        template_name: &str,
        variables: HashMap<String, String>,
        session_id: String,
    ) -> Result<String> {
        let template = self.template_system.get_template(template_name)
            .ok_or_else(|| anyhow!("Template not found: {}", template_name))?;

        let content = self.template_system.render_template(template, &variables)?;
        
        self.add_context_entry(
            ContextEntryType::SystemPrompt,
            content,
            session_id,
            template.priority.clone(),
            vec!["template".to_string(), template_name.to_string()],
        ).await
    }

    /// Update context entry relevance
    pub async fn update_entry_relevance(&self, entry_id: &str, relevance: f32) -> Result<()> {
        let mut history = self.context_history.write().await;
        
        for entry in history.iter_mut() {
            if entry.id == entry_id {
                entry.relevance_score = relevance.clamp(0.0, 1.0);
                entry.last_accessed = SystemTime::now();
                entry.access_count += 1;
                break;
            }
        }

        Ok(())
    }

    /// Provide feedback for relevance scoring
    pub async fn provide_scoring_feedback(
        &mut self,
        entry_id: String,
        predicted_score: f32,
        actual_relevance: f32,
        feedback_type: FeedbackType,
    ) -> Result<()> {
        let feedback = ScoringFeedback {
            entry_id,
            predicted_score,
            actual_relevance,
            feedback_type,
        };

        self.relevance_scorer.update_with_feedback(feedback).await?;
        Ok(())
    }

    /// Get context statistics
    pub async fn get_statistics(&self) -> ContextStatistics {
        self.stats.read().await.clone()
    }

    /// Clear old context entries
    pub async fn cleanup_old_entries(&self, max_age_days: u32) -> Result<usize> {
        let cutoff_time = SystemTime::now() - std::time::Duration::from_secs(max_age_days as u64 * 24 * 3600);
        let mut history = self.context_history.write().await;
        
        let original_len = history.len();
        history.retain(|entry| entry.timestamp > cutoff_time);
        let removed_count = original_len - history.len();

        // Update statistics
        drop(history);
        self.update_statistics().await;

        Ok(removed_count)
    }

    /// Export context for analysis
    pub async fn export_context(&self, session_id: &str) -> Result<Vec<ContextEntry>> {
        Ok(self.get_session_entries(session_id).await)
    }

    /// Import context from external source
    pub async fn import_context(&self, entries: Vec<ContextEntry>) -> Result<()> {
        let mut history = self.context_history.write().await;
        
        for entry in entries {
            history.push_back(entry);
        }

        // Update statistics
        drop(history);
        self.update_statistics().await;

        Ok(())
    }

    // Private helper methods
    async fn get_session_entries(&self, session_id: &str) -> Vec<ContextEntry> {
        let history = self.context_history.read().await;
        history.iter()
            .filter(|entry| entry.session_id == session_id)
            .cloned()
            .collect()
    }

    async fn score_entries_for_task(
        &self,
        entries: &[ContextEntry],
        current_task: &str,
        session_id: &str,
    ) -> Result<Vec<ContextEntry>> {
        let session_context = self.get_session_context(session_id).await;
        
        let scoring_context = ContextScoringContext {
            current_task: Some(current_task.to_string()),
            recent_entries: entries.iter().rev().take(10).cloned().collect(),
            session_goals: session_context.map(|sc| sc.goals).unwrap_or_default(),
            query_keywords: self.extract_keywords(current_task),
            time_factors: TimeFactors {
                recency: 1.0,
                staleness: 0.0,
                frequency: 1.0,
            },
        };

        let mut scored_entries = Vec::new();
        for mut entry in entries.iter().cloned() {
            entry.relevance_score = self.relevance_scorer.score_entry(&entry, &scoring_context).await;
            scored_entries.push(entry);
        }

        // Sort by relevance score (highest first)
        scored_entries.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        Ok(scored_entries)
    }

    async fn apply_optimization_strategies(
        &self,
        entries: &[ContextEntry],
        target_tokens: usize,
    ) -> Result<Vec<ContextEntry>> {
        let mut optimized_entries = entries.to_vec();
        let mut current_tokens: usize = optimized_entries.iter().map(|e| e.token_count).sum();

        // Apply strategies in order until we fit within token limit
        for strategy in &self.optimization_strategies {
            if current_tokens <= target_tokens {
                break;
            }

            optimized_entries = self.apply_single_strategy(strategy, optimized_entries, target_tokens).await?;
            current_tokens = optimized_entries.iter().map(|e| e.token_count).sum();
        }

        Ok(optimized_entries)
    }

    async fn apply_single_strategy(
        &self,
        strategy: &OptimizationStrategy,
        entries: Vec<ContextEntry>,
        target_tokens: usize,
    ) -> Result<Vec<ContextEntry>> {
        match strategy {
            OptimizationStrategy::PriorityBased => {
                let mut result = entries;
                result.sort_by(|a, b| b.priority.cmp(&a.priority));
                
                let mut tokens = 0;
                result.retain(|entry| {
                    if tokens + entry.token_count <= target_tokens {
                        tokens += entry.token_count;
                        true
                    } else {
                        false
                    }
                });
                
                Ok(result)
            }
            OptimizationStrategy::LowestRelevance => {
                let mut result = entries;
                result.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
                
                let mut tokens = 0;
                result.retain(|entry| {
                    if tokens + entry.token_count <= target_tokens {
                        tokens += entry.token_count;
                        true
                    } else {
                        false
                    }
                });
                
                Ok(result)
            }
            OptimizationStrategy::LeastRecentlyUsed => {
                let mut result = entries;
                result.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
                
                let mut tokens = 0;
                result.retain(|entry| {
                    if tokens + entry.token_count <= target_tokens {
                        tokens += entry.token_count;
                        true
                    } else {
                        false
                    }
                });
                
                Ok(result)
            }
            OptimizationStrategy::RedundancyElimination => {
                // Simple redundancy elimination based on content similarity
                let mut result = Vec::new();
                let mut tokens = 0;
                
                for entry in entries {
                    if tokens + entry.token_count > target_tokens {
                        break;
                    }
                    
                    // Check for similar content
                    let is_redundant = result.iter().any(|existing: &ContextEntry| {
                        self.calculate_content_similarity(&entry.content, &existing.content) > 0.8
                    });
                    
                    if !is_redundant {
                        tokens += entry.token_count;
                        result.push(entry);
                    }
                }
                
                Ok(result)
            }
            _ => Ok(entries), // Other strategies not implemented yet
        }
    }

    fn calculate_content_similarity(&self, content1: &str, content2: &str) -> f32 {
        // Simple similarity calculation based on word overlap
        let words1: std::collections::HashSet<&str> = content1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = content2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    async fn update_session_context(&self, session_id: &str, entry: &ContextEntry) -> Result<()> {
        let mut sessions = self.session_contexts.write().await;
        let session = sessions.entry(session_id.to_string()).or_insert_with(|| {
            SessionContext {
                session_id: session_id.to_string(),
                current_task: None,
                workflow_context: None,
                variables: HashMap::new(),
                conversation_summary: None,
                goals: Vec::new(),
                preferences: ContextPreferences::default(),
                stats: SessionStats::default(),
            }
        });

        session.stats.total_entries += 1;
        Ok(())
    }

    async fn get_session_context(&self, session_id: &str) -> Option<SessionContext> {
        let sessions = self.session_contexts.read().await;
        sessions.get(session_id).cloned()
    }

    async fn should_optimize(&self) -> Result<bool> {
        let history = self.context_history.read().await;
        let total_tokens: usize = history.iter().map(|e| e.token_count).sum();
        
        Ok(total_tokens as f32 / self.context_window as f32 > self.compression_threshold)
    }

    async fn optimize_context(&self, session_id: &str) -> Result<()> {
        // Get optimized context and update the history
        let optimized = self.get_optimized_context(session_id, "optimization", None).await?;
        
        // Update context history with optimized entries
        let mut history = self.context_history.write().await;
        history.retain(|entry| entry.session_id != session_id);
        
        for entry in optimized.entries {
            history.push_back(entry);
        }

        // Update statistics
        drop(history);
        self.update_statistics().await;

        Ok(())
    }

    async fn update_statistics(&self) {
        let history = self.context_history.read().await;
        let mut stats = self.stats.write().await;
        
        stats.total_entries = history.len();
        
        if !history.is_empty() {
            let total_relevance: f32 = history.iter().map(|e| e.relevance_score).sum();
            stats.average_relevance = total_relevance / history.len() as f32;
            
            let total_tokens: usize = history.iter().map(|e| e.token_count).sum();
            stats.avg_window_usage = total_tokens as f32 / self.context_window as f32;
        }
    }

    fn estimate_token_count(&self, content: &str) -> usize {
        // Simple token estimation: ~4 characters per token for English text
        (content.len() / 4).max(1)
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() > 3)
            .map(|word| word.to_lowercase())
            .collect()
    }
}

impl RelevanceScorer {
    fn new() -> Self {
        let algorithms: Vec<Box<dyn ScoringAlgorithm>> = vec![
            Box::new(RecencyAlgorithm::new()),
            Box::new(KeywordMatchAlgorithm::new()),
            Box::new(PriorityAlgorithm::new()),
            Box::new(AccessFrequencyAlgorithm::new()),
        ];

        let mut weights = HashMap::new();
        weights.insert("recency".to_string(), 0.3);
        weights.insert("keyword_match".to_string(), 0.4);
        weights.insert("priority".to_string(), 0.2);
        weights.insert("access_frequency".to_string(), 0.1);

        Self {
            algorithms,
            weights,
            learning_system: Some(RelevanceLearningSystem::new()),
        }
    }

    async fn score_entry(&self, entry: &ContextEntry, context: &ContextScoringContext) -> f32 {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        for algorithm in &self.algorithms {
            let score = algorithm.score(entry, context);
            let weight = self.weights.get(algorithm.name()).unwrap_or(&1.0);
            
            total_score += score * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.0
        }
    }

    async fn update_with_feedback(&mut self, feedback: ScoringFeedback) -> Result<()> {
        for algorithm in &mut self.algorithms {
            algorithm.update(&feedback)?;
        }

        if let Some(learning_system) = &mut self.learning_system {
            learning_system.update(feedback)?;
        }

        Ok(())
    }
}

// Scoring algorithm implementations
#[derive(Debug)]
struct RecencyAlgorithm;

impl RecencyAlgorithm {
    fn new() -> Self {
        Self
    }
}

impl ScoringAlgorithm for RecencyAlgorithm {
    fn name(&self) -> &str {
        "recency"
    }

    fn score(&self, entry: &ContextEntry, _context: &ContextScoringContext) -> f32 {
        let now = SystemTime::now();
        let age = now.duration_since(entry.timestamp).unwrap_or_default();
        let hours_old = age.as_secs() as f32 / 3600.0;
        
        // Exponential decay: score decreases as content gets older
        (-hours_old / 24.0).exp()
    }

    fn update(&mut self, _feedback: &ScoringFeedback) -> Result<()> {
        Ok(()) // Simple algorithm doesn't need updates
    }
}

#[derive(Debug)]
struct KeywordMatchAlgorithm;

impl KeywordMatchAlgorithm {
    fn new() -> Self {
        Self
    }
}

impl ScoringAlgorithm for KeywordMatchAlgorithm {
    fn name(&self) -> &str {
        "keyword_match"
    }

    fn score(&self, entry: &ContextEntry, context: &ContextScoringContext) -> f32 {
        let entry_content_lower = entry.content.to_lowercase();
        let entry_words: std::collections::HashSet<&str> = 
            entry_content_lower.split_whitespace().collect();
        
        let keyword_matches = context.query_keywords.iter()
            .filter(|keyword| entry_words.contains(keyword.as_str()))
            .count();
        
        if context.query_keywords.is_empty() {
            0.5 // Default score when no keywords
        } else {
            keyword_matches as f32 / context.query_keywords.len() as f32
        }
    }

    fn update(&mut self, _feedback: &ScoringFeedback) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct PriorityAlgorithm;

impl PriorityAlgorithm {
    fn new() -> Self {
        Self
    }
}

impl ScoringAlgorithm for PriorityAlgorithm {
    fn name(&self) -> &str {
        "priority"
    }

    fn score(&self, entry: &ContextEntry, _context: &ContextScoringContext) -> f32 {
        match entry.priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.6,
            Priority::Low => 0.4,
            Priority::Minimal => 0.2,
        }
    }

    fn update(&mut self, _feedback: &ScoringFeedback) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct AccessFrequencyAlgorithm;

impl AccessFrequencyAlgorithm {
    fn new() -> Self {
        Self
    }
}

impl ScoringAlgorithm for AccessFrequencyAlgorithm {
    fn name(&self) -> &str {
        "access_frequency"
    }

    fn score(&self, entry: &ContextEntry, _context: &ContextScoringContext) -> f32 {
        // Score based on access frequency (logarithmic scale)
        (entry.access_count as f32 + 1.0).log10() / 10.0
    }

    fn update(&mut self, _feedback: &ScoringFeedback) -> Result<()> {
        Ok(())
    }
}

impl RelevanceLearningSystem {
    fn new() -> Self {
        Self {
            training_data: Vec::new(),
            parameters: HashMap::new(),
            learning_rate: 0.01,
        }
    }

    fn update(&mut self, feedback: ScoringFeedback) -> Result<()> {
        self.training_data.push(feedback);
        
        // Simple learning: adjust parameters based on prediction error
        if let Some(latest) = self.training_data.last() {
            let error = latest.actual_relevance - latest.predicted_score;
            
            // Update parameters (simplified gradient descent)
            for (key, value) in self.parameters.iter_mut() {
                *value += self.learning_rate * error;
            }
        }

        // Keep only recent training data
        if self.training_data.len() > 1000 {
            self.training_data.remove(0);
        }

        Ok(())
    }
}

impl ContextTemplateSystem {
    fn new() -> Self {
        let mut templates = HashMap::new();
        
        // Add default templates
        templates.insert("system_prompt".to_string(), ContextTemplate {
            name: "system_prompt".to_string(),
            template: "You are a helpful AI assistant. Current task: {task}. Context: {context}".to_string(),
            required_variables: vec!["task".to_string()],
            optional_variables: vec!["context".to_string()],
            priority: Priority::High,
            usage_count: 0,
        });

        Self {
            templates,
            usage_stats: HashMap::new(),
        }
    }

    fn get_template(&self, name: &str) -> Option<&ContextTemplate> {
        self.templates.get(name)
    }

    fn render_template(&self, template: &ContextTemplate, variables: &HashMap<String, String>) -> Result<String> {
        let mut content = template.template.clone();
        
        // Replace variables
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            content = content.replace(&placeholder, value);
        }

        // Check for required variables
        for required in &template.required_variables {
            if !variables.contains_key(required) {
                return Err(anyhow!("Missing required variable: {}", required));
            }
        }

        Ok(content)
    }
}

impl Default for ContextPreferences {
    fn default() -> Self {
        Self {
            max_window_usage: 0.8,
            include_file_content: true,
            include_error_history: true,
            include_tool_history: true,
            compression_preference: CompressionPreference::Balanced,
        }
    }
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            avg_relevance: 0.0,
            compression_events: 0,
            token_usage_history: Vec::new(),
        }
    }
}

impl Default for ContextStatistics {
    fn default() -> Self {
        Self {
            total_entries: 0,
            average_relevance: 0.0,
            compression_events: 0,
            token_efficiency: 0.0,
            context_hit_rate: 0.0,
            avg_window_usage: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_manager_creation() {
        let manager = ContextManager::new(4000, 0.8);
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_add_context_entry() {
        let manager = ContextManager::new(4000, 0.8);
        
        let entry_id = manager.add_context_entry(
            ContextEntryType::UserInput,
            "Test content".to_string(),
            "test_session".to_string(),
            Priority::Medium,
            vec!["test".to_string()],
        ).await.unwrap();

        assert!(!entry_id.is_empty());
        
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_entries, 1);
    }

    #[tokio::test]
    async fn test_context_optimization() {
        let manager = ContextManager::new(1000, 0.5); // Small window for testing
        
        // Add multiple entries
        for i in 0..10 {
            manager.add_context_entry(
                ContextEntryType::UserInput,
                format!("Test content {}", i),
                "test_session".to_string(),
                Priority::Medium,
                vec!["test".to_string()],
            ).await.unwrap();
        }

        let optimized = manager.get_optimized_context("test_session", "test task", Some(500)).await.unwrap();
        
        assert!(optimized.total_tokens <= 500);
        assert!(!optimized.entries.is_empty());
    }

    #[tokio::test]
    async fn test_relevance_scoring() {
        let manager = ContextManager::new(4000, 0.8);
        
        let entry_id = manager.add_context_entry(
            ContextEntryType::UserInput,
            "Important test content with keywords".to_string(),
            "test_session".to_string(),
            Priority::High,
            vec!["important".to_string()],
        ).await.unwrap();

        manager.update_entry_relevance(&entry_id, 0.9).await.unwrap();

        let entries = manager.get_session_entries("test_session").await;
        assert_eq!(entries[0].relevance_score, 0.9);
    }
}