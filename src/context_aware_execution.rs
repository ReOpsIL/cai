use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::logger::{log_debug, log_info, log_warn};
use crate::openrouter_client::OpenRouterClient;
use crate::advanced_error_recovery::ErrorType;

/// Context-aware task execution system for CAI
/// Provides intelligent task execution that adapts to environmental context,
/// user preferences, and system state for optimal performance and reliability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAwareConfig {
    /// Enable context-aware execution
    pub enabled: bool,
    /// Enable LLM-powered context analysis
    pub llm_context_analysis: bool,
    /// Context cache duration (minutes)
    pub context_cache_minutes: u64,
    /// Maximum context history to maintain
    pub max_context_history: usize,
    /// Enable adaptive execution strategies
    pub adaptive_strategies: bool,
    /// Execution optimization settings
    pub optimization_settings: OptimizationSettings,
    /// Context monitoring settings
    pub monitoring_settings: MonitoringSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSettings {
    /// Enable resource-aware execution
    pub resource_aware: bool,
    /// Enable user preference learning
    pub preference_learning: bool,
    /// Enable execution pattern optimization
    pub pattern_optimization: bool,
    /// Maximum parallel executions
    pub max_parallel_executions: usize,
    /// Minimum confidence for auto-execution
    pub auto_execution_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    /// Monitor system resources
    pub monitor_resources: bool,
    /// Monitor user behavior patterns
    pub monitor_user_behavior: bool,
    /// Monitor execution success rates
    pub monitor_success_rates: bool,
    /// Context update interval (seconds)
    pub update_interval_seconds: u64,
}

impl Default for ContextAwareConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_context_analysis: true,
            context_cache_minutes: 30,
            max_context_history: 500,
            adaptive_strategies: true,
            optimization_settings: OptimizationSettings {
                resource_aware: true,
                preference_learning: true,
                pattern_optimization: true,
                max_parallel_executions: 4,
                auto_execution_confidence: 0.8,
            },
            monitoring_settings: MonitoringSettings {
                monitor_resources: true,
                monitor_user_behavior: true,
                monitor_success_rates: true,
                update_interval_seconds: 60,
            },
        }
    }
}

/// Comprehensive execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub id: String,
    pub timestamp: SystemTime,
    pub user_context: UserContext,
    pub environment_context: EnvironmentContext,
    pub system_context: SystemContext,
    pub task_context: TaskContext,
    pub historical_context: HistoricalContext,
}

/// User-specific context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Option<String>,
    pub session_id: String,
    pub preferences: UserPreferences,
    pub behavioral_patterns: BehavioralPatterns,
    pub skill_level: SkillLevel,
    pub current_goals: Vec<String>,
}

/// User preferences for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub execution_speed: ExecutionSpeed,
    pub safety_level: SafetyLevel,
    pub verbosity_level: VerbosityLevel,
    pub preferred_tools: Vec<String>,
    pub avoided_patterns: Vec<String>,
    pub confirmation_preferences: ConfirmationPreferences,
}

/// User behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralPatterns {
    pub common_operations: Vec<OperationPattern>,
    pub error_handling_style: ErrorHandlingStyle,
    pub learning_progression: LearningProgression,
    pub time_patterns: TimePatterns,
}

/// Environment context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentContext {
    pub operating_system: String,
    pub working_directory: String,
    pub available_tools: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub network_status: NetworkStatus,
    pub docker_status: DockerStatus,
    pub git_status: Option<GitStatus>,
}

/// System performance and resource context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub resource_usage: ResourceUsage,
    pub performance_metrics: PerformanceMetrics,
    pub system_health: SystemHealth,
    pub concurrent_operations: usize,
    pub system_load: f64,
}

/// Task-specific context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task_type: String,
    pub complexity_level: ComplexityLevel,
    pub estimated_duration: Option<Duration>,
    pub required_resources: Vec<String>,
    pub dependencies: Vec<String>,
    pub risk_assessment: RiskAssessment,
}

/// Historical execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalContext {
    pub recent_executions: Vec<ExecutionRecord>,
    pub success_patterns: Vec<SuccessPattern>,
    pub failure_patterns: Vec<FailurePattern>,
    pub performance_trends: PerformanceTrends,
}

/// Execution speed preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionSpeed {
    Fast,        // Prioritize speed over safety
    Balanced,    // Balance speed and safety
    Careful,     // Prioritize safety over speed
    Thorough,    // Maximum validation and checks
}

impl ExecutionSpeed {
    fn as_u8(&self) -> u8 {
        match self {
            ExecutionSpeed::Fast => 0,
            ExecutionSpeed::Balanced => 1,
            ExecutionSpeed::Careful => 2,
            ExecutionSpeed::Thorough => 3,
        }
    }
}

/// Safety level preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyLevel {
    Minimal,     // Trust user, minimal safety checks
    Standard,    // Standard safety protocols
    Enhanced,    // Enhanced safety with validation
    Paranoid,    // Maximum safety, extensive validation
}

/// Verbosity level for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerbosityLevel {
    Silent,      // Minimal output
    Quiet,       // Essential output only
    Normal,      // Standard output
    Verbose,     // Detailed output
    Debug,       // All debugging information
}

/// User skill level assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillLevel {
    Beginner,    // New to the tools/domain
    Intermediate, // Some experience
    Advanced,    // Experienced user
    Expert,      // Domain expert
}

/// Confirmation preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationPreferences {
    pub require_for_destructive: bool,
    pub require_for_high_risk: bool,
    pub require_for_new_operations: bool,
    pub auto_confirm_trusted: bool,
}

/// Operation patterns for behavioral analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPattern {
    pub operation_type: String,
    pub frequency: usize,
    pub success_rate: f64,
    pub average_duration: Duration,
    pub typical_parameters: HashMap<String, String>,
}

/// Error handling style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStyle {
    RetryImmediately,    // Retry errors quickly
    AskForGuidance,      // Ask user how to handle errors
    TryAlternatives,     // Automatically try alternative approaches
    Abort,              // Stop on first error
}

/// Learning progression tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningProgression {
    pub operations_learned: Vec<String>,
    pub mastery_levels: HashMap<String, f64>,
    pub learning_velocity: f64,
    pub help_seeking_frequency: f64,
}

/// Time-based behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePatterns {
    pub active_hours: Vec<(u8, u8)>, // (start_hour, end_hour)
    pub peak_productivity_hours: Vec<u8>,
    pub preferred_session_duration: Duration,
    pub break_patterns: Vec<Duration>,
}

/// Network connectivity status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub connected: bool,
    pub connection_type: String,
    pub bandwidth_estimate: Option<f64>,
    pub latency_ms: Option<u64>,
}

/// Docker environment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerStatus {
    pub available: bool,
    pub version: Option<String>,
    pub running_containers: Vec<String>,
    pub container_health: HashMap<String, String>,
}

/// Git repository status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub is_repo: bool,
    pub current_branch: Option<String>,
    pub has_uncommitted_changes: bool,
    pub remote_status: Option<String>,
}

/// System resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_usage_mbps: f64,
    pub open_file_descriptors: usize,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub average_response_time_ms: f64,
    pub throughput_ops_per_second: f64,
    pub error_rate_percent: f64,
    pub cache_hit_rate_percent: f64,
}

/// Overall system health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemHealth {
    Excellent,   // All systems optimal
    Good,        // Minor issues, good performance
    Fair,        // Some performance issues
    Poor,        // Significant issues
    Critical,    // System unstable
}

/// Task complexity assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Simple,      // Single operation, low risk
    Moderate,    // Multiple steps, some dependencies
    Complex,     // Many steps, complex dependencies
    Advanced,    // Sophisticated operations, high risk
}

/// Risk assessment for tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: f64,
    pub data_risk: f64,
    pub system_risk: f64,
    pub user_risk: f64,
    pub mitigation_strategies: Vec<String>,
}

/// Execution record for historical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub task_id: String,
    pub task_type: String,
    pub start_time: SystemTime,
    pub duration: Duration,
    pub success: bool,
    pub error_type: Option<ErrorType>,
    pub context_snapshot: ExecutionContext,
    pub resource_usage: ResourceUsage,
}

/// Success pattern identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    pub pattern_id: String,
    pub task_types: Vec<String>,
    pub context_conditions: HashMap<String, String>,
    pub success_rate: f64,
    pub confidence: f64,
}

/// Failure pattern identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_id: String,
    pub task_types: Vec<String>,
    pub failure_conditions: HashMap<String, String>,
    pub common_errors: Vec<ErrorType>,
    pub mitigation_suggestions: Vec<String>,
}

/// Performance trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub execution_time_trend: f64, // Positive = getting slower, negative = getting faster
    pub success_rate_trend: f64,   // Positive = improving, negative = degrading
    pub resource_usage_trend: f64, // Positive = using more resources
    pub user_satisfaction_trend: f64,
}

/// Execution strategy based on context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStrategy {
    pub strategy_id: String,
    pub strategy_type: StrategyType,
    pub execution_plan: ExecutionPlan,
    pub resource_allocation: ResourceAllocation,
    pub risk_mitigation: Vec<String>,
    pub fallback_strategies: Vec<String>,
    pub confidence: f64,
}

/// Types of execution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    Sequential,      // Execute tasks one by one
    Parallel,        // Execute tasks concurrently
    Adaptive,        // Dynamically adjust execution
    Conservative,    // Prioritize safety over speed
    Aggressive,      // Prioritize speed over safety
    UserGuided,      // Let user make decisions
}

/// Detailed execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub steps: Vec<ExecutionStep>,
    pub estimated_duration: Duration,
    pub checkpoints: Vec<String>,
    pub rollback_points: Vec<String>,
    pub monitoring_points: Vec<String>,
}

/// Individual execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_id: String,
    pub description: String,
    pub tool: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub prerequisites: Vec<String>,
    pub success_criteria: Vec<String>,
    pub estimated_duration: Duration,
    pub risk_level: f64,
}

/// Resource allocation for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub cpu_priority: Priority,
    pub memory_limit_mb: Option<usize>,
    pub network_bandwidth_limit: Option<f64>,
    pub concurrent_operations: usize,
    pub timeout_seconds: u64,
}

/// Priority levels for resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Main context-aware execution engine
pub struct ContextAwareExecutor {
    config: ContextAwareConfig,
    openrouter_client: Option<OpenRouterClient>,
    context_cache: Arc<Mutex<HashMap<String, (ExecutionContext, Instant)>>>,
    execution_history: Arc<Mutex<VecDeque<ExecutionRecord>>>,
    user_profiles: Arc<Mutex<HashMap<String, UserContext>>>,
    strategy_cache: Arc<Mutex<HashMap<String, (ExecutionStrategy, Instant)>>>,
}

impl ContextAwareExecutor {
    pub async fn new(config: ContextAwareConfig) -> Result<Self> {
        let openrouter_client = if config.llm_context_analysis {
            match OpenRouterClient::new().await {
                Ok(client) => Some(client),
                Err(e) => {
                    log_warn!("context_aware", "⚠️ Failed to initialize OpenRouter client: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            openrouter_client,
            context_cache: Arc::new(Mutex::new(HashMap::new())),
            execution_history: Arc::new(Mutex::new(VecDeque::new())),
            user_profiles: Arc::new(Mutex::new(HashMap::new())),
            strategy_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn default() -> Result<Self> {
        tokio::runtime::Handle::current().block_on(Self::new(ContextAwareConfig::default()))
    }

    /// Gather comprehensive execution context
    pub async fn gather_context(&self, task_type: &str, user_id: Option<&str>) -> Result<ExecutionContext> {
        if !self.config.enabled {
            return Ok(self.create_minimal_context(task_type).await);
        }

        let context_key = format!("{}:{}", task_type, user_id.unwrap_or("anonymous"));
        
        // Check cache first
        if let Some((cached_context, created_at)) = {
            let cache = self.context_cache.lock().await;
            cache.get(&context_key).cloned()
        } {
            let age_minutes = created_at.elapsed().as_secs() / 60;
            if age_minutes < self.config.context_cache_minutes {
                log_debug!("context_aware", "🎯 Using cached context for {}", context_key);
                return Ok(cached_context);
            }
        }

        log_info!("context_aware", "🔍 Gathering fresh execution context for {}", task_type);
        
        let context = ExecutionContext {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            user_context: self.gather_user_context(user_id).await?,
            environment_context: self.gather_environment_context().await?,
            system_context: self.gather_system_context().await?,
            task_context: self.gather_task_context(task_type).await?,
            historical_context: self.gather_historical_context(task_type).await?,
        };

        // Cache the context
        {
            let mut cache = self.context_cache.lock().await;
            cache.insert(context_key, (context.clone(), Instant::now()));
            
            // Clean old cache entries
            let cutoff = Instant::now() - Duration::from_secs(self.config.context_cache_minutes * 60);
            cache.retain(|_, (_, created_at)| *created_at > cutoff);
        }

        Ok(context)
    }

    /// Create optimal execution strategy based on context
    pub async fn create_execution_strategy(&self, context: &ExecutionContext, task_description: &str) -> Result<ExecutionStrategy> {
        let strategy_key = format!("{}:{}", context.task_context.task_type, 
            context.user_context.preferences.execution_speed.as_u8());
        
        // Check strategy cache
        if let Some((cached_strategy, created_at)) = {
            let cache = self.strategy_cache.lock().await;
            cache.get(&strategy_key).cloned()
        } {
            let age_minutes = created_at.elapsed().as_secs() / 60;
            if age_minutes < self.config.context_cache_minutes {
                log_debug!("context_aware", "⚡ Using cached strategy for {}", strategy_key);
                return Ok(cached_strategy);
            }
        }

        log_info!("context_aware", "🎯 Creating execution strategy for task: {}", task_description);

        let strategy = if self.config.llm_context_analysis && self.openrouter_client.is_some() {
            self.create_llm_strategy(context, task_description).await?
        } else {
            self.create_heuristic_strategy(context, task_description).await?
        };

        // Cache the strategy
        {
            let mut cache = self.strategy_cache.lock().await;
            cache.insert(strategy_key, (strategy.clone(), Instant::now()));
        }

        Ok(strategy)
    }

    /// Execute task with context-aware optimizations
    pub async fn execute_with_context<F, T>(&self, 
        task_description: &str, 
        user_id: Option<&str>,
        execution_fn: F
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start_time = Instant::now();
        
        // Gather context
        let context = self.gather_context(task_description, user_id).await?;
        
        // Create execution strategy
        let strategy = self.create_execution_strategy(&context, task_description).await?;
        
        // Apply pre-execution optimizations
        self.apply_pre_execution_optimizations(&context, &strategy).await?;
        
        // Execute with monitoring
        let result = self.execute_with_monitoring(execution_fn, &context, &strategy).await;
        
        // Record execution for learning
        self.record_execution(&context, &strategy, start_time.elapsed(), result.is_ok()).await;
        
        result
    }

    /// Gather user-specific context
    async fn gather_user_context(&self, user_id: Option<&str>) -> Result<UserContext> {
        let user_id = user_id.unwrap_or("anonymous");
        
        // Check cached user profile
        if let Some(cached_profile) = {
            let profiles = self.user_profiles.lock().await;
            profiles.get(user_id).cloned()
        } {
            return Ok(cached_profile);
        }

        // Create new user context
        let user_context = UserContext {
            user_id: Some(user_id.to_string()),
            session_id: Uuid::new_v4().to_string(),
            preferences: UserPreferences {
                execution_speed: ExecutionSpeed::Balanced,
                safety_level: SafetyLevel::Standard,
                verbosity_level: VerbosityLevel::Normal,
                preferred_tools: vec!["filesystem".to_string(), "web".to_string()],
                avoided_patterns: Vec::new(),
                confirmation_preferences: ConfirmationPreferences {
                    require_for_destructive: true,
                    require_for_high_risk: true,
                    require_for_new_operations: false,
                    auto_confirm_trusted: true,
                },
            },
            behavioral_patterns: BehavioralPatterns {
                common_operations: Vec::new(),
                error_handling_style: ErrorHandlingStyle::TryAlternatives,
                learning_progression: LearningProgression {
                    operations_learned: Vec::new(),
                    mastery_levels: HashMap::new(),
                    learning_velocity: 1.0,
                    help_seeking_frequency: 0.2,
                },
                time_patterns: TimePatterns {
                    active_hours: vec![(9, 17)], // 9 AM to 5 PM
                    peak_productivity_hours: vec![10, 11, 14, 15],
                    preferred_session_duration: Duration::from_secs(3600), // 1 hour
                    break_patterns: vec![Duration::from_secs(300)], // 5 minute breaks
                },
            },
            skill_level: SkillLevel::Intermediate,
            current_goals: Vec::new(),
        };

        // Cache user profile
        {
            let mut profiles = self.user_profiles.lock().await;
            profiles.insert(user_id.to_string(), user_context.clone());
        }

        Ok(user_context)
    }

    /// Gather environment context
    async fn gather_environment_context(&self) -> Result<EnvironmentContext> {
        Ok(EnvironmentContext {
            operating_system: std::env::consts::OS.to_string(),
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            available_tools: vec!["filesystem".to_string(), "web".to_string()],
            environment_variables: std::env::vars().collect(),
            network_status: NetworkStatus {
                connected: true, // Simplified - would check actual connectivity
                connection_type: "ethernet".to_string(),
                bandwidth_estimate: Some(100.0), // Mbps
                latency_ms: Some(20),
            },
            docker_status: DockerStatus {
                available: true, // Would check actual Docker availability
                version: Some("20.10.0".to_string()),
                running_containers: vec!["filesystem".to_string(), "web".to_string()],
                container_health: [
                    ("filesystem".to_string(), "healthy".to_string()),
                    ("web".to_string(), "healthy".to_string()),
                ].into_iter().collect(),
            },
            git_status: Some(GitStatus {
                is_repo: true,
                current_branch: Some("refactor/chat-modules-ui-complexity".to_string()),
                has_uncommitted_changes: true,
                remote_status: Some("ahead".to_string()),
            }),
        })
    }

    /// Gather system performance context
    async fn gather_system_context(&self) -> Result<SystemContext> {
        Ok(SystemContext {
            resource_usage: ResourceUsage {
                memory_usage_percent: 45.0, // Would get actual system metrics
                cpu_usage_percent: 25.0,
                disk_usage_percent: 60.0,
                network_usage_mbps: 5.0,
                open_file_descriptors: 150,
            },
            performance_metrics: PerformanceMetrics {
                average_response_time_ms: 250.0,
                throughput_ops_per_second: 10.0,
                error_rate_percent: 2.0,
                cache_hit_rate_percent: 85.0,
            },
            system_health: SystemHealth::Good,
            concurrent_operations: 2,
            system_load: 0.5,
        })
    }

    /// Gather task-specific context
    async fn gather_task_context(&self, task_type: &str) -> Result<TaskContext> {
        let complexity = self.assess_task_complexity(task_type);
        let risk = self.assess_task_risk(task_type, complexity.clone()).await?;
        
        Ok(TaskContext {
            task_type: task_type.to_string(),
            complexity_level: complexity.clone(),
            estimated_duration: self.estimate_task_duration(task_type, &complexity),
            required_resources: self.identify_required_resources(task_type),
            dependencies: self.identify_task_dependencies(task_type),
            risk_assessment: risk,
        })
    }

    /// Gather historical execution context
    async fn gather_historical_context(&self, task_type: &str) -> Result<HistoricalContext> {
        let history = self.execution_history.lock().await;
        
        let recent_executions: Vec<ExecutionRecord> = history
            .iter()
            .filter(|record| record.task_type == task_type)
            .take(10)
            .cloned()
            .collect();
        
        Ok(HistoricalContext {
            recent_executions,
            success_patterns: Vec::new(), // Would analyze historical data
            failure_patterns: Vec::new(),
            performance_trends: PerformanceTrends {
                execution_time_trend: 0.0,
                success_rate_trend: 0.05, // Slight improvement
                resource_usage_trend: -0.02, // Slight improvement
                user_satisfaction_trend: 0.1,
            },
        })
    }

    /// Create minimal context for basic operation
    async fn create_minimal_context(&self, task_type: &str) -> ExecutionContext {
        ExecutionContext {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            user_context: UserContext {
                user_id: None,
                session_id: Uuid::new_v4().to_string(),
                preferences: UserPreferences {
                    execution_speed: ExecutionSpeed::Balanced,
                    safety_level: SafetyLevel::Standard,
                    verbosity_level: VerbosityLevel::Normal,
                    preferred_tools: Vec::new(),
                    avoided_patterns: Vec::new(),
                    confirmation_preferences: ConfirmationPreferences {
                        require_for_destructive: true,
                        require_for_high_risk: true,
                        require_for_new_operations: false,
                        auto_confirm_trusted: false,
                    },
                },
                behavioral_patterns: BehavioralPatterns {
                    common_operations: Vec::new(),
                    error_handling_style: ErrorHandlingStyle::AskForGuidance,
                    learning_progression: LearningProgression {
                        operations_learned: Vec::new(),
                        mastery_levels: HashMap::new(),
                        learning_velocity: 1.0,
                        help_seeking_frequency: 0.5,
                    },
                    time_patterns: TimePatterns {
                        active_hours: vec![(0, 24)],
                        peak_productivity_hours: vec![10, 14],
                        preferred_session_duration: Duration::from_secs(1800),
                        break_patterns: Vec::new(),
                    },
                },
                skill_level: SkillLevel::Intermediate,
                current_goals: Vec::new(),
            },
            environment_context: EnvironmentContext {
                operating_system: std::env::consts::OS.to_string(),
                working_directory: ".".to_string(),
                available_tools: Vec::new(),
                environment_variables: HashMap::new(),
                network_status: NetworkStatus {
                    connected: true,
                    connection_type: "unknown".to_string(),
                    bandwidth_estimate: None,
                    latency_ms: None,
                },
                docker_status: DockerStatus {
                    available: false,
                    version: None,
                    running_containers: Vec::new(),
                    container_health: HashMap::new(),
                },
                git_status: None,
            },
            system_context: SystemContext {
                resource_usage: ResourceUsage {
                    memory_usage_percent: 50.0,
                    cpu_usage_percent: 20.0,
                    disk_usage_percent: 50.0,
                    network_usage_mbps: 0.0,
                    open_file_descriptors: 100,
                },
                performance_metrics: PerformanceMetrics {
                    average_response_time_ms: 500.0,
                    throughput_ops_per_second: 1.0,
                    error_rate_percent: 5.0,
                    cache_hit_rate_percent: 70.0,
                },
                system_health: SystemHealth::Fair,
                concurrent_operations: 1,
                system_load: 0.5,
            },
            task_context: TaskContext {
                task_type: task_type.to_string(),
                complexity_level: ComplexityLevel::Moderate,
                estimated_duration: Some(Duration::from_secs(30)),
                required_resources: Vec::new(),
                dependencies: Vec::new(),
                risk_assessment: RiskAssessment {
                    overall_risk: 0.3,
                    data_risk: 0.2,
                    system_risk: 0.2,
                    user_risk: 0.1,
                    mitigation_strategies: Vec::new(),
                },
            },
            historical_context: HistoricalContext {
                recent_executions: Vec::new(),
                success_patterns: Vec::new(),
                failure_patterns: Vec::new(),
                performance_trends: PerformanceTrends {
                    execution_time_trend: 0.0,
                    success_rate_trend: 0.0,
                    resource_usage_trend: 0.0,
                    user_satisfaction_trend: 0.0,
                },
            },
        }
    }

    /// Assess task complexity based on type and historical data
    fn assess_task_complexity(&self, task_type: &str) -> ComplexityLevel {
        match task_type {
            t if t.contains("file_read") || t.contains("simple") => ComplexityLevel::Simple,
            t if t.contains("file_write") || t.contains("search") => ComplexityLevel::Moderate,
            t if t.contains("workflow") || t.contains("complex") => ComplexityLevel::Complex,
            t if t.contains("advanced") || t.contains("system") => ComplexityLevel::Advanced,
            _ => ComplexityLevel::Moderate,
        }
    }

    /// Assess risk for task execution
    async fn assess_task_risk(&self, _task_type: &str, complexity: ComplexityLevel) -> Result<RiskAssessment> {
        let base_risk: f64 = match complexity {
            ComplexityLevel::Simple => 0.1,
            ComplexityLevel::Moderate => 0.3,
            ComplexityLevel::Complex => 0.6,
            ComplexityLevel::Advanced => 0.8,
        };

        // Simplified risk calculation without predictive system for now
        let prediction_risk: f64 = 0.1; // Would integrate with predictive error prevention later
        
        let overall_risk = (base_risk + prediction_risk).min(1.0);

        Ok(RiskAssessment {
            overall_risk,
            data_risk: base_risk * 0.3,
            system_risk: base_risk * 0.4,
            user_risk: base_risk * 0.3,
            mitigation_strategies: vec![
                "Backup before execution".to_string(),
                "Use safe mode execution".to_string(),
                "Monitor resource usage".to_string(),
            ],
        })
    }

    /// Estimate task duration based on historical data
    fn estimate_task_duration(&self, task_type: &str, complexity: &ComplexityLevel) -> Option<Duration> {
        let base_seconds = match complexity {
            ComplexityLevel::Simple => 5,
            ComplexityLevel::Moderate => 30,
            ComplexityLevel::Complex => 120,
            ComplexityLevel::Advanced => 300,
        };

        let multiplier = match task_type {
            t if t.contains("network") => 2.0,
            t if t.contains("file") => 1.0,
            t if t.contains("analysis") => 1.5,
            _ => 1.0,
        };

        Some(Duration::from_secs((base_seconds as f64 * multiplier) as u64))
    }

    /// Identify required resources for task
    fn identify_required_resources(&self, task_type: &str) -> Vec<String> {
        let mut resources = Vec::new();
        
        if task_type.contains("file") {
            resources.push("filesystem_access".to_string());
        }
        if task_type.contains("network") {
            resources.push("network_access".to_string());
        }
        if task_type.contains("docker") {
            resources.push("docker_access".to_string());
        }
        if task_type.contains("git") {
            resources.push("git_access".to_string());
        }
        
        resources
    }

    /// Identify task dependencies
    fn identify_task_dependencies(&self, task_type: &str) -> Vec<String> {
        let mut dependencies = Vec::new();
        
        if task_type.contains("docker") {
            dependencies.push("docker_service".to_string());
        }
        if task_type.contains("network") {
            dependencies.push("network_connectivity".to_string());
        }
        if task_type.contains("git") {
            dependencies.push("git_repository".to_string());
        }
        
        dependencies
    }

    /// Create LLM-powered execution strategy
    async fn create_llm_strategy(&self, context: &ExecutionContext, task_description: &str) -> Result<ExecutionStrategy> {
        let client = self.openrouter_client.as_ref().unwrap();
        
        let prompt = format!(
            "Analyze the following task execution context and create an optimal execution strategy.\n\n\
            Task: {}\n\
            User Skill Level: {:?}\n\
            Execution Speed Preference: {:?}\n\
            Safety Level: {:?}\n\
            System Health: {:?}\n\
            Task Complexity: {:?}\n\
            Overall Risk: {:.2}\n\
            Available Resources: CPU: {:.1}%, Memory: {:.1}%, Disk: {:.1}%\n\n\
            Create an execution strategy that balances the user's preferences with system capabilities and safety requirements.\n\n\
            Respond in JSON format:\n\
            {{\n  \
              \"strategy_type\": \"Sequential|Parallel|Adaptive|Conservative|Aggressive|UserGuided\",\n  \
              \"confidence\": 0.85,\n  \
              \"reasoning\": \"Why this strategy is optimal\",\n  \
              \"estimated_duration_seconds\": 120,\n  \
              \"resource_allocation\": {{\n    \
                \"cpu_priority\": \"Normal\",\n    \
                \"memory_limit_mb\": 500,\n    \
                \"concurrent_operations\": 2\n  \
              }},\n  \
              \"risk_mitigation\": [\"strategy1\", \"strategy2\"]\n\
            }}",
            task_description,
            context.user_context.skill_level,
            context.user_context.preferences.execution_speed,
            context.user_context.preferences.safety_level,
            context.system_context.system_health,
            context.task_context.complexity_level,
            context.task_context.risk_assessment.overall_risk,
            100.0 - context.system_context.resource_usage.cpu_usage_percent,
            100.0 - context.system_context.resource_usage.memory_usage_percent,
            100.0 - context.system_context.resource_usage.disk_usage_percent
        );

        let messages = vec![crate::openrouter_client::ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        match client.chat_completion(messages).await {
            Ok(response) => {
                if let Ok(strategy) = self.parse_llm_strategy_response(&response, context) {
                    log_debug!("context_aware", "🧠 Created LLM-powered execution strategy");
                    return Ok(strategy);
                }
            }
            Err(e) => {
                log_warn!("context_aware", "⚠️ LLM strategy creation failed: {}", e);
            }
        }

        // Fallback to heuristic strategy
        self.create_heuristic_strategy(context, task_description).await
    }

    /// Create heuristic-based execution strategy
    async fn create_heuristic_strategy(&self, context: &ExecutionContext, task_description: &str) -> Result<ExecutionStrategy> {
        let strategy_type = match (&context.user_context.preferences.execution_speed, 
                                  &context.task_context.complexity_level) {
            (ExecutionSpeed::Fast, _) => StrategyType::Aggressive,
            (ExecutionSpeed::Careful, _) => StrategyType::Conservative,
            (_, ComplexityLevel::Complex | ComplexityLevel::Advanced) => StrategyType::Sequential,
            (_, ComplexityLevel::Simple) => StrategyType::Parallel,
            _ => StrategyType::Adaptive,
        };

        let estimated_duration = context.task_context.estimated_duration
            .unwrap_or(Duration::from_secs(60));

        let concurrent_ops = match strategy_type {
            StrategyType::Parallel => self.config.optimization_settings.max_parallel_executions,
            StrategyType::Aggressive => self.config.optimization_settings.max_parallel_executions.min(2),
            _ => 1,
        };

        Ok(ExecutionStrategy {
            strategy_id: Uuid::new_v4().to_string(),
            strategy_type,
            execution_plan: ExecutionPlan {
                steps: vec![ExecutionStep {
                    step_id: Uuid::new_v4().to_string(),
                    description: task_description.to_string(),
                    tool: "default".to_string(),
                    parameters: HashMap::new(),
                    prerequisites: Vec::new(),
                    success_criteria: vec!["task_completed".to_string()],
                    estimated_duration,
                    risk_level: context.task_context.risk_assessment.overall_risk,
                }],
                estimated_duration,
                checkpoints: vec!["pre_execution".to_string(), "post_execution".to_string()],
                rollback_points: vec!["initial_state".to_string()],
                monitoring_points: vec!["resource_usage".to_string(), "progress".to_string()],
            },
            resource_allocation: ResourceAllocation {
                cpu_priority: match context.user_context.preferences.execution_speed {
                    ExecutionSpeed::Fast => Priority::High,
                    ExecutionSpeed::Careful => Priority::Low,
                    _ => Priority::Normal,
                },
                memory_limit_mb: Some(1024),
                network_bandwidth_limit: None,
                concurrent_operations: concurrent_ops,
                timeout_seconds: estimated_duration.as_secs() * 2, // 2x estimated time
            },
            risk_mitigation: context.task_context.risk_assessment.mitigation_strategies.clone(),
            fallback_strategies: vec!["retry_with_safe_mode".to_string(), "ask_user_guidance".to_string()],
            confidence: 0.8,
        })
    }

    /// Parse LLM strategy response
    fn parse_llm_strategy_response(&self, response: &str, _context: &ExecutionContext) -> Result<ExecutionStrategy> {
        // Extract JSON from markdown if present
        let json_str = if response.contains("```json") {
            response.split("```json").nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
        } else {
            response
        };

        #[derive(Deserialize)]
        struct LLMStrategyResponse {
            strategy_type: String,
            confidence: f64,
            reasoning: String,
            estimated_duration_seconds: u64,
            resource_allocation: LLMResourceAllocation,
            risk_mitigation: Vec<String>,
        }

        #[derive(Deserialize)]
        struct LLMResourceAllocation {
            cpu_priority: String,
            memory_limit_mb: Option<usize>,
            concurrent_operations: usize,
        }

        let llm_response: LLMStrategyResponse = serde_json::from_str(json_str)?;

        let strategy_type = match llm_response.strategy_type.as_str() {
            "Sequential" => StrategyType::Sequential,
            "Parallel" => StrategyType::Parallel,
            "Adaptive" => StrategyType::Adaptive,
            "Conservative" => StrategyType::Conservative,
            "Aggressive" => StrategyType::Aggressive,
            "UserGuided" => StrategyType::UserGuided,
            _ => StrategyType::Adaptive,
        };

        let cpu_priority = match llm_response.resource_allocation.cpu_priority.as_str() {
            "Low" => Priority::Low,
            "High" => Priority::High,
            "Critical" => Priority::Critical,
            _ => Priority::Normal,
        };

        Ok(ExecutionStrategy {
            strategy_id: Uuid::new_v4().to_string(),
            strategy_type,
            execution_plan: ExecutionPlan {
                steps: vec![], // Would be populated based on LLM analysis
                estimated_duration: Duration::from_secs(llm_response.estimated_duration_seconds),
                checkpoints: vec!["pre_execution".to_string()],
                rollback_points: vec!["initial_state".to_string()],
                monitoring_points: vec!["progress".to_string()],
            },
            resource_allocation: ResourceAllocation {
                cpu_priority,
                memory_limit_mb: llm_response.resource_allocation.memory_limit_mb,
                network_bandwidth_limit: None,
                concurrent_operations: llm_response.resource_allocation.concurrent_operations,
                timeout_seconds: llm_response.estimated_duration_seconds * 2,
            },
            risk_mitigation: llm_response.risk_mitigation,
            fallback_strategies: vec!["retry".to_string()],
            confidence: llm_response.confidence,
        })
    }

    /// Apply pre-execution optimizations
    async fn apply_pre_execution_optimizations(&self, _context: &ExecutionContext, _strategy: &ExecutionStrategy) -> Result<()> {
        // Would implement resource allocation, cache warming, etc.
        log_debug!("context_aware", "⚙️ Applied pre-execution optimizations");
        Ok(())
    }

    /// Execute with monitoring and context awareness
    async fn execute_with_monitoring<F, T>(&self, execution_fn: F, context: &ExecutionContext, strategy: &ExecutionStrategy) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        log_info!("context_aware", "🚀 Executing with strategy: {:?}", strategy.strategy_type);
        
        // Set up monitoring based on strategy
        let _monitor_handle = self.setup_execution_monitoring(context, strategy).await;
        
        // Execute with timeout from strategy
        let timeout_duration = Duration::from_secs(strategy.resource_allocation.timeout_seconds);
        
        match tokio::time::timeout(timeout_duration, execution_fn).await {
            Ok(result) => {
                log_info!("context_aware", "✅ Execution completed successfully");
                result
            }
            Err(_) => {
                log_warn!("context_aware", "⏱️ Execution timed out after {:?}", timeout_duration);
                Err(anyhow::anyhow!("Execution timed out"))
            }
        }
    }

    /// Set up execution monitoring
    async fn setup_execution_monitoring(&self, _context: &ExecutionContext, _strategy: &ExecutionStrategy) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async {
            // Would implement resource monitoring, progress tracking, etc.
            log_debug!("context_aware", "📊 Monitoring execution progress");
        })
    }

    /// Record execution for learning and optimization
    async fn record_execution(&self, context: &ExecutionContext, strategy: &ExecutionStrategy, duration: Duration, success: bool) {
        let execution_record = ExecutionRecord {
            task_id: strategy.strategy_id.clone(),
            task_type: context.task_context.task_type.clone(),
            start_time: context.timestamp,
            duration,
            success,
            error_type: None, // Would be populated if there was an error
            context_snapshot: context.clone(),
            resource_usage: context.system_context.resource_usage.clone(),
        };

        let mut history = self.execution_history.lock().await;
        history.push_back(execution_record);

        // Limit history size
        while history.len() > self.config.max_context_history {
            history.pop_front();
        }

        log_debug!("context_aware", "📝 Recorded execution: success={}, duration={:?}", success, duration);
    }

    /// Get execution statistics
    pub async fn get_statistics(&self) -> ContextExecutionStatistics {
        let history = self.execution_history.lock().await;
        let total_executions = history.len();
        
        let successful_executions = history.iter().filter(|r| r.success).count();
        let success_rate = if total_executions > 0 {
            (successful_executions as f64) / (total_executions as f64) * 100.0
        } else {
            0.0
        };

        let avg_duration = if total_executions > 0 {
            let total_ms: u64 = history.iter().map(|r| r.duration.as_millis() as u64).sum();
            Duration::from_millis(total_ms / total_executions as u64)
        } else {
            Duration::from_secs(0)
        };

        ContextExecutionStatistics {
            total_executions,
            successful_executions,
            success_rate,
            average_duration: avg_duration,
            context_cache_size: self.context_cache.lock().await.len(),
            strategy_cache_size: self.strategy_cache.lock().await.len(),
        }
    }
}

/// Statistics for context-aware execution
#[derive(Debug, Clone)]
pub struct ContextExecutionStatistics {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub success_rate: f64,
    pub average_duration: Duration,
    pub context_cache_size: usize,
    pub strategy_cache_size: usize,
}

/// Global context-aware executor instance
use once_cell::sync::Lazy;

static GLOBAL_CONTEXT_EXECUTOR: Lazy<Mutex<Option<ContextAwareExecutor>>> = 
    Lazy::new(|| Mutex::new(None));

/// Initialize global context-aware executor
pub async fn initialize_context_aware_executor(config: ContextAwareConfig) -> Result<()> {
    let executor = ContextAwareExecutor::new(config).await?;
    let mut global = GLOBAL_CONTEXT_EXECUTOR.lock().await;
    *global = Some(executor);
    log_info!("context_aware", "🎯 Context-aware execution system initialized");
    Ok(())
}

/// Execute with context awareness using global executor
pub async fn execute_with_global_context<F, T>(
    task_description: &str,
    user_id: Option<&str>,
    execution_fn: F,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let executor_guard = GLOBAL_CONTEXT_EXECUTOR.lock().await;
    if let Some(executor) = executor_guard.as_ref() {
        executor.execute_with_context(task_description, user_id, execution_fn).await
    } else {
        // Fallback to direct execution if context system not initialized
        execution_fn.await
    }
}

/// Get global context execution statistics
pub async fn get_global_context_statistics() -> Option<ContextExecutionStatistics> {
    let executor_guard = GLOBAL_CONTEXT_EXECUTOR.lock().await;
    if let Some(executor) = executor_guard.as_ref() {
        Some(executor.get_statistics().await)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_aware_executor_creation() {
        let config = ContextAwareConfig::default();
        let executor = ContextAwareExecutor::new(config).await.unwrap();
        
        let stats = executor.get_statistics().await;
        assert_eq!(stats.total_executions, 0);
    }

    #[tokio::test]
    async fn test_context_gathering() {
        let executor = ContextAwareExecutor::default().unwrap();
        
        let context = executor.gather_context("test_task", Some("test_user")).await.unwrap();
        
        assert_eq!(context.task_context.task_type, "test_task");
        assert_eq!(context.user_context.user_id, Some("test_user".to_string()));
    }

    #[tokio::test]
    async fn test_execution_strategy_creation() {
        let executor = ContextAwareExecutor::default().unwrap();
        
        let context = executor.gather_context("file_read", None).await.unwrap();
        let strategy = executor.create_execution_strategy(&context, "Read file content").await.unwrap();
        
        assert!(!strategy.strategy_id.is_empty());
        assert!(strategy.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_context_aware_execution() {
        let executor = ContextAwareExecutor::default().unwrap();
        
        let result = executor.execute_with_context(
            "test_operation",
            None,
            async { Ok("success".to_string()) }
        ).await.unwrap();
        
        assert_eq!(result, "success");
        
        let stats = executor.get_statistics().await;
        assert_eq!(stats.total_executions, 1);
        assert_eq!(stats.successful_executions, 1);
    }
}