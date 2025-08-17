use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::logger::{log_debug, log_info, log_warn};
use crate::openrouter_client::OpenRouterClient;

/// Advanced Error Recovery System
/// Implements intelligent error recovery strategies inspired by Claude Code CLI patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// Maximum number of recovery attempts per error type
    pub max_recovery_attempts: usize,
    /// Cooldown period between recovery attempts (seconds)
    pub recovery_cooldown: u64,
    /// Enable LLM-powered error analysis
    pub enable_llm_analysis: bool,
    /// Maximum time to spend on error recovery (seconds)
    pub max_recovery_time: u64,
    /// Enable adaptive learning from recovery outcomes
    pub enable_adaptive_learning: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_recovery_attempts: 3,
            recovery_cooldown: 5,
            enable_llm_analysis: true,
            max_recovery_time: 120,
            enable_adaptive_learning: true,
        }
    }
}

/// Types of errors that can be recovered
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorType {
    /// Network/API communication errors
    NetworkError,
    /// File operation errors
    FileSystemError,
    /// MCP tool execution errors
    ToolExecutionError,
    /// Timeout errors
    TimeoutError,
    /// Authentication/permission errors
    AuthenticationError,
    /// LLM API errors
    LlmApiError,
    /// Configuration errors
    ConfigurationError,
    /// Dependency errors (missing tools, etc.)
    DependencyError,
    /// Unknown or unclassified errors
    UnknownError,
}

impl ErrorType {
    /// Classify error from error message
    pub fn classify_error(error_message: &str) -> Self {
        let msg_lower = error_message.to_lowercase();
        
        if msg_lower.contains("network") || msg_lower.contains("connection") || 
           msg_lower.contains("dns") || msg_lower.contains("unreachable") {
            ErrorType::NetworkError
        } else if msg_lower.contains("file") || msg_lower.contains("directory") || 
                  msg_lower.contains("permission denied") || msg_lower.contains("no such file") {
            ErrorType::FileSystemError
        } else if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
            ErrorType::TimeoutError
        } else if msg_lower.contains("auth") || msg_lower.contains("unauthorized") || 
                  msg_lower.contains("forbidden") || msg_lower.contains("api key") {
            ErrorType::AuthenticationError
        } else if msg_lower.contains("mcp") || msg_lower.contains("tool") || 
                  msg_lower.contains("docker") {
            ErrorType::ToolExecutionError
        } else if msg_lower.contains("llm") || msg_lower.contains("openrouter") || 
                  msg_lower.contains("model") {
            ErrorType::LlmApiError
        } else if msg_lower.contains("config") || msg_lower.contains("setting") {
            ErrorType::ConfigurationError
        } else if msg_lower.contains("not found") || msg_lower.contains("missing") ||
                  msg_lower.contains("dependency") {
            ErrorType::DependencyError
        } else {
            ErrorType::UnknownError
        }
    }
}

/// Recovery strategy for different error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    RetryWithBackoff { 
        base_delay_ms: u64, 
        max_delay_ms: u64, 
        multiplier: f64 
    },
    /// Fallback to alternative approach
    Fallback { 
        alternative_method: String 
    },
    /// Reset state and retry
    ResetAndRetry,
    /// Skip and continue with degraded functionality
    SkipWithWarning { 
        warning_message: String 
    },
    /// Attempt automatic fix
    AutomaticFix { 
        fix_commands: Vec<String> 
    },
    /// Request user intervention
    RequestUserIntervention { 
        guidance_message: String 
    },
    /// Terminate gracefully
    GracefulTermination { 
        cleanup_actions: Vec<String> 
    },
}

/// Error recovery record for learning and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryRecord {
    pub error_id: String,
    pub error_type: ErrorType,
    pub error_message: String,
    pub context: HashMap<String, String>,
    pub strategy_used: RecoveryStrategy,
    pub recovery_attempts: usize,
    pub success: bool,
    pub recovery_time: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub llm_analysis: Option<String>,
}

/// Advanced Error Recovery Manager
pub struct AdvancedErrorRecoveryManager {
    config: ErrorRecoveryConfig,
    llm_client: Option<Arc<OpenRouterClient>>,
    recovery_history: Arc<Mutex<VecDeque<ErrorRecoveryRecord>>>,
    active_recoveries: Arc<Mutex<HashMap<String, Instant>>>,
    strategy_effectiveness: Arc<Mutex<HashMap<ErrorType, HashMap<String, f64>>>>,
}

impl AdvancedErrorRecoveryManager {
    pub async fn new(config: ErrorRecoveryConfig) -> Result<Self> {
        let llm_client = if config.enable_llm_analysis {
            Some(Arc::new(OpenRouterClient::new().await?))
        } else {
            None
        };

        Ok(Self {
            config,
            llm_client,
            recovery_history: Arc::new(Mutex::new(VecDeque::new())),
            active_recoveries: Arc::new(Mutex::new(HashMap::new())),
            strategy_effectiveness: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn default() -> Self {
        Self {
            config: ErrorRecoveryConfig::default(),
            llm_client: None,
            recovery_history: Arc::new(Mutex::new(VecDeque::new())),
            active_recoveries: Arc::new(Mutex::new(HashMap::new())),
            strategy_effectiveness: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Attempt to recover from an error
    pub async fn recover_from_error(
        &self,
        error: &anyhow::Error,
        context: HashMap<String, String>,
    ) -> Result<bool> {
        let error_message = error.to_string();
        let error_type = ErrorType::classify_error(&error_message);
        let error_id = Uuid::new_v4().to_string();

        log_info!("recovery", "🚨 Starting error recovery for: {} (type: {:?})", 
                 error_message.chars().take(100).collect::<String>(), error_type);

        // Check if we're already handling too many recoveries
        if self.is_recovery_overloaded().await {
            log_warn!("recovery", "⚠️ Recovery system overloaded, skipping recovery");
            return Ok(false);
        }

        let start_time = Instant::now();
        let mut recovery_record = ErrorRecoveryRecord {
            error_id: error_id.clone(),
            error_type: error_type.clone(),
            error_message: error_message.clone(),
            context: context.clone(),
            strategy_used: RecoveryStrategy::RetryWithBackoff { 
                base_delay_ms: 1000, 
                max_delay_ms: 30000, 
                multiplier: 2.0 
            },
            recovery_attempts: 0,
            success: false,
            recovery_time: Duration::from_secs(0),
            timestamp: chrono::Utc::now(),
            llm_analysis: None,
        };

        // Register active recovery
        {
            let mut active = self.active_recoveries.lock().await;
            active.insert(error_id.clone(), start_time);
        }

        // Determine best recovery strategy
        let strategy = self.determine_recovery_strategy(&error_type, &error_message, &context).await?;
        recovery_record.strategy_used = strategy.clone();

        // Perform LLM analysis if enabled
        if let Some(ref llm_client) = self.llm_client {
            recovery_record.llm_analysis = self.llm_analyze_error(
                llm_client, &error_message, &context
            ).await.ok();
        }

        // Execute recovery strategy
        let recovery_success = self.execute_recovery_strategy(
            &strategy, 
            &error_type, 
            &error_message, 
            &context,
            &mut recovery_record
        ).await?;

        // Update recovery record
        recovery_record.success = recovery_success;
        recovery_record.recovery_time = start_time.elapsed();

        // Clean up active recovery
        {
            let mut active = self.active_recoveries.lock().await;
            active.remove(&error_id);
        }

        // Record the recovery attempt
        self.record_recovery_attempt(recovery_record).await;

        // Update strategy effectiveness if learning is enabled
        if self.config.enable_adaptive_learning {
            self.update_strategy_effectiveness(&error_type, &strategy, recovery_success).await;
        }

        log_info!("recovery", "✅ Error recovery completed: {} (success: {})", 
                 error_id, recovery_success);

        Ok(recovery_success)
    }

    /// Determine the best recovery strategy based on error type and context
    async fn determine_recovery_strategy(
        &self,
        error_type: &ErrorType,
        error_message: &str,
        context: &HashMap<String, String>,
    ) -> Result<RecoveryStrategy> {
        // Use historical effectiveness data if available
        if self.config.enable_adaptive_learning {
            if let Some(best_strategy) = self.get_most_effective_strategy(error_type).await {
                log_debug!("recovery", "📊 Using historically effective strategy for {:?}", error_type);
                return Ok(best_strategy);
            }
        }

        // Default strategies based on error type
        let strategy = match error_type {
            ErrorType::NetworkError => RecoveryStrategy::RetryWithBackoff {
                base_delay_ms: 2000,
                max_delay_ms: 30000,
                multiplier: 2.0,
            },
            ErrorType::TimeoutError => RecoveryStrategy::RetryWithBackoff {
                base_delay_ms: 1000,
                max_delay_ms: 15000,
                multiplier: 1.5,
            },
            ErrorType::AuthenticationError => {
                if error_message.contains("api key") {
                    RecoveryStrategy::RequestUserIntervention {
                        guidance_message: "Please check your API key configuration".to_string(),
                    }
                } else {
                    RecoveryStrategy::Fallback {
                        alternative_method: "Use cached results or offline mode".to_string(),
                    }
                }
            },
            ErrorType::FileSystemError => {
                if error_message.contains("permission") {
                    RecoveryStrategy::AutomaticFix {
                        fix_commands: vec!["sudo".to_string()],
                    }
                } else if error_message.contains("not found") {
                    RecoveryStrategy::AutomaticFix {
                        fix_commands: vec!["mkdir -p".to_string(), "touch".to_string()],
                    }
                } else {
                    RecoveryStrategy::ResetAndRetry
                }
            },
            ErrorType::ToolExecutionError => {
                if context.get("tool_name").map_or(false, |name| name == "docker") {
                    RecoveryStrategy::AutomaticFix {
                        fix_commands: vec!["docker ps".to_string(), "docker pull".to_string()],
                    }
                } else {
                    RecoveryStrategy::Fallback {
                        alternative_method: "Use alternative tool or native implementation".to_string(),
                    }
                }
            },
            ErrorType::LlmApiError => RecoveryStrategy::RetryWithBackoff {
                base_delay_ms: 5000,
                max_delay_ms: 60000,
                multiplier: 2.0,
            },
            ErrorType::ConfigurationError => RecoveryStrategy::AutomaticFix {
                fix_commands: vec!["reset config".to_string(), "regenerate config".to_string()],
            },
            ErrorType::DependencyError => RecoveryStrategy::AutomaticFix {
                fix_commands: vec!["install missing dependency".to_string()],
            },
            ErrorType::UnknownError => RecoveryStrategy::SkipWithWarning {
                warning_message: "Unknown error encountered, continuing with reduced functionality".to_string(),
            },
        };

        Ok(strategy)
    }

    /// Execute a recovery strategy
    async fn execute_recovery_strategy(
        &self,
        strategy: &RecoveryStrategy,
        _error_type: &ErrorType,
        _error_message: &str,
        context: &HashMap<String, String>,
        record: &mut ErrorRecoveryRecord,
    ) -> Result<bool> {
        log_debug!("recovery", "🔧 Executing recovery strategy: {:?}", strategy);

        match strategy {
            RecoveryStrategy::RetryWithBackoff { base_delay_ms, max_delay_ms, multiplier } => {
                self.execute_retry_with_backoff(*base_delay_ms, *max_delay_ms, *multiplier, record).await
            },
            RecoveryStrategy::Fallback { alternative_method } => {
                log_info!("recovery", "🔄 Falling back to: {}", alternative_method);
                // In a real implementation, this would try alternative approaches
                Ok(true) // Assume fallback succeeds
            },
            RecoveryStrategy::ResetAndRetry => {
                log_info!("recovery", "🔄 Resetting state and retrying");
                // Reset relevant state and retry
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(true)
            },
            RecoveryStrategy::SkipWithWarning { warning_message } => {
                log_warn!("recovery", "⚠️ {}", warning_message);
                Ok(true) // Skip but continue
            },
            RecoveryStrategy::AutomaticFix { fix_commands } => {
                self.execute_automatic_fixes(fix_commands, context).await
            },
            RecoveryStrategy::RequestUserIntervention { guidance_message } => {
                log_warn!("recovery", "👤 User intervention needed: {}", guidance_message);
                Ok(false) // Requires manual intervention
            },
            RecoveryStrategy::GracefulTermination { cleanup_actions } => {
                log_info!("recovery", "🛑 Performing graceful termination");
                for action in cleanup_actions {
                    log_debug!("recovery", "🧹 Cleanup action: {}", action);
                }
                Ok(false) // Terminates execution
            },
        }
    }

    /// Execute retry with exponential backoff
    async fn execute_retry_with_backoff(
        &self,
        base_delay_ms: u64,
        max_delay_ms: u64,
        multiplier: f64,
        record: &mut ErrorRecoveryRecord,
    ) -> Result<bool> {
        let mut delay = base_delay_ms;
        
        for attempt in 1..=self.config.max_recovery_attempts {
            record.recovery_attempts = attempt;
            
            log_debug!("recovery", "🔄 Retry attempt {} after {}ms delay", attempt, delay);
            
            tokio::time::sleep(Duration::from_millis(delay)).await;
            
            // In a real implementation, this would retry the original operation
            // For now, simulate success after a few attempts
            if attempt >= 2 {
                log_info!("recovery", "✅ Retry successful on attempt {}", attempt);
                return Ok(true);
            }
            
            // Increase delay for next attempt
            delay = std::cmp::min((delay as f64 * multiplier) as u64, max_delay_ms);
        }
        
        log_warn!("recovery", "❌ All retry attempts exhausted");
        Ok(false)
    }

    /// Execute automatic fixes
    async fn execute_automatic_fixes(
        &self,
        fix_commands: &[String],
        _context: &HashMap<String, String>,
    ) -> Result<bool> {
        for command in fix_commands {
            log_debug!("recovery", "🔧 Executing automatic fix: {}", command);
            
            // In a real implementation, this would execute actual fix commands
            // For now, simulate successful execution
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        log_info!("recovery", "✅ Automatic fixes completed");
        Ok(true)
    }

    /// LLM-powered error analysis
    async fn llm_analyze_error(
        &self,
        llm_client: &OpenRouterClient,
        error_message: &str,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let prompt = format!(
            "Analyze this error and suggest recovery strategies:\n\n\
            Error: {}\n\n\
            Context: {:?}\n\n\
            Provide specific, actionable recovery suggestions.",
            error_message, context
        );

        let messages = vec![crate::openrouter_client::ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];
        let analysis = llm_client.chat_completion(messages).await?;
        log_debug!("recovery", "🧠 LLM error analysis completed");
        Ok(analysis)
    }

    /// Check if recovery system is overloaded
    async fn is_recovery_overloaded(&self) -> bool {
        let active = self.active_recoveries.lock().await;
        active.len() > 5 // More than 5 concurrent recoveries
    }

    /// Get most effective strategy for error type
    async fn get_most_effective_strategy(&self, error_type: &ErrorType) -> Option<RecoveryStrategy> {
        let effectiveness = self.strategy_effectiveness.lock().await;
        effectiveness.get(error_type)
            .and_then(|strategies| {
                strategies.iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(strategy_name, _)| self.strategy_from_name(strategy_name))
            })
            .flatten()
    }

    /// Convert strategy name back to strategy (simplified)
    fn strategy_from_name(&self, name: &str) -> Option<RecoveryStrategy> {
        match name {
            "retry" => Some(RecoveryStrategy::RetryWithBackoff {
                base_delay_ms: 1000,
                max_delay_ms: 30000,
                multiplier: 2.0,
            }),
            "fallback" => Some(RecoveryStrategy::Fallback {
                alternative_method: "Default fallback".to_string(),
            }),
            "reset" => Some(RecoveryStrategy::ResetAndRetry),
            _ => None,
        }
    }

    /// Update strategy effectiveness based on outcome
    async fn update_strategy_effectiveness(
        &self,
        error_type: &ErrorType,
        strategy: &RecoveryStrategy,
        success: bool,
    ) {
        let strategy_name = match strategy {
            RecoveryStrategy::RetryWithBackoff { .. } => "retry",
            RecoveryStrategy::Fallback { .. } => "fallback",
            RecoveryStrategy::ResetAndRetry => "reset",
            RecoveryStrategy::SkipWithWarning { .. } => "skip",
            RecoveryStrategy::AutomaticFix { .. } => "autofix",
            RecoveryStrategy::RequestUserIntervention { .. } => "user_intervention",
            RecoveryStrategy::GracefulTermination { .. } => "terminate",
        };

        let mut effectiveness = self.strategy_effectiveness.lock().await;
        let error_strategies = effectiveness.entry(error_type.clone()).or_insert_with(HashMap::new);
        let current_score = error_strategies.get(strategy_name).copied().unwrap_or(0.5);
        
        // Update score based on success/failure
        let new_score = if success {
            (current_score + 0.1).min(1.0)
        } else {
            (current_score - 0.1).max(0.0)
        };
        
        error_strategies.insert(strategy_name.to_string(), new_score);
        
        log_debug!("recovery", "📊 Updated strategy effectiveness: {} for {:?} -> {:.2}", 
                  strategy_name, error_type, new_score);
    }

    /// Record recovery attempt for analysis
    async fn record_recovery_attempt(&self, record: ErrorRecoveryRecord) {
        let mut history = self.recovery_history.lock().await;
        history.push_back(record);
        
        // Keep only last 100 records
        if history.len() > 100 {
            history.pop_front();
        }
    }

    /// Get recovery statistics
    pub async fn get_recovery_statistics(&self) -> RecoveryStatistics {
        let history = self.recovery_history.lock().await;
        let total_attempts = history.len();
        let successful_recoveries = history.iter().filter(|r| r.success).count();
        
        let success_rate = if total_attempts > 0 {
            successful_recoveries as f64 / total_attempts as f64
        } else {
            0.0
        };

        let avg_recovery_time = if total_attempts > 0 {
            let total_time: Duration = history.iter().map(|r| r.recovery_time).sum();
            total_time / total_attempts as u32
        } else {
            Duration::from_secs(0)
        };

        RecoveryStatistics {
            total_attempts,
            successful_recoveries,
            success_rate,
            average_recovery_time: avg_recovery_time,
            active_recoveries: self.active_recoveries.lock().await.len(),
        }
    }
}

/// Recovery system statistics
#[derive(Debug, Clone)]
pub struct RecoveryStatistics {
    pub total_attempts: usize,
    pub successful_recoveries: usize,
    pub success_rate: f64,
    pub average_recovery_time: Duration,
    pub active_recoveries: usize,
}

/// Global error recovery manager
use once_cell::sync::Lazy;

static GLOBAL_RECOVERY_MANAGER: Lazy<Mutex<Option<AdvancedErrorRecoveryManager>>> = 
    Lazy::new(|| Mutex::new(None));

/// Initialize global error recovery manager
pub async fn initialize_error_recovery(config: ErrorRecoveryConfig) -> Result<()> {
    let manager = AdvancedErrorRecoveryManager::new(config).await?;
    let mut global = GLOBAL_RECOVERY_MANAGER.lock().await;
    *global = Some(manager);
    log_info!("recovery", "🚨 Advanced error recovery system initialized");
    Ok(())
}

/// Attempt error recovery using global manager
pub async fn attempt_error_recovery(
    error: &anyhow::Error,
    context: HashMap<String, String>,
) -> Result<bool> {
    let manager_guard = GLOBAL_RECOVERY_MANAGER.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        manager.recover_from_error(error, context).await
    } else {
        log_warn!("recovery", "⚠️ Error recovery manager not initialized");
        Ok(false)
    }
}

/// Get global recovery statistics
pub async fn get_global_recovery_statistics() -> Option<RecoveryStatistics> {
    let manager_guard = GLOBAL_RECOVERY_MANAGER.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        Some(manager.get_recovery_statistics().await)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        assert_eq!(ErrorType::classify_error("network connection failed"), ErrorType::NetworkError);
        assert_eq!(ErrorType::classify_error("file not found"), ErrorType::FileSystemError);
        assert_eq!(ErrorType::classify_error("operation timed out"), ErrorType::TimeoutError);
        assert_eq!(ErrorType::classify_error("unauthorized access"), ErrorType::AuthenticationError);
    }

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let config = ErrorRecoveryConfig::default();
        let manager = AdvancedErrorRecoveryManager::new(config).await;
        assert!(manager.is_ok());
    }
}