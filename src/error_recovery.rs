use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime};
use tokio::time;
use uuid::Uuid;

/// Enhanced error recovery manager with multiple recovery strategies
#[derive(Debug)]
pub struct ErrorRecoveryManager {
    recovery_strategies: HashMap<ErrorType, RecoveryStrategy>,
    error_history: Vec<ErrorContext>,
    fallback_enabled: bool,
    max_history_size: usize,
    global_retry_budget: u32,
    used_retry_budget: u32,
}

/// Classification of error types for appropriate recovery strategies
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorType {
    // Network/API errors
    NetworkTimeout,
    ApiRateLimited,
    ApiQuotaExceeded,
    ConnectionFailed,
    AuthenticationFailed,
    
    // File system errors
    FileNotFound,
    PermissionDenied,
    FileCorrupted,
    DiskFull,
    FileInUse,
    
    // Application errors
    ConfigurationError,
    ValidationError,
    ParseError,
    StateCorruption,
    ResourceExhausted,
    
    // MCP/Tool errors
    ToolNotFound,
    ToolExecutionFailed,
    ToolTimeout,
    MCPConnectionLost,
    
    // LLM/AI errors
    LLMResponseInvalid,
    LLMContextTooLong,
    LLMServiceUnavailable,
    
    // System errors
    OutOfMemory,
    SystemOverloaded,
    DependencyMissing,
    
    // Unknown/Generic
    Unknown,
}

/// Recovery strategy definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry { 
        max_attempts: u32, 
        backoff: BackoffStrategy,
        conditions: Vec<RetryCondition>,
    },
    Fallback { 
        alternative_approach: FallbackApproach,
        description: String,
    },
    UserIntervention { 
        prompt: String, 
        suggestions: Vec<String>,
        timeout: Option<Duration>,
    },
    GracefulDegrade { 
        reduced_functionality: String,
        impact_description: String,
    },
    Escalate {
        escalation_path: EscalationPath,
        context_required: Vec<String>,
    },
}

/// Backoff strategies for retry operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed(Duration),
    Linear(Duration),
    Exponential { base: Duration, multiplier: f64, max: Duration },
    Jittered { base: Duration, jitter_percent: f64 },
}

/// Conditions under which retries should be attempted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    Always,
    IfTransient,
    IfNetworkError,
    IfResourcesAvailable,
    IfWithinTimeLimit(Duration),
    IfRetryBudgetAvailable,
}

/// Fallback approaches when primary method fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackApproach {
    AlternativeMethod(String),
    OfflineMode,
    CachedResponse,
    ManualOperation,
    SkipOperation,
    ReducedQuality,
}

/// Escalation paths for complex errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationPath {
    LogOnly,
    NotifyUser,
    SaveStateAndExit,
    RequestSupport,
    RestartService,
}

/// Error context for tracking and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub error_id: String,
    pub error_type: ErrorType,
    pub timestamp: SystemTime,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub operation_context: OperationContext,
    pub recovery_attempts: Vec<RecoveryAttempt>,
    pub final_outcome: Option<RecoveryOutcome>,
}

/// Context of the operation that failed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContext {
    pub operation_id: String,
    pub operation_type: String,
    pub session_id: String,
    pub user_request: String,
    pub system_state: HashMap<String, String>,
    pub resources_in_use: Vec<String>,
}

/// Record of a recovery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub attempt_id: String,
    pub strategy: RecoveryStrategy,
    pub timestamp: SystemTime,
    pub duration: Duration,
    pub success: bool,
    pub error_if_failed: Option<String>,
    pub changes_made: Vec<String>,
}

/// Final outcome of error recovery process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryOutcome {
    FullRecovery,
    PartialRecovery { limitations: Vec<String> },
    GracefulDegradation { reduced_functionality: String },
    ManualInterventionRequired { instructions: String },
    OperationAborted { reason: String },
}

/// Action to take after error recovery analysis
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    Retry { delay: Duration, modified_params: Option<HashMap<String, String>> },
    Fallback { approach: FallbackApproach },
    RequestUserInput { prompt: String, options: Vec<String> },
    Escalate { path: EscalationPath, context: ErrorContext },
    Abort { reason: String },
    Continue { warnings: Vec<String> },
}

impl ErrorRecoveryManager {
    /// Create a new error recovery manager with default strategies
    pub fn new() -> Self {
        let mut manager = Self {
            recovery_strategies: HashMap::new(),
            error_history: Vec::new(),
            fallback_enabled: true,
            max_history_size: 1000,
            global_retry_budget: 50,
            used_retry_budget: 0,
        };
        
        manager.initialize_default_strategies();
        manager
    }

    /// Initialize default recovery strategies for common error types
    fn initialize_default_strategies(&mut self) {
        // Network/API errors
        self.recovery_strategies.insert(
            ErrorType::NetworkTimeout,
            RecoveryStrategy::Retry {
                max_attempts: 3,
                backoff: BackoffStrategy::Exponential {
                    base: Duration::from_secs(1),
                    multiplier: 2.0,
                    max: Duration::from_secs(30),
                },
                conditions: vec![RetryCondition::IfNetworkError, RetryCondition::IfRetryBudgetAvailable],
            },
        );

        self.recovery_strategies.insert(
            ErrorType::ApiRateLimited,
            RecoveryStrategy::Retry {
                max_attempts: 5,
                backoff: BackoffStrategy::Exponential {
                    base: Duration::from_secs(5),
                    multiplier: 1.5,
                    max: Duration::from_secs(300),
                },
                conditions: vec![RetryCondition::IfWithinTimeLimit(Duration::from_secs(600))],
            },
        );

        self.recovery_strategies.insert(
            ErrorType::ApiQuotaExceeded,
            RecoveryStrategy::Fallback {
                alternative_approach: FallbackApproach::OfflineMode,
                description: "Switch to offline mode due to API quota exceeded".to_string(),
            },
        );

        // File system errors
        self.recovery_strategies.insert(
            ErrorType::FileNotFound,
            RecoveryStrategy::UserIntervention {
                prompt: "File not found. Please specify the correct file path or create the missing file.".to_string(),
                suggestions: vec![
                    "Check if the file path is correct".to_string(),
                    "Create the missing file".to_string(),
                    "Use a different file".to_string(),
                ],
                timeout: Some(Duration::from_secs(300)),
            },
        );

        self.recovery_strategies.insert(
            ErrorType::PermissionDenied,
            RecoveryStrategy::UserIntervention {
                prompt: "Permission denied. Please grant the necessary permissions or run with elevated privileges.".to_string(),
                suggestions: vec![
                    "Change file permissions".to_string(),
                    "Run with sudo/administrator privileges".to_string(),
                    "Use a different file location".to_string(),
                ],
                timeout: Some(Duration::from_secs(300)),
            },
        );

        self.recovery_strategies.insert(
            ErrorType::DiskFull,
            RecoveryStrategy::GracefulDegrade {
                reduced_functionality: "Operating in read-only mode due to insufficient disk space".to_string(),
                impact_description: "Cannot write new files or modify existing ones".to_string(),
            },
        );

        // Application errors
        self.recovery_strategies.insert(
            ErrorType::ConfigurationError,
            RecoveryStrategy::Fallback {
                alternative_approach: FallbackApproach::ManualOperation,
                description: "Use default configuration or manual setup".to_string(),
            },
        );

        // MCP/Tool errors
        self.recovery_strategies.insert(
            ErrorType::ToolExecutionFailed,
            RecoveryStrategy::Retry {
                max_attempts: 2,
                backoff: BackoffStrategy::Fixed(Duration::from_secs(2)),
                conditions: vec![RetryCondition::IfTransient],
            },
        );

        self.recovery_strategies.insert(
            ErrorType::MCPConnectionLost,
            RecoveryStrategy::Fallback {
                alternative_approach: FallbackApproach::AlternativeMethod("Use built-in tools instead of MCP".to_string()),
                description: "MCP connection lost, falling back to built-in functionality".to_string(),
            },
        );

        // LLM errors
        self.recovery_strategies.insert(
            ErrorType::LLMContextTooLong,
            RecoveryStrategy::Fallback {
                alternative_approach: FallbackApproach::ReducedQuality,
                description: "Reduce context size and retry with simplified prompt".to_string(),
            },
        );

        // System errors
        self.recovery_strategies.insert(
            ErrorType::OutOfMemory,
            RecoveryStrategy::GracefulDegrade {
                reduced_functionality: "Operating with reduced memory usage".to_string(),
                impact_description: "Some features may be disabled to conserve memory".to_string(),
            },
        );

        // Default strategy for unknown errors
        self.recovery_strategies.insert(
            ErrorType::Unknown,
            RecoveryStrategy::Escalate {
                escalation_path: EscalationPath::LogOnly,
                context_required: vec!["error_message".to_string(), "stack_trace".to_string()],
            },
        );
    }

    /// Handle an error and determine recovery action
    pub async fn handle_error(
        &mut self,
        error: &anyhow::Error,
        operation_context: OperationContext,
    ) -> Result<RecoveryAction> {
        let error_type = self.classify_error(error);
        let error_context = self.create_error_context(error, error_type.clone(), operation_context);
        
        // Add to history
        self.add_to_history(error_context.clone());
        
        // Get recovery strategy
        let strategy = self.get_recovery_strategy(&error_type);
        
        // Execute recovery strategy
        self.execute_recovery_strategy(strategy, &error_context).await
    }

    /// Classify an error into a specific error type
    fn classify_error(&self, error: &anyhow::Error) -> ErrorType {
        let error_str = error.to_string().to_lowercase();
        
        // Network/API classification
        if error_str.contains("timeout") || error_str.contains("timed out") {
            return ErrorType::NetworkTimeout;
        }
        if error_str.contains("rate limit") || error_str.contains("too many requests") {
            return ErrorType::ApiRateLimited;
        }
        if error_str.contains("quota") || error_str.contains("limit exceeded") {
            return ErrorType::ApiQuotaExceeded;
        }
        if error_str.contains("connection") && (error_str.contains("failed") || error_str.contains("refused")) {
            return ErrorType::ConnectionFailed;
        }
        if error_str.contains("unauthorized") || error_str.contains("authentication") {
            return ErrorType::AuthenticationFailed;
        }
        
        // File system classification
        if error_str.contains("no such file") || error_str.contains("file not found") {
            return ErrorType::FileNotFound;
        }
        if error_str.contains("permission denied") || error_str.contains("access denied") {
            return ErrorType::PermissionDenied;
        }
        if error_str.contains("disk full") || error_str.contains("no space left") {
            return ErrorType::DiskFull;
        }
        if error_str.contains("file in use") || error_str.contains("resource busy") {
            return ErrorType::FileInUse;
        }
        
        // Application classification
        if error_str.contains("config") || error_str.contains("configuration") {
            return ErrorType::ConfigurationError;
        }
        if error_str.contains("validation") || error_str.contains("invalid") {
            return ErrorType::ValidationError;
        }
        if error_str.contains("parse") || error_str.contains("parsing") {
            return ErrorType::ParseError;
        }
        
        // MCP/Tool classification
        if error_str.contains("mcp") && error_str.contains("connection") {
            return ErrorType::MCPConnectionLost;
        }
        if error_str.contains("tool") && (error_str.contains("failed") || error_str.contains("error")) {
            return ErrorType::ToolExecutionFailed;
        }
        if error_str.contains("tool") && error_str.contains("not found") {
            return ErrorType::ToolNotFound;
        }
        
        // LLM classification
        if error_str.contains("context") && (error_str.contains("too long") || error_str.contains("limit")) {
            return ErrorType::LLMContextTooLong;
        }
        if error_str.contains("llm") || error_str.contains("language model") {
            return ErrorType::LLMResponseInvalid;
        }
        
        // System classification
        if error_str.contains("out of memory") || error_str.contains("oom") {
            return ErrorType::OutOfMemory;
        }
        
        ErrorType::Unknown
    }

    /// Create error context from error and operation details
    fn create_error_context(
        &self,
        error: &anyhow::Error,
        error_type: ErrorType,
        operation_context: OperationContext,
    ) -> ErrorContext {
        ErrorContext {
            error_id: Uuid::new_v4().to_string(),
            error_type,
            timestamp: SystemTime::now(),
            error_message: error.to_string(),
            stack_trace: Some(format!("{:?}", error)),
            operation_context,
            recovery_attempts: Vec::new(),
            final_outcome: None,
        }
    }

    /// Get recovery strategy for an error type
    fn get_recovery_strategy(&self, error_type: &ErrorType) -> RecoveryStrategy {
        self.recovery_strategies
            .get(error_type)
            .cloned()
            .unwrap_or_else(|| {
                self.recovery_strategies
                    .get(&ErrorType::Unknown)
                    .cloned()
                    .unwrap_or(RecoveryStrategy::Escalate {
                        escalation_path: EscalationPath::LogOnly,
                        context_required: vec!["error_message".to_string()],
                    })
            })
    }

    /// Execute a recovery strategy
    async fn execute_recovery_strategy(
        &mut self,
        strategy: RecoveryStrategy,
        error_context: &ErrorContext,
    ) -> Result<RecoveryAction> {
        match strategy {
            RecoveryStrategy::Retry { max_attempts, backoff, conditions } => {
                if self.should_retry(error_context, max_attempts, &conditions)? {
                    let delay = self.calculate_backoff(&backoff, self.get_retry_count(error_context));
                    self.used_retry_budget += 1;
                    Ok(RecoveryAction::Retry {
                        delay,
                        modified_params: None,
                    })
                } else {
                    Ok(RecoveryAction::Escalate {
                        path: EscalationPath::LogOnly,
                        context: error_context.clone(),
                    })
                }
            }
            RecoveryStrategy::Fallback { alternative_approach, description } => {
                Ok(RecoveryAction::Fallback { approach: alternative_approach })
            }
            RecoveryStrategy::UserIntervention { prompt, suggestions, .. } => {
                Ok(RecoveryAction::RequestUserInput { prompt, options: suggestions })
            }
            RecoveryStrategy::GracefulDegrade { reduced_functionality, .. } => {
                Ok(RecoveryAction::Continue {
                    warnings: vec![format!("Operating with reduced functionality: {}", reduced_functionality)],
                })
            }
            RecoveryStrategy::Escalate { escalation_path, .. } => {
                Ok(RecoveryAction::Escalate {
                    path: escalation_path,
                    context: error_context.clone(),
                })
            }
        }
    }

    /// Check if retry should be attempted
    fn should_retry(
        &self,
        error_context: &ErrorContext,
        max_attempts: u32,
        conditions: &[RetryCondition],
    ) -> Result<bool> {
        let retry_count = self.get_retry_count(error_context);
        
        if retry_count >= max_attempts {
            return Ok(false);
        }
        
        for condition in conditions {
            if !self.check_retry_condition(condition, error_context)? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Check a specific retry condition
    fn check_retry_condition(
        &self,
        condition: &RetryCondition,
        error_context: &ErrorContext,
    ) -> Result<bool> {
        match condition {
            RetryCondition::Always => Ok(true),
            RetryCondition::IfTransient => {
                // Consider network, rate limit, and timeout errors as transient
                Ok(matches!(
                    error_context.error_type,
                    ErrorType::NetworkTimeout
                        | ErrorType::ApiRateLimited
                        | ErrorType::ConnectionFailed
                        | ErrorType::ToolTimeout
                ))
            }
            RetryCondition::IfNetworkError => {
                Ok(matches!(
                    error_context.error_type,
                    ErrorType::NetworkTimeout | ErrorType::ConnectionFailed
                ))
            }
            RetryCondition::IfResourcesAvailable => {
                // Check if we have sufficient resources (simplified check)
                Ok(!matches!(
                    error_context.error_type,
                    ErrorType::OutOfMemory | ErrorType::DiskFull | ErrorType::ResourceExhausted
                ))
            }
            RetryCondition::IfWithinTimeLimit(time_limit) => {
                let elapsed = SystemTime::now()
                    .duration_since(error_context.timestamp)
                    .unwrap_or(Duration::from_secs(0));
                Ok(elapsed < *time_limit)
            }
            RetryCondition::IfRetryBudgetAvailable => {
                Ok(self.used_retry_budget < self.global_retry_budget)
            }
        }
    }

    /// Calculate backoff delay
    fn calculate_backoff(&self, strategy: &BackoffStrategy, attempt: u32) -> Duration {
        match strategy {
            BackoffStrategy::Fixed(duration) => *duration,
            BackoffStrategy::Linear(base) => Duration::from_millis(base.as_millis() as u64 * (attempt + 1) as u64),
            BackoffStrategy::Exponential { base, multiplier, max } => {
                let delay = base.as_millis() as f64 * multiplier.powi(attempt as i32);
                Duration::from_millis((delay as u64).min(max.as_millis() as u64))
            }
            BackoffStrategy::Jittered { base, jitter_percent } => {
                let base_ms = base.as_millis() as f64;
                let jitter = base_ms * jitter_percent / 100.0;
                let random_jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter;
                Duration::from_millis((base_ms + random_jitter).max(0.0) as u64)
            }
        }
    }

    /// Get retry count for an error context
    fn get_retry_count(&self, error_context: &ErrorContext) -> u32 {
        error_context.recovery_attempts.len() as u32
    }

    /// Add error to history
    fn add_to_history(&mut self, error_context: ErrorContext) {
        self.error_history.push(error_context);
        
        // Trim history if it gets too large
        if self.error_history.len() > self.max_history_size {
            self.error_history.remove(0);
        }
    }

    /// Record a recovery attempt
    pub fn record_recovery_attempt(
        &mut self,
        error_id: &str,
        strategy: RecoveryStrategy,
        success: bool,
        duration: Duration,
        error_if_failed: Option<String>,
        changes_made: Vec<String>,
    ) -> Result<()> {
        if let Some(error_context) = self.error_history.iter_mut().find(|e| e.error_id == error_id) {
            let attempt = RecoveryAttempt {
                attempt_id: Uuid::new_v4().to_string(),
                strategy,
                timestamp: SystemTime::now(),
                duration,
                success,
                error_if_failed,
                changes_made,
            };
            error_context.recovery_attempts.push(attempt);
        }
        Ok(())
    }

    /// Set final outcome for an error
    pub fn set_final_outcome(&mut self, error_id: &str, outcome: RecoveryOutcome) -> Result<()> {
        if let Some(error_context) = self.error_history.iter_mut().find(|e| e.error_id == error_id) {
            error_context.final_outcome = Some(outcome);
        }
        Ok(())
    }

    /// Get error statistics
    pub fn get_error_statistics(&self) -> ErrorStatistics {
        let total_errors = self.error_history.len();
        let recovered_errors = self
            .error_history
            .iter()
            .filter(|e| matches!(
                e.final_outcome,
                Some(RecoveryOutcome::FullRecovery) | Some(RecoveryOutcome::PartialRecovery { .. })
            ))
            .count();

        let error_type_counts = self
            .error_history
            .iter()
            .fold(HashMap::new(), |mut acc, e| {
                *acc.entry(e.error_type.clone()).or_insert(0) += 1;
                acc
            });

        ErrorStatistics {
            total_errors,
            recovered_errors,
            recovery_rate: if total_errors > 0 {
                recovered_errors as f64 / total_errors as f64
            } else {
                0.0
            },
            error_type_distribution: error_type_counts,
            retry_budget_used: self.used_retry_budget,
            retry_budget_total: self.global_retry_budget,
        }
    }

    /// Reset retry budget (e.g., at the start of a new session)
    pub fn reset_retry_budget(&mut self) {
        self.used_retry_budget = 0;
    }
}

/// Error recovery statistics
#[derive(Debug, Clone)]
pub struct ErrorStatistics {
    pub total_errors: usize,
    pub recovered_errors: usize,
    pub recovery_rate: f64,
    pub error_type_distribution: HashMap<ErrorType, usize>,
    pub retry_budget_used: u32,
    pub retry_budget_total: u32,
}

impl fmt::Display for ErrorStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error Recovery Statistics:\n\
             Total Errors: {}\n\
             Recovered: {}\n\
             Recovery Rate: {:.1}%\n\
             Retry Budget: {}/{}\n\
             Top Error Types: {:?}",
            self.total_errors,
            self.recovered_errors,
            self.recovery_rate * 100.0,
            self.retry_budget_used,
            self.retry_budget_total,
            self.error_type_distribution
                .iter()
                .take(5)
                .collect::<Vec<_>>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_classification() {
        let manager = ErrorRecoveryManager::new();
        
        let timeout_error = anyhow!("Connection timed out");
        assert_eq!(manager.classify_error(&timeout_error), ErrorType::NetworkTimeout);
        
        let file_error = anyhow!("No such file or directory");
        assert_eq!(manager.classify_error(&file_error), ErrorType::FileNotFound);
        
        let permission_error = anyhow!("Permission denied");
        assert_eq!(manager.classify_error(&permission_error), ErrorType::PermissionDenied);
    }

    #[tokio::test]
    async fn test_retry_strategy() {
        let mut manager = ErrorRecoveryManager::new();
        
        let operation_context = OperationContext {
            operation_id: "test-op".to_string(),
            operation_type: "test".to_string(),
            session_id: "test-session".to_string(),
            user_request: "test request".to_string(),
            system_state: HashMap::new(),
            resources_in_use: Vec::new(),
        };
        
        let error = anyhow!("Connection timed out");
        let action = manager.handle_error(&error, operation_context).await.unwrap();
        
        // Should get retry action for timeout error
        assert!(matches!(action, RecoveryAction::Retry { .. }));
    }

    #[tokio::test]
    async fn test_backoff_calculation() {
        let manager = ErrorRecoveryManager::new();
        
        let fixed_backoff = BackoffStrategy::Fixed(Duration::from_secs(5));
        assert_eq!(manager.calculate_backoff(&fixed_backoff, 0), Duration::from_secs(5));
        assert_eq!(manager.calculate_backoff(&fixed_backoff, 3), Duration::from_secs(5));
        
        let exponential_backoff = BackoffStrategy::Exponential {
            base: Duration::from_secs(1),
            multiplier: 2.0,
            max: Duration::from_secs(10),
        };
        assert_eq!(manager.calculate_backoff(&exponential_backoff, 0), Duration::from_secs(1));
        assert_eq!(manager.calculate_backoff(&exponential_backoff, 1), Duration::from_secs(2));
        assert_eq!(manager.calculate_backoff(&exponential_backoff, 2), Duration::from_secs(4));
    }
}