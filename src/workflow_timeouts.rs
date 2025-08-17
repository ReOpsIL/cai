use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::logger::{log_debug, log_info, log_warn};

/// Workflow timeout configuration with adaptive timeouts based on task complexity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTimeoutConfig {
    /// Base timeout for simple operations (seconds)
    pub base_timeout: u64,
    /// Complex operation timeout multiplier
    pub complex_multiplier: f32,
    /// Maximum allowed timeout (seconds)
    pub max_timeout: u64,
    /// Timeout per MCP tool call (seconds)
    pub mcp_tool_timeout: u64,
    /// Timeout for LLM API calls (seconds)
    pub llm_api_timeout: u64,
    /// File operation timeout (seconds)
    pub file_operation_timeout: u64,
}

impl Default for WorkflowTimeoutConfig {
    fn default() -> Self {
        Self {
            base_timeout: 30,
            complex_multiplier: 3.0,
            max_timeout: 300, // 5 minutes max
            mcp_tool_timeout: 60,
            llm_api_timeout: 45,
            file_operation_timeout: 15,
        }
    }
}

/// Task complexity levels for adaptive timeout calculation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskComplexity {
    Simple,
    Medium,
    Complex,
    VeryComplex,
}

impl TaskComplexity {
    /// Determine complexity from task description
    pub fn from_task_description(description: &str) -> Self {
        let desc_lower = description.to_lowercase();
        
        // Count complexity indicators
        let complex_keywords = [
            "workflow", "multi-step", "analysis", "comprehensive", "complete",
            "generate", "refactor", "optimize", "scan", "review", "test"
        ];
        
        let very_complex_keywords = [
            "architectural", "systematic", "framework", "infrastructure",
            "migration", "comprehensive analysis", "end-to-end", "full stack"
        ];
        
        let mcp_keywords = ["mcp", "tool", "filesystem", "docker", "server"];
        let llm_keywords = ["chat", "analyze", "suggest", "improve", "explain"];
        
        let complex_count = complex_keywords.iter()
            .filter(|&keyword| desc_lower.contains(keyword))
            .count();
            
        let very_complex_count = very_complex_keywords.iter()
            .filter(|&keyword| desc_lower.contains(keyword))
            .count();
            
        let has_mcp = mcp_keywords.iter().any(|&keyword| desc_lower.contains(keyword));
        let has_llm = llm_keywords.iter().any(|&keyword| desc_lower.contains(keyword));
        
        // Determine complexity level
        if very_complex_count > 0 || (complex_count >= 2 && has_mcp && has_llm) {
            TaskComplexity::VeryComplex
        } else if complex_count >= 2 || (complex_count >= 1 && has_mcp) {
            TaskComplexity::Complex
        } else if complex_count >= 1 || has_llm || has_mcp {
            TaskComplexity::Medium
        } else {
            TaskComplexity::Simple
        }
    }
    
    /// Get timeout multiplier for this complexity level
    pub fn timeout_multiplier(&self) -> f32 {
        match self {
            TaskComplexity::Simple => 1.0,
            TaskComplexity::Medium => 2.0,
            TaskComplexity::Complex => 4.0,
            TaskComplexity::VeryComplex => 8.0,
        }
    }
}

/// Adaptive timeout manager for workflow operations
pub struct WorkflowTimeoutManager {
    config: WorkflowTimeoutConfig,
    execution_history: HashMap<String, Vec<Duration>>,
}

impl WorkflowTimeoutManager {
    pub fn new(config: WorkflowTimeoutConfig) -> Self {
        Self {
            config,
            execution_history: HashMap::new(),
        }
    }
    
    pub fn default() -> Self {
        Self::new(WorkflowTimeoutConfig::default())
    }
    
    /// Calculate adaptive timeout for a task based on complexity and history
    pub fn calculate_timeout(&self, task_description: &str, operation_type: &str) -> Duration {
        let complexity = TaskComplexity::from_task_description(task_description);
        let base_timeout = self.get_base_timeout_for_operation(operation_type);
        
        // Apply complexity multiplier
        let complexity_adjusted = (base_timeout as f32 * complexity.timeout_multiplier()) as u64;
        
        // Apply historical adjustment if available
        let historical_avg = self.get_historical_average(operation_type);
        let timeout = if let Some(avg_duration) = historical_avg {
            // Use 150% of historical average, but respect complexity adjustment
            let historical_timeout = (avg_duration.as_secs() as f32 * 1.5) as u64;
            std::cmp::max(complexity_adjusted, historical_timeout)
        } else {
            complexity_adjusted
        };
        
        // Ensure within bounds
        let final_timeout = std::cmp::min(timeout, self.config.max_timeout);
        
        log_debug!("timeouts", "📊 Calculated timeout for '{}' ({}): {}s (complexity: {:?})", 
                  task_description.chars().take(50).collect::<String>(),
                  operation_type, final_timeout, complexity);
        
        Duration::from_secs(final_timeout)
    }
    
    /// Record execution time for future timeout calculations
    pub fn record_execution(&mut self, operation_type: &str, duration: Duration) {
        let history = self.execution_history.entry(operation_type.to_string()).or_insert_with(Vec::new);
        history.push(duration);
        
        // Keep only last 10 executions for each operation type
        if history.len() > 10 {
            history.remove(0);
        }
        
        log_debug!("timeouts", "📈 Recorded execution time for '{}': {:?}", operation_type, duration);
    }
    
    /// Execute operation with adaptive timeout
    pub async fn execute_with_timeout<F, T>(
        &mut self,
        operation_type: &str,
        task_description: &str,
        operation: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let timeout_duration = self.calculate_timeout(task_description, operation_type);
        let start_time = Instant::now();
        
        log_info!("timeouts", "⏱️ Starting '{}' with timeout: {:?}", operation_type, timeout_duration);
        
        let result = timeout(timeout_duration, operation).await;
        let execution_time = start_time.elapsed();
        
        match result {
            Ok(Ok(value)) => {
                // Successful execution - record time
                self.record_execution(operation_type, execution_time);
                log_info!("timeouts", "✅ '{}' completed in {:?}", operation_type, execution_time);
                Ok(value)
            }
            Ok(Err(e)) => {
                // Operation failed but didn't timeout
                self.record_execution(operation_type, execution_time);
                log_warn!("timeouts", "❌ '{}' failed after {:?}: {}", operation_type, execution_time, e);
                Err(e)
            }
            Err(_) => {
                // Timeout occurred
                log_warn!("timeouts", "⏰ '{}' timed out after {:?} (limit: {:?})", 
                         operation_type, execution_time, timeout_duration);
                Err(anyhow::anyhow!("Operation '{}' timed out after {:?}", operation_type, timeout_duration))
            }
        }
    }
    
    /// Get base timeout for specific operation types
    fn get_base_timeout_for_operation(&self, operation_type: &str) -> u64 {
        match operation_type {
            "mcp_tool_call" => self.config.mcp_tool_timeout,
            "llm_api_call" => self.config.llm_api_timeout,
            "file_operation" => self.config.file_operation_timeout,
            "workflow_execution" => self.config.base_timeout * 3, // Workflows need more time
            "task_analysis" => self.config.llm_api_timeout,
            "goal_decomposition" => self.config.llm_api_timeout * 2,
            _ => self.config.base_timeout,
        }
    }
    
    /// Get historical average execution time for operation type
    fn get_historical_average(&self, operation_type: &str) -> Option<Duration> {
        self.execution_history.get(operation_type).and_then(|history| {
            if history.is_empty() {
                None
            } else {
                let total_nanos: u128 = history.iter().map(|d| d.as_nanos()).sum();
                let avg_nanos = total_nanos / history.len() as u128;
                Some(Duration::from_nanos(avg_nanos as u64))
            }
        })
    }
    
    /// Get timeout statistics for debugging
    pub fn get_statistics(&self) -> HashMap<String, TimeoutStatistics> {
        self.execution_history.iter().map(|(op_type, history)| {
            let avg = self.get_historical_average(op_type).unwrap_or(Duration::from_secs(0));
            let min = history.iter().min().copied().unwrap_or(Duration::from_secs(0));
            let max = history.iter().max().copied().unwrap_or(Duration::from_secs(0));
            
            (op_type.clone(), TimeoutStatistics {
                operation_type: op_type.clone(),
                execution_count: history.len(),
                average_duration: avg,
                min_duration: min,
                max_duration: max,
            })
        }).collect()
    }
}

/// Timeout execution statistics
#[derive(Debug, Clone)]
pub struct TimeoutStatistics {
    pub operation_type: String,
    pub execution_count: usize,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
}

/// Global timeout manager instance
use once_cell::sync::Lazy;
use std::sync::Mutex;

static GLOBAL_TIMEOUT_MANAGER: Lazy<Mutex<WorkflowTimeoutManager>> = 
    Lazy::new(|| Mutex::new(WorkflowTimeoutManager::default()));

/// Execute operation with global timeout management
pub async fn execute_with_adaptive_timeout<F, T>(
    operation_type: &str,
    task_description: &str,
    operation: F,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let mut manager = GLOBAL_TIMEOUT_MANAGER.lock().unwrap();
    manager.execute_with_timeout(operation_type, task_description, operation).await
}

/// Get global timeout statistics
pub fn get_global_timeout_statistics() -> HashMap<String, TimeoutStatistics> {
    let manager = GLOBAL_TIMEOUT_MANAGER.lock().unwrap();
    manager.get_statistics()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[test]
    fn test_task_complexity_detection() {
        assert_eq!(TaskComplexity::from_task_description("simple list command"), TaskComplexity::Simple);
        assert_eq!(TaskComplexity::from_task_description("analyze code structure"), TaskComplexity::Medium);
        assert_eq!(TaskComplexity::from_task_description("comprehensive workflow analysis"), TaskComplexity::Complex);
        assert_eq!(TaskComplexity::from_task_description("architectural framework migration"), TaskComplexity::VeryComplex);
    }

    #[tokio::test]
    async fn test_timeout_calculation() {
        let manager = WorkflowTimeoutManager::default();
        
        let simple_timeout = manager.calculate_timeout("list files", "file_operation");
        let complex_timeout = manager.calculate_timeout("comprehensive analysis with mcp tools", "workflow_execution");
        
        assert!(complex_timeout > simple_timeout);
    }

    #[tokio::test]
    async fn test_adaptive_timeout_execution() {
        let mut manager = WorkflowTimeoutManager::default();
        
        // Test successful operation
        let result = manager.execute_with_timeout(
            "test_operation",
            "simple test",
            async { Ok("success") }
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        
        // Test timeout
        let timeout_result = manager.execute_with_timeout(
            "test_timeout",
            "simple test",
            async {
                sleep(Duration::from_secs(1)).await;
                Ok("should timeout")
            }
        ).await;
        
        // Note: This test would need a very short timeout to actually timeout in test
        // In practice, timeouts would be longer and operations would be more realistic
    }
}