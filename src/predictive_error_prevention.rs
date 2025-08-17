use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Predictive error prevention system using machine learning and pattern recognition
#[derive(Debug)]
pub struct PredictiveErrorPrevention {
    /// Error pattern database
    error_patterns: Arc<RwLock<Vec<ErrorPattern>>>,
    /// Risk assessment engine
    risk_assessor: RiskAssessmentEngine,
    /// Prevention strategies
    prevention_strategies: Vec<PreventionStrategy>,
    /// Learning system for pattern recognition
    learning_system: LearningSystem,
    /// Historical error data
    error_history: Arc<RwLock<VecDeque<HistoricalError>>>,
    /// Prevention statistics
    stats: Arc<RwLock<PreventionStatistics>>,
    /// Configuration
    config: PredictiveConfig,
}

/// Configuration for predictive error prevention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveConfig {
    /// Enable predictive prevention
    pub enabled: bool,
    /// Minimum confidence threshold for prevention (0.0 - 1.0)
    pub confidence_threshold: f32,
    /// Maximum risk score to allow operation (0.0 - 1.0)
    pub max_risk_threshold: f32,
    /// Learning system settings
    pub learning: LearningConfig,
    /// Pattern matching settings
    pub pattern_matching: PatternMatchingConfig,
    /// Prevention strategy settings
    pub strategy_settings: StrategySettings,
}

/// Learning system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Enable online learning
    pub online_learning: bool,
    /// Learning rate
    pub learning_rate: f32,
    /// Minimum samples for pattern recognition
    pub min_pattern_samples: usize,
    /// Pattern decay rate (how quickly old patterns lose relevance)
    pub pattern_decay_rate: f32,
    /// Maximum history size
    pub max_history_size: usize,
}

/// Pattern matching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatchingConfig {
    /// Enable fuzzy matching
    pub fuzzy_matching: bool,
    /// Similarity threshold for fuzzy matching
    pub similarity_threshold: f32,
    /// Context window size for pattern matching
    pub context_window: usize,
    /// Enable temporal pattern matching
    pub temporal_patterns: bool,
}

/// Prevention strategy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySettings {
    /// Enable automatic prevention
    pub auto_prevention: bool,
    /// Enable user warnings
    pub user_warnings: bool,
    /// Enable alternative suggestions
    pub alternative_suggestions: bool,
    /// Maximum prevention attempts per operation
    pub max_prevention_attempts: u32,
}

/// Error pattern for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Unique pattern ID
    pub id: String,
    /// Pattern name/description
    pub name: String,
    /// Operation context patterns
    pub operation_patterns: Vec<OperationPattern>,
    /// Environment condition patterns
    pub environment_patterns: Vec<EnvironmentPattern>,
    /// Error signature
    pub error_signature: ErrorSignature,
    /// Prevention strategies
    pub prevention_strategies: Vec<String>,
    /// Pattern confidence (learned over time)
    pub confidence: f32,
    /// Times this pattern has been observed
    pub observation_count: u32,
    /// Times prevention based on this pattern was successful
    pub success_count: u32,
    /// Last observed time
    pub last_observed: SystemTime,
    /// Pattern weight (importance)
    pub weight: f32,
}

/// Operation pattern for error prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPattern {
    /// Type of operation
    pub operation_type: String,
    /// Parameters pattern
    pub parameter_patterns: HashMap<String, ParameterPattern>,
    /// File patterns involved
    pub file_patterns: Vec<String>,
    /// Temporal patterns (sequence, timing)
    pub temporal_aspects: TemporalPattern,
    /// Resource usage patterns
    pub resource_patterns: ResourcePattern,
}

/// Parameter pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterPattern {
    /// Parameter name
    pub name: String,
    /// Value patterns
    pub value_patterns: Vec<ValuePattern>,
    /// Required vs optional
    pub required: bool,
    /// Type constraints
    pub type_constraint: Option<String>,
}

/// Value pattern for parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValuePattern {
    Exact(String),
    Range { min: f64, max: f64 },
    Regex(String),
    Enum(Vec<String>),
    Length { min: Option<usize>, max: Option<usize> },
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

/// Temporal pattern for operation sequences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPattern {
    /// Previous operations that lead to errors
    pub preceding_operations: Vec<String>,
    /// Time windows between operations
    pub timing_constraints: Vec<TimingConstraint>,
    /// Operation frequency patterns
    pub frequency_patterns: Vec<FrequencyPattern>,
}

/// Timing constraint for operation sequences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConstraint {
    /// Operations involved
    pub operations: Vec<String>,
    /// Minimum time between operations
    pub min_interval: Option<Duration>,
    /// Maximum time between operations
    pub max_interval: Option<Duration>,
    /// Expected interval
    pub expected_interval: Option<Duration>,
}

/// Frequency pattern for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyPattern {
    /// Operation type
    pub operation: String,
    /// Time window
    pub time_window: Duration,
    /// Maximum frequency before risk increases
    pub max_frequency: u32,
    /// Risk multiplier for exceeding frequency
    pub risk_multiplier: f32,
}

/// Resource usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePattern {
    /// Memory usage patterns
    pub memory_patterns: Vec<ResourceThreshold>,
    /// CPU usage patterns
    pub cpu_patterns: Vec<ResourceThreshold>,
    /// Disk usage patterns
    pub disk_patterns: Vec<ResourceThreshold>,
    /// Network patterns
    pub network_patterns: Vec<ResourceThreshold>,
}

/// Resource threshold pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceThreshold {
    /// Resource metric name
    pub metric: String,
    /// Threshold value
    pub threshold: f64,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Risk level when threshold is exceeded
    pub risk_level: RiskLevel,
}

/// Comparison operators for thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    GreaterThanOrEqual,
    LessThanOrEqual,
    NotEqual,
}

/// Environment condition patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentPattern {
    /// Environment variables
    pub env_variables: HashMap<String, ValuePattern>,
    /// System state patterns
    pub system_state: HashMap<String, ValuePattern>,
    /// Configuration patterns
    pub config_patterns: HashMap<String, ValuePattern>,
    /// External service dependencies
    pub service_dependencies: Vec<ServiceDependency>,
}

/// Service dependency pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependency {
    /// Service name
    pub service_name: String,
    /// Expected availability
    pub expected_availability: f32,
    /// Required response time
    pub max_response_time: Option<Duration>,
    /// Health check pattern
    pub health_check: Option<String>,
}

/// Error signature for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSignature {
    /// Error type/category
    pub error_type: String,
    /// Error message patterns
    pub message_patterns: Vec<String>,
    /// Stack trace patterns
    pub stack_trace_patterns: Vec<String>,
    /// Exit code patterns
    pub exit_code_patterns: Vec<i32>,
    /// Error context patterns
    pub context_patterns: HashMap<String, ValuePattern>,
}

/// Risk assessment engine
#[derive(Debug)]
pub struct RiskAssessmentEngine {
    /// Risk factors and their weights
    risk_factors: HashMap<String, f32>,
    /// Risk calculation strategies
    strategies: Vec<Box<dyn RiskStrategy>>,
    /// Historical risk data
    risk_history: VecDeque<RiskAssessment>,
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Unique assessment ID
    pub id: String,
    /// Overall risk score (0.0 - 1.0)
    pub overall_risk: f32,
    /// Individual risk factors
    pub risk_factors: HashMap<String, f32>,
    /// Risk level category
    pub risk_level: RiskLevel,
    /// Confidence in assessment
    pub confidence: f32,
    /// Contributing patterns
    pub contributing_patterns: Vec<String>,
    /// Recommended actions
    pub recommended_actions: Vec<PreventionAction>,
    /// Assessment timestamp
    pub timestamp: SystemTime,
    /// Context information
    pub context: AssessmentContext,
}

/// Risk levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Minimal = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Assessment context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentContext {
    /// Operation being assessed
    pub operation: OperationContext,
    /// Current environment state
    pub environment: EnvironmentContext,
    /// Recent operation history
    pub recent_history: Vec<String>,
    /// System resource state
    pub resource_state: ResourceState,
}

/// Operation context for risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContext {
    /// Operation type
    pub operation_type: String,
    /// Parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Target files/resources
    pub targets: Vec<String>,
    /// User context
    pub user_context: Option<String>,
    /// Session context
    pub session_context: Option<String>,
}

/// Environment context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentContext {
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// System information
    pub system_info: HashMap<String, String>,
    /// Available services
    pub available_services: Vec<String>,
    /// Configuration state
    pub config_state: HashMap<String, String>,
}

/// Current resource state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    /// Memory usage (percentage)
    pub memory_usage: f32,
    /// CPU usage (percentage)
    pub cpu_usage: f32,
    /// Disk usage (percentage)
    pub disk_usage: f32,
    /// Network activity
    pub network_activity: f32,
    /// Open file descriptors
    pub open_files: u32,
    /// Process count
    pub process_count: u32,
}

/// Prevention actions that can be taken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreventionAction {
    /// Block the operation entirely
    Block { reason: String },
    /// Warn user but allow operation
    Warn { message: String },
    /// Modify operation parameters
    ModifyParameters { changes: HashMap<String, serde_json::Value> },
    /// Suggest alternative approach
    SuggestAlternative { alternative: String, reason: String },
    /// Require additional confirmation
    RequireConfirmation { prompt: String },
    /// Delay operation
    Delay { duration: Duration, reason: String },
    /// Retry with different parameters
    RetryWithChanges { parameters: HashMap<String, serde_json::Value> },
    /// Execute pre-requisite actions
    ExecutePrerequisites { actions: Vec<String> },
}

/// Prevention strategies
#[derive(Debug)]
pub enum PreventionStrategy {
    /// Pattern-based prevention
    PatternMatching,
    /// Resource-based prevention
    ResourceMonitoring,
    /// Temporal analysis
    TemporalAnalysis,
    /// Machine learning prediction
    MLPrediction,
    /// Rule-based prevention
    RuleBased,
    /// Ensemble method combining multiple strategies
    Ensemble,
}

/// Learning system for pattern recognition
#[derive(Debug)]
pub struct LearningSystem {
    /// Feature extractors
    feature_extractors: Vec<Box<dyn FeatureExtractor>>,
    /// Pattern recognition models
    models: HashMap<String, Box<dyn PredictionModel>>,
    /// Training data
    training_data: VecDeque<TrainingExample>,
    /// Learning configuration
    config: LearningConfig,
}

/// Feature extractor trait
pub trait FeatureExtractor: Send + Sync + std::fmt::Debug {
    /// Extract features from operation context
    fn extract_features(&self, context: &AssessmentContext) -> Vec<f32>;
    
    /// Get feature names
    fn feature_names(&self) -> Vec<String>;
}

/// Prediction model trait
pub trait PredictionModel: Send + Sync + std::fmt::Debug {
    /// Predict risk based on features
    fn predict(&self, features: &[f32]) -> f32;
    
    /// Update model with new training data
    fn update(&mut self, features: &[f32], target: f32) -> Result<()>;
    
    /// Get model confidence
    fn confidence(&self) -> f32;
}

/// Risk calculation strategy trait
pub trait RiskStrategy: Send + Sync + std::fmt::Debug {
    /// Calculate risk for given context
    fn calculate_risk(&self, context: &AssessmentContext) -> f32;
    
    /// Get strategy name
    fn name(&self) -> &str;
}

/// Training example for learning
#[derive(Debug, Clone)]
pub struct TrainingExample {
    /// Features extracted from context
    pub features: Vec<f32>,
    /// Actual outcome (0.0 = no error, 1.0 = error occurred)
    pub target: f32,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Context metadata
    pub metadata: HashMap<String, String>,
}

/// Historical error data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalError {
    /// Error ID
    pub id: String,
    /// Error details
    pub error_details: ErrorDetails,
    /// Context when error occurred
    pub context: AssessmentContext,
    /// Prevention attempts that were made
    pub prevention_attempts: Vec<PreventionAttempt>,
    /// Whether error was prevented
    pub was_prevented: bool,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Stack trace
    pub stack_trace: Option<String>,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Additional context
    pub additional_context: HashMap<String, String>,
}

/// Prevention attempt record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreventionAttempt {
    /// Attempt ID
    pub id: String,
    /// Strategy used
    pub strategy: String,
    /// Action taken
    pub action: PreventionAction,
    /// Result of prevention
    pub result: PreventionResult,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Result of prevention attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreventionResult {
    /// Successfully prevented error
    Success,
    /// Failed to prevent error
    Failed { reason: String },
    /// User overrode prevention
    UserOverride,
    /// Prevention was not applicable
    NotApplicable,
}

/// Prevention statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreventionStatistics {
    /// Total assessments performed
    pub total_assessments: u64,
    /// Errors successfully prevented
    pub errors_prevented: u64,
    /// False positives (incorrect predictions)
    pub false_positives: u64,
    /// False negatives (missed errors)
    pub false_negatives: u64,
    /// Average prediction accuracy
    pub accuracy: f32,
    /// Precision (true positives / (true positives + false positives))
    pub precision: f32,
    /// Recall (true positives / (true positives + false negatives))
    pub recall: f32,
    /// F1 score
    pub f1_score: f32,
    /// Statistics by risk level
    pub by_risk_level: HashMap<RiskLevel, LevelStatistics>,
}

/// Statistics for specific risk level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelStatistics {
    /// Assessments at this level
    pub assessments: u64,
    /// Correct predictions at this level
    pub correct_predictions: u64,
    /// Accuracy for this level
    pub accuracy: f32,
}

impl PredictiveErrorPrevention {
    /// Create a new predictive error prevention system
    pub async fn new(config: PredictiveConfig) -> Result<Self> {
        let error_patterns = Arc::new(RwLock::new(Vec::new()));
        let risk_assessor = RiskAssessmentEngine::new();
        let prevention_strategies = vec![
            PreventionStrategy::PatternMatching,
            PreventionStrategy::ResourceMonitoring,
            PreventionStrategy::TemporalAnalysis,
        ];
        let learning_system = LearningSystem::new(config.learning.clone());
        let error_history = Arc::new(RwLock::new(VecDeque::new()));
        let stats = Arc::new(RwLock::new(PreventionStatistics::default()));

        let mut system = Self {
            error_patterns,
            risk_assessor,
            prevention_strategies,
            learning_system,
            error_history,
            stats,
            config,
        };

        // Load default patterns
        system.load_default_patterns().await?;

        Ok(system)
    }

    /// Assess risk for a given operation
    pub async fn assess_operation_risk(
        &self,
        operation: OperationContext,
        environment: EnvironmentContext,
    ) -> Result<RiskAssessment> {
        if !self.config.enabled {
            return Ok(RiskAssessment::minimal(operation, environment));
        }

        let context = AssessmentContext {
            operation,
            environment,
            recent_history: self.get_recent_operation_history().await,
            resource_state: self.get_current_resource_state().await?,
        };

        // Pattern matching
        let pattern_risk = self.assess_pattern_risk(&context).await?;
        
        // Resource monitoring
        let resource_risk = self.assess_resource_risk(&context).await?;
        
        // Temporal analysis
        let temporal_risk = self.assess_temporal_risk(&context).await?;
        
        // Machine learning prediction
        let ml_risk = self.learning_system.predict_risk(&context).await?;

        // Combine risks using weighted average
        let overall_risk = self.combine_risk_scores(vec![
            ("pattern", pattern_risk, 0.3),
            ("resource", resource_risk, 0.2),
            ("temporal", temporal_risk, 0.2),
            ("ml", ml_risk, 0.3),
        ]);

        let risk_level = Self::risk_score_to_level(overall_risk);
        let contributing_patterns = self.get_contributing_patterns(&context, overall_risk).await;
        let recommended_actions = self.generate_prevention_actions(&context, overall_risk).await;

        let assessment = RiskAssessment {
            id: Uuid::new_v4().to_string(),
            overall_risk,
            risk_factors: [
                ("pattern".to_string(), pattern_risk),
                ("resource".to_string(), resource_risk),
                ("temporal".to_string(), temporal_risk),
                ("ml".to_string(), ml_risk),
            ].into_iter().collect(),
            risk_level,
            confidence: self.calculate_confidence(&context).await,
            contributing_patterns,
            recommended_actions,
            timestamp: SystemTime::now(),
            context,
        };

        // Update statistics
        self.update_assessment_stats(&assessment).await;

        Ok(assessment)
    }

    /// Execute prevention actions based on risk assessment
    pub async fn execute_prevention(
        &self,
        assessment: &RiskAssessment,
    ) -> Result<Vec<PreventionAttempt>> {
        let mut attempts = Vec::new();

        for action in &assessment.recommended_actions {
            let attempt = self.execute_single_prevention_action(action, assessment).await?;
            
            // Stop if we successfully prevented or user wants to continue anyway
            let should_break = matches!(attempt.result, PreventionResult::Success | PreventionResult::UserOverride);
            attempts.push(attempt);
            
            if should_break {
                break;
            }
        }

        // Record prevention attempts
        self.record_prevention_attempts(&attempts).await;

        Ok(attempts)
    }

    /// Learn from error occurrences to improve prediction
    pub async fn learn_from_error(
        &mut self,
        error_details: ErrorDetails,
        context: AssessmentContext,
        prevention_attempts: Vec<PreventionAttempt>,
    ) -> Result<()> {
        // Create historical error record
        let historical_error = HistoricalError {
            id: Uuid::new_v4().to_string(),
            error_details: error_details.clone(),
            context: context.clone(),
            prevention_attempts: prevention_attempts.clone(),
            was_prevented: false,
            timestamp: SystemTime::now(),
        };

        // Add to history
        {
            let mut history = self.error_history.write().await;
            history.push_back(historical_error);

            // Limit history size
            if history.len() > self.config.learning.max_history_size {
                history.pop_front();
            }
        }

        // Extract patterns from the error
        let new_patterns = self.extract_patterns_from_error(&error_details, &context).await?;
        
        // Update or add patterns
        {
            let mut patterns = self.error_patterns.write().await;
            for new_pattern in new_patterns {
                if let Some(existing) = patterns.iter_mut().find(|p| p.name == new_pattern.name) {
                    // Update existing pattern
                    existing.observation_count += 1;
                    existing.last_observed = SystemTime::now();
                    existing.confidence = self.calculate_pattern_confidence(existing);
                } else {
                    // Add new pattern
                    patterns.push(new_pattern);
                }
            }
        }

        // Update learning system
        let features = self.learning_system.extract_features(&context).await?;
        self.learning_system.update_model(features, 1.0).await?; // 1.0 = error occurred

        // Update statistics
        self.update_error_stats(&error_details, &prevention_attempts).await;

        Ok(())
    }

    /// Get prevention statistics
    pub async fn get_statistics(&self) -> PreventionStatistics {
        self.stats.read().await.clone()
    }

    /// Get current error patterns
    pub async fn get_error_patterns(&self) -> Vec<ErrorPattern> {
        self.error_patterns.read().await.clone()
    }

    /// Add custom error pattern
    pub async fn add_error_pattern(&self, pattern: ErrorPattern) -> Result<()> {
        let mut patterns = self.error_patterns.write().await;
        patterns.push(pattern);
        Ok(())
    }

    // Private implementation methods
    async fn load_default_patterns(&mut self) -> Result<()> {
        let default_patterns = vec![
            self.create_file_not_found_pattern(),
            self.create_permission_denied_pattern(),
            self.create_out_of_memory_pattern(),
            self.create_network_timeout_pattern(),
            self.create_disk_full_pattern(),
        ];

        let mut patterns = self.error_patterns.write().await;
        patterns.extend(default_patterns);

        Ok(())
    }

    fn create_file_not_found_pattern(&self) -> ErrorPattern {
        ErrorPattern {
            id: Uuid::new_v4().to_string(),
            name: "File Not Found".to_string(),
            operation_patterns: vec![OperationPattern {
                operation_type: "file_operation".to_string(),
                parameter_patterns: [
                    ("path".to_string(), ParameterPattern {
                        name: "path".to_string(),
                        value_patterns: vec![ValuePattern::Regex(r".*\.(txt|md|rs|py)$".to_string())],
                        required: true,
                        type_constraint: Some("string".to_string()),
                    })
                ].into_iter().collect(),
                file_patterns: vec!["*".to_string()],
                temporal_aspects: TemporalPattern {
                    preceding_operations: vec!["file_delete".to_string()],
                    timing_constraints: vec![],
                    frequency_patterns: vec![],
                },
                resource_patterns: ResourcePattern {
                    memory_patterns: vec![],
                    cpu_patterns: vec![],
                    disk_patterns: vec![],
                    network_patterns: vec![],
                },
            }],
            environment_patterns: vec![],
            error_signature: ErrorSignature {
                error_type: "FileNotFound".to_string(),
                message_patterns: vec![
                    "No such file or directory".to_string(),
                    "File not found".to_string(),
                    "cannot find".to_string(),
                ],
                stack_trace_patterns: vec![],
                exit_code_patterns: vec![2],
                context_patterns: HashMap::new(),
            },
            prevention_strategies: vec![
                "check_file_exists".to_string(),
                "suggest_similar_files".to_string(),
            ],
            confidence: 0.9,
            observation_count: 0,
            success_count: 0,
            last_observed: SystemTime::now(),
            weight: 1.0,
        }
    }

    fn create_permission_denied_pattern(&self) -> ErrorPattern {
        ErrorPattern {
            id: Uuid::new_v4().to_string(),
            name: "Permission Denied".to_string(),
            operation_patterns: vec![OperationPattern {
                operation_type: "file_operation".to_string(),
                parameter_patterns: HashMap::new(),
                file_patterns: vec!["*".to_string()],
                temporal_aspects: TemporalPattern {
                    preceding_operations: vec![],
                    timing_constraints: vec![],
                    frequency_patterns: vec![],
                },
                resource_patterns: ResourcePattern {
                    memory_patterns: vec![],
                    cpu_patterns: vec![],
                    disk_patterns: vec![],
                    network_patterns: vec![],
                },
            }],
            environment_patterns: vec![],
            error_signature: ErrorSignature {
                error_type: "PermissionDenied".to_string(),
                message_patterns: vec![
                    "Permission denied".to_string(),
                    "Access denied".to_string(),
                    "Operation not permitted".to_string(),
                ],
                stack_trace_patterns: vec![],
                exit_code_patterns: vec![1, 13],
                context_patterns: HashMap::new(),
            },
            prevention_strategies: vec![
                "check_permissions".to_string(),
                "suggest_elevated_privileges".to_string(),
            ],
            confidence: 0.8,
            observation_count: 0,
            success_count: 0,
            last_observed: SystemTime::now(),
            weight: 1.0,
        }
    }

    fn create_out_of_memory_pattern(&self) -> ErrorPattern {
        ErrorPattern {
            id: Uuid::new_v4().to_string(),
            name: "Out of Memory".to_string(),
            operation_patterns: vec![OperationPattern {
                operation_type: "memory_intensive".to_string(),
                parameter_patterns: HashMap::new(),
                file_patterns: vec![],
                temporal_aspects: TemporalPattern {
                    preceding_operations: vec![],
                    timing_constraints: vec![],
                    frequency_patterns: vec![],
                },
                resource_patterns: ResourcePattern {
                    memory_patterns: vec![ResourceThreshold {
                        metric: "memory_usage".to_string(),
                        threshold: 85.0,
                        operator: ComparisonOperator::GreaterThan,
                        risk_level: RiskLevel::High,
                    }],
                    cpu_patterns: vec![],
                    disk_patterns: vec![],
                    network_patterns: vec![],
                },
            }],
            environment_patterns: vec![],
            error_signature: ErrorSignature {
                error_type: "OutOfMemory".to_string(),
                message_patterns: vec![
                    "out of memory".to_string(),
                    "memory allocation failed".to_string(),
                    "cannot allocate memory".to_string(),
                ],
                stack_trace_patterns: vec![],
                exit_code_patterns: vec![12],
                context_patterns: HashMap::new(),
            },
            prevention_strategies: vec![
                "check_memory_usage".to_string(),
                "suggest_memory_cleanup".to_string(),
            ],
            confidence: 0.85,
            observation_count: 0,
            success_count: 0,
            last_observed: SystemTime::now(),
            weight: 1.2,
        }
    }

    fn create_network_timeout_pattern(&self) -> ErrorPattern {
        ErrorPattern {
            id: Uuid::new_v4().to_string(),
            name: "Network Timeout".to_string(),
            operation_patterns: vec![OperationPattern {
                operation_type: "network_operation".to_string(),
                parameter_patterns: HashMap::new(),
                file_patterns: vec![],
                temporal_aspects: TemporalPattern {
                    preceding_operations: vec![],
                    timing_constraints: vec![],
                    frequency_patterns: vec![FrequencyPattern {
                        operation: "network_request".to_string(),
                        time_window: Duration::from_secs(60),
                        max_frequency: 10,
                        risk_multiplier: 1.5,
                    }],
                },
                resource_patterns: ResourcePattern {
                    memory_patterns: vec![],
                    cpu_patterns: vec![],
                    disk_patterns: vec![],
                    network_patterns: vec![ResourceThreshold {
                        metric: "network_latency".to_string(),
                        threshold: 5000.0, // 5 seconds
                        operator: ComparisonOperator::GreaterThan,
                        risk_level: RiskLevel::Medium,
                    }],
                },
            }],
            environment_patterns: vec![EnvironmentPattern {
                env_variables: HashMap::new(),
                system_state: HashMap::new(),
                config_patterns: HashMap::new(),
                service_dependencies: vec![ServiceDependency {
                    service_name: "internet".to_string(),
                    expected_availability: 0.99,
                    max_response_time: Some(Duration::from_secs(5)),
                    health_check: Some("ping".to_string()),
                }],
            }],
            error_signature: ErrorSignature {
                error_type: "NetworkTimeout".to_string(),
                message_patterns: vec![
                    "timeout".to_string(),
                    "connection timed out".to_string(),
                    "no response".to_string(),
                ],
                stack_trace_patterns: vec![],
                exit_code_patterns: vec![],
                context_patterns: HashMap::new(),
            },
            prevention_strategies: vec![
                "check_network_connectivity".to_string(),
                "increase_timeout".to_string(),
                "retry_with_backoff".to_string(),
            ],
            confidence: 0.75,
            observation_count: 0,
            success_count: 0,
            last_observed: SystemTime::now(),
            weight: 0.8,
        }
    }

    fn create_disk_full_pattern(&self) -> ErrorPattern {
        ErrorPattern {
            id: Uuid::new_v4().to_string(),
            name: "Disk Full".to_string(),
            operation_patterns: vec![OperationPattern {
                operation_type: "file_write".to_string(),
                parameter_patterns: HashMap::new(),
                file_patterns: vec!["*".to_string()],
                temporal_aspects: TemporalPattern {
                    preceding_operations: vec![],
                    timing_constraints: vec![],
                    frequency_patterns: vec![],
                },
                resource_patterns: ResourcePattern {
                    memory_patterns: vec![],
                    cpu_patterns: vec![],
                    disk_patterns: vec![ResourceThreshold {
                        metric: "disk_usage".to_string(),
                        threshold: 95.0,
                        operator: ComparisonOperator::GreaterThan,
                        risk_level: RiskLevel::Critical,
                    }],
                    network_patterns: vec![],
                },
            }],
            environment_patterns: vec![],
            error_signature: ErrorSignature {
                error_type: "DiskFull".to_string(),
                message_patterns: vec![
                    "no space left on device".to_string(),
                    "disk full".to_string(),
                    "insufficient storage".to_string(),
                ],
                stack_trace_patterns: vec![],
                exit_code_patterns: vec![28],
                context_patterns: HashMap::new(),
            },
            prevention_strategies: vec![
                "check_disk_space".to_string(),
                "cleanup_temp_files".to_string(),
                "suggest_alternative_location".to_string(),
            ],
            confidence: 0.95,
            observation_count: 0,
            success_count: 0,
            last_observed: SystemTime::now(),
            weight: 1.5,
        }
    }

    async fn assess_pattern_risk(&self, context: &AssessmentContext) -> Result<f32> {
        let patterns = self.error_patterns.read().await;
        let mut max_risk: f32 = 0.0;

        for pattern in patterns.iter() {
            let pattern_match_score = self.calculate_pattern_match(pattern, context).await;
            let risk_contribution = pattern_match_score * pattern.confidence * pattern.weight;
            max_risk = max_risk.max(risk_contribution);
        }

        Ok(max_risk.clamp(0.0, 1.0))
    }

    async fn calculate_pattern_match(&self, pattern: &ErrorPattern, context: &AssessmentContext) -> f32 {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        // Check operation patterns
        for op_pattern in &pattern.operation_patterns {
            if op_pattern.operation_type == context.operation.operation_type {
                let op_score = self.match_operation_pattern(op_pattern, context);
                total_score += op_score * 0.5;
                total_weight += 0.5;
            }
        }

        // Check environment patterns
        for env_pattern in &pattern.environment_patterns {
            let env_score = self.match_environment_pattern(env_pattern, context);
            total_score += env_score * 0.3;
            total_weight += 0.3;
        }

        // Check resource patterns
        let resource_score = self.match_resource_patterns(pattern, context);
        total_score += resource_score * 0.2;
        total_weight += 0.2;

        if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.0
        }
    }

    fn match_operation_pattern(&self, pattern: &OperationPattern, context: &AssessmentContext) -> f32 {
        let mut match_score = 0.0;
        let mut checks = 0;

        // Check parameter patterns
        for (param_name, param_pattern) in &pattern.parameter_patterns {
            checks += 1;
            if let Some(param_value) = context.operation.parameters.get(param_name) {
                if self.match_parameter_pattern(param_pattern, param_value) {
                    match_score += 1.0;
                }
            }
        }

        // Check file patterns
        for file_pattern in &pattern.file_patterns {
            checks += 1;
            for target in &context.operation.targets {
                if glob_match::glob_match(file_pattern, target) {
                    match_score += 1.0;
                    break;
                }
            }
        }

        if checks > 0 {
            match_score / checks as f32
        } else {
            0.0
        }
    }

    fn match_parameter_pattern(&self, pattern: &ParameterPattern, value: &serde_json::Value) -> bool {
        for value_pattern in &pattern.value_patterns {
            if self.match_value_pattern(value_pattern, value) {
                return true;
            }
        }
        false
    }

    fn match_value_pattern(&self, pattern: &ValuePattern, value: &serde_json::Value) -> bool {
        match pattern {
            ValuePattern::Exact(expected) => {
                value.as_str().map_or(false, |s| s == expected)
            }
            ValuePattern::Contains(substring) => {
                value.as_str().map_or(false, |s| s.contains(substring))
            }
            ValuePattern::StartsWith(prefix) => {
                value.as_str().map_or(false, |s| s.starts_with(prefix))
            }
            ValuePattern::EndsWith(suffix) => {
                value.as_str().map_or(false, |s| s.ends_with(suffix))
            }
            ValuePattern::Regex(regex_str) => {
                if let Ok(regex) = regex::Regex::new(regex_str) {
                    value.as_str().map_or(false, |s| regex.is_match(s))
                } else {
                    false
                }
            }
            ValuePattern::Range { min, max } => {
                value.as_f64().map_or(false, |n| n >= *min && n <= *max)
            }
            ValuePattern::Enum(options) => {
                value.as_str().map_or(false, |s| options.contains(&s.to_string()))
            }
            ValuePattern::Length { min, max } => {
                value.as_str().map_or(false, |s| {
                    let len = s.len();
                    min.map_or(true, |m| len >= m) && max.map_or(true, |m| len <= m)
                })
            }
        }
    }

    fn match_environment_pattern(&self, pattern: &EnvironmentPattern, context: &AssessmentContext) -> f32 {
        let mut match_score = 0.0;
        let mut checks = 0;

        // Check environment variables
        for (env_var, value_pattern) in &pattern.env_variables {
            checks += 1;
            if let Some(env_value) = context.environment.env_vars.get(env_var) {
                let json_value = serde_json::Value::String(env_value.clone());
                if self.match_value_pattern(value_pattern, &json_value) {
                    match_score += 1.0;
                }
            }
        }

        // Check system state
        for (state_key, value_pattern) in &pattern.system_state {
            checks += 1;
            if let Some(state_value) = context.environment.system_info.get(state_key) {
                let json_value = serde_json::Value::String(state_value.clone());
                if self.match_value_pattern(value_pattern, &json_value) {
                    match_score += 1.0;
                }
            }
        }

        if checks > 0 {
            match_score / checks as f32
        } else {
            0.0
        }
    }

    fn match_resource_patterns(&self, pattern: &ErrorPattern, context: &AssessmentContext) -> f32 {
        let mut risk_score: f64 = 0.0;

        for op_pattern in &pattern.operation_patterns {
            // Check memory patterns
            for threshold in &op_pattern.resource_patterns.memory_patterns {
                if threshold.metric == "memory_usage" {
                    let exceeds_threshold = self.check_threshold(
                        context.resource_state.memory_usage as f64,
                        threshold.threshold,
                        &threshold.operator,
                    );
                    if exceeds_threshold {
                        risk_score = risk_score.max(self.risk_level_to_score(&threshold.risk_level) as f64);
                    }
                }
            }

            // Check disk patterns
            for threshold in &op_pattern.resource_patterns.disk_patterns {
                if threshold.metric == "disk_usage" {
                    let exceeds_threshold = self.check_threshold(
                        context.resource_state.disk_usage as f64,
                        threshold.threshold,
                        &threshold.operator,
                    );
                    if exceeds_threshold {
                        risk_score = risk_score.max(self.risk_level_to_score(&threshold.risk_level) as f64);
                    }
                }
            }

            // Check CPU patterns
            for threshold in &op_pattern.resource_patterns.cpu_patterns {
                if threshold.metric == "cpu_usage" {
                    let exceeds_threshold = self.check_threshold(
                        context.resource_state.cpu_usage as f64,
                        threshold.threshold,
                        &threshold.operator,
                    );
                    if exceeds_threshold {
                        risk_score = risk_score.max(self.risk_level_to_score(&threshold.risk_level) as f64);
                    }
                }
            }
        }

        risk_score as f32
    }

    fn check_threshold(&self, value: f64, threshold: f64, operator: &ComparisonOperator) -> bool {
        match operator {
            ComparisonOperator::GreaterThan => value > threshold,
            ComparisonOperator::LessThan => value < threshold,
            ComparisonOperator::Equal => (value - threshold).abs() < f64::EPSILON,
            ComparisonOperator::GreaterThanOrEqual => value >= threshold,
            ComparisonOperator::LessThanOrEqual => value <= threshold,
            ComparisonOperator::NotEqual => (value - threshold).abs() >= f64::EPSILON,
        }
    }

    fn risk_level_to_score(&self, level: &RiskLevel) -> f32 {
        match level {
            RiskLevel::Minimal => 0.0,
            RiskLevel::Low => 0.2,
            RiskLevel::Medium => 0.5,
            RiskLevel::High => 0.8,
            RiskLevel::Critical => 1.0,
        }
    }

    async fn assess_resource_risk(&self, context: &AssessmentContext) -> Result<f32> {
        let resource_state = &context.resource_state;
        let mut risk_factors = Vec::new();

        // Memory risk
        if resource_state.memory_usage > 90.0 {
            risk_factors.push(0.9);
        } else if resource_state.memory_usage > 80.0 {
            risk_factors.push(0.6);
        } else if resource_state.memory_usage > 70.0 {
            risk_factors.push(0.3);
        }

        // CPU risk
        if resource_state.cpu_usage > 95.0 {
            risk_factors.push(0.8);
        } else if resource_state.cpu_usage > 85.0 {
            risk_factors.push(0.5);
        }

        // Disk risk
        if resource_state.disk_usage > 95.0 {
            risk_factors.push(1.0);
        } else if resource_state.disk_usage > 90.0 {
            risk_factors.push(0.7);
        } else if resource_state.disk_usage > 80.0 {
            risk_factors.push(0.4);
        }

        // Open files risk
        if resource_state.open_files > 1000 {
            risk_factors.push(0.6);
        }

        if risk_factors.is_empty() {
            Ok(0.0)
        } else {
            Ok(risk_factors.iter().sum::<f32>() / risk_factors.len() as f32)
        }
    }

    async fn assess_temporal_risk(&self, context: &AssessmentContext) -> Result<f32> {
        // Simplified temporal analysis
        let recent_operations = &context.recent_history;
        
        // Check for rapid repeated operations
        let operation_type = &context.operation.operation_type;
        let recent_same_ops = recent_operations.iter()
            .filter(|op| op == &operation_type)
            .count();

        let frequency_risk = if recent_same_ops > 10 {
            0.8
        } else if recent_same_ops > 5 {
            0.5
        } else if recent_same_ops > 2 {
            0.2
        } else {
            0.0
        };

        Ok(frequency_risk)
    }

    fn combine_risk_scores(&self, risks: Vec<(&str, f32, f32)>) -> f32 {
        let total_weight: f32 = risks.iter().map(|(_, _, weight)| weight).sum();
        if total_weight == 0.0 {
            return 0.0;
        }

        let weighted_sum: f32 = risks.iter()
            .map(|(_, score, weight)| score * weight)
            .sum();

        (weighted_sum / total_weight).clamp(0.0, 1.0)
    }

    fn risk_score_to_level(score: f32) -> RiskLevel {
        if score >= 0.8 {
            RiskLevel::Critical
        } else if score >= 0.6 {
            RiskLevel::High
        } else if score >= 0.4 {
            RiskLevel::Medium
        } else if score >= 0.2 {
            RiskLevel::Low
        } else {
            RiskLevel::Minimal
        }
    }

    async fn get_contributing_patterns(&self, context: &AssessmentContext, _risk_score: f32) -> Vec<String> {
        let patterns = self.error_patterns.read().await;
        let mut contributing = Vec::new();

        for pattern in patterns.iter() {
            let pattern_match = self.calculate_pattern_match(pattern, context).await;
            if pattern_match > 0.5 {
                contributing.push(pattern.name.clone());
            }
        }

        contributing
    }

    async fn generate_prevention_actions(&self, context: &AssessmentContext, risk_score: f32) -> Vec<PreventionAction> {
        let mut actions = Vec::new();

        if risk_score >= 0.8 {
            actions.push(PreventionAction::Block {
                reason: "High risk of operation failure detected".to_string(),
            });
        } else if risk_score >= 0.6 {
            actions.push(PreventionAction::RequireConfirmation {
                prompt: "This operation has elevated risk. Continue anyway?".to_string(),
            });
        } else if risk_score >= 0.4 {
            actions.push(PreventionAction::Warn {
                message: "This operation may fail. Consider the suggested alternatives.".to_string(),
            });
        }

        // Add specific actions based on context
        if context.resource_state.disk_usage > 90.0 {
            actions.push(PreventionAction::SuggestAlternative {
                alternative: "Clean up disk space before proceeding".to_string(),
                reason: "Low disk space detected".to_string(),
            });
        }

        if context.resource_state.memory_usage > 85.0 {
            actions.push(PreventionAction::SuggestAlternative {
                alternative: "Close other applications to free memory".to_string(),
                reason: "High memory usage detected".to_string(),
            });
        }

        actions
    }

    async fn calculate_confidence(&self, context: &AssessmentContext) -> f32 {
        // Simplified confidence calculation based on pattern matches and historical data
        let patterns = self.error_patterns.read().await;
        let matching_patterns = patterns.iter()
            .filter(|p| {
                let pattern_match = futures::executor::block_on(
                    self.calculate_pattern_match(p, context)
                );
                pattern_match > 0.3
            })
            .count();

        if matching_patterns > 3 {
            0.9
        } else if matching_patterns > 1 {
            0.7
        } else if matching_patterns > 0 {
            0.5
        } else {
            0.3
        }
    }

    async fn get_recent_operation_history(&self) -> Vec<String> {
        // This would typically come from the operation history system
        // For now, return empty vector
        Vec::new()
    }

    async fn get_current_resource_state(&self) -> Result<ResourceState> {
        // This would typically query system resources
        // For now, return mock data
        Ok(ResourceState {
            memory_usage: 45.0,
            cpu_usage: 25.0,
            disk_usage: 60.0,
            network_activity: 10.0,
            open_files: 150,
            process_count: 200,
        })
    }

    async fn execute_single_prevention_action(
        &self,
        action: &PreventionAction,
        _assessment: &RiskAssessment,
    ) -> Result<PreventionAttempt> {
        let attempt_id = Uuid::new_v4().to_string();
        let strategy = "default".to_string();
        let timestamp = SystemTime::now();

        // Simulate action execution
        let result = match action {
            PreventionAction::Block { .. } => PreventionResult::Success,
            PreventionAction::Warn { .. } => {
                // In a real implementation, this would show the warning to the user
                println!("Warning: {}", 
                    if let PreventionAction::Warn { message } = action { message } else { "Unknown warning" });
                PreventionResult::Success
            }
            PreventionAction::RequireConfirmation { prompt } => {
                // In a real implementation, this would prompt the user
                println!("Confirmation required: {}", prompt);
                // For testing, assume user continues
                PreventionResult::UserOverride
            }
            _ => PreventionResult::Success,
        };

        Ok(PreventionAttempt {
            id: attempt_id,
            strategy,
            action: action.clone(),
            result,
            timestamp,
        })
    }

    async fn record_prevention_attempts(&self, attempts: &[PreventionAttempt]) {
        // In a real implementation, this would store the attempts for analysis
        for attempt in attempts {
            println!("Prevention attempt: {} - {:?}", attempt.strategy, attempt.result);
        }
    }

    async fn update_assessment_stats(&self, assessment: &RiskAssessment) {
        let mut stats = self.stats.write().await;
        stats.total_assessments += 1;

        // Update level-specific statistics
        let level_stats = stats.by_risk_level.entry(assessment.risk_level.clone()).or_insert(LevelStatistics {
            assessments: 0,
            correct_predictions: 0,
            accuracy: 0.0,
        });
        level_stats.assessments += 1;
    }

    async fn update_error_stats(&self, _error_details: &ErrorDetails, prevention_attempts: &[PreventionAttempt]) {
        let mut stats = self.stats.write().await;
        
        let was_prevented = prevention_attempts.iter()
            .any(|attempt| matches!(attempt.result, PreventionResult::Success));

        if was_prevented {
            stats.errors_prevented += 1;
        } else {
            stats.false_negatives += 1;
        }

        // Recalculate accuracy, precision, recall
        let total_positive_predictions = stats.errors_prevented + stats.false_positives;
        let total_actual_errors = stats.errors_prevented + stats.false_negatives;

        if total_positive_predictions > 0 {
            stats.precision = stats.errors_prevented as f32 / total_positive_predictions as f32;
        }

        if total_actual_errors > 0 {
            stats.recall = stats.errors_prevented as f32 / total_actual_errors as f32;
        }

        if stats.precision + stats.recall > 0.0 {
            stats.f1_score = 2.0 * (stats.precision * stats.recall) / (stats.precision + stats.recall);
        }

        let total_assessments = stats.total_assessments;
        if total_assessments > 0 {
            stats.accuracy = (stats.errors_prevented + (total_assessments - stats.false_positives - stats.false_negatives)) as f32 / total_assessments as f32;
        }
    }

    async fn extract_patterns_from_error(&self, _error_details: &ErrorDetails, _context: &AssessmentContext) -> Result<Vec<ErrorPattern>> {
        // This would implement pattern extraction from error data
        // For now, return empty vector
        Ok(Vec::new())
    }

    fn calculate_pattern_confidence(&self, pattern: &ErrorPattern) -> f32 {
        if pattern.observation_count == 0 {
            return 0.0;
        }

        let success_rate = pattern.success_count as f32 / pattern.observation_count as f32;
        let observation_factor = (pattern.observation_count as f32).log10().min(1.0);
        
        success_rate * observation_factor
    }
}

// Implementation of helper structs and traits
impl RiskAssessmentEngine {
    fn new() -> Self {
        let mut risk_factors = HashMap::new();
        risk_factors.insert("pattern_match".to_string(), 0.4);
        risk_factors.insert("resource_usage".to_string(), 0.3);
        risk_factors.insert("temporal_factors".to_string(), 0.2);
        risk_factors.insert("environment".to_string(), 0.1);

        Self {
            risk_factors,
            strategies: Vec::new(),
            risk_history: VecDeque::new(),
        }
    }
}

impl LearningSystem {
    fn new(config: LearningConfig) -> Self {
        Self {
            feature_extractors: Vec::new(),
            models: HashMap::new(),
            training_data: VecDeque::new(),
            config,
        }
    }

    async fn predict_risk(&self, _context: &AssessmentContext) -> Result<f32> {
        // Simplified ML prediction
        // In a real implementation, this would use trained models
        Ok(0.5) // Default neutral prediction
    }

    async fn extract_features(&self, context: &AssessmentContext) -> Result<Vec<f32>> {
        // Extract basic features from context
        let features = vec![
            context.resource_state.memory_usage / 100.0,
            context.resource_state.cpu_usage / 100.0,
            context.resource_state.disk_usage / 100.0,
            context.recent_history.len() as f32 / 10.0,
            context.operation.parameters.len() as f32 / 5.0,
        ];

        Ok(features)
    }

    async fn update_model(&mut self, features: Vec<f32>, target: f32) -> Result<()> {
        let training_example = TrainingExample {
            features,
            target,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        };

        self.training_data.push_back(training_example);

        // Limit training data size
        if self.training_data.len() > self.config.max_history_size {
            self.training_data.pop_front();
        }

        Ok(())
    }
}

impl RiskAssessment {
    fn minimal(operation: OperationContext, environment: EnvironmentContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            overall_risk: 0.0,
            risk_factors: HashMap::new(),
            risk_level: RiskLevel::Minimal,
            confidence: 1.0,
            contributing_patterns: Vec::new(),
            recommended_actions: Vec::new(),
            timestamp: SystemTime::now(),
            context: AssessmentContext {
                operation,
                environment,
                recent_history: Vec::new(),
                resource_state: ResourceState {
                    memory_usage: 0.0,
                    cpu_usage: 0.0,
                    disk_usage: 0.0,
                    network_activity: 0.0,
                    open_files: 0,
                    process_count: 0,
                },
            },
        }
    }
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            confidence_threshold: 0.7,
            max_risk_threshold: 0.8,
            learning: LearningConfig {
                online_learning: true,
                learning_rate: 0.01,
                min_pattern_samples: 5,
                pattern_decay_rate: 0.1,
                max_history_size: 1000,
            },
            pattern_matching: PatternMatchingConfig {
                fuzzy_matching: true,
                similarity_threshold: 0.8,
                context_window: 10,
                temporal_patterns: true,
            },
            strategy_settings: StrategySettings {
                auto_prevention: false,
                user_warnings: true,
                alternative_suggestions: true,
                max_prevention_attempts: 3,
            },
        }
    }
}

impl Default for PreventionStatistics {
    fn default() -> Self {
        Self {
            total_assessments: 0,
            errors_prevented: 0,
            false_positives: 0,
            false_negatives: 0,
            accuracy: 0.0,
            precision: 0.0,
            recall: 0.0,
            f1_score: 0.0,
            by_risk_level: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_predictive_error_prevention_creation() {
        let config = PredictiveConfig::default();
        let system = PredictiveErrorPrevention::new(config).await.unwrap();
        
        let stats = system.get_statistics().await;
        assert_eq!(stats.total_assessments, 0);
    }

    #[tokio::test]
    async fn test_risk_assessment() {
        let config = PredictiveConfig::default();
        let system = PredictiveErrorPrevention::new(config).await.unwrap();

        let operation = OperationContext {
            operation_type: "file_operation".to_string(),
            parameters: HashMap::new(),
            targets: vec!["/nonexistent/file.txt".to_string()],
            user_context: None,
            session_context: None,
        };

        let environment = EnvironmentContext {
            env_vars: HashMap::new(),
            system_info: HashMap::new(),
            available_services: Vec::new(),
            config_state: HashMap::new(),
        };

        let assessment = system.assess_operation_risk(operation, environment).await.unwrap();
        
        assert!(assessment.overall_risk >= 0.0 && assessment.overall_risk <= 1.0);
        assert!(!assessment.id.is_empty());
    }

    #[tokio::test]
    async fn test_pattern_matching() {
        let config = PredictiveConfig::default();
        let system = PredictiveErrorPrevention::new(config).await.unwrap();

        let patterns = system.get_error_patterns().await;
        assert!(!patterns.is_empty());
        
        // Should have default patterns
        let pattern_names: Vec<String> = patterns.iter().map(|p| p.name.clone()).collect();
        assert!(pattern_names.contains(&"File Not Found".to_string()));
        assert!(pattern_names.contains(&"Permission Denied".to_string()));
    }
}