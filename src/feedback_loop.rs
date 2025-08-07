use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::logger::{log_info, log_debug, log_warn, ops};
use crate::openrouter_client::{OpenRouterClient, ChatMessage};

/// Represents different types of feedback in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    ContextRefinement,
    PlanValidation,
    IterativeImprovement,
    ArchitecturalKnowledge,
    ToolResultAnalysis,
    TestDrivenDevelopment,
}

/// Feedback entry that captures learning and adaptation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub id: String,
    pub feedback_type: FeedbackType,
    pub timestamp: DateTime<Utc>,
    pub context: String,
    pub input: Value,
    pub output: Value,
    pub quality_score: Option<f64>,
    pub human_validation: Option<bool>,
    pub iteration_number: u32,
    pub improvement_notes: Option<String>,
}

/// Plan validation result with human feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidation {
    pub plan_id: String,
    pub approved: bool,
    pub feedback: String,
    pub suggested_modifications: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// Context accumulation for continuous learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAccumulation {
    pub domain: String,
    pub patterns: HashMap<String, u32>,
    pub successful_approaches: Vec<String>,
    pub failed_approaches: Vec<String>,
    pub architectural_insights: Vec<String>,
}

/// Iterative improvement tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterativeImprovement {
    pub task_id: String,
    pub iterations: Vec<IterationResult>,
    pub convergence_metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationResult {
    pub iteration: u32,
    pub input: Value,
    pub output: Value,
    pub quality_metrics: HashMap<String, f64>,
    pub feedback_incorporated: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// Dynamic feedback loop manager
pub struct FeedbackLoopManager {
    feedback_history: Arc<Mutex<VecDeque<FeedbackEntry>>>,
    context_accumulations: Arc<Mutex<HashMap<String, ContextAccumulation>>>,
    plan_validations: Arc<Mutex<HashMap<String, PlanValidation>>>,
    iterative_improvements: Arc<Mutex<HashMap<String, IterativeImprovement>>>,
    openrouter_client: Option<OpenRouterClient>,
    max_history_size: usize,
}

impl FeedbackLoopManager {
    pub fn new() -> Self {
        Self {
            feedback_history: Arc::new(Mutex::new(VecDeque::new())),
            context_accumulations: Arc::new(Mutex::new(HashMap::new())),
            plan_validations: Arc::new(Mutex::new(HashMap::new())),
            iterative_improvements: Arc::new(Mutex::new(HashMap::new())),
            openrouter_client: None,
            max_history_size: 1000,
        }
    }

    pub async fn with_llm_client() -> Result<Self> {
        let client = OpenRouterClient::new().await?;
        Ok(Self {
            feedback_history: Arc::new(Mutex::new(VecDeque::new())),
            context_accumulations: Arc::new(Mutex::new(HashMap::new())),
            plan_validations: Arc::new(Mutex::new(HashMap::new())),
            iterative_improvements: Arc::new(Mutex::new(HashMap::new())),
            openrouter_client: Some(client),
            max_history_size: 1000,
        })
    }

    /// Add feedback entry to the system
    pub async fn add_feedback(&self, 
        feedback_type: FeedbackType,
        context: String,
        input: Value,
        output: Value,
        quality_score: Option<f64>,
    ) -> Result<String> {
        let entry = FeedbackEntry {
            id: Uuid::new_v4().to_string(),
            feedback_type: feedback_type.clone(),
            timestamp: Utc::now(),
            context,
            input,
            output,
            quality_score,
            human_validation: None,
            iteration_number: 1,
            improvement_notes: None,
        };

        let entry_id = entry.id.clone();
        
        {
            let mut history = self.feedback_history.lock().await;
            history.push_back(entry.clone());
            
            // Maintain history size limit
            if history.len() > self.max_history_size {
                history.pop_front();
            }
        }

        log_info!("feedback", "📝 Added feedback entry: {:?} (ID: {})", feedback_type, entry_id);
        ops::feedback_operation("ADD_FEEDBACK", &format!("{:?}", feedback_type));

        Ok(entry_id)
    }

    /// Gather context from recent feedback to inform future decisions
    pub async fn gather_context_for_task(&self, task_context: &str) -> Result<String> {
        log_debug!("feedback", "🔍 Gathering context for task: {}", task_context);
        
        let history = self.feedback_history.lock().await;
        let recent_feedback: Vec<&FeedbackEntry> = history
            .iter()
            .rev()
            .take(20) // Last 20 entries
            .filter(|entry| {
                entry.context.to_lowercase().contains(&task_context.to_lowercase()) ||
                self.is_context_relevant(&entry.context, task_context)
            })
            .collect();

        if recent_feedback.is_empty() {
            return Ok("No relevant historical context found.".to_string());
        }

        let mut context_summary = String::new();
        context_summary.push_str("## Relevant Historical Context\n\n");

        // Group by feedback type
        let mut grouped_feedback: HashMap<String, Vec<&FeedbackEntry>> = HashMap::new();
        for entry in recent_feedback {
            let type_key = format!("{:?}", entry.feedback_type);
            grouped_feedback.entry(type_key).or_default().push(entry);
        }

        for (feedback_type, entries) in grouped_feedback {
            context_summary.push_str(&format!("### {} Insights\n", feedback_type));
            
            for entry in entries.iter().take(3) { // Top 3 per type
                if let Some(score) = entry.quality_score {
                    context_summary.push_str(&format!("- **Quality Score: {:.2}** - {}\n", 
                        score, entry.context));
                } else {
                    context_summary.push_str(&format!("- {}\n", entry.context));
                }
                
                if let Some(notes) = &entry.improvement_notes {
                    context_summary.push_str(&format!("  *Improvement: {}*\n", notes));
                }
            }
            context_summary.push('\n');
        }

        log_debug!("feedback", "📊 Generated context summary ({} chars)", context_summary.len());
        Ok(context_summary)
    }

    /// Create a plan with validation checkpoint
    pub async fn create_validated_plan(&self, user_request: &str, task_context: &str) -> Result<(String, String)> {
        if let Some(ref client) = self.openrouter_client {
            log_info!("feedback", "📋 Creating validated plan for request: {}", user_request);
            
            // Gather historical context
            let historical_context = self.gather_context_for_task(task_context).await?;
            
            // Create comprehensive prompt for plan generation
            let plan_prompt = format!(
                r#"You are an expert software architect creating a detailed implementation plan. Use the historical context to inform your planning decisions.

## User Request
{}

## Task Context
{}

## Historical Context
{}

## Plan Requirements
Create a detailed, step-by-step implementation plan that:
1. Breaks down the request into specific, actionable tasks
2. Identifies potential risks and mitigation strategies
3. Considers architectural patterns from historical context
4. Includes validation checkpoints
5. Provides clear success criteria

## Response Format
Provide your response as a structured plan with:
- **Overview**: High-level approach summary
- **Tasks**: Numbered list of specific implementation steps
- **Risks**: Potential issues and how to address them
- **Validation**: How to verify each step's success
- **Success Criteria**: How to know the implementation is complete

Implementation Plan:"#,
                user_request, task_context, historical_context
            );

            let messages = vec![ChatMessage {
                role: "user".to_string(),
                content: plan_prompt,
            }];

            let plan = client.chat_completion(messages).await?;
            let plan_id = Uuid::new_v4().to_string();

            // Record plan creation
            self.add_feedback(
                FeedbackType::PlanValidation,
                format!("Plan created for: {}", user_request),
                serde_json::json!({"request": user_request, "context": task_context}),
                serde_json::json!({"plan": plan.clone(), "plan_id": plan_id.clone()}),
                None,
            ).await?;

            log_info!("feedback", "✅ Created plan {} for validation", plan_id);
            Ok((plan_id, plan))
        } else {
            Err(anyhow!("LLM client not available for plan creation"))
        }
    }

    /// Validate a plan with human feedback
    pub async fn validate_plan(&self, plan_id: &str, approved: bool, feedback: String, modifications: Vec<String>) -> Result<()> {
        let validation = PlanValidation {
            plan_id: plan_id.to_string(),
            approved,
            feedback: feedback.clone(),
            suggested_modifications: modifications,
            timestamp: Utc::now(),
        };

        {
            let mut validations = self.plan_validations.lock().await;
            validations.insert(plan_id.to_string(), validation);
        }

        // Record validation feedback
        self.add_feedback(
            FeedbackType::PlanValidation,
            format!("Plan validation: {}", if approved { "approved" } else { "rejected" }),
            serde_json::json!({"plan_id": plan_id}),
            serde_json::json!({"approved": approved, "feedback": feedback}),
            Some(if approved { 1.0 } else { 0.0 }),
        ).await?;

        log_info!("feedback", "🔍 Plan {} validation: {}", plan_id, if approved { "approved" } else { "rejected" });
        Ok(())
    }

    /// Perform iterative improvement on a task
    pub async fn iterative_improvement(&self, task_id: &str, initial_input: Value, max_iterations: u32) -> Result<Value> {
        if let Some(ref client) = self.openrouter_client {
            log_info!("feedback", "🔄 Starting iterative improvement for task: {}", task_id);
            
            let mut current_input = initial_input.clone();
            let mut iterations = Vec::new();
            let mut best_output = None;
            let mut best_score = 0.0;

            for iteration in 1..=max_iterations {
                log_debug!("feedback", "🔄 Iteration {} for task {}", iteration, task_id);
                
                // Gather context from previous iterations
                let iteration_context = if iterations.is_empty() {
                    "First iteration - no previous context available.".to_string()
                } else {
                    self.build_iteration_context(&iterations)
                };

                // Create improvement prompt
                let improvement_prompt = format!(
                    r#"You are improving a solution through iterative refinement. 

## Current Input/Task
{}

## Previous Iterations Context
{}

## Improvement Goals
- Enhance quality, clarity, and effectiveness
- Address any issues from previous iterations
- Incorporate learnings from iteration context
- Make meaningful improvements, not just cosmetic changes

## Instructions
Analyze the current solution and provide an improved version. Be specific about what improvements you're making and why.

Improved Solution:"#,
                    serde_json::to_string_pretty(&current_input)?,
                    iteration_context
                );

                let messages = vec![ChatMessage {
                    role: "user".to_string(),
                    content: improvement_prompt,
                }];

                let improved_output = client.chat_completion(messages).await?;
                
                // Calculate quality score (simplified - could be more sophisticated)
                let quality_score = self.calculate_quality_score(&improved_output, iteration);
                
                let iteration_result = IterationResult {
                    iteration,
                    input: current_input.clone(),
                    output: serde_json::json!(improved_output.clone()),
                    quality_metrics: {
                        let mut metrics = HashMap::new();
                        metrics.insert("quality_score".to_string(), quality_score);
                        metrics.insert("length".to_string(), improved_output.len() as f64);
                        metrics
                    },
                    feedback_incorporated: vec![], // Could be extracted from analysis
                    timestamp: Utc::now(),
                };

                iterations.push(iteration_result);

                // Track best result
                if quality_score > best_score {
                    best_score = quality_score;
                    best_output = Some(serde_json::json!(improved_output));
                }

                // Update input for next iteration
                current_input = serde_json::json!(improved_output);

                log_debug!("feedback", "✅ Iteration {} completed with quality score: {:.2}", iteration, quality_score);
            }

            // Store improvement tracking
            let improvement_tracking = IterativeImprovement {
                task_id: task_id.to_string(),
                iterations,
                convergence_metrics: {
                    let mut metrics = HashMap::new();
                    metrics.insert("final_quality_score".to_string(), best_score);
                    metrics.insert("total_iterations".to_string(), max_iterations as f64);
                    metrics
                },
            };

            {
                let mut improvements = self.iterative_improvements.lock().await;
                improvements.insert(task_id.to_string(), improvement_tracking);
            }

            // Record overall improvement feedback
            self.add_feedback(
                FeedbackType::IterativeImprovement,
                format!("Iterative improvement completed for task: {}", task_id),
                initial_input,
                best_output.clone().unwrap_or_default(),
                Some(best_score),
            ).await?;

            log_info!("feedback", "🎉 Iterative improvement completed for task {} with final score: {:.2}", task_id, best_score);
            return Ok(best_output.unwrap_or_default());
        }

        Err(anyhow!("LLM client not available for iterative improvement"))
    }

    /// Accumulate architectural knowledge from successful patterns
    pub async fn accumulate_architectural_knowledge(&self, domain: &str, pattern: &str, success: bool) -> Result<()> {
        let mut accumulations = self.context_accumulations.lock().await;
        
        let accumulation = accumulations.entry(domain.to_string()).or_insert_with(|| {
            ContextAccumulation {
                domain: domain.to_string(),
                patterns: HashMap::new(),
                successful_approaches: Vec::new(),
                failed_approaches: Vec::new(),
                architectural_insights: Vec::new(),
            }
        });

        // Update pattern frequency
        *accumulation.patterns.entry(pattern.to_string()).or_insert(0) += 1;

        // Track success/failure
        if success {
            if !accumulation.successful_approaches.contains(&pattern.to_string()) {
                accumulation.successful_approaches.push(pattern.to_string());
            }
        } else {
            if !accumulation.failed_approaches.contains(&pattern.to_string()) {
                accumulation.failed_approaches.push(pattern.to_string());
            }
        }

        log_info!("feedback", "🏗️ Accumulated architectural knowledge: {} -> {} ({})", 
                 domain, pattern, if success { "success" } else { "failure" });

        Ok(())
    }

    /// Get architectural insights for a domain
    pub async fn get_architectural_insights(&self, domain: &str) -> Result<String> {
        let accumulations = self.context_accumulations.lock().await;
        
        if let Some(acc) = accumulations.get(domain) {
            let mut insights = String::new();
            insights.push_str(&format!("## Architectural Insights for {}\n\n", domain));
            
            if !acc.successful_approaches.is_empty() {
                insights.push_str("### Successful Approaches\n");
                for approach in &acc.successful_approaches {
                    let frequency = acc.patterns.get(approach).unwrap_or(&0);
                    insights.push_str(&format!("- {} (used {} time(s))\n", approach, frequency));
                }
                insights.push('\n');
            }
            
            if !acc.failed_approaches.is_empty() {
                insights.push_str("### Approaches to Avoid\n");
                for approach in &acc.failed_approaches {
                    insights.push_str(&format!("- {}\n", approach));
                }
                insights.push('\n');
            }
            
            Ok(insights)
        } else {
            Ok(format!("No architectural insights available for domain: {}", domain))
        }
    }

    // Helper methods
    fn is_context_relevant(&self, entry_context: &str, task_context: &str) -> bool {
        // Simple relevance check - could be more sophisticated with semantic similarity
        let entry_lower = entry_context.to_lowercase();
        let task_lower = task_context.to_lowercase();
        let entry_words: Vec<&str> = entry_lower.split_whitespace().collect();
        let task_words: Vec<&str> = task_lower.split_whitespace().collect();
        
        let common_words = entry_words.iter()
            .filter(|word| task_words.contains(word))
            .count();
            
        common_words > 0 && (common_words as f64 / task_words.len() as f64) > 0.1
    }

    fn build_iteration_context(&self, iterations: &[IterationResult]) -> String {
        let mut context = String::new();
        context.push_str("Previous iteration analysis:\n");
        
        for (i, iter) in iterations.iter().enumerate() {
            let quality = iter.quality_metrics.get("quality_score").unwrap_or(&0.0);
            context.push_str(&format!("Iteration {}: Quality score {:.2}\n", i + 1, quality));
        }
        
        if iterations.len() > 1 {
            let latest_quality = iterations.last().unwrap().quality_metrics.get("quality_score").unwrap_or(&0.0);
            let previous_quality = iterations[iterations.len()-2].quality_metrics.get("quality_score").unwrap_or(&0.0);
            let trend = if latest_quality > previous_quality { "improving" } else { "declining" };
            context.push_str(&format!("Trend: Quality is {}\n", trend));
        }
        
        context
    }

    fn calculate_quality_score(&self, output: &str, iteration: u32) -> f64 {
        // Simplified quality scoring - in practice, this could be much more sophisticated
        let base_score = 0.5;
        let length_bonus = (output.len() as f64 / 1000.0).min(0.3);
        let iteration_bonus = (iteration as f64 * 0.1).min(0.2);
        
        (base_score + length_bonus + iteration_bonus).min(1.0)
    }

    /// Get feedback statistics
    pub async fn get_feedback_stats(&self) -> Result<HashMap<String, Value>> {
        let history = self.feedback_history.lock().await;
        let mut stats = HashMap::new();
        
        stats.insert("total_entries".to_string(), json!(history.len()));
        
        // Count by feedback type
        let mut type_counts: HashMap<String, u32> = HashMap::new();
        let mut quality_scores: Vec<f64> = Vec::new();
        
        for entry in history.iter() {
            let type_key = format!("{:?}", entry.feedback_type);
            *type_counts.entry(type_key).or_insert(0) += 1;
            
            if let Some(score) = entry.quality_score {
                quality_scores.push(score);
            }
        }
        
        stats.insert("feedback_types".to_string(), serde_json::to_value(type_counts)?);
        
        if !quality_scores.is_empty() {
            let avg_quality = quality_scores.iter().sum::<f64>() / quality_scores.len() as f64;
            stats.insert("average_quality_score".to_string(), json!(avg_quality));
        }
        
        Ok(stats)
    }
}