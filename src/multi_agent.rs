// Multi-agent architecture module
pub mod agents;

// Re-export the core types and traits
pub use self::agents::*;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::logger::{log_debug, log_info, log_warn};
use crate::openrouter_client::OpenRouterClient;

/// Core trait defining agent capabilities and behavior
#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique identifier for the agent type
    fn agent_type(&self) -> &'static str;
    
    /// Human-readable description of agent capabilities
    fn description(&self) -> String;
    
    /// List of capabilities this agent can handle
    fn capabilities(&self) -> Vec<AgentCapability>;
    
    /// Analyze if this agent can handle a given task
    async fn can_handle_task(&self, task: &AgentTask) -> bool;
    
    /// Execute a task and return the result
    async fn execute_task(&self, task: &AgentTask, context: &mut AgentContext) -> Result<AgentResult>;
    
    /// Prepare the agent for task execution (setup, validation, etc.)
    async fn prepare(&self, _context: &AgentContext) -> Result<()> {
        log_debug!("multi_agent", "Agent {} prepared for execution", self.agent_type());
        Ok(())
    }
    
    /// Cleanup after task execution
    async fn cleanup(&self, _context: &AgentContext) -> Result<()> {
        log_debug!("multi_agent", "Agent {} cleaned up after execution", self.agent_type());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentCapability {
    /// File system operations (read, write, list, etc.)
    FileSystem,
    /// Code analysis, generation, and modification
    CodeProcessing,
    /// Web requests, API calls, and data fetching
    WebOperations,
    /// Task planning and coordination
    TaskOrchestration,
    /// Data processing and transformation
    DataProcessing,
    /// Search and information retrieval
    SearchOperations,
    /// Command execution and system interaction
    SystemOperations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub parameters: Value,
    pub priority: TaskPriority,
    pub required_capabilities: Vec<AgentCapability>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    FileOperation,
    CodeAnalysis,
    WebRequest,
    DataProcessing,
    SystemCommand,
    Planning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub task_id: String,
    pub agent_type: String,
    pub success: bool,
    pub result_data: Value,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
    pub sub_tasks_created: Vec<String>,
}

/// Context shared between agents during task execution
#[derive(Debug)]
pub struct AgentContext {
    pub session_id: String,
    pub shared_memory: Arc<RwLock<HashMap<String, Value>>>,
    pub task_results: Arc<RwLock<HashMap<String, AgentResult>>>,
    pub llm_client: Option<Arc<OpenRouterClient>>,
    pub working_directory: String,
}

impl AgentContext {
    pub fn new(session_id: String, working_directory: String, llm_client: Option<Arc<OpenRouterClient>>) -> Self {
        Self {
            session_id,
            shared_memory: Arc::new(RwLock::new(HashMap::new())),
            task_results: Arc::new(RwLock::new(HashMap::new())),
            llm_client,
            working_directory,
        }
    }
    
    pub async fn set_memory(&self, key: String, value: Value) {
        let mut memory = self.shared_memory.write().await;
        memory.insert(key, value);
    }
    
    pub async fn get_memory(&self, key: &str) -> Option<Value> {
        let memory = self.shared_memory.read().await;
        memory.get(key).cloned()
    }
    
    pub async fn store_result(&self, result: AgentResult) {
        let mut results = self.task_results.write().await;
        results.insert(result.task_id.clone(), result);
    }
    
    pub async fn get_result(&self, task_id: &str) -> Option<AgentResult> {
        let results = self.task_results.read().await;
        results.get(task_id).cloned()
    }
}

impl AgentTask {
    pub fn new(description: String, task_type: TaskType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description,
            task_type,
            parameters: Value::Null,
            priority: TaskPriority::Normal,
            required_capabilities: Vec::new(),
            dependencies: Vec::new(),
        }
    }
    
    pub fn with_capability(mut self, capability: AgentCapability) -> Self {
        self.required_capabilities.push(capability);
        self
    }
    
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_parameters(mut self, parameters: Value) -> Self {
        self.parameters = parameters;
        self
    }
    
    pub fn with_dependency(mut self, task_id: String) -> Self {
        self.dependencies.push(task_id);
        self
    }
}

/// Multi-agent orchestration system
pub struct MultiAgentSystem {
    agents: Vec<Arc<dyn Agent>>,
    task_queue: Arc<Mutex<Vec<AgentTask>>>,
    active_tasks: Arc<Mutex<HashMap<String, String>>>, // task_id -> agent_type
}

impl MultiAgentSystem {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            task_queue: Arc::new(Mutex::new(Vec::new())),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Create a new system with default agents
    pub fn with_default_agents(llm_client: Option<Arc<OpenRouterClient>>) -> Self {
        let mut system = Self::new();
        
        // Register default agents
        system.register_agent(Arc::new(agents::FileSystemAgent));
        system.register_agent(Arc::new(agents::CodeAgent::new(llm_client.clone())));
        system.register_agent(Arc::new(agents::WebAgent));
        
        system
    }
    
    /// Register a new agent with the system
    pub fn register_agent(&mut self, agent: Arc<dyn Agent>) {
        log_info!("multi_agent", "Registering agent: {} - {}", 
                 agent.agent_type(), agent.description());
        self.agents.push(agent);
    }
    
    /// Add a task to the execution queue
    pub async fn add_task(&self, task: AgentTask) -> Result<()> {
        log_info!("multi_agent", "Adding task {} to queue: {}", task.id, task.description);
        let mut queue = self.task_queue.lock().await;
        queue.push(task);
        // Sort by priority (higher priority first)
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }
    
    /// Find the best agent for a given task
    async fn find_best_agent(&self, task: &AgentTask) -> Option<Arc<dyn Agent>> {
        for agent in &self.agents {
            // Check if agent has required capabilities
            let agent_caps = agent.capabilities();
            let has_required_caps = task.required_capabilities.is_empty() || 
                task.required_capabilities.iter()
                    .all(|req_cap| agent_caps.contains(req_cap));
            
            if has_required_caps && agent.can_handle_task(task).await {
                log_debug!("multi_agent", "Agent {} selected for task {}", 
                          agent.agent_type(), task.id);
                return Some(agent.clone());
            }
        }
        
        log_warn!("multi_agent", "No suitable agent found for task: {}", task.description);
        None
    }
    
    /// Execute all queued tasks
    pub async fn execute_tasks(&self, context: &mut AgentContext) -> Result<Vec<AgentResult>> {
        let mut results = Vec::new();
        
        loop {
            // Get next task from queue
            let task = {
                let mut queue = self.task_queue.lock().await;
                if queue.is_empty() {
                    break;
                }
                
                // Find a task whose dependencies are satisfied
                let mut task_index = None;
                for (i, task) in queue.iter().enumerate() {
                    let dependencies_satisfied = task.dependencies.iter()
                        .all(|dep_id| {
                            let results = futures::executor::block_on(context.task_results.read());
                            results.contains_key(dep_id)
                        });
                    
                    if dependencies_satisfied {
                        task_index = Some(i);
                        break;
                    }
                }
                
                match task_index {
                    Some(i) => queue.remove(i),
                    None => {
                        log_warn!("multi_agent", "All remaining tasks have unsatisfied dependencies");
                        break;
                    }
                }
            };
            
            // Find appropriate agent
            let agent = match self.find_best_agent(&task).await {
                Some(agent) => agent,
                None => {
                    log_warn!("multi_agent", "Skipping task {}: no suitable agent", task.id);
                    continue;
                }
            };
            
            // Mark task as active
            {
                let mut active = self.active_tasks.lock().await;
                active.insert(task.id.clone(), agent.agent_type().to_string());
            }
            
            // Execute task
            log_info!("multi_agent", "Executing task {} with agent {}", task.id, agent.agent_type());
            
            let start_time = std::time::Instant::now();
            let result = match agent.prepare(context).await {
                Ok(_) => {
                    match agent.execute_task(&task, context).await {
                        Ok(mut result) => {
                            result.execution_time_ms = start_time.elapsed().as_millis() as u64;
                            agent.cleanup(context).await.ok();
                            result
                        }
                        Err(e) => {
                            agent.cleanup(context).await.ok();
                            AgentResult {
                                task_id: task.id.clone(),
                                agent_type: agent.agent_type().to_string(),
                                success: false,
                                result_data: Value::Null,
                                error_message: Some(e.to_string()),
                                execution_time_ms: start_time.elapsed().as_millis() as u64,
                                sub_tasks_created: Vec::new(),
                            }
                        }
                    }
                }
                Err(e) => {
                    AgentResult {
                        task_id: task.id.clone(),
                        agent_type: agent.agent_type().to_string(),
                        success: false,
                        result_data: Value::Null,
                        error_message: Some(format!("Agent preparation failed: {}", e)),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        sub_tasks_created: Vec::new(),
                    }
                }
            };
            
            // Store result and remove from active tasks
            context.store_result(result.clone()).await;
            {
                let mut active = self.active_tasks.lock().await;
                active.remove(&task.id);
            }
            
            results.push(result);
            
            log_info!("multi_agent", "Completed task {} in {}ms", 
                     task.id, results.last().unwrap().execution_time_ms);
        }
        
        Ok(results)
    }
    
    /// Get system status including active agents and tasks
    pub async fn get_system_status(&self) -> SystemStatus {
        let queue_size = self.task_queue.lock().await.len();
        let active_task_count = self.active_tasks.lock().await.len();
        
        let agent_info: Vec<AgentInfo> = self.agents.iter().map(|agent| {
            AgentInfo {
                agent_type: agent.agent_type().to_string(),
                description: agent.description(),
                capabilities: agent.capabilities(),
            }
        }).collect();
        
        SystemStatus {
            registered_agents: agent_info,
            queued_tasks: queue_size,
            active_tasks: active_task_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub registered_agents: Vec<AgentInfo>,
    pub queued_tasks: usize,
    pub active_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_type: String,
    pub description: String,
    pub capabilities: Vec<AgentCapability>,
}