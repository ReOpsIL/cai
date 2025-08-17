use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Mutex;

use crate::logger::{log_debug, log_error, log_info, log_warn};
use crate::openrouter_client::{ChatMessage, OpenRouterClient};

/// Loop Detection Service inspired by gemini-cli
/// Detects when AI gets stuck in unproductive repetitive patterns
#[derive(Debug)]
pub struct LoopDetectionService {
    /// Recent events for pattern analysis
    event_history: Arc<Mutex<VecDeque<DetectionEvent>>>,
    /// LLM client for advanced loop detection
    llm_client: Option<Arc<OpenRouterClient>>,
    /// Detection configuration
    config: LoopDetectionConfig,
    /// Current detection state
    state: Arc<Mutex<DetectionState>>,
}

#[derive(Debug, Clone)]
pub struct LoopDetectionConfig {
    /// Maximum number of events to keep in history
    pub max_history_size: usize,
    /// Time window for loop detection (in seconds)
    pub detection_window: Duration,
    /// Minimum repetitions to consider a loop
    pub min_repetitions: usize,
    /// Enable LLM-based advanced loop detection
    pub enable_llm_detection: bool,
    /// Similarity threshold for content comparison (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Enable tool call loop detection
    pub detect_tool_loops: bool,
    /// Enable content repetition detection
    pub detect_content_loops: bool,
    /// Enable pattern-based detection
    pub detect_pattern_loops: bool,
}

#[derive(Debug, Clone)]
pub struct DetectionState {
    /// Whether a loop is currently detected
    pub loop_detected: bool,
    /// Type of loop detected
    pub loop_type: Option<LoopType>,
    /// When the loop was first detected
    pub detection_time: Option<Instant>,
    /// Number of iterations in the current loop
    pub iteration_count: usize,
    /// Confidence level of detection (0.0 to 1.0)
    pub confidence: f32,
    /// Description of the detected loop
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopType {
    /// Repeating the same tool calls
    ToolCallLoop,
    /// Generating repetitive content
    ContentLoop,
    /// Stuck in error-retry cycles
    ErrorLoop,
    /// LLM-detected unproductive pattern
    LLMDetectedLoop,
    /// Pattern-based detection
    PatternLoop,
}

#[derive(Debug, Clone)]
pub struct DetectionEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: EventType,
    /// Event content/payload
    pub content: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp when event occurred
    pub timestamp: Instant,
    /// Session ID this event belongs to
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    /// Tool call request
    ToolCallRequest,
    /// AI-generated content
    ContentGeneration,
    /// Error occurrence
    Error,
    /// Task execution
    TaskExecution,
    /// User input
    UserInput,
    /// System message
    SystemMessage,
}

#[derive(Debug, Clone)]
pub struct LoopDetectionResult {
    /// Whether a loop was detected
    pub loop_detected: bool,
    /// Type of loop (if detected)
    pub loop_type: Option<LoopType>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Description of the detected pattern
    pub description: String,
    /// Suggested actions to break the loop
    pub suggested_actions: Vec<LoopBreakAction>,
    /// Events that form the loop pattern
    pub loop_pattern: Vec<DetectionEvent>,
}

#[derive(Debug, Clone)]
pub enum LoopBreakAction {
    /// Clear conversation context
    ClearContext,
    /// Reset to previous state
    ResetState,
    /// Request user intervention
    RequestUserInput,
    /// Change approach/strategy
    ChangeStrategy,
    /// Escalate to higher-level reasoning
    EscalateReasoning,
    /// Abort current task
    AbortTask,
}

impl LoopDetectionService {
    pub async fn new(config: LoopDetectionConfig, llm_client: Option<Arc<OpenRouterClient>>) -> Self {
        Self {
            event_history: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_history_size))),
            llm_client,
            config,
            state: Arc::new(Mutex::new(DetectionState::new())),
        }
    }

    pub fn with_defaults(llm_client: Option<Arc<OpenRouterClient>>) -> Self {
        let config = LoopDetectionConfig::default();
        Self {
            event_history: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_history_size))),
            llm_client,
            config,
            state: Arc::new(Mutex::new(DetectionState::new())),
        }
    }

    /// Add an event and check for loops
    pub async fn add_and_check(&self, event: DetectionEvent) -> Result<LoopDetectionResult> {
        log_debug!("loop_detection", "Adding event: {:?}", event.event_type);

        // Add event to history
        {
            let mut history = self.event_history.lock().await;
            history.push_back(event.clone());
            
            // Trim history if it exceeds max size
            while history.len() > self.config.max_history_size {
                history.pop_front();
            }
        }

        // Check for different types of loops
        let mut detection_result = LoopDetectionResult::new();

        // Tool call loop detection
        if self.config.detect_tool_loops && event.event_type == EventType::ToolCallRequest {
            if let Some(tool_loop) = self.check_tool_call_loop(&event).await? {
                detection_result = tool_loop;
            }
        }

        // Content loop detection
        if self.config.detect_content_loops && event.event_type == EventType::ContentGeneration {
            if let Some(content_loop) = self.check_content_loop(&event).await? {
                detection_result = content_loop;
            }
        }

        // Pattern-based detection
        if self.config.detect_pattern_loops {
            if let Some(pattern_loop) = self.check_pattern_loop().await? {
                detection_result = pattern_loop;
            }
        }

        // LLM-based advanced detection
        if self.config.enable_llm_detection && detection_result.confidence < 0.7 {
            if let Some(llm_loop) = self.check_for_loop_with_llm().await? {
                detection_result = llm_loop;
            }
        }

        // Update detection state
        self.update_detection_state(&detection_result).await;

        Ok(detection_result)
    }

    /// Check for tool call loops (same tool called repeatedly)
    async fn check_tool_call_loop(&self, current_event: &DetectionEvent) -> Result<Option<LoopDetectionResult>> {
        let history = self.event_history.lock().await;
        
        // Look for recent tool call events
        let tool_events: Vec<&DetectionEvent> = history
            .iter()
            .rev()
            .take(10) // Check last 10 events
            .filter(|e| e.event_type == EventType::ToolCallRequest)
            .collect();

        if tool_events.len() < self.config.min_repetitions {
            return Ok(None);
        }

        // Check if the same tool is being called repeatedly
        let current_tool = current_event.metadata.get("tool_name");
        let mut repetition_count = 0;
        let mut similar_events = Vec::new();

        for event in &tool_events {
            if let Some(tool_name) = event.metadata.get("tool_name") {
                if Some(tool_name) == current_tool {
                    repetition_count += 1;
                    similar_events.push((*event).clone());
                }
            }
        }

        if repetition_count >= self.config.min_repetitions {
            log_warn!("loop_detection", "Tool call loop detected: {} repetitions of tool {:?}", 
                     repetition_count, current_tool);

            return Ok(Some(LoopDetectionResult {
                loop_detected: true,
                loop_type: Some(LoopType::ToolCallLoop),
                confidence: (repetition_count as f32 / 10.0).min(1.0),
                description: format!("Tool '{}' called {} times in succession", 
                                   current_tool.unwrap_or(&"unknown".to_string()), repetition_count),
                suggested_actions: vec![
                    LoopBreakAction::ChangeStrategy,
                    LoopBreakAction::RequestUserInput,
                    LoopBreakAction::EscalateReasoning,
                ],
                loop_pattern: similar_events,
            }));
        }

        Ok(None)
    }

    /// Check for content loops (repetitive text generation)
    async fn check_content_loop(&self, current_event: &DetectionEvent) -> Result<Option<LoopDetectionResult>> {
        let history = self.event_history.lock().await;
        
        // Look for recent content generation events
        let content_events: Vec<&DetectionEvent> = history
            .iter()
            .rev()
            .take(5) // Check last 5 content events
            .filter(|e| e.event_type == EventType::ContentGeneration)
            .collect();

        if content_events.len() < self.config.min_repetitions {
            return Ok(None);
        }

        // Check content similarity
        let current_content = &current_event.content;
        let mut high_similarity_count = 0;
        let mut similar_events = Vec::new();

        for event in &content_events {
            let similarity = self.calculate_content_similarity(current_content, &event.content);
            if similarity > self.config.similarity_threshold {
                high_similarity_count += 1;
                similar_events.push((*event).clone());
            }
        }

        if high_similarity_count >= self.config.min_repetitions {
            log_warn!("loop_detection", "Content loop detected: {} similar content generations", 
                     high_similarity_count);

            return Ok(Some(LoopDetectionResult {
                loop_detected: true,
                loop_type: Some(LoopType::ContentLoop),
                confidence: (high_similarity_count as f32 / content_events.len() as f32).min(1.0),
                description: format!("Generated {} similar content responses", high_similarity_count),
                suggested_actions: vec![
                    LoopBreakAction::ClearContext,
                    LoopBreakAction::ChangeStrategy,
                    LoopBreakAction::RequestUserInput,
                ],
                loop_pattern: similar_events,
            }));
        }

        Ok(None)
    }

    /// Check for general behavioral patterns that indicate loops
    async fn check_pattern_loop(&self) -> Result<Option<LoopDetectionResult>> {
        let history = self.event_history.lock().await;
        
        if history.len() < 6 {
            return Ok(None);
        }

        // Look for alternating patterns (A-B-A-B-A-B)
        let recent_events: Vec<&DetectionEvent> = history.iter().rev().take(6).collect();
        
        // Check for A-B-A-B pattern
        if recent_events.len() >= 4 {
            let is_alternating = (0..recent_events.len() - 2).all(|i| {
                self.events_similar(recent_events[i], recent_events[i + 2])
            });

            if is_alternating {
                log_warn!("loop_detection", "Alternating pattern loop detected");
                
                return Ok(Some(LoopDetectionResult {
                    loop_detected: true,
                    loop_type: Some(LoopType::PatternLoop),
                    confidence: 0.8,
                    description: "Detected alternating behavior pattern".to_string(),
                    suggested_actions: vec![
                        LoopBreakAction::EscalateReasoning,
                        LoopBreakAction::ChangeStrategy,
                    ],
                    loop_pattern: recent_events.into_iter().cloned().collect(),
                }));
            }
        }

        Ok(None)
    }

    /// Use LLM to detect sophisticated loop patterns
    async fn check_for_loop_with_llm(&self) -> Result<Option<LoopDetectionResult>> {
        let Some(ref client) = self.llm_client else {
            return Ok(None);
        };

        let history = self.event_history.lock().await;
        
        if history.len() < 5 {
            return Ok(None);
        }

        // Prepare conversation history for analysis
        let recent_events: Vec<String> = history
            .iter()
            .rev()
            .take(10)
            .map(|e| format!("{:?}: {}", e.event_type, e.content.chars().take(100).collect::<String>()))
            .collect();

        let analysis_prompt = format!(
            "You are a sophisticated AI diagnostic agent specializing in identifying when a conversational AI is stuck in an unproductive state. 

Analyze the following sequence of recent AI actions and determine if the AI appears to be in a loop or unproductive pattern:

{}

Respond with a JSON object in this format:
{{
    \"loop_detected\": boolean,
    \"confidence\": number (0.0 to 1.0),
    \"description\": \"description of the pattern\",
    \"pattern_type\": \"repetitive_actions|circular_reasoning|error_cycle|other\"
}}

Look for:
1. Repetitive actions that aren't making progress
2. Circular reasoning patterns
3. Error-retry cycles without learning
4. Stuck behavior that suggests the AI needs intervention

Be conservative - only detect loops when you're confident the AI is truly stuck.",
            recent_events.join("\n")
        );

        log_debug!("loop_detection", "Requesting LLM loop analysis");

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: analysis_prompt,
        }];

        match client.chat_completion(messages).await {
            Ok(response) => {
                if let Ok(analysis) = self.parse_llm_loop_analysis(&response) {
                    if analysis.loop_detected && analysis.confidence > 0.7 {
                        log_warn!("loop_detection", "LLM detected loop: {} (confidence: {})", 
                                 analysis.description, analysis.confidence);

                        return Ok(Some(LoopDetectionResult {
                            loop_detected: true,
                            loop_type: Some(LoopType::LLMDetectedLoop),
                            confidence: analysis.confidence,
                            description: analysis.description,
                            suggested_actions: vec![
                                LoopBreakAction::ClearContext,
                                LoopBreakAction::EscalateReasoning,
                                LoopBreakAction::RequestUserInput,
                            ],
                            loop_pattern: history.iter().rev().take(5).cloned().collect(),
                        }));
                    }
                }
            }
            Err(e) => {
                log_debug!("loop_detection", "LLM loop analysis failed: {}", e);
            }
        }

        Ok(None)
    }

    /// Parse LLM response for loop analysis
    fn parse_llm_loop_analysis(&self, response: &str) -> Result<LLMLoopAnalysis> {
        // Extract JSON from response (handle markdown wrapping)
        let json_str = if response.contains("```json") {
            response.split("```json").nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
        } else {
            response
        };

        let analysis: serde_json::Value = serde_json::from_str(json_str)?;
        
        Ok(LLMLoopAnalysis {
            loop_detected: analysis["loop_detected"].as_bool().unwrap_or(false),
            confidence: analysis["confidence"].as_f64().unwrap_or(0.0) as f32,
            description: analysis["description"].as_str().unwrap_or("Unknown pattern").to_string(),
            pattern_type: analysis["pattern_type"].as_str().unwrap_or("other").to_string(),
        })
    }

    /// Calculate similarity between two text contents
    fn calculate_content_similarity(&self, content1: &str, content2: &str) -> f32 {
        // Simple word-based similarity (in a full implementation, use more sophisticated algorithms)
        let words1: Vec<&str> = content1.split_whitespace().collect();
        let words2: Vec<&str> = content2.split_whitespace().collect();
        
        let mut common_words = 0;
        let total_words = words1.len().max(words2.len());
        
        for word1 in &words1 {
            if words2.contains(word1) {
                common_words += 1;
            }
        }
        
        if total_words == 0 {
            0.0
        } else {
            common_words as f32 / total_words as f32
        }
    }

    /// Check if two events are similar
    fn events_similar(&self, event1: &DetectionEvent, event2: &DetectionEvent) -> bool {
        event1.event_type == event2.event_type && 
        self.calculate_content_similarity(&event1.content, &event2.content) > self.config.similarity_threshold
    }

    /// Update internal detection state
    async fn update_detection_state(&self, result: &LoopDetectionResult) {
        let mut state = self.state.lock().await;
        
        if result.loop_detected {
            if !state.loop_detected {
                // First detection
                state.detection_time = Some(Instant::now());
                state.iteration_count = 1;
            } else {
                // Ongoing loop
                state.iteration_count += 1;
            }
            
            state.loop_detected = true;
            state.loop_type = result.loop_type.clone();
            state.confidence = result.confidence;
            state.description = result.description.clone();
        } else if state.loop_detected {
            // Reset if no loop detected and we were previously in a loop
            *state = DetectionState::new();
        }
    }

    /// Get current detection state
    pub async fn get_current_state(&self) -> DetectionState {
        self.state.lock().await.clone()
    }

    /// Clear detection history and reset state
    pub async fn reset(&self) {
        let mut history = self.event_history.lock().await;
        let mut state = self.state.lock().await;
        
        history.clear();
        *state = DetectionState::new();
        
        log_info!("loop_detection", "Loop detection service reset");
    }

    /// Get detection statistics
    pub async fn get_statistics(&self) -> LoopDetectionStats {
        let history = self.event_history.lock().await;
        let state = self.state.lock().await;
        
        let event_type_counts = history.iter().fold(HashMap::new(), |mut acc, event| {
            *acc.entry(event.event_type.clone()).or_insert(0) += 1;
            acc
        });

        LoopDetectionStats {
            total_events: history.len(),
            event_type_counts,
            current_loop_detected: state.loop_detected,
            current_confidence: state.confidence,
            config: self.config.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LLMLoopAnalysis {
    loop_detected: bool,
    confidence: f32,
    description: String,
    pattern_type: String,
}

impl DetectionState {
    fn new() -> Self {
        Self {
            loop_detected: false,
            loop_type: None,
            detection_time: None,
            iteration_count: 0,
            confidence: 0.0,
            description: String::new(),
        }
    }
}

impl LoopDetectionResult {
    fn new() -> Self {
        Self {
            loop_detected: false,
            loop_type: None,
            confidence: 0.0,
            description: String::new(),
            suggested_actions: Vec::new(),
            loop_pattern: Vec::new(),
        }
    }
}

impl LoopDetectionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn permissive() -> Self {
        Self {
            max_history_size: 50,
            detection_window: Duration::from_secs(300), // 5 minutes
            min_repetitions: 5,
            enable_llm_detection: true,
            similarity_threshold: 0.8,
            detect_tool_loops: true,
            detect_content_loops: true,
            detect_pattern_loops: false, // Less aggressive
        }
    }

    pub fn strict() -> Self {
        Self {
            max_history_size: 20,
            detection_window: Duration::from_secs(60), // 1 minute
            min_repetitions: 2,
            enable_llm_detection: true,
            similarity_threshold: 0.6,
            detect_tool_loops: true,
            detect_content_loops: true,
            detect_pattern_loops: true,
        }
    }
}

impl Default for LoopDetectionConfig {
    fn default() -> Self {
        Self {
            max_history_size: 30,
            detection_window: Duration::from_secs(120), // 2 minutes
            min_repetitions: 3,
            enable_llm_detection: true,
            similarity_threshold: 0.7,
            detect_tool_loops: true,
            detect_content_loops: true,
            detect_pattern_loops: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoopDetectionStats {
    pub total_events: usize,
    pub event_type_counts: HashMap<EventType, usize>,
    pub current_loop_detected: bool,
    pub current_confidence: f32,
    pub config: LoopDetectionConfig,
}

impl std::fmt::Display for LoopDetectionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Loop Detection: {} events tracked, loop detected: {}, confidence: {:.2}", 
               self.total_events, self.current_loop_detected, self.current_confidence)
    }
}

impl std::fmt::Display for LoopType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopType::ToolCallLoop => write!(f, "Tool Call Loop"),
            LoopType::ContentLoop => write!(f, "Content Loop"),
            LoopType::ErrorLoop => write!(f, "Error Loop"),
            LoopType::LLMDetectedLoop => write!(f, "LLM Detected Loop"),
            LoopType::PatternLoop => write!(f, "Pattern Loop"),
        }
    }
}

/// Global loop detection service singleton
static mut GLOBAL_LOOP_DETECTION: Option<LoopDetectionService> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global loop detection service
pub async fn initialize_loop_detection(llm_client: Option<Arc<OpenRouterClient>>) {
    INIT.call_once(|| {
        let service = LoopDetectionService::with_defaults(llm_client);
        unsafe {
            GLOBAL_LOOP_DETECTION = Some(service);
        }
        log_info!("loop_detection", "Loop detection service initialized");
    });
}

/// Get reference to global loop detection service
pub fn get_loop_detection() -> Option<&'static LoopDetectionService> {
    unsafe { GLOBAL_LOOP_DETECTION.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_call_loop_detection() {
        let service = LoopDetectionService::with_defaults(None);
        
        // Add repeated tool calls
        for i in 0..5 {
            let mut metadata = HashMap::new();
            metadata.insert("tool_name".to_string(), "filesystem".to_string());
            
            let event = DetectionEvent {
                id: format!("event_{}", i),
                event_type: EventType::ToolCallRequest,
                content: "list files".to_string(),
                metadata,
                timestamp: Instant::now(),
                session_id: "test_session".to_string(),
            };
            
            let result = service.add_and_check(event).await.unwrap();
            
            if i >= 2 { // After 3 repetitions, should detect loop
                assert!(result.loop_detected);
                assert_eq!(result.loop_type, Some(LoopType::ToolCallLoop));
            }
        }
    }

    #[tokio::test]
    async fn test_content_similarity() {
        let service = LoopDetectionService::with_defaults(None);
        
        let similarity = service.calculate_content_similarity(
            "Hello world how are you",
            "Hello world how are you doing"
        );
        
        assert!(similarity > 0.5);
    }
}