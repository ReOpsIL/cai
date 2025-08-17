use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::task_executor::{Task, TaskStatus, McpToolCall};

/// Represents a single execution step with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_id: String,
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Vec<McpToolCall>,
    pub execution_metadata: ExecutionMetadata,
    pub result: ExecutionStepResult,
}

/// Rich metadata about execution environment and results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub working_directory: Option<String>,
    pub environment_changes: HashMap<String, String>,
    pub files_modified: Vec<String>,
    pub files_created: Vec<String>,
    pub files_deleted: Vec<String>,
    pub execution_time_ms: u64,
    pub memory_usage_mb: Option<f64>,
    pub tool_count: usize,
    pub error_count: usize,
}

/// Result of an execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStepResult {
    Success {
        output: String,
        confidence_score: f64,
        impact_assessment: ImpactAssessment,
    },
    Failure {
        error: String,
        error_type: ErrorType,
        recovery_suggestions: Vec<String>,
    },
    PartialSuccess {
        completed_parts: Vec<String>,
        failed_parts: Vec<String>,
        overall_impact: ImpactAssessment,
    },
}

/// Assessment of the impact of an execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub progress_made: f64, // 0.0 to 1.0
    pub goal_alignment: f64, // 0.0 to 1.0
    pub risk_level: RiskLevel,
    pub next_action_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    ToolFailure,
    PermissionDenied,
    ResourceUnavailable,
    NetworkError,
    ValidationError,
    UnexpectedOutput,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Comprehensive execution context that accumulates state and results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub session_id: String,
    pub workflow_id: String,
    pub started_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub execution_steps: Vec<ExecutionStep>,
    pub accumulated_state: AccumulatedState,
    pub pattern_history: Vec<ExecutionPattern>,
    pub decision_points: Vec<DecisionPoint>,
}

/// Accumulated state across all execution steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatedState {
    pub total_steps: usize,
    pub successful_steps: usize,
    pub failed_steps: usize,
    pub total_execution_time_ms: u64,
    pub files_modified_total: Vec<String>,
    pub working_directories_used: Vec<String>,
    pub tools_used: HashMap<String, usize>, // tool_name -> usage_count
    pub error_patterns: HashMap<String, usize>, // error_type -> count
    pub progress_trajectory: Vec<ProgressPoint>,
}

/// Point in execution progress for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPoint {
    pub timestamp: DateTime<Utc>,
    pub step_index: usize,
    pub cumulative_progress: f64,
    pub confidence_level: f64,
}

/// Pattern detection for execution behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub detected_at: DateTime<Utc>,
    pub confidence: f64,
    pub steps_involved: Vec<String>, // step_ids
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    RepetitiveAction,
    ErrorLoop,
    ProgressStall,
    RapidProgress,
    ContextSwitch,
    ResourceHeavy,
}

/// Decision points where execution flow could change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPoint {
    pub decision_id: String,
    pub timestamp: DateTime<Utc>,
    pub context_summary: String,
    pub options_considered: Vec<String>,
    pub decision_made: String,
    pub reasoning: String,
    pub confidence: f64,
}

/// Reasons for continuing or changing execution flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContinuationDecision {
    Continue,
    Replan(ReplanningContext),
    Complete,
    Error(ErrorRecoveryContext),
    UserIntervention(InterventionRequest),
}

/// Context for replanning decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanningContext {
    pub trigger_reason: String,
    pub current_state_summary: String,
    pub problematic_patterns: Vec<ExecutionPattern>,
    pub suggested_approach: String,
    pub confidence_in_suggestion: f64,
}

/// Context for error recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryContext {
    pub error_summary: String,
    pub recovery_strategies: Vec<RecoveryStrategy>,
    pub estimated_recovery_time: Option<u64>, // milliseconds
    pub risk_of_continuation: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    pub strategy_id: String,
    pub description: String,
    pub estimated_success_rate: f64,
    pub required_resources: Vec<String>,
    pub estimated_time: u64, // milliseconds
}

/// Request for user intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionRequest {
    pub reason: String,
    pub context: String,
    pub suggested_user_actions: Vec<String>,
    pub urgency_level: UrgencyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl ExecutionContext {
    pub fn new(session_id: String, workflow_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            workflow_id,
            started_at: now,
            last_updated: now,
            execution_steps: Vec::new(),
            accumulated_state: AccumulatedState {
                total_steps: 0,
                successful_steps: 0,
                failed_steps: 0,
                total_execution_time_ms: 0,
                files_modified_total: Vec::new(),
                working_directories_used: Vec::new(),
                tools_used: HashMap::new(),
                error_patterns: HashMap::new(),
                progress_trajectory: Vec::new(),
            },
            pattern_history: Vec::new(),
            decision_points: Vec::new(),
        }
    }

    /// Add a new execution step and update accumulated state
    pub fn add_execution_step(&mut self, step: ExecutionStep) {
        self.last_updated = Utc::now();
        
        // Update accumulated state
        self.accumulated_state.total_steps += 1;
        self.accumulated_state.total_execution_time_ms += step.execution_metadata.execution_time_ms;
        
        // Track success/failure
        match &step.result {
            ExecutionStepResult::Success { confidence_score, impact_assessment, .. } => {
                self.accumulated_state.successful_steps += 1;
                self.accumulated_state.progress_trajectory.push(ProgressPoint {
                    timestamp: step.timestamp,
                    step_index: self.accumulated_state.total_steps - 1,
                    cumulative_progress: impact_assessment.progress_made,
                    confidence_level: *confidence_score,
                });
            }
            ExecutionStepResult::Failure { error_type, .. } => {
                self.accumulated_state.failed_steps += 1;
                let error_key = format!("{:?}", error_type);
                *self.accumulated_state.error_patterns.entry(error_key).or_insert(0) += 1;
            }
            ExecutionStepResult::PartialSuccess { overall_impact, .. } => {
                self.accumulated_state.successful_steps += 1; // Partial success counts as progress
                self.accumulated_state.progress_trajectory.push(ProgressPoint {
                    timestamp: step.timestamp,
                    step_index: self.accumulated_state.total_steps - 1,
                    cumulative_progress: overall_impact.progress_made,
                    confidence_level: overall_impact.goal_alignment,
                });
            }
        }
        
        // Track tool usage
        for tool_call in &step.tool_calls {
            *self.accumulated_state.tools_used.entry(tool_call.tool_name.clone()).or_insert(0) += 1;
        }
        
        // Track file modifications
        for file in &step.execution_metadata.files_modified {
            if !self.accumulated_state.files_modified_total.contains(file) {
                self.accumulated_state.files_modified_total.push(file.clone());
            }
        }
        
        // Track working directory
        if let Some(wd) = &step.execution_metadata.working_directory {
            if !self.accumulated_state.working_directories_used.contains(wd) {
                self.accumulated_state.working_directories_used.push(wd.clone());
            }
        }
        
        self.execution_steps.push(step);
    }

    /// Extract planning context for LLM-based replanning
    pub fn extract_planning_context(&self) -> Value {
        serde_json::json!({
            "session_summary": {
                "session_id": self.session_id,
                "workflow_id": self.workflow_id,
                "duration_minutes": (Utc::now() - self.started_at).num_minutes(),
                "total_steps": self.accumulated_state.total_steps,
                "success_rate": self.calculate_success_rate(),
            },
            "execution_summary": {
                "total_execution_time_ms": self.accumulated_state.total_execution_time_ms,
                "files_affected": self.accumulated_state.files_modified_total.len(),
                "tools_used": self.accumulated_state.tools_used,
                "working_directories": self.accumulated_state.working_directories_used,
            },
            "recent_patterns": self.pattern_history.iter().rev().take(5).collect::<Vec<_>>(),
            "error_analysis": self.accumulated_state.error_patterns,
            "progress_trend": self.analyze_progress_trend(),
            "recent_steps": self.execution_steps.iter().rev().take(3).collect::<Vec<_>>(),
        })
    }

    /// Calculate success rate of execution steps
    pub fn calculate_success_rate(&self) -> f64 {
        if self.accumulated_state.total_steps == 0 {
            return 0.0;
        }
        self.accumulated_state.successful_steps as f64 / self.accumulated_state.total_steps as f64
    }

    /// Analyze progress trend from recent trajectory
    pub fn analyze_progress_trend(&self) -> String {
        let recent_points = self.accumulated_state.progress_trajectory
            .iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>();
        
        if recent_points.len() < 2 {
            return "insufficient_data".to_string();
        }
        
        let trend = recent_points.windows(2)
            .map(|window| window[0].cumulative_progress - window[1].cumulative_progress)
            .collect::<Vec<_>>();
        
        let avg_change = trend.iter().sum::<f64>() / trend.len() as f64;
        
        match avg_change {
            x if x > 0.1 => "rapid_progress".to_string(),
            x if x > 0.05 => "steady_progress".to_string(),
            x if x > -0.05 => "stable".to_string(),
            x if x > -0.1 => "slow_decline".to_string(),
            _ => "stalled_or_regressing".to_string(),
        }
    }

    /// Get the most recent execution steps for analysis
    pub fn get_recent_steps(&self, count: usize) -> Vec<&ExecutionStep> {
        self.execution_steps.iter().rev().take(count).collect()
    }

    /// Add a decision point to track execution decisions
    pub fn add_decision_point(&mut self, decision: DecisionPoint) {
        self.decision_points.push(decision);
        self.last_updated = Utc::now();
    }

    /// Add a detected pattern to the history
    pub fn add_pattern(&mut self, pattern: ExecutionPattern) {
        self.pattern_history.push(pattern);
        self.last_updated = Utc::now();
    }

    /// Check if execution should continue based on accumulated context
    pub fn should_continue(&self) -> bool {
        // Basic continuation logic - can be enhanced
        let success_rate = self.calculate_success_rate();
        let recent_failures = self.execution_steps.iter()
            .rev()
            .take(3)
            .filter(|step| matches!(step.result, ExecutionStepResult::Failure { .. }))
            .count();
        
        // Continue if success rate is reasonable and not too many recent failures
        success_rate > 0.3 && recent_failures < 3
    }

    /// Get execution summary for display or logging
    pub fn get_execution_summary(&self) -> String {
        format!(
            "Execution Summary - Steps: {}, Success Rate: {:.1}%, Duration: {}min, Tools Used: {}",
            self.accumulated_state.total_steps,
            self.calculate_success_rate() * 100.0,
            (Utc::now() - self.started_at).num_minutes(),
            self.accumulated_state.tools_used.len()
        )
    }
}

impl ExecutionStep {
    pub fn from_task(task: &Task, _execution_time_ms: u64, metadata: ExecutionMetadata) -> Self {
        let result = match &task.status {
            TaskStatus::Done => {
                ExecutionStepResult::Success {
                    output: task.result.clone().unwrap_or_default(),
                    confidence_score: 0.8, // Default confidence
                    impact_assessment: ImpactAssessment {
                        progress_made: 1.0,
                        goal_alignment: 0.8,
                        risk_level: RiskLevel::Low,
                        next_action_suggestions: vec!["Continue with next task".to_string()],
                    },
                }
            }
            TaskStatus::Failed => {
                ExecutionStepResult::Failure {
                    error: task.error.clone().unwrap_or("Unknown error".to_string()),
                    error_type: ErrorType::ToolFailure, // Default error type
                    recovery_suggestions: vec!["Retry with different approach".to_string()],
                }
            }
            _ => {
                ExecutionStepResult::Success {
                    output: "In progress".to_string(),
                    confidence_score: 0.5,
                    impact_assessment: ImpactAssessment {
                        progress_made: 0.0,
                        goal_alignment: 0.5,
                        risk_level: RiskLevel::Low,
                        next_action_suggestions: vec!["Continue execution".to_string()],
                    },
                }
            }
        };

        Self {
            step_id: uuid::Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            timestamp: Utc::now(),
            tool_calls: task.mcp_tool_calls.clone(),
            execution_metadata: metadata,
            result,
        }
    }
}