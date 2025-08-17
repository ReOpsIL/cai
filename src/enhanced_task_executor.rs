use anyhow::{anyhow, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::logger::{log_debug, log_error, log_info, log_warn, ops};
use crate::mcp_manager;
use crate::openrouter_client::{OpenRouterClient, ToolMetadata};
use crate::task_executor::{Task, TaskStatus};
use crate::workflow_timeouts::execute_with_adaptive_timeout;

/// Enhanced Task Executor implementing Claude Code's two-stage execution pattern
/// Stage 1: Strategic decomposition and planning
/// Stage 2: Systematic execution with context preservation
#[derive(Debug)]
pub struct EnhancedTaskExecutor {
    openrouter_client: Option<Arc<OpenRouterClient>>,
    task_queue: Arc<Mutex<VecDeque<Task>>>,
    execution_context: Arc<Mutex<ExecutionContext>>,
    recovery_strategies: Vec<RecoveryStrategy>,
    thinking_budget: ThinkingBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub session_id: String,
    pub codebase_structure: Option<CodebaseMap>,
    pub execution_history: Vec<ExecutionRecord>,
    pub architectural_patterns: Vec<ArchitecturalPattern>,
    pub risk_assessment: RiskAssessment,
    pub available_tools: Vec<ToolMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseMap {
    pub root_directory: String,
    pub file_structure: HashMap<String, FileMetadata>,
    pub dependencies: Vec<String>,
    pub frameworks: Vec<String>,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub file_type: String,
    pub size: u64,
    pub last_modified: u64,
    pub complexity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalPattern {
    pub name: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub implications: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub mitigation_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub category: String,
    pub description: String,
    pub severity: RiskLevel,
    pub probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub task_id: String,
    pub execution_time: Duration,
    pub success: bool,
    pub error_message: Option<String>,
    pub tools_used: Vec<String>,
    pub context_changes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Replan,           // Re-analyze task with LLM
    ToolSubstitution, // Try alternative MCP tools
    ContextReset,     // Clear task context
    UserIntervention, // Prompt user for guidance
    ThinkingEscalation, // Increase thinking budget
}

#[derive(Debug, Clone)]
pub struct ThinkingBudget {
    pub standard_tokens: u32,
    pub hard_tokens: u32,
    pub harder_tokens: u32,
    pub ultra_tokens: u32,
    pub current_level: ThinkingLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThinkingLevel {
    Standard,
    Hard,
    Harder,
    Ultra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: String,
    pub phases: Vec<ExecutionPhase>,
    pub dependencies: Vec<TaskDependency>,
    pub estimated_complexity: ComplexityLevel,
    pub resource_requirements: ResourceRequirements,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    pub name: String,
    pub description: String,
    pub tasks: Vec<PhaseTask>,
    pub prerequisites: Vec<String>,
    pub deliverables: Vec<String>,
    pub estimated_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTask {
    pub id: String,
    pub description: String,
    pub tool_requirements: Vec<String>,
    pub validation_criteria: Vec<String>,
    pub rollback_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub dependent_task: String,
    pub prerequisite_task: String,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Sequential,  // Must complete before starting
    Parallel,    // Can run in parallel
    Conditional, // Depends on outcome
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Trivial,
    Simple,
    Moderate,
    Complex,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub tools_needed: Vec<String>,
    pub permissions_required: Vec<String>,
    pub estimated_time: Duration,
    pub memory_usage: Option<u64>,
    pub network_access: bool,
}

impl EnhancedTaskExecutor {
    pub async fn new(openrouter_client: Option<Arc<OpenRouterClient>>) -> Self {
        let execution_context = ExecutionContext {
            session_id: Uuid::new_v4().to_string(),
            codebase_structure: None,
            execution_history: Vec::new(),
            architectural_patterns: Vec::new(),
            risk_assessment: RiskAssessment {
                overall_risk: RiskLevel::Low,
                risk_factors: Vec::new(),
                mitigation_strategies: Vec::new(),
            },
            available_tools: Vec::new(),
        };

        Self {
            openrouter_client,
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            execution_context: Arc::new(Mutex::new(execution_context)),
            recovery_strategies: vec![
                RecoveryStrategy::Replan,
                RecoveryStrategy::ToolSubstitution,
                RecoveryStrategy::ThinkingEscalation,
                RecoveryStrategy::ContextReset,
                RecoveryStrategy::UserIntervention,
            ],
            thinking_budget: ThinkingBudget {
                standard_tokens: 2000,
                hard_tokens: 4000,
                harder_tokens: 8000,
                ultra_tokens: 16000,
                current_level: ThinkingLevel::Standard,
            },
        }
    }

    /// Stage 1: Strategic decomposition and planning
    pub async fn breakdown_task(&self, task_description: &str) -> Result<TaskPlan> {
        log_info!("enhanced_executor", "🧠 Starting strategic decomposition for task: {}", task_description);
        
        let start_time = Instant::now();
        
        // Gather context for planning
        let context = self.execution_context.lock().await;
        let codebase_info = context.codebase_structure.as_ref()
            .map(|cb| serde_json::to_string(&cb).unwrap_or_default())
            .unwrap_or_else(|| "No codebase context available".to_string());
        
        // Use LLM for strategic planning if available
        let task_plan = if let Some(ref client) = self.openrouter_client {
            self.llm_strategic_planning(client, task_description, &codebase_info).await?
        } else {
            self.heuristic_planning(task_description).await?
        };

        let planning_duration = start_time.elapsed();
        ops::performance("TASK_PLANNING", planning_duration.as_millis() as u64);
        
        log_info!("enhanced_executor", "✅ Strategic decomposition complete in {:?}", planning_duration);
        log_info!("enhanced_executor", "📋 Plan phases: {}", task_plan.phases.len());
        
        Ok(task_plan)
    }

    /// LLM-powered strategic planning
    async fn llm_strategic_planning(&self, client: &OpenRouterClient, task_description: &str, codebase_context: &str) -> Result<TaskPlan> {
        let thinking_tokens = match self.thinking_budget.current_level {
            ThinkingLevel::Standard => self.thinking_budget.standard_tokens,
            ThinkingLevel::Hard => self.thinking_budget.hard_tokens,
            ThinkingLevel::Harder => self.thinking_budget.harder_tokens,
            ThinkingLevel::Ultra => self.thinking_budget.ultra_tokens,
        };

        let planning_prompt = format!(
            "As an expert software architect and project manager, create a comprehensive execution plan for this task: \"{}\"

Codebase Context:
{}

Please create a detailed execution plan with the following structure:
1. Break down into phases (Analysis, Implementation, Verification)
2. Identify dependencies between tasks
3. Assess complexity and risk factors
4. Specify tool requirements and validation criteria
5. Include rollback strategies for critical operations

Respond in this JSON format:
{{
    \"phases\": [
        {{
            \"name\": \"Phase name\",
            \"description\": \"Phase description\",
            \"tasks\": [
                {{
                    \"id\": \"task_id\",
                    \"description\": \"Task description\",
                    \"tool_requirements\": [\"tool1\", \"tool2\"],
                    \"validation_criteria\": [\"criterion1\", \"criterion2\"],
                    \"rollback_strategy\": \"Optional rollback approach\"
                }}
            ],
            \"prerequisites\": [\"prerequisite1\"],
            \"deliverables\": [\"deliverable1\"],
            \"estimated_duration_minutes\": 30
        }}
    ],
    \"dependencies\": [
        {{
            \"dependent_task\": \"task_id\",
            \"prerequisite_task\": \"prerequisite_id\",
            \"dependency_type\": \"Sequential\"
        }}
    ],
    \"estimated_complexity\": \"Moderate\",
    \"resource_requirements\": {{
        \"tools_needed\": [\"tool1\", \"tool2\"],
        \"permissions_required\": [\"read\", \"write\"],
        \"estimated_time_minutes\": 60,
        \"network_access\": false
    }},
    \"success_criteria\": [\"criterion1\", \"criterion2\"]
}}

Focus on creating a robust, executable plan with proper error handling and recovery mechanisms.",
            task_description, codebase_context
        );

        log_debug!("enhanced_executor", "🤖 Sending planning request to LLM with {} thinking tokens", thinking_tokens);
        
        // Create a chat message for the planning request
        use crate::openrouter_client::ChatMessage;
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: planning_prompt,
        }];
        
        match client.chat_completion(messages).await {
            Ok(response) => {
                log_debug!("enhanced_executor", "📄 Received LLM planning response");
                self.parse_task_plan_response(&response).await
            }
            Err(e) => {
                log_warn!("enhanced_executor", "⚠️ LLM planning failed: {}, falling back to heuristic", e);
                self.heuristic_planning(task_description).await
            }
        }
    }

    /// Parse LLM response into TaskPlan
    async fn parse_task_plan_response(&self, response: &str) -> Result<TaskPlan> {
        // Extract JSON from response (handle markdown wrapping)
        let json_str = if response.contains("```json") {
            response.split("```json").nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
        } else {
            response
        };

        let plan_data: Value = serde_json::from_str(json_str)?;
        
        // Convert to TaskPlan structure
        let phases = plan_data["phases"].as_array()
            .ok_or_else(|| anyhow!("Missing phases in plan"))?
            .iter()
            .map(|phase| {
                let tasks = phase["tasks"].as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|task| PhaseTask {
                        id: task["id"].as_str().unwrap_or("unknown").to_string(),
                        description: task["description"].as_str().unwrap_or("").to_string(),
                        tool_requirements: task["tool_requirements"].as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(|t| t.as_str().unwrap_or("").to_string())
                            .collect(),
                        validation_criteria: task["validation_criteria"].as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(|c| c.as_str().unwrap_or("").to_string())
                            .collect(),
                        rollback_strategy: task["rollback_strategy"].as_str().map(String::from),
                    })
                    .collect();

                ExecutionPhase {
                    name: phase["name"].as_str().unwrap_or("Unknown Phase").to_string(),
                    description: phase["description"].as_str().unwrap_or("").to_string(),
                    tasks,
                    prerequisites: phase["prerequisites"].as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|p| p.as_str().unwrap_or("").to_string())
                        .collect(),
                    deliverables: phase["deliverables"].as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|d| d.as_str().unwrap_or("").to_string())
                        .collect(),
                    estimated_duration: Duration::from_secs(
                        phase["estimated_duration_minutes"].as_u64().unwrap_or(30) * 60
                    ),
                }
            })
            .collect();

        let dependencies = plan_data["dependencies"].as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|dep| TaskDependency {
                dependent_task: dep["dependent_task"].as_str().unwrap_or("").to_string(),
                prerequisite_task: dep["prerequisite_task"].as_str().unwrap_or("").to_string(),
                dependency_type: match dep["dependency_type"].as_str().unwrap_or("Sequential") {
                    "Parallel" => DependencyType::Parallel,
                    "Conditional" => DependencyType::Conditional,
                    _ => DependencyType::Sequential,
                },
            })
            .collect();

        let complexity = match plan_data["estimated_complexity"].as_str().unwrap_or("Simple") {
            "Trivial" => ComplexityLevel::Trivial,
            "Moderate" => ComplexityLevel::Moderate,
            "Complex" => ComplexityLevel::Complex,
            "Critical" => ComplexityLevel::Critical,
            _ => ComplexityLevel::Simple,
        };

        let resource_requirements = ResourceRequirements {
            tools_needed: plan_data["resource_requirements"]["tools_needed"].as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|t| t.as_str().unwrap_or("").to_string())
                .collect(),
            permissions_required: plan_data["resource_requirements"]["permissions_required"].as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|p| p.as_str().unwrap_or("").to_string())
                .collect(),
            estimated_time: Duration::from_secs(
                plan_data["resource_requirements"]["estimated_time_minutes"].as_u64().unwrap_or(60) * 60
            ),
            memory_usage: None,
            network_access: plan_data["resource_requirements"]["network_access"].as_bool().unwrap_or(false),
        };

        let success_criteria = plan_data["success_criteria"].as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|c| c.as_str().unwrap_or("").to_string())
            .collect();

        Ok(TaskPlan {
            id: Uuid::new_v4().to_string(),
            phases,
            dependencies,
            estimated_complexity: complexity,
            resource_requirements,
            success_criteria,
        })
    }

    /// Fallback heuristic planning when LLM is unavailable
    async fn heuristic_planning(&self, task_description: &str) -> Result<TaskPlan> {
        log_info!("enhanced_executor", "🔧 Using heuristic planning for task");
        
        // Simple heuristic-based planning
        let analysis_phase = ExecutionPhase {
            name: "Analysis".to_string(),
            description: "Analyze the task requirements and current state".to_string(),
            tasks: vec![PhaseTask {
                id: "analyze_task".to_string(),
                description: format!("Analyze: {}", task_description),
                tool_requirements: vec!["filesystem".to_string()],
                validation_criteria: vec!["Requirements understood".to_string()],
                rollback_strategy: None,
            }],
            prerequisites: Vec::new(),
            deliverables: vec!["Analysis report".to_string()],
            estimated_duration: Duration::from_secs(300), // 5 minutes
        };

        let implementation_phase = ExecutionPhase {
            name: "Implementation".to_string(),
            description: "Execute the main task implementation".to_string(),
            tasks: vec![PhaseTask {
                id: "implement_task".to_string(),
                description: format!("Implement: {}", task_description),
                tool_requirements: vec!["filesystem".to_string()],
                validation_criteria: vec!["Implementation complete".to_string()],
                rollback_strategy: Some("Revert changes".to_string()),
            }],
            prerequisites: vec!["analyze_task".to_string()],
            deliverables: vec!["Implementation".to_string()],
            estimated_duration: Duration::from_secs(1800), // 30 minutes
        };

        let verification_phase = ExecutionPhase {
            name: "Verification".to_string(),
            description: "Verify the implementation meets requirements".to_string(),
            tasks: vec![PhaseTask {
                id: "verify_task".to_string(),
                description: "Verify task completion and quality".to_string(),
                tool_requirements: vec!["filesystem".to_string()],
                validation_criteria: vec!["All criteria met".to_string()],
                rollback_strategy: None,
            }],
            prerequisites: vec!["implement_task".to_string()],
            deliverables: vec!["Verification report".to_string()],
            estimated_duration: Duration::from_secs(600), // 10 minutes
        };

        Ok(TaskPlan {
            id: Uuid::new_v4().to_string(),
            phases: vec![analysis_phase, implementation_phase, verification_phase],
            dependencies: vec![
                TaskDependency {
                    dependent_task: "implement_task".to_string(),
                    prerequisite_task: "analyze_task".to_string(),
                    dependency_type: DependencyType::Sequential,
                },
                TaskDependency {
                    dependent_task: "verify_task".to_string(),
                    prerequisite_task: "implement_task".to_string(),
                    dependency_type: DependencyType::Sequential,
                },
            ],
            estimated_complexity: ComplexityLevel::Simple,
            resource_requirements: ResourceRequirements {
                tools_needed: vec!["filesystem".to_string()],
                permissions_required: vec!["read".to_string(), "write".to_string()],
                estimated_time: Duration::from_secs(2700), // 45 minutes total
                memory_usage: None,
                network_access: false,
            },
            success_criteria: vec!["Task completed successfully".to_string()],
        })
    }

    /// Stage 2: Systematic execution with recovery
    pub async fn execute_plan_with_recovery(&self, plan: TaskPlan) -> Result<Vec<ExecutionRecord>> {
        log_info!("enhanced_executor", "⚡ Starting systematic execution of plan: {}", plan.id);
        
        let mut execution_records = Vec::new();
        
        for phase in &plan.phases {
            log_info!("enhanced_executor", "🚀 Executing phase: {}", phase.name);
            
            for task in &phase.tasks {
                let execution_start = Instant::now();
                
                match self.execute_phase_task_with_recovery(task).await {
                    Ok(_) => {
                        let execution_time = execution_start.elapsed();
                        execution_records.push(ExecutionRecord {
                            task_id: task.id.clone(),
                            execution_time,
                            success: true,
                            error_message: None,
                            tools_used: task.tool_requirements.clone(),
                            context_changes: vec!["Task completed successfully".to_string()],
                        });
                        log_info!("enhanced_executor", "✅ Task '{}' completed in {:?}", task.id, execution_time);
                    }
                    Err(e) => {
                        let execution_time = execution_start.elapsed();
                        execution_records.push(ExecutionRecord {
                            task_id: task.id.clone(),
                            execution_time,
                            success: false,
                            error_message: Some(e.to_string()),
                            tools_used: task.tool_requirements.clone(),
                            context_changes: vec!["Task failed".to_string()],
                        });
                        log_error!("enhanced_executor", "❌ Task '{}' failed after {:?}: {}", task.id, execution_time, e);
                        
                        // Apply recovery strategy based on failure
                        if !self.apply_recovery_strategies(&task.id, &e).await? {
                            return Err(anyhow!("Task execution failed and recovery unsuccessful: {}", e));
                        }
                    }
                }
            }
        }
        
        log_info!("enhanced_executor", "🎉 Plan execution completed with {} records", execution_records.len());
        Ok(execution_records)
    }

    /// Execute individual phase task with recovery
    async fn execute_phase_task_with_recovery(&self, task: &PhaseTask) -> Result<()> {
        log_debug!("enhanced_executor", "🔄 Executing task: {}", task.description);
        
        // For now, this is a placeholder that simulates task execution
        // In a real implementation, this would:
        // 1. Select appropriate MCP tools based on tool_requirements
        // 2. Execute the actual task using those tools
        // 3. Validate results against validation_criteria
        // 4. Apply rollback_strategy if needed
        
        // Simulate some work
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate occasional failures for testing recovery
        if task.description.contains("fail") {
            return Err(anyhow!("Simulated task failure"));
        }
        
        Ok(())
    }

    /// Apply recovery strategies in order until one succeeds
    async fn apply_recovery_strategies(&self, task_id: &str, error: &anyhow::Error) -> Result<bool> {
        log_warn!("enhanced_executor", "🔧 Applying recovery strategies for task: {}", task_id);
        
        for strategy in &self.recovery_strategies {
            log_debug!("enhanced_executor", "🛠️ Trying recovery strategy: {:?}", strategy);
            
            match self.apply_recovery_strategy(strategy, task_id, error).await {
                Ok(true) => {
                    log_info!("enhanced_executor", "✅ Recovery strategy {:?} succeeded", strategy);
                    return Ok(true);
                }
                Ok(false) => {
                    log_debug!("enhanced_executor", "⚠️ Recovery strategy {:?} did not resolve the issue", strategy);
                    continue;
                }
                Err(recovery_error) => {
                    log_warn!("enhanced_executor", "❌ Recovery strategy {:?} failed: {}", strategy, recovery_error);
                    continue;
                }
            }
        }
        
        log_error!("enhanced_executor", "💥 All recovery strategies exhausted for task: {}", task_id);
        Ok(false)
    }

    /// Apply a specific recovery strategy
    async fn apply_recovery_strategy(&self, strategy: &RecoveryStrategy, task_id: &str, error: &anyhow::Error) -> Result<bool> {
        match strategy {
            RecoveryStrategy::Replan => {
                log_info!("enhanced_executor", "🧠 Attempting to replan task: {}", task_id);
                // TODO: Re-analyze task with LLM
                Ok(false) // Placeholder
            }
            RecoveryStrategy::ToolSubstitution => {
                log_info!("enhanced_executor", "🔄 Attempting tool substitution for task: {}", task_id);
                // TODO: Try alternative MCP tools
                Ok(false) // Placeholder
            }
            RecoveryStrategy::ContextReset => {
                log_info!("enhanced_executor", "🧹 Resetting context for task: {}", task_id);
                let mut context = self.execution_context.lock().await;
                context.execution_history.clear();
                Ok(true)
            }
            RecoveryStrategy::UserIntervention => {
                log_info!("enhanced_executor", "👤 Requesting user intervention for task: {}", task_id);
                // TODO: Implement interactive user prompt
                Ok(false) // Placeholder
            }
            RecoveryStrategy::ThinkingEscalation => {
                log_info!("enhanced_executor", "🧠 Escalating thinking level for task: {}", task_id);
                // TODO: Increase thinking budget and retry
                Ok(false) // Placeholder
            }
        }
    }

    /// Update execution context with codebase information
    pub async fn update_codebase_context(&self, codebase_map: CodebaseMap) -> Result<()> {
        let mut context = self.execution_context.lock().await;
        context.codebase_structure = Some(codebase_map);
        log_info!("enhanced_executor", "📁 Updated codebase context");
        Ok(())
    }

    /// Get current execution context (for debugging/monitoring)
    pub async fn get_execution_context(&self) -> ExecutionContext {
        self.execution_context.lock().await.clone()
    }
}

impl ThinkingBudget {
    pub fn escalate(&mut self) -> bool {
        match self.current_level {
            ThinkingLevel::Standard => {
                self.current_level = ThinkingLevel::Hard;
                true
            }
            ThinkingLevel::Hard => {
                self.current_level = ThinkingLevel::Harder;
                true
            }
            ThinkingLevel::Harder => {
                self.current_level = ThinkingLevel::Ultra;
                true
            }
            ThinkingLevel::Ultra => false, // Already at maximum
        }
    }

    pub fn reset(&mut self) {
        self.current_level = ThinkingLevel::Standard;
    }
}

impl std::fmt::Display for ComplexityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplexityLevel::Trivial => write!(f, "Trivial"),
            ComplexityLevel::Simple => write!(f, "Simple"),
            ComplexityLevel::Moderate => write!(f, "Moderate"),
            ComplexityLevel::Complex => write!(f, "Complex"),
            ComplexityLevel::Critical => write!(f, "Critical"),
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}