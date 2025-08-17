use anyhow::{anyhow, Result};
use chrono::Utc;
use colored::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use uuid::Uuid;

use crate::execution_context::{
    ContinuationDecision, ErrorRecoveryContext, ExecutionContext, 
    ExecutionMetadata, ExecutionStep, InterventionRequest, 
    ReplanningContext, UrgencyLevel
};
use crate::feedback_loop::FeedbackLoopManager;
use crate::logger::{log_debug, log_error, log_info};
use crate::openrouter_client::OpenRouterClient;
use crate::task_executor::TaskExecutor;
use crate::workflow_orchestrator::WorkflowOrchestrator;

/// Status of the workflow execution
#[derive(Debug, Clone)]
pub enum WorkflowStatus {
    Complete,
    Blocked,
    NeedsReplanning,
    InProgress,
    Error(String),
}

/// Continuous executor that manages autonomous task execution
pub struct ContinuousExecutor {
    workflow_orchestrator: Arc<WorkflowOrchestrator>,
    task_executor: Arc<TaskExecutor>,
    feedback_manager: Arc<FeedbackLoopManager>,
    llm_client: Arc<OpenRouterClient>,
    execution_context: Arc<Mutex<ExecutionContext>>,
    is_running: Arc<Mutex<bool>>,
    max_continuous_steps: usize,
    step_delay_ms: u64,
}

impl ContinuousExecutor {
    pub async fn new(
        workflow_orchestrator: Arc<WorkflowOrchestrator>,
        task_executor: Arc<TaskExecutor>,
        feedback_manager: Arc<FeedbackLoopManager>,
    ) -> Result<Self> {
        let llm_client = Arc::new(OpenRouterClient::new().await?);
        
        Ok(Self {
            workflow_orchestrator,
            task_executor,
            feedback_manager,
            llm_client,
            execution_context: Arc::new(Mutex::new(ExecutionContext::new(
                Uuid::new_v4().to_string(),
                "default".to_string(),
            ))),
            is_running: Arc::new(Mutex::new(false)),
            max_continuous_steps: 50, // Safety limit
            step_delay_ms: 1000, // 1 second between steps
        })
    }

    /// Main continuous execution loop
    pub async fn run_continuous(&self, workflow_id: &str) -> Result<()> {
        {
            let mut running = self.is_running.lock().await;
            if *running {
                return Err(anyhow!("Continuous execution already in progress"));
            }
            *running = true;
        }

        // Initialize execution context
        {
            let mut context = self.execution_context.lock().await;
            *context = ExecutionContext::new(Uuid::new_v4().to_string(), workflow_id.to_string());
        }

        println!("{} Starting continuous execution mode...", "🚀".green().bold());
        log_info!("continuous_executor", "🚀 Starting continuous execution for workflow {}", workflow_id);

        let result = self.execute_continuous_loop(workflow_id).await;

        // Cleanup
        {
            let mut running = self.is_running.lock().await;
            *running = false;
        }

        match &result {
            Ok(_) => {
                println!("{} Continuous execution completed successfully", "✅".green());
                log_info!("continuous_executor", "✅ Continuous execution completed for workflow {}", workflow_id);
            }
            Err(e) => {
                println!("{} Continuous execution failed: {}", "❌".red(), e);
                log_error!("continuous_executor", "❌ Continuous execution failed for workflow {}: {}", workflow_id, e);
            }
        }

        result
    }

    async fn execute_continuous_loop(&self, workflow_id: &str) -> Result<()> {
        let mut step_count = 0;
        
        loop {
            // Safety check - prevent infinite loops
            if step_count >= self.max_continuous_steps {
                let intervention = InterventionRequest {
                    reason: "Maximum continuous steps reached".to_string(),
                    context: format!("Executed {} steps without completion", step_count),
                    suggested_user_actions: vec![
                        "Review workflow progress".to_string(),
                        "Check for execution loops".to_string(),
                        "Consider manual intervention".to_string(),
                    ],
                    urgency_level: UrgencyLevel::High,
                };
                
                self.request_user_intervention(&intervention).await?;
                break;
            }

            step_count += 1;
            log_debug!("continuous_executor", "🔄 Continuous execution step {}/{}", step_count, self.max_continuous_steps);

            // 1. Check for executable goals
            let next_goal_result = self.find_next_executable_goal(workflow_id).await?;
            
            match next_goal_result {
                Some(goal_id) => {
                    println!("{} Executing goal: {}", "🎯".blue(), goal_id);
                    
                    // 2. Execute goal with full context feedback
                    let execution_result = self.execute_goal_with_feedback(workflow_id, &goal_id).await?;
                    
                    // 3. Record execution step
                    self.record_execution_step(&goal_id, &execution_result).await?;
                    
                    // 4. Analyze results and determine continuation
                    let continuation = self.analyze_execution_results(&goal_id, &execution_result).await?;
                    
                    match continuation {
                        ContinuationDecision::Continue => {
                            println!("{} Continuing execution...", "➡️".cyan());
                            sleep(Duration::from_millis(self.step_delay_ms)).await;
                            continue;
                        }
                        ContinuationDecision::Replan(context) => {
                            println!("{} Replanning workflow based on results...", "🔄".yellow());
                            self.trigger_replanning(workflow_id, context).await?;
                            continue;
                        }
                        ContinuationDecision::Complete => {
                            println!("{} Workflow completed successfully!", "🎉".green());
                            break;
                        }
                        ContinuationDecision::Error(recovery_context) => {
                            println!("{} Handling execution error...", "⚠️".red());
                            match self.handle_execution_error(workflow_id, recovery_context).await? {
                                true => continue, // Recovery successful, continue
                                false => break,   // Recovery failed, stop
                            }
                        }
                        ContinuationDecision::UserIntervention(intervention) => {
                            self.request_user_intervention(&intervention).await?;
                            break;
                        }
                    }
                }
                None => {
                    // 4. Check if workflow is complete or needs replanning
                    let status = self.assess_workflow_status(workflow_id).await?;
                    match status {
                        WorkflowStatus::Complete => {
                            println!("{} All workflow goals completed!", "🎉".green());
                            break;
                        }
                        WorkflowStatus::Blocked => {
                            let intervention = InterventionRequest {
                                reason: "Workflow execution blocked".to_string(),
                                context: "No executable goals found, workflow may be stuck".to_string(),
                                suggested_user_actions: vec![
                                    "Review goal dependencies".to_string(),
                                    "Add missing prerequisites".to_string(),
                                    "Modify blocked goals".to_string(),
                                ],
                                urgency_level: UrgencyLevel::Medium,
                            };
                            self.request_user_intervention(&intervention).await?;
                            break;
                        }
                        WorkflowStatus::NeedsReplanning => {
                            println!("{} Triggering adaptive replanning...", "🧠".yellow());
                            let context = ReplanningContext {
                                trigger_reason: "No executable goals, adaptive replanning needed".to_string(),
                                current_state_summary: "Workflow appears stuck or incomplete".to_string(),
                                problematic_patterns: vec![],
                                suggested_approach: "Analyze current state and create new executable goals".to_string(),
                                confidence_in_suggestion: 0.7,
                            };
                            self.trigger_adaptive_replanning(workflow_id, context).await?;
                            continue;
                        }
                        WorkflowStatus::InProgress => {
                            // This shouldn't happen if no goals are executable, but handle gracefully
                            println!("{} Workflow in progress but no executable goals found", "⚠️".yellow());
                            sleep(Duration::from_millis(self.step_delay_ms * 2)).await;
                            continue;
                        }
                        WorkflowStatus::Error(error) => {
                            return Err(anyhow!("Workflow error: {}", error));
                        }
                    }
                }
            }
        }

        // Show final execution summary
        self.display_execution_summary().await?;
        
        Ok(())
    }

    async fn find_next_executable_goal(&self, workflow_id: &str) -> Result<Option<String>> {
        log_debug!("continuous_executor", "🔍 Finding next executable goal for workflow {}", workflow_id);
        
        // Use the workflow orchestrator to find the next goal
        match self.workflow_orchestrator.execute_next_goal(workflow_id).await {
            Ok(true) => {
                // A goal was found and executed (or started execution)
                // We need to get the current goal that's being worked on
                // For now, return a placeholder - this would need workflow orchestrator API updates
                Ok(Some("current_goal".to_string()))
            }
            Ok(false) => {
                // No executable goals found
                Ok(None)
            }
            Err(e) => {
                log_error!("continuous_executor", "❌ Error finding next goal: {}", e);
                Err(e)
            }
        }
    }

    async fn execute_goal_with_feedback(&self, workflow_id: &str, goal_id: &str) -> Result<String> {
        log_info!("continuous_executor", "🎯 Executing goal {} with feedback collection", goal_id);
        
        // This is a simplified implementation - would need more sophisticated goal execution
        // For now, we'll use the task executor to handle the execution
        
        // Create a simple feedback collection mechanism
        let start_time = std::time::Instant::now();
        
        // Execute tasks for this goal (simplified approach)
        match self.task_executor.execute_all().await {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                log_info!("continuous_executor", "✅ Goal {} executed successfully in {}ms", goal_id, execution_time);
                Ok(format!("Goal {} executed successfully in {}ms", goal_id, execution_time))
            }
            Err(e) => {
                log_error!("continuous_executor", "❌ Goal {} execution failed: {}", goal_id, e);
                Err(anyhow!("Goal execution failed: {}", e))
            }
        }
    }

    async fn record_execution_step(&self, goal_id: &str, execution_result: &str) -> Result<()> {
        let mut context = self.execution_context.lock().await;
        
        let metadata = ExecutionMetadata {
            working_directory: std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()),
            environment_changes: std::env::vars().collect(),
            files_modified: Vec::new(), // Would be populated by actual file monitoring
            files_created: Vec::new(),
            files_deleted: Vec::new(),
            execution_time_ms: 1000, // Placeholder - would be actual execution time
            memory_usage_mb: None,
            tool_count: 1,
            error_count: if execution_result.contains("failed") { 1 } else { 0 },
        };

        let step = ExecutionStep {
            step_id: Uuid::new_v4().to_string(),
            task_id: goal_id.to_string(),
            timestamp: Utc::now(),
            tool_calls: Vec::new(), // Would be populated from actual execution
            execution_metadata: metadata,
            result: if execution_result.contains("failed") {
                crate::execution_context::ExecutionStepResult::Failure {
                    error: execution_result.to_string(),
                    error_type: crate::execution_context::ErrorType::ToolFailure,
                    recovery_suggestions: vec!["Retry with different approach".to_string()],
                }
            } else {
                crate::execution_context::ExecutionStepResult::Success {
                    output: execution_result.to_string(),
                    confidence_score: 0.8,
                    impact_assessment: crate::execution_context::ImpactAssessment {
                        progress_made: 1.0,
                        goal_alignment: 0.8,
                        risk_level: crate::execution_context::RiskLevel::Low,
                        next_action_suggestions: vec!["Continue with next goal".to_string()],
                    },
                }
            },
        };

        context.add_execution_step(step);
        log_debug!("continuous_executor", "📊 Recorded execution step for goal {}", goal_id);
        
        Ok(())
    }

    async fn analyze_execution_results(&self, goal_id: &str, execution_result: &str) -> Result<ContinuationDecision> {
        log_debug!("continuous_executor", "🔍 Analyzing execution results for goal {}", goal_id);
        
        let context = self.execution_context.lock().await;
        
        // Simple analysis logic - in a real implementation, this would use the LLM
        if execution_result.contains("failed") {
            let error_count = context.accumulated_state.error_patterns.values().sum::<usize>();
            
            if error_count > 3 {
                return Ok(ContinuationDecision::Error(ErrorRecoveryContext {
                    error_summary: execution_result.to_string(),
                    recovery_strategies: vec![
                        crate::execution_context::RecoveryStrategy {
                            strategy_id: "retry".to_string(),
                            description: "Retry with modified approach".to_string(),
                            estimated_success_rate: 0.6,
                            required_resources: vec!["LLM analysis".to_string()],
                            estimated_time: 30000, // 30 seconds
                        }
                    ],
                    estimated_recovery_time: Some(30000),
                    risk_of_continuation: crate::execution_context::RiskLevel::Medium,
                }));
            }
        }

        // Check if we should continue
        if context.should_continue() {
            Ok(ContinuationDecision::Continue)
        } else {
            Ok(ContinuationDecision::Complete)
        }
    }

    async fn trigger_replanning(&self, workflow_id: &str, _context: ReplanningContext) -> Result<()> {
        log_info!("continuous_executor", "🔄 Triggering replanning for workflow {}", workflow_id);
        
        // This would integrate with the workflow orchestrator's replanning capabilities
        // For now, just log the action
        println!("{} Replanning triggered - would analyze context and update goals", "🧠".yellow());
        
        Ok(())
    }

    async fn trigger_adaptive_replanning(&self, workflow_id: &str, context: ReplanningContext) -> Result<()> {
        log_info!("continuous_executor", "🧠 Triggering adaptive replanning for workflow {}", workflow_id);
        
        println!("{} Adaptive replanning: {}", "🔄".yellow(), context.trigger_reason);
        println!("   Suggested approach: {}", context.suggested_approach);
        
        // This would use the LLM to analyze the situation and create new goals
        // For now, implement a basic approach
        Ok(())
    }

    async fn handle_execution_error(&self, workflow_id: &str, recovery_context: ErrorRecoveryContext) -> Result<bool> {
        log_error!("continuous_executor", "⚠️ Handling execution error for workflow {}: {}", workflow_id, recovery_context.error_summary);
        
        println!("{} Execution error: {}", "❌".red(), recovery_context.error_summary);
        
        if !recovery_context.recovery_strategies.is_empty() {
            let strategy = &recovery_context.recovery_strategies[0];
            println!("{} Attempting recovery: {}", "🔧".yellow(), strategy.description);
            
            // Simulate recovery attempt
            sleep(Duration::from_millis(strategy.estimated_time)).await;
            
            // For now, assume recovery succeeds 60% of the time
            let success = strategy.estimated_success_rate > 0.5;
            
            if success {
                println!("{} Recovery successful", "✅".green());
                log_info!("continuous_executor", "✅ Error recovery successful for workflow {}", workflow_id);
                Ok(true)
            } else {
                println!("{} Recovery failed", "❌".red());
                log_error!("continuous_executor", "❌ Error recovery failed for workflow {}", workflow_id);
                Ok(false)
            }
        } else {
            log_error!("continuous_executor", "❌ No recovery strategies available for workflow {}", workflow_id);
            Ok(false)
        }
    }

    async fn request_user_intervention(&self, intervention: &InterventionRequest) -> Result<()> {
        println!("\n{} User Intervention Required", "🚨".red().bold());
        println!("{} Reason: {}", "❓".yellow(), intervention.reason);
        println!("{} Context: {}", "📋".cyan(), intervention.context);
        println!("{} Urgency: {:?}", "⚡".red(), intervention.urgency_level);
        
        println!("{} Suggested actions:", "💡".yellow());
        for (i, action) in intervention.suggested_user_actions.iter().enumerate() {
            println!("   {}. {}", i + 1, action);
        }
        
        println!("\n{} Continuous execution paused. Use chat commands to continue.", "⏸️".yellow());
        
        log_info!("continuous_executor", "🚨 User intervention requested: {}", intervention.reason);
        
        Ok(())
    }

    async fn assess_workflow_status(&self, workflow_id: &str) -> Result<WorkflowStatus> {
        log_debug!("continuous_executor", "📊 Assessing workflow status for {}", workflow_id);
        
        // This would integrate with the workflow orchestrator to check status
        // For now, return a basic assessment
        
        let context = self.execution_context.lock().await;
        let success_rate = context.calculate_success_rate();
        
        if success_rate > 0.8 && context.accumulated_state.total_steps > 0 {
            Ok(WorkflowStatus::Complete)
        } else if success_rate < 0.3 {
            Ok(WorkflowStatus::NeedsReplanning)
        } else if context.accumulated_state.total_steps == 0 {
            Ok(WorkflowStatus::Blocked)
        } else {
            Ok(WorkflowStatus::InProgress)
        }
    }

    async fn display_execution_summary(&self) -> Result<()> {
        let context = self.execution_context.lock().await;
        let summary = context.get_execution_summary();
        
        println!("\n{} Execution Summary", "📊".blue().bold());
        println!("{}", summary);
        
        if !context.accumulated_state.tools_used.is_empty() {
            println!("{} Tools used:", "🔧".cyan());
            for (tool, count) in &context.accumulated_state.tools_used {
                println!("   • {} ({}x)", tool, count);
            }
        }
        
        if !context.accumulated_state.files_modified_total.is_empty() {
            println!("{} Files affected: {}", "📁".green(), context.accumulated_state.files_modified_total.len());
        }
        
        let trend = context.analyze_progress_trend();
        println!("{} Progress trend: {}", "📈".yellow(), trend);
        
        Ok(())
    }

    /// Check if continuous execution is currently running
    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }

    /// Stop continuous execution
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.is_running.lock().await;
        if *running {
            *running = false;
            println!("{} Stopping continuous execution...", "🛑".yellow());
            log_info!("continuous_executor", "🛑 Continuous execution stopped by user request");
            Ok(())
        } else {
            Err(anyhow!("Continuous execution is not running"))
        }
    }
}