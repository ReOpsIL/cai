use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::logger::{log_debug, log_info, log_warn};
use crate::openrouter_client::{ChatMessage, OpenRouterClient};
use crate::pub_sub::{Event, EventSubscriber, EventPattern, EventEnvelope, get_global_event_broker};

/// Enhanced session with hierarchical structure and auto-summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSession {
    pub id: String,
    pub parent_session_id: Option<String>,
    pub title: String,
    pub session_type: SessionType,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    
    // Content and context
    pub messages: Vec<SessionMessage>,
    pub summary: Option<SessionSummary>,
    pub context: SessionContext,
    
    // Metrics and statistics
    pub message_count: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_cost: f64,
    pub task_count: usize,
    pub success_rate: f64,
    
    // Metadata
    pub tags: Vec<String>,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    /// Main interactive chat session
    Chat,
    /// Task execution session
    Task,
    /// Workflow orchestration session
    Workflow,
    /// Sub-session for specific operations
    SubTask,
    /// Auto-generated summary session
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub content: String,
    pub metadata: HashMap<String, Value>,
    pub tokens_used: Option<u32>,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Assistant,
    System,
    Tool,
    Error,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub summary_type: SummaryType,
    pub content: String,
    pub key_points: Vec<String>,
    pub outcomes: Vec<String>,
    pub next_actions: Vec<String>,
    pub confidence_score: f64,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SummaryType {
    /// Periodic summary during long sessions
    Periodic,
    /// Summary when session ends
    Final,
    /// Summary of task execution
    TaskCompletion,
    /// Summary for archival
    Archive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub working_directory: String,
    pub environment_vars: HashMap<String, String>,
    pub active_tools: Vec<String>,
    pub shared_state: HashMap<String, Value>,
    pub file_references: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Enhanced session manager with intelligent features
pub struct EnhancedSessionManager {
    sessions: Arc<RwLock<HashMap<String, EnhancedSession>>>,
    llm_client: Option<Arc<OpenRouterClient>>,
    storage_path: PathBuf,
    auto_summarize_threshold: usize,
    auto_archive_days: u64,
}

impl EnhancedSessionManager {
    pub fn new(storage_path: PathBuf, llm_client: Option<Arc<OpenRouterClient>>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            llm_client,
            storage_path,
            auto_summarize_threshold: 50, // Auto-summarize after 50 messages
            auto_archive_days: 30, // Archive sessions after 30 days
        }
    }
    
    /// Create a new session
    pub async fn create_session(
        &self,
        title: String,
        session_type: SessionType,
        parent_session_id: Option<String>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        
        let session = EnhancedSession {
            id: session_id.clone(),
            parent_session_id,
            title: title.clone(),
            session_type: session_type.clone(),
            status: SessionStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ended_at: None,
            messages: Vec::new(),
            summary: None,
            context: SessionContext {
                working_directory: std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .to_string_lossy()
                    .to_string(),
                environment_vars: HashMap::new(),
                active_tools: Vec::new(),
                shared_state: HashMap::new(),
                file_references: Vec::new(),
                dependencies: Vec::new(),
            },
            message_count: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_cost: 0.0,
            task_count: 0,
            success_rate: 0.0,
            tags: Vec::new(),
            user_id: None,
            metadata: HashMap::new(),
        };
        
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }
        
        // Publish session created event
        let broker = get_global_event_broker();
        let event = Event::SessionCreated {
            session_id: session_id.clone(),
            title,
        };
        broker.publish_event(event, "enhanced_session_manager".to_string()).await.ok();
        
        // Save to storage
        self.save_session(&session_id).await.ok();
        
        log_info!("enhanced_session", "Created session {} of type {:?}", session_id, session_type);
        Ok(session_id)
    }
    
    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<EnhancedSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }
    
    /// Add message to session
    pub async fn add_message(
        &self,
        session_id: &str,
        message_type: MessageType,
        content: String,
        metadata: Option<HashMap<String, Value>>,
    ) -> Result<String> {
        let message_id = Uuid::new_v4().to_string();
        let message = SessionMessage {
            id: message_id.clone(),
            timestamp: Utc::now(),
            message_type,
            content,
            metadata: metadata.unwrap_or_default(),
            tokens_used: None,
            cost: None,
        };
        
        let should_summarize = {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.messages.push(message);
                session.message_count += 1;
                session.updated_at = Utc::now();
                
                // Check if we should trigger auto-summarization
                session.message_count >= self.auto_summarize_threshold && session.summary.is_none()
            } else {
                return Err(anyhow!("Session {} not found", session_id));
            }
        };
        
        // Trigger auto-summarization if threshold reached
        if should_summarize {
            log_info!("enhanced_session", "Auto-summarizing session {} after {} messages", 
                     session_id, self.auto_summarize_threshold);
            self.auto_summarize_session(session_id).await.ok();
        }
        
        // Save to storage
        self.save_session(session_id).await.ok();
        
        Ok(message_id)
    }
    
    /// Update session context
    pub async fn update_context(
        &self,
        session_id: &str,
        updates: HashMap<String, Value>,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.updated_at = Utc::now();
            
            // Apply context updates
            for (key, value) in updates.iter() {
                match key.as_str() {
                    "working_directory" => {
                        if let Some(dir) = value.as_str() {
                            session.context.working_directory = dir.to_string();
                        }
                    }
                    "active_tools" => {
                        if let Some(tools) = value.as_array() {
                            session.context.active_tools = tools
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }
                    "file_references" => {
                        if let Some(files) = value.as_array() {
                            session.context.file_references = files
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }
                    _ => {
                        session.context.shared_state.insert(key.clone(), value.clone());
                    }
                }
            }
            
            // Publish update event
            let broker = get_global_event_broker();
            let event = Event::SessionUpdated {
                session_id: session_id.to_string(),
                changes: updates,
            };
            broker.publish_event(event, "enhanced_session_manager".to_string()).await.ok();
            
            Ok(())
        } else {
            Err(anyhow!("Session {} not found", session_id))
        }
    }
    
    /// End a session
    pub async fn end_session(&self, session_id: &str) -> Result<()> {
        let duration_ms = {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.status = SessionStatus::Completed;
                session.ended_at = Some(Utc::now());
                session.updated_at = Utc::now();
                
                session.ended_at.unwrap().timestamp_millis() as u64 -
                    session.created_at.timestamp_millis() as u64
            } else {
                return Err(anyhow!("Session {} not found", session_id));
            }
        };
        
        // Generate final summary if we have an LLM client
        self.generate_final_summary(session_id).await.ok();
        
        // Publish session ended event
        let broker = get_global_event_broker();
        let event = Event::SessionEnded {
            session_id: session_id.to_string(),
            duration_ms,
        };
        broker.publish_event(event, "enhanced_session_manager".to_string()).await.ok();
        
        // Save to storage
        self.save_session(session_id).await.ok();
        
        log_info!("enhanced_session", "Ended session {} after {}ms", session_id, duration_ms);
        Ok(())
    }
    
    /// Auto-summarize session using LLM
    async fn auto_summarize_session(&self, session_id: &str) -> Result<()> {
        if let Some(ref llm_client) = self.llm_client {
            let session = {
                let sessions = self.sessions.read().await;
                sessions.get(session_id).cloned()
                    .ok_or_else(|| anyhow!("Session {} not found", session_id))?
            };
            
            // Extract recent messages for summarization
            let recent_messages: Vec<&SessionMessage> = session.messages
                .iter()
                .rev() // Most recent first
                .take(20) // Last 20 messages
                .collect();
            
            if recent_messages.is_empty() {
                return Ok(());
            }
            
            // Create summarization prompt
            let mut conversation_text = String::new();
            for msg in recent_messages.iter().rev() { // Back to chronological order
                let role = match msg.message_type {
                    MessageType::User => "User",
                    MessageType::Assistant => "Assistant",
                    MessageType::System => "System",
                    MessageType::Tool => "Tool",
                    MessageType::Error => "Error",
                    MessageType::Summary => "Summary",
                };
                conversation_text.push_str(&format!("{}: {}\n", role, msg.content));
            }
            
            let summary_prompt = format!(
                "Please provide a concise summary of this conversation session. Include:\n\
                1. Key topics discussed\n\
                2. Main outcomes or results\n\
                3. Any pending actions or next steps\n\
                4. Overall progress assessment\n\n\
                Conversation:\n{}\n\n\
                Summary:",
                conversation_text
            );
            
            // Generate summary using LLM
            match llm_client.chat_completion(vec![ChatMessage {
                role: "user".to_string(),
                content: summary_prompt,
            }]).await {
                Ok(summary_content) => {
                    let summary = SessionSummary {
                        id: Uuid::new_v4().to_string(),
                        created_at: Utc::now(),
                        summary_type: SummaryType::Periodic,
                        content: summary_content.clone(),
                        key_points: vec![], // Could be parsed from content
                        outcomes: vec![], // Could be parsed from content
                        next_actions: vec![], // Could be parsed from content
                        confidence_score: 0.8, // Default confidence
                        metadata: HashMap::new(),
                    };
                    
                    // Update session with summary
                    {
                        let mut sessions = self.sessions.write().await;
                        if let Some(session) = sessions.get_mut(session_id) {
                            session.summary = Some(summary);
                            session.updated_at = Utc::now();
                        }
                    }
                    
                    log_info!("enhanced_session", "Generated auto-summary for session {}", session_id);
                }
                Err(e) => {
                    log_warn!("enhanced_session", "Failed to generate summary for session {}: {}", session_id, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate final summary when session ends
    async fn generate_final_summary(&self, session_id: &str) -> Result<()> {
        // Similar to auto_summarize_session but marks as Final summary
        // Implementation would be similar but with SummaryType::Final
        log_debug!("enhanced_session", "Generating final summary for session {}", session_id);
        Ok(())
    }
    
    /// List sessions with filtering
    pub async fn list_sessions(&self, filters: SessionFilters) -> Result<Vec<EnhancedSession>> {
        let sessions = self.sessions.read().await;
        let mut filtered_sessions: Vec<EnhancedSession> = sessions
            .values()
            .filter(|session| {
                // Apply filters
                if let Some(ref status) = filters.status {
                    if session.status != *status {
                        return false;
                    }
                }
                
                if let Some(ref session_type) = filters.session_type {
                    if std::mem::discriminant(&session.session_type) != std::mem::discriminant(session_type) {
                        return false;
                    }
                }
                
                if let Some(ref user_id) = filters.user_id {
                    if session.user_id.as_ref() != Some(user_id) {
                        return false;
                    }
                }
                
                if let Some(ref parent_id) = filters.parent_session_id {
                    if session.parent_session_id.as_ref() != Some(parent_id) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Sort by creation date (most recent first)
        filtered_sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Apply limit
        if let Some(limit) = filters.limit {
            filtered_sessions.truncate(limit);
        }
        
        Ok(filtered_sessions)
    }
    
    /// Archive old sessions
    pub async fn archive_old_sessions(&self) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(self.auto_archive_days as i64);
        let mut archived_count = 0;
        
        {
            let mut sessions = self.sessions.write().await;
            for session in sessions.values_mut() {
                if session.status == SessionStatus::Completed && 
                   session.ended_at.unwrap_or(session.updated_at) < cutoff_date &&
                   session.status != SessionStatus::Archived {
                    session.status = SessionStatus::Archived;
                    archived_count += 1;
                }
            }
        }
        
        log_info!("enhanced_session", "Archived {} old sessions", archived_count);
        Ok(archived_count)
    }
    
    /// Save session to storage
    async fn save_session(&self, session_id: &str) -> Result<()> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).cloned()
                .ok_or_else(|| anyhow!("Session {} not found", session_id))?
        };
        
        let session_file = self.storage_path.join(format!("{}.json", session_id));
        let session_json = serde_json::to_string_pretty(&session)?;
        tokio::fs::write(session_file, session_json).await?;
        
        Ok(())
    }
    
    /// Load session from storage
    pub async fn load_session(&self, session_id: &str) -> Result<()> {
        let session_file = self.storage_path.join(format!("{}.json", session_id));
        if !session_file.exists() {
            return Err(anyhow!("Session file not found: {:?}", session_file));
        }
        
        let session_json = tokio::fs::read_to_string(session_file).await?;
        let session: EnhancedSession = serde_json::from_str(&session_json)?;
        
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.to_string(), session);
        }
        
        log_info!("enhanced_session", "Loaded session {} from storage", session_id);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SessionFilters {
    pub status: Option<SessionStatus>,
    pub session_type: Option<SessionType>,
    pub user_id: Option<String>,
    pub parent_session_id: Option<String>,
    pub limit: Option<usize>,
}

/// Event subscriber for session management
pub struct SessionEventSubscriber {
    session_manager: Arc<EnhancedSessionManager>,
}

impl SessionEventSubscriber {
    pub fn new(session_manager: Arc<EnhancedSessionManager>) -> Self {
        Self { session_manager }
    }
}

impl EventSubscriber for SessionEventSubscriber {
    fn handle_event(&self, envelope: &EventEnvelope) -> Result<()> {
        match &envelope.event {
            Event::TaskCompleted { task_id, success, execution_time_ms } => {
                // Could update session metrics based on task completion
                log_debug!("session_events", "Task {} completed: {} in {}ms", 
                          task_id, success, execution_time_ms);
            }
            Event::SafetyViolation { tool_name, reason } => {
                // Could add safety violation to session context
                log_debug!("session_events", "Safety violation in {}: {}", tool_name, reason);
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn interested_events(&self) -> Vec<EventPattern> {
        vec![
            EventPattern::Custom(Box::new(|envelope| {
                matches!(envelope.event,
                    Event::TaskCreated { .. } |
                    Event::TaskCompleted { .. } |
                    Event::SafetyViolation { .. } |
                    Event::ToolExecuted { .. }
                )
            })),
        ]
    }
}