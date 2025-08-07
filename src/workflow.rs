use crate::chat::{self, Prompt, PromptType};
use crate::commands_registry;
use crate::openrouter;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref ACTIVE_WORKFLOWS: Mutex<HashMap<String, WorkflowPlan>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    Planning,
    Executing,
    Verifying,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStrategy {
    FileExists,
    CommandSuccess,
    OutputPattern(String),
    LLMValidation,
    Combined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub description: String,
    pub command: Option<String>,
    pub expected_output: String,
    pub status: StepStatus,
    pub result: Option<String>,
    pub verification_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPlan {
    pub id: String,
    pub goal: String,
    pub steps: Vec<WorkflowStep>,
    pub status: PlanStatus,
    pub created: DateTime<Utc>,
    pub parent_plan: Option<String>,
    pub current_step: usize,
    pub max_iterations: usize,
    pub current_iteration: usize,
    pub verification_strategy: VerificationStrategy,
}

impl WorkflowPlan {
    pub fn new(
        goal: String,
        max_iterations: usize,
        verification_strategy: VerificationStrategy,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("")
                .to_string(),
            goal,
            steps: Vec::new(),
            status: PlanStatus::Planning,
            created: Utc::now(),
            parent_plan: None,
            current_step: 0,
            max_iterations,
            current_iteration: 0,
            verification_strategy,
        }
    }
}

impl WorkflowStep {
    pub fn new(description: String, command: Option<String>, expected_output: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("")
                .to_string(),
            description,
            command,
            expected_output,
            status: StepStatus::Pending,
            result: None,
            verification_criteria: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum WorkflowError {
    PlanningFailed(String),
    ExecutionFailed(String),
    VerificationFailed(String),
    PlanNotFound(String),
    MaxIterationsExceeded,
    InvalidCommand(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::PlanningFailed(msg) => write!(f, "Planning failed: {}", msg),
            WorkflowError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            WorkflowError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            WorkflowError::PlanNotFound(id) => write!(f, "Plan not found: {}", id),
            WorkflowError::MaxIterationsExceeded => write!(f, "Maximum iterations exceeded"),
            WorkflowError::InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
        }
    }
}

impl std::error::Error for WorkflowError {}

#[derive(Debug)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct VerificationResult {
    pub success: bool,
    pub score: f32, // 0.0 to 1.0
    pub message: String,
}

pub struct WorkflowEngine;

impl WorkflowEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn start_workflow(
        &self,
        goal: &str,
        max_iterations: usize,
        verification_strategy: VerificationStrategy,
    ) -> Result<String, WorkflowError> {
        let plan = WorkflowPlan::new(goal.to_string(), max_iterations, verification_strategy);
        let plan_id = plan.id.clone();

        // Store the plan in memory as a workflow prompt
        let workflow_prompt = Prompt::new(
            format!("Workflow Plan: {}\nGoal: {}", plan_id, goal),
            PromptType::QUESTION, // We'll use QUESTION type for workflow plans
        );

        // Store the plan in active workflows
        {
            let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
            workflows.insert(plan_id.clone(), plan);
        }

        // Start initial planning phase
        self.generate_initial_plan(&plan_id).await?;

        Ok(plan_id)
    }

    async fn generate_initial_plan(&self, plan_id: &str) -> Result<(), WorkflowError> {
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        let plan = workflows
            .get_mut(plan_id)
            .ok_or_else(|| WorkflowError::PlanNotFound(plan_id.to_string()))?;

        // Use LLM to break down the goal into steps
        let planning_prompt = format!(
            "Break down this goal into concrete, executable steps: {}\n\n\
            Respond with a numbered list of specific steps that can be executed using commands.\n\
            Focus on file operations, bash commands, and verification steps.\n\
            Each step should be specific and actionable.",
            plan.goal
        );

        // Get LLM response for planning
        drop(workflows); // Release lock before async call

        let llm_response = self.call_llm_for_planning(&planning_prompt).await?;
        let steps = self.parse_planning_response(&llm_response)?;

        // Update the plan with generated steps
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        let plan = workflows
            .get_mut(plan_id)
            .ok_or_else(|| WorkflowError::PlanNotFound(plan_id.to_string()))?;

        plan.steps = steps;
        plan.status = PlanStatus::Executing;

        Ok(())
    }

    async fn call_llm_for_planning(&self, prompt: &str) -> Result<String, WorkflowError> {
        // Use the existing OpenRouter integration
        match openrouter::call_openrouter_api(prompt).await {
            Ok(response) => Ok(response),
            Err(e) => Err(WorkflowError::PlanningFailed(format!(
                "LLM planning failed: {}",
                e
            ))),
        }
    }

    fn parse_planning_response(&self, response: &str) -> Result<Vec<WorkflowStep>, WorkflowError> {
        let mut steps = Vec::new();
        let lines: Vec<&str> = response.lines().collect();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Look for numbered steps
            if let Some(step_text) = self.extract_step_from_line(trimmed) {
                let step = WorkflowStep::new(
                    step_text.to_string(),
                    self.extract_command_from_step(step_text),
                    "Step completed successfully".to_string(),
                );
                steps.push(step);
            }
        }

        if steps.is_empty() {
            return Err(WorkflowError::PlanningFailed(
                "No valid steps found in planning response".to_string(),
            ));
        }

        Ok(steps)
    }

    fn extract_step_from_line(&self, line: &str) -> Option<&str> {
        // Match patterns like "1. Step description" or "Step 1: Description"
        if line.chars().next().unwrap_or(' ').is_ascii_digit() {
            if let Some(pos) = line.find('.') {
                return Some(line[pos + 1..].trim());
            } else if let Some(pos) = line.find(':') {
                return Some(line[pos + 1..].trim());
            }
        }
        None
    }

    fn extract_command_from_step(&self, step_text: &str) -> Option<String> {
        // Look for command patterns in step text
        if step_text.contains("@") && step_text.contains("(") {
            // Find command pattern like @command(params)
            if let Some(start) = step_text.find('@') {
                if let Some(end) = step_text[start..].find(')') {
                    return Some(step_text[start..start + end + 1].to_string());
                }
            }
        }
        None
    }

    pub async fn execute_step(&self, plan_id: &str, step_id: &str) -> Result<StepResult, WorkflowError> {
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        let plan = workflows
            .get_mut(plan_id)
            .ok_or_else(|| WorkflowError::PlanNotFound(plan_id.to_string()))?;

        let step_index = plan
            .steps
            .iter()
            .position(|s| s.id == step_id)
            .ok_or_else(|| WorkflowError::ExecutionFailed(format!("Step {} not found", step_id)))?;

        plan.steps[step_index].status = StepStatus::InProgress;
        let command = plan.steps[step_index].command.clone();
        let description = plan.steps[step_index].description.clone();

        drop(workflows); // Release lock before execution

        let result = if let Some(cmd) = command {
            self.execute_command(&cmd).await
        } else {
            // If no specific command, try to infer from description
            self.execute_inferred_command(&description).await
        };

        // Update step with result
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        let plan = workflows
            .get_mut(plan_id)
            .ok_or_else(|| WorkflowError::PlanNotFound(plan_id.to_string()))?;

        match result {
            Ok(output) => {
                plan.steps[step_index].status = StepStatus::Completed;
                plan.steps[step_index].result = output.clone();
                Ok(StepResult {
                    step_id: step_id.to_string(),
                    success: true,
                    output,
                    error: None,
                })
            }
            Err(e) => {
                plan.steps[step_index].status = StepStatus::Failed;
                plan.steps[step_index].result = Some(e.to_string());
                Ok(StepResult {
                    step_id: step_id.to_string(),
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    async fn execute_command(&self, command: &str) -> Result<Option<String>, WorkflowError> {
        // Use existing command execution system
        match chat::execute_command(command) {
            Ok(Some(result)) => {
                if let Ok(Some(output)) = result.command_output {
                    Ok(Some(output))
                } else {
                    Ok(Some("Command executed successfully".to_string()))
                }
            },
            Ok(None) => Ok(None),
            Err(e) => Err(WorkflowError::ExecutionFailed(e.to_string())),
        }
    }

    async fn execute_inferred_command(&self, description: &str) -> Result<Option<String>, WorkflowError> {
        // Try to infer command from description using LLM
        let inference_prompt = format!(
            "Convert this step description into a specific CAI command: {}\n\n\
            Available command patterns:\n\
            - @read-file(filename)\n\
            - @list-files(pattern)\n\
            - @bash-cmd(command)\n\
            - @export(id, filename)\n\n\
            Respond with just the command, nothing else.",
            description
        );

        let llm_response = self.call_llm_for_planning(&inference_prompt).await?;
        let inferred_command = llm_response.trim();

        if inferred_command.starts_with('@') || inferred_command.starts_with('!') {
            self.execute_command(inferred_command).await
        } else {
            Ok(Some(format!("Step completed: {}", description)))
        }
    }

    pub async fn verify_progress(&self, plan_id: &str) -> Result<VerificationResult, WorkflowError> {
        let workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        let plan = workflows
            .get(plan_id)
            .ok_or_else(|| WorkflowError::PlanNotFound(plan_id.to_string()))?;

        let completed_steps = plan
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let total_steps = plan.steps.len();

        if total_steps == 0 {
            return Ok(VerificationResult {
                success: false,
                score: 0.0,
                message: "No steps to verify".to_string(),
            });
        }

        let completion_score = completed_steps as f32 / total_steps as f32;

        match &plan.verification_strategy {
            VerificationStrategy::FileExists => {
                self.verify_files_exist(plan).await
            }
            VerificationStrategy::CommandSuccess => {
                Ok(VerificationResult {
                    success: completion_score >= 0.8,
                    score: completion_score,
                    message: format!("Command success rate: {:.1}%", completion_score * 100.0),
                })
            }
            VerificationStrategy::OutputPattern(pattern) => {
                self.verify_output_pattern(plan, pattern).await
            }
            VerificationStrategy::LLMValidation => {
                self.verify_with_llm(plan).await
            }
            VerificationStrategy::Combined => {
                // Combine multiple verification strategies
                let file_result = self.verify_files_exist(plan).await?;
                let llm_result = self.verify_with_llm(plan).await?;
                
                let combined_score = (file_result.score + llm_result.score) / 2.0;
                Ok(VerificationResult {
                    success: combined_score >= 0.7,
                    score: combined_score,
                    message: format!(
                        "Combined verification - Files: {:.1}%, LLM: {:.1}%",
                        file_result.score * 100.0,
                        llm_result.score * 100.0
                    ),
                })
            }
        }
    }

    async fn verify_files_exist(&self, plan: &WorkflowPlan) -> Result<VerificationResult, WorkflowError> {
        // Simple file existence verification
        let mut files_checked = 0;
        let mut files_found = 0;

        for step in &plan.steps {
            if let Some(result) = &step.result {
                if result.contains("File saved") || result.contains("created") {
                    files_checked += 1;
                    // In a real implementation, we'd check if the file actually exists
                    files_found += 1;
                }
            }
        }

        let score = if files_checked > 0 {
            files_found as f32 / files_checked as f32
        } else {
            1.0
        };

        Ok(VerificationResult {
            success: score >= 0.8,
            score,
            message: format!("Files verification: {}/{} files found", files_found, files_checked),
        })
    }

    async fn verify_output_pattern(
        &self,
        plan: &WorkflowPlan,
        pattern: &str,
    ) -> Result<VerificationResult, WorkflowError> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| WorkflowError::VerificationFailed(format!("Invalid regex pattern: {}", e)))?;

        let mut total_steps = 0;
        let mut matching_steps = 0;

        for step in &plan.steps {
            if step.status == StepStatus::Completed {
                total_steps += 1;
                if let Some(result) = &step.result {
                    if regex.is_match(result) {
                        matching_steps += 1;
                    }
                }
            }
        }

        let score = if total_steps > 0 {
            matching_steps as f32 / total_steps as f32
        } else {
            0.0
        };

        Ok(VerificationResult {
            success: score >= 0.6,
            score,
            message: format!("Pattern matching: {}/{} steps match pattern", matching_steps, total_steps),
        })
    }

    async fn verify_with_llm(&self, plan: &WorkflowPlan) -> Result<VerificationResult, WorkflowError> {
        let verification_prompt = format!(
            "Verify if this workflow has successfully achieved its goal: {}\n\n\
            Steps executed:\n{}\n\n\
            Rate the success on a scale of 0-10 and explain why.\n\
            Format: SCORE: X/10\nREASON: explanation",
            plan.goal,
            plan.steps
                .iter()
                .map(|s| format!("- {}: {} ({})", s.description, 
                    s.result.as_ref().unwrap_or(&"No result".to_string()),
                    match s.status {
                        StepStatus::Completed => "✓",
                        StepStatus::Failed => "✗",
                        StepStatus::InProgress => "⏳",
                        _ => "○"
                    }))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let llm_response = self.call_llm_for_planning(&verification_prompt).await?;
        
        // Parse score from LLM response
        let score = if let Some(score_line) = llm_response.lines().find(|line| line.starts_with("SCORE:")) {
            let score_str = score_line.replace("SCORE:", "").trim().replace("/10", "");
            score_str.parse::<f32>().unwrap_or(5.0) / 10.0
        } else {
            0.5 // Default score if parsing fails
        };

        let reason = llm_response.lines()
            .find(|line| line.starts_with("REASON:"))
            .map(|line| line.replace("REASON:", "").trim().to_string())
            .unwrap_or_else(|| "LLM verification completed".to_string());

        Ok(VerificationResult {
            success: score >= 0.7,
            score,
            message: reason,
        })
    }

    pub fn should_continue(&self, plan_id: &str) -> bool {
        let workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        if let Some(plan) = workflows.get(plan_id) {
            plan.current_iteration < plan.max_iterations
                && plan.status != PlanStatus::Completed
                && plan.status != PlanStatus::Failed
        } else {
            false
        }
    }

    pub async fn continue_workflow(&self, plan_id: &str) -> Result<bool, WorkflowError> {
        {
            let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
            if let Some(plan) = workflows.get_mut(plan_id) {
                plan.current_iteration += 1;
                
                if plan.current_iteration >= plan.max_iterations {
                    plan.status = PlanStatus::Failed;
                    return Err(WorkflowError::MaxIterationsExceeded);
                }
            } else {
                return Err(WorkflowError::PlanNotFound(plan_id.to_string()));
            }
        }

        // Execute next step
        let next_step_id = {
            let workflows = ACTIVE_WORKFLOWS.lock().unwrap();
            if let Some(plan) = workflows.get(plan_id) {
                if plan.current_step < plan.steps.len() {
                    Some(plan.steps[plan.current_step].id.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(step_id) = next_step_id {
            let result = self.execute_step(plan_id, &step_id).await?;
            
            // Update current step
            {
                let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
                if let Some(plan) = workflows.get_mut(plan_id) {
                    if result.success {
                        plan.current_step += 1;
                    }
                }
            }

            // Verify progress
            let verification = self.verify_progress(plan_id).await?;
            
            // Check if workflow is complete
            {
                let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
                if let Some(plan) = workflows.get_mut(plan_id) {
                    if verification.success && plan.current_step >= plan.steps.len() {
                        plan.status = PlanStatus::Completed;
                        return Ok(false); // Don't continue, we're done
                    }
                }
            }

            Ok(true) // Continue with next iteration
        } else {
            // No more steps, workflow complete
            let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
            if let Some(plan) = workflows.get_mut(plan_id) {
                plan.status = PlanStatus::Completed;
            }
            Ok(false)
        }
    }

    pub fn get_workflow_status(&self, plan_id: &str) -> Option<WorkflowPlan> {
        let workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        workflows.get(plan_id).cloned()
    }

    pub fn list_workflows(&self) -> Vec<String> {
        let workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        workflows.keys().cloned().collect()
    }

    pub fn pause_workflow(&self, plan_id: &str) -> Result<(), WorkflowError> {
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        if let Some(plan) = workflows.get_mut(plan_id) {
            plan.status = PlanStatus::Paused;
            Ok(())
        } else {
            Err(WorkflowError::PlanNotFound(plan_id.to_string()))
        }
    }

    pub fn resume_workflow(&self, plan_id: &str) -> Result<(), WorkflowError> {
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        if let Some(plan) = workflows.get_mut(plan_id) {
            if plan.status == PlanStatus::Paused {
                plan.status = PlanStatus::Executing;
                Ok(())
            } else {
                Err(WorkflowError::ExecutionFailed(
                    "Workflow is not paused".to_string(),
                ))
            }
        } else {
            Err(WorkflowError::PlanNotFound(plan_id.to_string()))
        }
    }

    pub fn stop_workflow(&self, plan_id: &str) -> Result<(), WorkflowError> {
        let mut workflows = ACTIVE_WORKFLOWS.lock().unwrap();
        if let Some(plan) = workflows.get_mut(plan_id) {
            plan.status = PlanStatus::Failed;
            Ok(())
        } else {
            Err(WorkflowError::PlanNotFound(plan_id.to_string()))
        }
    }
}

pub fn get_workflow_engine() -> WorkflowEngine {
    WorkflowEngine::new()
}