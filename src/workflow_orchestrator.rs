use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;
use colored::*;
use std::path::PathBuf;
use std::fs;

use crate::openrouter_client::{OpenRouterClient, ChatMessage};
use crate::task_executor::TaskExecutor;
use crate::feedback_loop::{FeedbackLoopManager, FeedbackType};
use crate::logger::{log_info, log_debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGoal {
    pub id: String,
    pub description: String,
    pub parent_goal_id: Option<String>,
    pub sub_goals: Vec<String>,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: Value,
    pub success_criteria: Vec<String>,
    pub completion_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Pending,
    Planning,
    InProgress,
    Completed,
    Failed,
    Refined,
}

impl GoalStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            GoalStatus::Pending => "⏳",
            GoalStatus::Planning => "🧠",
            GoalStatus::InProgress => "🔄",
            GoalStatus::Completed => "✅",
            GoalStatus::Failed => "❌",
            GoalStatus::Refined => "🔄",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub root_goal: String,
    pub current_focus: Option<String>,
    pub goals: HashMap<String, WorkflowGoal>,
    pub goal_hierarchy: Vec<String>,
    pub execution_history: Vec<WorkflowAction>,
    pub shared_context: Value,
    pub created_at: DateTime<Utc>,
    pub last_refinement: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAction {
    pub action_type: ActionType,
    pub goal_id: String,
    pub timestamp: DateTime<Utc>,
    pub input: Value,
    pub output: Value,
    pub llm_reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    GoalCreated,
    GoalRefined,
    SubGoalsGenerated,
    TasksPlanned,
    TaskCompleted,
    WorkflowCompleted,
}

pub struct WorkflowOrchestrator {
    llm_client: OpenRouterClient,
    task_executor: TaskExecutor,
    feedback_manager: FeedbackLoopManager,
    active_workflows: Arc<Mutex<HashMap<String, WorkflowState>>>,
}

impl WorkflowOrchestrator {
    pub async fn new() -> Result<Self> {
        log_info!("workflow", "🧠 Initializing LLM-driven Workflow Orchestrator");
        
        let llm_client = OpenRouterClient::new().await?;
        let task_executor = TaskExecutor::with_llm_analysis().await
            .unwrap_or_else(|_| TaskExecutor::new());
        let feedback_manager = FeedbackLoopManager::with_llm_client().await
            .unwrap_or_else(|_| FeedbackLoopManager::new());
        
        let orchestrator = Self {
            llm_client,
            task_executor,
            feedback_manager,
            active_workflows: Arc::new(Mutex::new(HashMap::new())),
        };
        
        // Load existing workflows from disk
        orchestrator.load_workflows_from_disk().await?;
        
        Ok(orchestrator)
    }

    /// Get the workflows storage directory
    fn get_workflows_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?;
        let workflows_dir = home_dir.join(".cai").join("workflows");
        
        // Create directory if it doesn't exist
        if !workflows_dir.exists() {
            fs::create_dir_all(&workflows_dir)
                .map_err(|e| anyhow!("Failed to create workflows directory: {}", e))?;
        }
        
        Ok(workflows_dir)
    }

    /// Load workflows from disk
    async fn load_workflows_from_disk(&self) -> Result<()> {
        let workflows_dir = Self::get_workflows_dir()?;
        
        if !workflows_dir.exists() {
            return Ok(()); // No workflows to load
        }
        
        let entries = fs::read_dir(&workflows_dir)
            .map_err(|e| anyhow!("Failed to read workflows directory: {}", e))?;
        
        let mut workflows = self.active_workflows.lock().await;
        let mut loaded_count = 0;
        
        for entry in entries {
            let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match fs::read_to_string(&path) {
                        Ok(content) => {
                            match serde_json::from_str::<WorkflowState>(&content) {
                                Ok(workflow_state) => {
                                    workflows.insert(stem.to_string(), workflow_state);
                                    loaded_count += 1;
                                }
                                Err(e) => {
                                    log_debug!("workflow", "⚠️ Failed to parse workflow {}: {}", stem, e);
                                }
                            }
                        }
                        Err(e) => {
                            log_debug!("workflow", "⚠️ Failed to read workflow file {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        
        if loaded_count > 0 {
            log_info!("workflow", "📂 Loaded {} existing workflow(s) from disk", loaded_count);
        }
        
        Ok(())
    }

    /// Save workflow to disk
    async fn save_workflow_to_disk(&self, workflow_id: &str) -> Result<()> {
        let workflows_dir = Self::get_workflows_dir()?;
        let workflow_file = workflows_dir.join(format!("{}.json", workflow_id));
        
        let workflow_state = {
            let workflows = self.active_workflows.lock().await;
            workflows.get(workflow_id).cloned()
        };
        
        if let Some(workflow_state) = workflow_state {
            let content = serde_json::to_string_pretty(&workflow_state)
                .map_err(|e| anyhow!("Failed to serialize workflow: {}", e))?;
            
            fs::write(&workflow_file, content)
                .map_err(|e| anyhow!("Failed to write workflow file: {}", e))?;
            
            log_debug!("workflow", "💾 Saved workflow {} to disk", workflow_id);
        }
        
        Ok(())
    }

    /// Remove workflow file from disk
    async fn remove_workflow_from_disk(&self, workflow_id: &str) -> Result<()> {
        let workflows_dir = Self::get_workflows_dir()?;
        let workflow_file = workflows_dir.join(format!("{}.json", workflow_id));
        
        if workflow_file.exists() {
            fs::remove_file(&workflow_file)
                .map_err(|e| anyhow!("Failed to remove workflow file: {}", e))?;
            log_debug!("workflow", "🗑️ Removed workflow {} from disk", workflow_id);
        }
        
        Ok(())
    }

    /// Start a new workflow with LLM-driven goal creation
    pub async fn start_workflow(&self, user_request: &str) -> Result<String> {
        log_info!("workflow", "🚀 Starting new workflow for: {}", user_request);
        
        // Use LLM to create the initial goal structure
        let initial_analysis = self.llm_analyze_request_for_goals(user_request).await?;
        
        let workflow_id = Uuid::new_v4().to_string();
        let root_goal_id = Uuid::new_v4().to_string();
        
        let root_goal = WorkflowGoal {
            id: root_goal_id.clone(),
            description: initial_analysis.main_goal,
            parent_goal_id: None,
            sub_goals: vec![],
            status: GoalStatus::Planning,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            context: serde_json::json!({"user_request": user_request}),
            success_criteria: initial_analysis.success_criteria,
            completion_percentage: 0.0,
        };

        let workflow_state = WorkflowState {
            workflow_id: workflow_id.clone(),
            root_goal: root_goal_id.clone(),
            current_focus: Some(root_goal_id.clone()),
            goals: {
                let mut goals = HashMap::new();
                goals.insert(root_goal_id.clone(), root_goal);
                goals
            },
            goal_hierarchy: vec![root_goal_id.clone()],
            execution_history: vec![],
            shared_context: serde_json::json!({"original_request": user_request}),
            created_at: Utc::now(),
            last_refinement: None,
        };

        {
            let mut workflows = self.active_workflows.lock().await;
            workflows.insert(workflow_id.clone(), workflow_state);
        }

        // Save workflow to disk
        self.save_workflow_to_disk(&workflow_id).await?;

        log_info!("workflow", "✅ Created workflow {} with root goal", workflow_id);
        
        // Immediately start planning sub-goals
        self.plan_sub_goals(&workflow_id, &root_goal_id).await?;
        
        Ok(workflow_id)
    }

    /// LLM-driven sub-goal planning and refinement
    pub async fn plan_sub_goals(&self, workflow_id: &str, goal_id: &str) -> Result<()> {
        log_info!("workflow", "🧠 LLM planning sub-goals for goal: {}", goal_id);
        
        let (goal_description, context, success_criteria) = {
            let workflows = self.active_workflows.lock().await;
            let workflow = workflows.get(workflow_id)
                .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;
            let goal = workflow.goals.get(goal_id)
                .ok_or_else(|| anyhow!("Goal not found: {}", goal_id))?;
            
            (goal.description.clone(), workflow.shared_context.clone(), goal.success_criteria.clone())
        };

        // Gather historical context for better planning
        let historical_context = self.feedback_manager
            .gather_context_for_task(&goal_description).await
            .unwrap_or_default();

        let sub_goal_analysis = self.llm_create_sub_goals(
            &goal_description,
            &context,
            &success_criteria,
            &historical_context
        ).await?;

        // Create sub-goals based on LLM analysis
        let mut sub_goal_ids = vec![];
        {
            let mut workflows = self.active_workflows.lock().await;
            let workflow = workflows.get_mut(workflow_id).unwrap();
            
            for sub_goal_desc in sub_goal_analysis.sub_goals.iter() {
                let sub_goal_id = Uuid::new_v4().to_string();
                let sub_goal = WorkflowGoal {
                    id: sub_goal_id.clone(),
                    description: sub_goal_desc.description.clone(),
                    parent_goal_id: Some(goal_id.to_string()),
                    sub_goals: vec![],
                    status: GoalStatus::Pending,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    context: serde_json::json!({
                        "priority": sub_goal_desc.priority,
                        "dependencies": sub_goal_desc.dependencies,
                        "estimated_complexity": sub_goal_desc.estimated_complexity
                    }),
                    success_criteria: sub_goal_desc.success_criteria.clone(),
                    completion_percentage: 0.0,
                };
                
                workflow.goals.insert(sub_goal_id.clone(), sub_goal);
                sub_goal_ids.push(sub_goal_id);
            }
            
            // Update parent goal with sub-goals
            if let Some(parent_goal) = workflow.goals.get_mut(goal_id) {
                parent_goal.sub_goals = sub_goal_ids.clone();
                parent_goal.status = GoalStatus::InProgress;
                parent_goal.updated_at = Utc::now();
            }

            // Record the planning action
            let action = WorkflowAction {
                action_type: ActionType::SubGoalsGenerated,
                goal_id: goal_id.to_string(),
                timestamp: Utc::now(),
                input: serde_json::json!({"goal_description": goal_description}),
                output: serde_json::json!({"sub_goals": sub_goal_ids.clone()}),
                llm_reasoning: Some(sub_goal_analysis.reasoning),
            };
            workflow.execution_history.push(action);
        }

        // Save workflow changes to disk
        self.save_workflow_to_disk(workflow_id).await?;

        log_info!("workflow", "✅ Created {} sub-goals for goal {}", sub_goal_ids.len(), goal_id);
        
        // Record feedback for learning
        let _ = self.feedback_manager.add_feedback(
            FeedbackType::ContextRefinement,
            format!("Sub-goal planning for: {}", goal_description),
            serde_json::json!({"goal": goal_description, "context": context}),
            serde_json::json!({"sub_goals": sub_goal_ids}),
            Some(0.8), // Initial quality score
        ).await;

        Ok(())
    }

    /// Execute next available goal with LLM guidance
    pub async fn execute_next_goal(&self, workflow_id: &str) -> Result<bool> {
        log_debug!("workflow", "🎯 Finding next goal to execute in workflow {}", workflow_id);
        
        let next_goal_id = {
            let workflows = self.active_workflows.lock().await;
            let workflow = workflows.get(workflow_id)
                .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;
            
            // Find next ready goal (no dependencies or dependencies completed)
            self.find_next_executable_goal(workflow)
        };

        match next_goal_id {
            Some(goal_id) => {
                log_info!("workflow", "🔄 Executing goal: {}", goal_id);
                self.execute_goal(workflow_id, &goal_id).await?;
                Ok(true)
            }
            None => {
                log_debug!("workflow", "⏸️ No executable goals found, checking for completion");
                self.check_workflow_completion(workflow_id).await?;
                Ok(false)
            }
        }
    }

    /// Execute a specific goal with LLM task planning
    async fn execute_goal(&self, workflow_id: &str, goal_id: &str) -> Result<()> {
        log_info!("workflow", "⚡ Executing goal: {}", goal_id);
        
        // Update goal status
        {
            let mut workflows = self.active_workflows.lock().await;
            let workflow = workflows.get_mut(workflow_id).unwrap();
            if let Some(goal) = workflow.goals.get_mut(goal_id) {
                goal.status = GoalStatus::InProgress;
                goal.updated_at = Utc::now();
            }
            workflow.current_focus = Some(goal_id.to_string());
        }

        // Get goal details and context
        let (goal_description, goal_context, shared_context) = {
            let workflows = self.active_workflows.lock().await;
            let workflow = workflows.get(workflow_id).unwrap();
            let goal = workflow.goals.get(goal_id).unwrap();
            (goal.description.clone(), goal.context.clone(), workflow.shared_context.clone())
        };

        // Use LLM to plan tasks for this goal
        let task_plan = self.llm_plan_tasks_for_goal(
            &goal_description,
            &goal_context,
            &shared_context
        ).await?;

        log_debug!("workflow", "📋 Generated {} tasks for goal", task_plan.len());

        // Execute tasks using existing TaskExecutor
        self.task_executor.add_tasks(task_plan).await?;
        self.task_executor.execute_all().await?;

        // Update goal status based on execution results
        let success = self.task_executor.all_tasks_completed().await;
        
        {
            let mut workflows = self.active_workflows.lock().await;
            let workflow = workflows.get_mut(workflow_id).unwrap();
            if let Some(goal) = workflow.goals.get_mut(goal_id) {
                goal.status = if success { GoalStatus::Completed } else { GoalStatus::Failed };
                goal.completion_percentage = if success { 100.0 } else { 50.0 };
                goal.updated_at = Utc::now();
            }

            // Record execution action
            let action = WorkflowAction {
                action_type: ActionType::TaskCompleted,
                goal_id: goal_id.to_string(),
                timestamp: Utc::now(),
                input: serde_json::json!({"goal_description": goal_description}),
                output: serde_json::json!({"success": success}),
                llm_reasoning: Some("Task execution completed".to_string()),
            };
            workflow.execution_history.push(action);
        }

        // Save workflow changes to disk
        self.save_workflow_to_disk(workflow_id).await?;

        // Clean up completed tasks
        self.task_executor.clear_completed_tasks().await;

        // Check if goal completion enables refinement of parent goals
        if success {
            self.trigger_goal_refinement_if_needed(workflow_id, goal_id).await?;
        }

        log_info!("workflow", "✅ Goal execution completed: {} (success: {})", goal_id, success);
        Ok(())
    }

    /// LLM-driven goal refinement based on results
    async fn trigger_goal_refinement_if_needed(&self, workflow_id: &str, completed_goal_id: &str) -> Result<()> {
        log_debug!("workflow", "🔄 Checking if goal refinement is needed after completing {}", completed_goal_id);
        
        let should_refine = {
            let workflows = self.active_workflows.lock().await;
            let workflow = workflows.get(workflow_id).unwrap();
            let goal = workflow.goals.get(completed_goal_id).unwrap();
            
            // Refine if this was a significant goal with a parent
            goal.parent_goal_id.is_some() && goal.completion_percentage > 80.0
        };

        if should_refine {
            let parent_goal_id = {
                let workflows = self.active_workflows.lock().await;
                let workflow = workflows.get(workflow_id).unwrap();
                workflow.goals.get(completed_goal_id).unwrap().parent_goal_id.clone()
            };

            if let Some(parent_id) = parent_goal_id {
                self.refine_goal_based_on_progress(workflow_id, &parent_id).await?;
            }
        }

        Ok(())
    }

    /// Refine goals using LLM analysis of current progress
    async fn refine_goal_based_on_progress(&self, workflow_id: &str, goal_id: &str) -> Result<()> {
        log_info!("workflow", "🧠 LLM refining goal based on progress: {}", goal_id);
        
        let (goal, completed_sub_goals, workflow_context) = {
            let workflows = self.active_workflows.lock().await;
            let workflow = workflows.get(workflow_id).unwrap();
            let goal = workflow.goals.get(goal_id).unwrap().clone();
            
            let completed_sub_goals: Vec<WorkflowGoal> = goal.sub_goals.iter()
                .filter_map(|id| workflow.goals.get(id))
                .filter(|g| g.status == GoalStatus::Completed)
                .cloned()
                .collect();
                
            (goal, completed_sub_goals, workflow.shared_context.clone())
        };

        if completed_sub_goals.is_empty() {
            return Ok(()); // Nothing to refine yet
        }

        let refinement_analysis = self.llm_refine_goal_based_on_results(
            &goal,
            &completed_sub_goals,
            &workflow_context
        ).await?;

        // Apply refinements
        let refinement_reasoning = refinement_analysis.reasoning.clone();
        {
            let mut workflows = self.active_workflows.lock().await;
            let workflow = workflows.get_mut(workflow_id).unwrap();
            
            let should_add_goals = refinement_analysis.should_add_goals;
            let new_goals = refinement_analysis.new_goals;
            let updated_success_criteria = refinement_analysis.updated_success_criteria;
            
            // Collect new goal IDs first
            let mut new_goal_ids = Vec::new();
            if should_add_goals {
                for new_goal_desc in new_goals {
                    let new_goal_id = Uuid::new_v4().to_string();
                    let new_goal = WorkflowGoal {
                        id: new_goal_id.clone(),
                        description: new_goal_desc.description,
                        parent_goal_id: Some(goal_id.to_string()),
                        sub_goals: vec![],
                        status: GoalStatus::Pending,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        context: serde_json::json!({"generated_from_refinement": true}),
                        success_criteria: new_goal_desc.success_criteria,
                        completion_percentage: 0.0,
                    };
                    
                    workflow.goals.insert(new_goal_id.clone(), new_goal);
                    new_goal_ids.push(new_goal_id);
                }
            }
            
            // Now update the refined goal
            if let Some(refined_goal) = workflow.goals.get_mut(goal_id) {
                refined_goal.status = GoalStatus::Refined;
                refined_goal.updated_at = Utc::now();
                refined_goal.sub_goals.extend(new_goal_ids);
                
                // Update success criteria if refined
                if !updated_success_criteria.is_empty() {
                    refined_goal.success_criteria = updated_success_criteria;
                }
            }

            workflow.last_refinement = Some(Utc::now());
            
            // Record refinement action
            let action = WorkflowAction {
                action_type: ActionType::GoalRefined,
                goal_id: goal_id.to_string(),
                timestamp: Utc::now(),
                input: serde_json::json!({"completed_sub_goals": completed_sub_goals.len()}),
                output: serde_json::json!({"should_add_goals": should_add_goals}),
                llm_reasoning: Some(refinement_reasoning),
            };
            workflow.execution_history.push(action);
        }

        // Save workflow changes to disk
        self.save_workflow_to_disk(workflow_id).await?;

        log_info!("workflow", "✅ Goal refinement completed for: {}", goal_id);
        Ok(())
    }

    /// Display workflow status with visual goal hierarchy
    pub async fn display_workflow_status(&self, workflow_id: &str) -> Result<()> {
        let workflows = self.active_workflows.lock().await;
        let workflow = workflows.get(workflow_id)
            .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;

        println!("\n{} Workflow Status: {}", "🧠".bright_blue().bold(), workflow_id);
        println!("{}", "─".repeat(60));
        
        // Display root goal
        if let Some(root_goal) = workflow.goals.get(&workflow.root_goal) {
            self.display_goal_tree(workflow, root_goal, 0);
        }

        // Display current focus
        if let Some(focus_id) = &workflow.current_focus {
            if let Some(focus_goal) = workflow.goals.get(focus_id) {
                println!("\n{} Current Focus: {}", "🎯".yellow(), focus_goal.description.bright_white());
            }
        }

        // Display recent actions
        let recent_actions: Vec<_> = workflow.execution_history.iter()
            .rev()
            .take(3)
            .collect();
            
        if !recent_actions.is_empty() {
            println!("\n{} Recent Actions:", "📋".cyan());
            for action in recent_actions {
                println!("  {} {:?}: {}", 
                    match action.action_type {
                        ActionType::GoalCreated => "🎯",
                        ActionType::GoalRefined => "🔄",
                        ActionType::SubGoalsGenerated => "🧠",
                        ActionType::TasksPlanned => "📋",
                        ActionType::TaskCompleted => "✅",
                        ActionType::WorkflowCompleted => "🎉",
                    },
                    action.action_type,
                    action.goal_id
                );
            }
        }

        println!("{}", "─".repeat(60));
        Ok(())
    }

    fn display_goal_tree(&self, workflow: &WorkflowState, goal: &WorkflowGoal, indent: usize) {
        let indent_str = "  ".repeat(indent);
        let status_icon = goal.status.icon();
        let progress = if goal.completion_percentage > 0.0 {
            format!(" ({}%)", goal.completion_percentage as u32)
        } else {
            String::new()
        };

        println!("{}{}️ {} {}{}", 
            indent_str, 
            status_icon,
            goal.description.bright_white(),
            progress.dimmed(),
            if goal.status == GoalStatus::InProgress { " ←" } else { "" }.yellow()
        );

        // Display sub-goals recursively
        for sub_goal_id in &goal.sub_goals {
            if let Some(sub_goal) = workflow.goals.get(sub_goal_id) {
                self.display_goal_tree(workflow, sub_goal, indent + 1);
            }
        }
    }

    // Helper method to find next executable goal
    fn find_next_executable_goal(&self, workflow: &WorkflowState) -> Option<String> {
        // Simple strategy: find first pending goal with no pending dependencies
        for goal in workflow.goals.values() {
            if goal.status == GoalStatus::Pending || goal.status == GoalStatus::Refined {
                // Check if this goal is ready (no dependencies or all dependencies completed)
                if self.is_goal_ready_for_execution(workflow, goal) {
                    return Some(goal.id.clone());
                }
            }
        }
        None
    }

    fn is_goal_ready_for_execution(&self, workflow: &WorkflowState, goal: &WorkflowGoal) -> bool {
        // For now, simple check - could be enhanced with actual dependency tracking
        goal.sub_goals.is_empty() || goal.sub_goals.iter().all(|sub_id| {
            workflow.goals.get(sub_id)
                .map(|sub_goal| sub_goal.status == GoalStatus::Completed)
                .unwrap_or(false)
        })
    }

    async fn check_workflow_completion(&self, workflow_id: &str) -> Result<()> {
        let all_completed = {
            let workflows = self.active_workflows.lock().await;
            let workflow = workflows.get(workflow_id).unwrap();
            
            workflow.goals.values().all(|goal| {
                matches!(goal.status, GoalStatus::Completed | GoalStatus::Failed)
            })
        };

        if all_completed {
            log_info!("workflow", "🎉 Workflow {} completed!", workflow_id);
            println!("\n{} Workflow completed! 🎉", "✅".green().bold());
            
            // Record completion
            {
                let mut workflows = self.active_workflows.lock().await;
                if let Some(workflow) = workflows.get_mut(workflow_id) {
                    let action = WorkflowAction {
                        action_type: ActionType::WorkflowCompleted,
                        goal_id: workflow.root_goal.clone(),
                        timestamp: Utc::now(),
                        input: serde_json::json!({"workflow_id": workflow_id}),
                        output: serde_json::json!({"success": true}),
                        llm_reasoning: Some("All goals completed".to_string()),
                    };
                    workflow.execution_history.push(action);
                }
            }
        }

        // Save workflow completion to disk
        self.save_workflow_to_disk(workflow_id).await?;

        Ok(())
    }

    // LLM interaction methods for intelligent workflow management

    /// LLM analyzes user request to create initial goal structure
    async fn llm_analyze_request_for_goals(&self, user_request: &str) -> Result<InitialGoalAnalysis> {
        let prompt = format!(
            r#"You are an intelligent workflow planner. Analyze the user request and create a comprehensive goal structure.

## User Request
{}

## Analysis Framework
1. **Main Goal**: What is the primary objective the user wants to achieve?
2. **Success Criteria**: How will we know when this is successfully completed?
3. **Complexity Assessment**: Is this simple, moderate, or complex?

## Response Format
Respond with ONLY a valid JSON object in this exact format:

```json
{{
  "main_goal": "Clear, actionable description of the primary objective",
  "success_criteria": [
    "Specific measurable criteria 1",
    "Specific measurable criteria 2"
  ],
  "estimated_complexity": "simple|moderate|complex",
  "reasoning": "Brief explanation of your analysis and approach"
}}
```

Focus on creating goals that are:
- Specific and actionable
- Measurable with clear success criteria  
- Achievable through task execution
- Relevant to the user's actual need

JSON Response:"#,
            user_request
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let response = self.llm_client.chat_completion(messages).await?;
        let json_str = self.extract_json_from_response(&response);
        
        serde_json::from_str::<InitialGoalAnalysis>(&json_str)
            .map_err(|e| anyhow!("Failed to parse initial goal analysis: {}", e))
    }

    /// LLM creates sub-goals based on a parent goal and context
    async fn llm_create_sub_goals(
        &self,
        goal_description: &str,
        context: &Value,
        success_criteria: &[String],
        historical_context: &str,
    ) -> Result<SubGoalAnalysis> {
        let prompt = format!(
            r#"You are breaking down a complex goal into actionable sub-goals. Use context and historical learnings to create an effective plan.

## Parent Goal
{}

## Success Criteria
{}

## Context
{}

## Historical Insights
{}

## Sub-Goal Planning Guidelines
1. **Decomposition**: Break the goal into 2-5 logical sub-goals
2. **Dependencies**: Consider what must be done before other things
3. **Actionability**: Each sub-goal should be concrete and executable
4. **Priority**: Order by importance and logical sequence
5. **Measurability**: Include clear success criteria for each sub-goal

## Response Format
Respond with ONLY a valid JSON object:

```json
{{
  "sub_goals": [
    {{
      "description": "Specific sub-goal description",
      "priority": 1,
      "dependencies": ["list", "of", "dependency", "descriptions"],
      "success_criteria": ["measurable criteria 1", "criteria 2"],
      "estimated_complexity": "simple|moderate|complex"
    }}
  ],
  "reasoning": "Explanation of decomposition strategy",
  "execution_strategy": "How these sub-goals work together"
}}
```

JSON Response:"#,
            goal_description,
            success_criteria.join(", "),
            serde_json::to_string_pretty(context)?,
            historical_context
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let response = self.llm_client.chat_completion(messages).await?;
        let json_str = self.extract_json_from_response(&response);
        
        serde_json::from_str::<SubGoalAnalysis>(&json_str)
            .map_err(|e| anyhow!("Failed to parse sub-goal analysis: {}", e))
    }

    /// LLM plans specific tasks for a goal
    async fn llm_plan_tasks_for_goal(
        &self,
        goal_description: &str,
        goal_context: &Value,
        shared_context: &Value,
    ) -> Result<Vec<String>> {
        let prompt = format!(
            r#"You are creating specific, actionable tasks to accomplish a goal. Focus on concrete actions that can be executed.

## Goal to Accomplish
{}

## Goal-Specific Context
{}

## Workflow Context
{}

## Task Planning Guidelines
1. **Actionable**: Each task should be a specific action
2. **Executable**: Tasks should be doable with available MCP tools
3. **Sequential**: Order tasks logically
4. **Atomic**: Each task should be focused on one outcome
5. **Clear**: No ambiguity about what needs to be done

## Response Format
Respond with a simple numbered list of tasks:

1. First specific task
2. Second specific task
3. Third specific task

Focus on tasks that are:
- Concrete and specific
- Can be executed with file system tools, code analysis, etc.
- Build toward the goal completion
- Are appropriately sized (not too big or small)

Task List:"#,
            goal_description,
            serde_json::to_string_pretty(goal_context)?,
            serde_json::to_string_pretty(shared_context)?
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let response = self.llm_client.chat_completion(messages).await?;
        
        // Parse numbered task list (reuse existing logic from openrouter_client)
        let tasks: Vec<String> = response
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if let Some(pos) = line.find(". ") {
                    let number_part = &line[..pos];
                    if number_part.chars().all(|c| c.is_ascii_digit()) {
                        return Some(line[pos + 2..].trim().to_string());
                    }
                }
                None
            })
            .collect();

        if tasks.is_empty() {
            // Fallback: treat entire response as single task
            Ok(vec![response.trim().to_string()])
        } else {
            Ok(tasks)
        }
    }

    /// LLM refines goals based on completion results
    async fn llm_refine_goal_based_on_results(
        &self,
        goal: &WorkflowGoal,
        completed_sub_goals: &[WorkflowGoal],
        workflow_context: &Value,
    ) -> Result<GoalRefinementAnalysis> {
        let completed_descriptions: Vec<String> = completed_sub_goals
            .iter()
            .map(|g| format!("✅ {} ({}%)", g.description, g.completion_percentage))
            .collect();

        let prompt = format!(
            r#"You are analyzing completed sub-goals to determine if the parent goal needs refinement or additional sub-goals.

## Parent Goal
**Description**: {}
**Current Status**: {:?}
**Success Criteria**: {}

## Completed Sub-Goals
{}

## Workflow Context
{}

## Refinement Analysis Guidelines
1. **Gap Analysis**: Are there missing pieces to fully achieve the parent goal?
2. **Quality Assessment**: Do completed sub-goals actually advance the parent goal?
3. **Success Criteria Check**: Are we on track to meet the original success criteria?
4. **Adaptive Planning**: What new insights suggest additional work?

## Response Format
Respond with ONLY a valid JSON object:

```json
{{
  "should_add_goals": true/false,
  "new_goals": [
    {{
      "description": "Additional sub-goal description",
      "priority": 1,
      "dependencies": [],
      "success_criteria": ["criteria"],
      "estimated_complexity": "simple"
    }}
  ],
  "updated_success_criteria": ["refined criteria if needed"],
  "reasoning": "Detailed explanation of refinement decisions"
}}
```

JSON Response:"#,
            goal.description,
            goal.status,
            goal.success_criteria.join(", "),
            completed_descriptions.join("\n"),
            serde_json::to_string_pretty(workflow_context)?
        );

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let response = self.llm_client.chat_completion(messages).await?;
        let json_str = self.extract_json_from_response(&response);
        
        serde_json::from_str::<GoalRefinementAnalysis>(&json_str)
            .map_err(|e| anyhow!("Failed to parse goal refinement analysis: {}", e))
    }

    /// Extract JSON from LLM response (same as in openrouter_client)
    fn extract_json_from_response(&self, response: &str) -> String {
        if let Some(start) = response.find("```json") {
            if let Some(end) = response[start..].find("```") {
                let json_start = start + 7;
                let json_end = start + end;
                if json_start < json_end {
                    return response[json_start..json_end].trim().to_string();
                }
            }
        }

        if let Some(start) = response.find("```") {
            if let Some(end) = response[start + 3..].find("```") {
                let json_start = start + 3;
                let json_end = start + 3 + end;
                if json_start < json_end {
                    let potential_json = response[json_start..json_end].trim();
                    if potential_json.starts_with('{') && potential_json.ends_with('}') {
                        return potential_json.to_string();
                    }
                }
            }
        }

        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if start < end {
                    return response[start..=end].trim().to_string();
                }
            }
        }

        response.trim().to_string()
    }

    /// Get all active workflows
    pub async fn list_active_workflows(&self) -> Result<Vec<String>> {
        let workflows = self.active_workflows.lock().await;
        Ok(workflows.keys().cloned().collect())
    }

    /// Remove completed workflow
    pub async fn cleanup_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.active_workflows.lock().await;
        workflows.remove(workflow_id);
        
        // Remove from disk
        self.remove_workflow_from_disk(workflow_id).await?;
        
        log_info!("workflow", "🧹 Cleaned up workflow: {}", workflow_id);
        Ok(())
    }
}

// LLM Response structures
#[derive(Debug, Deserialize)]
struct InitialGoalAnalysis {
    main_goal: String,
    success_criteria: Vec<String>,
    estimated_complexity: String,
    reasoning: String,
}

#[derive(Debug, Deserialize)]
struct SubGoalAnalysis {
    sub_goals: Vec<SubGoalDescription>,
    reasoning: String,
    execution_strategy: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubGoalDescription {
    description: String,
    priority: u32,
    dependencies: Vec<String>,
    success_criteria: Vec<String>,
    estimated_complexity: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoalRefinementAnalysis {
    should_add_goals: bool,
    new_goals: Vec<SubGoalDescription>,
    updated_success_criteria: Vec<String>,
    reasoning: String,
}